use bitflags::bitflags;

/// See https://www.nesdev.org/wiki/CPU_addressing_modes
#[allow(dead_code)]
enum AdressingMode {
    Immediate,
    NoneAddressing,

    IndirectX,
    IndirectY,

    Absolute,
    AbsoluteX,
    AbsoluteY,

    ZeroPage,
    ZeroPageX,
    ZeroPageY,
}

bitflags! {
    /*
        7  bit  0
        ---- ----
        NVss DIZC
        |||| ||||
        |||| |||+- Carry
        |||| ||+-- Zero
        |||| |+--- Interrupt Disable
        |||| +---- Decimal
        ||++------ Break
        |+-------- Overflow
        +--------- Negative
    */
    #[rustfmt::skip]
    #[derive(Debug, PartialEq)]
    struct CpuFlags: u8 {
        const CARRY =    0b0000_0001;
        const ZERO =     0b0000_0010;
        const IRQ =      0b0000_0100;
        const DECIMAL =  0b0000_1000;
        const BREAK =    0b0001_0000;
        const BREAK2 =   0b0010_0000;
        const OVERFLOW = 0b0100_0000;
        const NEGATIVE = 0b1000_0000;
    }
}

/// See https://www.nesdev.org/obelisk-6502-guide/reference.html
pub struct CPU {
    register_a: u8,
    register_x: u8,
    register_y: u8,
    program_counter: u16,
    status: CpuFlags,
    memory: [u8; CPU::RAM_SIZE],
}

impl CPU {
    const RAM_SIZE: usize = 0xFFFF;
    const PROGRAM_ROM_START: u16 = 0x8000;
    const INITIAL_PROGRAM_COUNTER: u16 = 0xFFFC;

    pub fn new() -> CPU {
        CPU {
            program_counter: 0,
            status: CpuFlags::empty(),
            memory: [0; CPU::RAM_SIZE],
            register_a: 0,
            register_x: 0,
            register_y: 0,
        }
    }

    pub fn load_program(&mut self, program: Vec<u8>) {
        self.program_counter = CPU::PROGRAM_ROM_START;
        self.memory[CPU::PROGRAM_ROM_START as usize
            ..(CPU::PROGRAM_ROM_START + program.len() as u16) as usize]
            .copy_from_slice(&program[..]);
        self.write_word(CPU::INITIAL_PROGRAM_COUNTER, CPU::PROGRAM_ROM_START);
    }

    fn reset(&mut self) {
        self.status = CpuFlags::empty();
        self.register_a = 0;
        self.register_x = 0;
        self.program_counter = self.read_word(CPU::INITIAL_PROGRAM_COUNTER);
    }

    fn param_from_adressing_mode(&self, mode: &AdressingMode) -> u16 {
        match mode {
            AdressingMode::NoneAddressing => panic!("no addressing mode, this should never occur"),
            AdressingMode::Immediate => self.program_counter,
            AdressingMode::Absolute => self.read_word(self.program_counter),
            AdressingMode::ZeroPage => self.read_byte(self.program_counter) as u16,

            AdressingMode::ZeroPageX => self
                .read_byte(self.program_counter)
                .wrapping_add(self.register_x) as u16,

            AdressingMode::ZeroPageY => self
                .read_byte(self.program_counter)
                .wrapping_add(self.register_y) as u16,

            AdressingMode::AbsoluteX => self
                .read_word(self.program_counter)
                .wrapping_add(self.register_x as u16),

            AdressingMode::AbsoluteY => self
                .read_word(self.program_counter)
                .wrapping_add(self.register_y as u16),

            AdressingMode::IndirectX => {
                let ptr: u8 = self
                    .read_byte(self.program_counter)
                    .wrapping_add(self.register_x);

                u16::from_le_bytes([
                    self.read_byte(ptr as u16),
                    self.read_byte((ptr as u16).wrapping_add(1)),
                ])
            }

            AdressingMode::IndirectY => {
                let ptr: u8 = self.read_byte(self.program_counter);

                u16::from_le_bytes([
                    self.read_byte(ptr as u16),
                    self.read_byte((ptr as u16).wrapping_add(1)),
                ])
                .wrapping_add(self.register_y as u16)
            }
        }
    }

    pub fn run(&mut self) {
        self.reset();

        loop {
            let opcode = self.read_byte(self.program_counter);
            self.program_counter += 1;

            match opcode {
                0x0 => return, // BRK
                0xAA => self.tax(),
                0xE8 => self.inx(),

                // LDA
                0xA9 => {
                    self.lda(&AdressingMode::Immediate);
                    self.program_counter += 1
                }
                0xA5 => {
                    self.lda(&AdressingMode::ZeroPage);
                    self.program_counter += 1
                }
                0xB5 => {
                    self.lda(&AdressingMode::ZeroPageX);
                    self.program_counter += 1
                }
                0xAD => {
                    self.lda(&AdressingMode::Absolute);
                    self.program_counter += 2
                }
                0xBD => {
                    self.lda(&AdressingMode::AbsoluteX);
                    self.program_counter += 2
                }
                0xB9 => {
                    self.lda(&AdressingMode::AbsoluteY);
                    self.program_counter += 2
                }
                0xA1 => {
                    self.lda(&AdressingMode::IndirectX);
                    self.program_counter += 1
                }
                0xB1 => {
                    self.lda(&AdressingMode::IndirectY);
                    self.program_counter += 1
                }

                // LDX
                0xA2 => {
                    self.ldx(&AdressingMode::Immediate);
                    self.program_counter += 1
                }

                _ => todo!("opcode {:02x} not implemented", opcode),
            }
        }
    }

    /*
      Helpers
    */

    fn read_byte(&self, address: u16) -> u8 {
        self.memory[address as usize]
    }

    fn write_byte(&mut self, address: u16, data: u8) {
        self.memory[address as usize] = data
    }

    fn read_word(&self, address: u16) -> u16 {
        u16::from_le_bytes([self.read_byte(address), self.read_byte(address + 1)])
    }

    fn write_word(&mut self, address: u16, data: u16) {
        self.write_byte(address, (data & 0xff) as u8);
        self.write_byte(address + 1, (data >> 8) as u8);
    }

    fn update_zero_and_negative_flags(&mut self, value: u8) {
        if value == 0 {
            self.status.insert(CpuFlags::ZERO);
        } else {
            self.status.remove(CpuFlags::ZERO);
        }

        if value & 0b1000_0000 != 0 {
            self.status.insert(CpuFlags::NEGATIVE);
        } else {
            self.status.remove(CpuFlags::NEGATIVE);
        }
    }

    /*
      Opcodes
    */

    fn lda(&mut self, mode: &AdressingMode) {
        let value = self.read_byte(self.param_from_adressing_mode(mode));
        self.register_a = value;
        self.update_zero_and_negative_flags(self.register_a)
    }

    fn ldx(&mut self, mode: &AdressingMode) {
        let value = self.read_byte(self.param_from_adressing_mode(mode));
        self.register_x = value;
        self.update_zero_and_negative_flags(self.register_x)
    }

    fn tax(&mut self) {
        self.register_x = self.register_a;
        self.update_zero_and_negative_flags(self.register_x)
    }

    fn inx(&mut self) {
        self.register_x = self.register_x.wrapping_add(1);
        self.update_zero_and_negative_flags(self.register_x)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    macro_rules! test_zero_flag {
        ($test_name:ident, $test_op:expr, $register:ident) => {
            #[test]
            fn $test_name() {
                let mut cpu = CPU::new();
                cpu.load_program(vec![$test_op, 0x00, 0x00]);
                cpu.run();
                assert_eq!(cpu.$register, 0x00);
                assert!(cpu.status.contains(CpuFlags::ZERO));
            }
        };
    }

    test_zero_flag!(test_lda_zero_flag, 0xa9, register_a);
    test_zero_flag!(test_ldx_zero_flag, 0xa2, register_x);
    test_zero_flag!(test_tax_zero_flag, 0xaa, register_x);

    #[test]
    fn test_tax() {
        let mut cpu = CPU::new();
        cpu.load_program(vec![0xA9, 0x42, 0xaa, 0x00]);
        cpu.run();
        assert_eq!(cpu.register_x, 0x42)
    }

    #[test]
    fn test_lda_immediate() {
        let mut cpu = CPU::new();
        cpu.load_program(vec![0xA9, 0x42, 0x00]);
        cpu.run();
        assert_eq!(cpu.register_a, 0x42);
        assert_eq!(cpu.status, CpuFlags::empty());
    }

    #[test]
    fn test_ldx_immediate() {
        let mut cpu = CPU::new();
        cpu.load_program(vec![0xA2, 0x32, 0x00]);
        cpu.run();
        assert_eq!(cpu.register_x, 0x32);
        assert_eq!(cpu.status, CpuFlags::empty());
    }

    #[test]
    fn test_5_ops_working_together() {
        let mut cpu = CPU::new();
        cpu.load_program(vec![0xa9, 0xc0, 0xaa, 0xe8, 0x00]);
        cpu.run();
        assert_eq!(cpu.register_x, 0xc1)
    }

    #[test]
    fn test_inx_overflow() {
        let mut cpu = CPU::new();
        cpu.register_x = 0xff;
        cpu.load_program(vec![0xe8, 0x00]);
        cpu.run();
        assert_eq!(cpu.register_x, 1)
    }

    #[test]
    fn test_lda_from_memory() {
        let mut cpu = CPU::new();
        cpu.write_byte(0x10, 0x55);
        cpu.load_program(vec![0xa5, 0x10, 0x00]);
        cpu.run();
        assert_eq!(cpu.register_a, 0x55);
    }
}
