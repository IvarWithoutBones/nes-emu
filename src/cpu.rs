use bitflags::bitflags;

/// See https://www.nesdev.org/wiki/CPU_addressing_modes
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
    accumulator: u8,
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
    const BITS_IN_BYTE: u8 = 8;

    pub fn new() -> CPU {
        CPU {
            program_counter: 0,
            status: CpuFlags::empty(),
            memory: [0; CPU::RAM_SIZE],
            accumulator: 0,
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
        self.accumulator = 0;
        self.register_x = 0;
        self.program_counter = self.read_word(CPU::INITIAL_PROGRAM_COUNTER);
    }

    fn param_from_adressing_mode(&self, mode: &AdressingMode) -> (u16, u16) {
        let addr = match mode {
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
        };

        (addr, self.consume_params(mode))
    }

    fn consume_params(&self, mode: &AdressingMode) -> u16 {
        match mode {
            AdressingMode::NoneAddressing => panic!("no addressing mode, this should never occur"),

            AdressingMode::Immediate
            | AdressingMode::IndirectX
            | AdressingMode::IndirectY
            | AdressingMode::ZeroPage
            | AdressingMode::ZeroPageX
            | AdressingMode::ZeroPageY => self.program_counter + 1,

            AdressingMode::Absolute | AdressingMode::AbsoluteX | AdressingMode::AbsoluteY => {
                self.program_counter + 2
            }
        }
    }

    pub fn run(&mut self) {
        self.reset();

        loop {
            let opcode = self.read_byte(self.program_counter);
            self.program_counter += 1;

            match opcode {
                0xEA => continue, // NOP
                0x00 => return,   // BRK

                0xAA => self.tax(),
                0xB8 => self.clv(),
                0x58 => self.cli(),
                0xD8 => self.cld(),
                0x18 => self.clc(),

                0xCA => self.dex(),
                0x88 => self.dey(),

                0xC6 => self.dec(&AdressingMode::ZeroPage),
                0xD6 => self.dec(&AdressingMode::ZeroPageX),
                0xCE => self.dec(&AdressingMode::Absolute),
                0xDE => self.dec(&AdressingMode::AbsoluteX),

                0xE8 => self.inx(),
                0xC8 => self.iny(),

                0xE6 => self.inc(&AdressingMode::ZeroPage),
                0xF6 => self.inc(&AdressingMode::ZeroPageX),
                0xEE => self.inc(&AdressingMode::Absolute),
                0xFE => self.inc(&AdressingMode::AbsoluteX),

                0x4A => self.lsr(&None), // Accumulator mode
                0x46 => self.lsr(&Some(AdressingMode::ZeroPage)),
                0x56 => self.lsr(&Some(AdressingMode::ZeroPageX)),
                0x4E => self.lsr(&Some(AdressingMode::Absolute)),
                0x5E => self.lsr(&Some(AdressingMode::AbsoluteX)),

                0x0A => self.asl(&None), // Accumulator mode
                0x06 => self.asl(&Some(AdressingMode::ZeroPage)),
                0x16 => self.asl(&Some(AdressingMode::ZeroPageX)),
                0x0E => self.asl(&Some(AdressingMode::Absolute)),
                0x1E => self.asl(&Some(AdressingMode::AbsoluteX)),

                0x6A => self.ror(&None), // Accumulator mode
                0x66 => self.ror(&Some(AdressingMode::ZeroPage)),
                0x76 => self.ror(&Some(AdressingMode::ZeroPageX)),
                0x6E => self.ror(&Some(AdressingMode::Absolute)),
                0x7E => self.ror(&Some(AdressingMode::AbsoluteX)),

                0x2A => self.rol(&None), // Accumulator mode
                0x26 => self.rol(&Some(AdressingMode::ZeroPage)),
                0x36 => self.rol(&Some(AdressingMode::ZeroPageX)),
                0x2E => self.rol(&Some(AdressingMode::Absolute)),
                0x3E => self.rol(&Some(AdressingMode::AbsoluteX)),

                0x29 => self.and(&AdressingMode::Immediate),
                0x25 => self.and(&AdressingMode::ZeroPage),
                0x35 => self.and(&AdressingMode::ZeroPageX),
                0x2D => self.and(&AdressingMode::Absolute),
                0x3D => self.and(&AdressingMode::AbsoluteX),
                0x39 => self.and(&AdressingMode::AbsoluteY),
                0x21 => self.and(&AdressingMode::IndirectX),
                0x31 => self.and(&AdressingMode::IndirectY),

                0x49 => self.eor(&AdressingMode::Immediate),
                0x45 => self.eor(&AdressingMode::ZeroPage),
                0x55 => self.eor(&AdressingMode::ZeroPageX),
                0x4D => self.eor(&AdressingMode::Absolute),
                0x5D => self.eor(&AdressingMode::AbsoluteX),
                0x59 => self.eor(&AdressingMode::AbsoluteY),
                0x41 => self.eor(&AdressingMode::IndirectX),
                0x51 => self.eor(&AdressingMode::IndirectY),

                0x09 => self.ora(&AdressingMode::Immediate),
                0x05 => self.ora(&AdressingMode::ZeroPage),
                0x15 => self.ora(&AdressingMode::ZeroPageX),
                0x0D => self.ora(&AdressingMode::Absolute),
                0x1D => self.ora(&AdressingMode::AbsoluteX),
                0x19 => self.ora(&AdressingMode::AbsoluteY),
                0x01 => self.ora(&AdressingMode::IndirectX),
                0x11 => self.ora(&AdressingMode::IndirectY),

                0xC9 => self.cmp(&AdressingMode::Immediate),
                0xC5 => self.cmp(&AdressingMode::ZeroPage),
                0xD5 => self.cmp(&AdressingMode::ZeroPageX),
                0xCD => self.cmp(&AdressingMode::Absolute),
                0xDD => self.cmp(&AdressingMode::AbsoluteX),
                0xD9 => self.cmp(&AdressingMode::AbsoluteY),
                0xC1 => self.cmp(&AdressingMode::IndirectX),
                0xD1 => self.cmp(&AdressingMode::IndirectY),

                0xE0 => self.cpx(&AdressingMode::Immediate),
                0xE4 => self.cpx(&AdressingMode::ZeroPage),
                0xEC => self.cpx(&AdressingMode::Absolute),

                0xC0 => self.cpy(&AdressingMode::Immediate),
                0xC4 => self.cpy(&AdressingMode::ZeroPage),
                0xCC => self.cpy(&AdressingMode::Absolute),

                0xA9 => self.lda(&AdressingMode::Immediate),
                0xA5 => self.lda(&AdressingMode::ZeroPage),
                0xB5 => self.lda(&AdressingMode::ZeroPageX),
                0xAD => self.lda(&AdressingMode::Absolute),
                0xBD => self.lda(&AdressingMode::AbsoluteX),
                0xB9 => self.lda(&AdressingMode::AbsoluteY),
                0xA1 => self.lda(&AdressingMode::IndirectX),
                0xB1 => self.lda(&AdressingMode::IndirectY),

                0xA2 => self.ldx(&AdressingMode::Immediate),
                0xA6 => self.ldx(&AdressingMode::ZeroPage),
                0xB6 => self.ldx(&AdressingMode::ZeroPageY),
                0xAE => self.ldx(&AdressingMode::Absolute),
                0xBE => self.ldx(&AdressingMode::AbsoluteY),

                0xA0 => self.ldy(&AdressingMode::Immediate),
                0xA4 => self.ldy(&AdressingMode::ZeroPage),
                0xB4 => self.ldy(&AdressingMode::ZeroPageX),
                0xAC => self.ldy(&AdressingMode::Absolute),
                0xBC => self.ldy(&AdressingMode::AbsoluteX),

                _ => todo!("opcode {:02x} not implemented", opcode),
            }
        }
    }

    /*
      Helpers
    */

    const fn nth_bit(value: u8, n: u8) -> bool {
        value & (1 << n) != 0
    }

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

    fn update_negative_flag(&mut self, value: u8) {
        if CPU::nth_bit(value, 7) {
            self.status.insert(CpuFlags::NEGATIVE);
        } else {
            self.status.remove(CpuFlags::NEGATIVE);
        }
    }

    fn update_zero_and_negative_flags(&mut self, value: u8) {
        self.update_negative_flag(value);

        if value == 0 {
            self.status.insert(CpuFlags::ZERO);
        } else {
            self.status.remove(CpuFlags::ZERO);
        }
    }

    /*
      Opcodes
    */

    fn cmp(&mut self, mode: &AdressingMode) {
        let (addr, new_pc) = self.param_from_adressing_mode(mode);
        let value = self.read_byte(addr);

        if self.accumulator == value {
            self.status.insert(CpuFlags::ZERO);
        } else {
            self.status.remove(CpuFlags::ZERO);
        }

        if self.accumulator >= value {
            self.status.insert(CpuFlags::CARRY);
        } else {
            self.status.remove(CpuFlags::CARRY);
        }

        self.update_negative_flag(value);
        self.program_counter = new_pc;
    }

    fn cpx(&mut self, mode: &AdressingMode) {
        let (addr, new_pc) = self.param_from_adressing_mode(mode);
        let value = self.read_byte(addr);

        if self.register_x == value {
            self.status.insert(CpuFlags::ZERO);
        } else {
            self.status.remove(CpuFlags::ZERO);
        }

        if self.register_x >= value {
            self.status.insert(CpuFlags::CARRY);
        } else {
            self.status.remove(CpuFlags::CARRY);
        }

        self.update_negative_flag(value);
        self.program_counter = new_pc;
    }

    fn cpy(&mut self, mode: &AdressingMode) {
        let (addr, new_pc) = self.param_from_adressing_mode(mode);
        let value = self.read_byte(addr);

        if self.register_y == value {
            self.status.insert(CpuFlags::ZERO);
        } else {
            self.status.remove(CpuFlags::ZERO);
        }

        if self.register_y >= value {
            self.status.insert(CpuFlags::CARRY);
        } else {
            self.status.remove(CpuFlags::CARRY);
        }

        self.update_negative_flag(value);
        self.program_counter = new_pc;
    }

    fn lda(&mut self, mode: &AdressingMode) {
        let (addr, new_pc) = self.param_from_adressing_mode(mode);
        let value = self.read_byte(addr);

        self.accumulator = value;
        self.update_zero_and_negative_flags(value);
        self.program_counter = new_pc;
    }

    fn ldx(&mut self, mode: &AdressingMode) {
        let (addr, new_pc) = self.param_from_adressing_mode(mode);
        let value = self.read_byte(addr);

        self.register_x = value;
        self.update_zero_and_negative_flags(value);
        self.program_counter = new_pc;
    }

    fn ldy(&mut self, mode: &AdressingMode) {
        let (addr, new_pc) = self.param_from_adressing_mode(mode);
        let value = self.read_byte(addr);

        self.register_y = value;
        self.update_zero_and_negative_flags(value);
        self.program_counter = new_pc;
    }

    fn inc(&mut self, mode: &AdressingMode) {
        let (addr, new_pc) = self.param_from_adressing_mode(mode);
        let value = self.read_byte(addr).wrapping_add(1); // Should this be a word?

        self.write_byte(addr, value);
        self.update_zero_and_negative_flags(value);
        self.program_counter = new_pc;
    }

    fn inx(&mut self) {
        self.register_x = self.register_x.wrapping_add(1);
        self.update_zero_and_negative_flags(self.register_x)
    }

    fn iny(&mut self) {
        self.register_y = self.register_y.wrapping_add(1);
        self.update_zero_and_negative_flags(self.register_y)
    }

    fn dec(&mut self, mode: &AdressingMode) {
        let (addr, new_pc) = self.param_from_adressing_mode(mode);
        let value = self.read_byte(addr).wrapping_sub(1); // Should this be a word?

        self.write_byte(addr, value);
        self.update_zero_and_negative_flags(value);
        self.program_counter = new_pc;
    }

    fn rol(&mut self, mode: &Option<AdressingMode>) {
        let value: u8;
        let new_pc: u16;

        // TODO: there is probably a nicer way of handling this?
        if mode.is_some() {
            let (addr, pc) = self.param_from_adressing_mode(&mode.as_ref().unwrap());
            value = self.read_byte(addr);
            new_pc = pc;
        } else {
            value = self.accumulator;
            new_pc = self.program_counter;
        }

        if CPU::nth_bit(value, 7) {
            self.status.insert(CpuFlags::CARRY);
        } else {
            self.status.remove(CpuFlags::CARRY);
        }

        let result = (value << 1) | (value >> (CPU::BITS_IN_BYTE - 1));
        self.update_zero_and_negative_flags(result);
        self.program_counter = new_pc;
    }

    fn ror(&mut self, mode: &Option<AdressingMode>) {
        let value: u8;
        let new_pc: u16;

        // TODO: there is probably a nicer way of handling this?
        if mode.is_some() {
            let (addr, pc) = self.param_from_adressing_mode(&mode.as_ref().unwrap());
            value = self.read_byte(addr);
            new_pc = pc;
        } else {
            value = self.accumulator;
            new_pc = self.program_counter;
        }

        if CPU::nth_bit(value, 0) {
            self.status.insert(CpuFlags::CARRY);
        } else {
            self.status.remove(CpuFlags::CARRY);
        }

        let result = (value >> 1) | (value << (CPU::BITS_IN_BYTE - 1));
        self.update_zero_and_negative_flags(result);
        self.program_counter = new_pc;
    }

    fn asl(&mut self, mode: &Option<AdressingMode>) {
        let value: u8;
        let new_pc: u16;

        // TODO: there is probably a nicer way of handling this?
        if mode.is_some() {
            let (addr, pc) = self.param_from_adressing_mode(&mode.as_ref().unwrap());
            value = self.read_byte(addr);
            new_pc = pc;
        } else {
            value = self.accumulator;
            new_pc = self.program_counter;
        }

        if CPU::nth_bit(value, 7) {
            self.status.insert(CpuFlags::CARRY);
        } else {
            self.status.remove(CpuFlags::CARRY);
        }

        let result = value << 1;
        self.update_zero_and_negative_flags(result);
        self.program_counter = new_pc;
    }

    fn lsr(&mut self, mode: &Option<AdressingMode>) {
        let value: u8;
        let new_pc: u16;

        // TODO: there is probably a nicer way of handling this?
        if mode.is_some() {
            let (addr, pc) = self.param_from_adressing_mode(&mode.as_ref().unwrap());
            value = self.read_byte(addr);
            new_pc = pc;
        } else {
            value = self.accumulator;
            // This is a hack to only consume 1 byte
            new_pc = self.consume_params(&AdressingMode::Immediate)
        }

        if CPU::nth_bit(value, 0) {
            self.status.insert(CpuFlags::CARRY);
        } else {
            self.status.remove(CpuFlags::CARRY);
        }

        let result = value >> 1;
        self.update_zero_and_negative_flags(result);
        self.program_counter = new_pc;
    }

    fn and(&mut self, mode: &AdressingMode) {
        let (addr, new_pc) = self.param_from_adressing_mode(&mode);
        let value = self.read_byte(addr);

        self.accumulator &= value;
        self.update_zero_and_negative_flags(self.accumulator);
        self.program_counter = new_pc;
    }

    fn eor(&mut self, mode: &AdressingMode) {
        let (addr, new_pc) = self.param_from_adressing_mode(&mode);
        let value = self.read_byte(addr);

        self.accumulator ^= value;
        self.update_zero_and_negative_flags(self.accumulator);
        self.program_counter = new_pc;
    }

    fn ora(&mut self, mode: &AdressingMode) {
        let (addr, new_pc) = self.param_from_adressing_mode(&mode);
        let value = self.read_byte(addr);

        self.accumulator |= value;
        self.update_zero_and_negative_flags(self.accumulator);
        self.program_counter = new_pc;
    }

    fn dex(&mut self) {
        self.register_x = self.register_x.wrapping_sub(1);
        self.update_zero_and_negative_flags(self.register_x)
    }

    fn dey(&mut self) {
        self.register_y = self.register_y.wrapping_sub(1);
        self.update_zero_and_negative_flags(self.register_y)
    }

    fn tax(&mut self) {
        self.register_x = self.accumulator;
        self.update_zero_and_negative_flags(self.register_x)
    }

    fn clv(&mut self) {
        self.status.remove(CpuFlags::OVERFLOW);
    }

    fn cli(&mut self) {
        self.status.remove(CpuFlags::IRQ);
    }

    fn clc(&mut self) {
        self.status.remove(CpuFlags::CARRY);
    }

    fn cld(&mut self) {
        // TODO: bootup sequence related stuff?
        self.status.remove(CpuFlags::DECIMAL);
    }
}

#[cfg(test)]
mod test {
    use super::*;

    macro_rules! test_cpu {
        ($test_name: ident, $asm:expr, $callback:expr) => {
            #[test]
            fn $test_name() {
                let mut cpu = CPU::new();
                let mut assembly = $asm.to_vec();
                assembly.push(0x00); // Terminating instruction
                cpu.load_program(assembly);
                cpu.run();
                $callback(cpu);
            }
        };

        ($test_name: ident, $asm:expr, $dont_execute:expr, $callback:expr) => {
            #[test]
            fn $test_name() {
                let mut cpu = CPU::new();
                let mut assembly = $asm.to_vec();
                assembly.push(0x00); // Terminating instruction
                cpu.load_program(assembly);
                $callback(&mut cpu);
            }
        };

        ($test_name: ident, $callback:expr) => {
            #[test]
            fn $test_name() {
                let mut cpu = CPU::new();
                $callback(&mut cpu);
            }
        };
    }

    test_cpu!(
        test_lda_from_memory,
        [0xa5, 0x10 /* LDA, 0x10 */],
        true,
        |cpu: &mut CPU| {
            cpu.write_byte(0x10, 0x55);
            cpu.run();
            assert_eq!(cpu.accumulator, 0x55);
        }
    );

    test_cpu!(
        test_inx,
        [0xa2, 5, /* LDX, 5 */ 0xe8 /* INX */],
        |cpu: CPU| {
            assert_eq!(cpu.register_x, 6);
        }
    );

    test_cpu!(
        test_tax,
        [0xa9, 5, /* LDA, 5 */ 0xaa /* TAX */],
        |cpu: CPU| {
            assert_eq!(cpu.register_x, 5);
        }
    );

    test_cpu!(
        test_inx_overflow,
        [0xa2, 0xff, /* LDX, 5 */ 0xe8, /* TAX */ 0xe8 /* TAX */],
        |cpu: CPU| { assert_eq!(cpu.register_x, 1) }
    );

    test_cpu!(
        test_multiple_ops,
        [0xa9, 0xc0, /* LDA, 0xc0 */ 0xaa, /* TAX */ 0xe8 /* TAX */],
        |cpu: CPU| { assert_eq!(cpu.register_x, 0xc1) }
    );

    test_cpu!(
        test_dec,
        [0xc6, 0x10 /* DEC, 0x10 */],
        true,
        |cpu: &mut CPU| {
            cpu.write_byte(0x10, 0x1);
            cpu.run();
            assert_eq!(cpu.read_byte(0x10), 0);
            assert!(cpu.status.contains(CpuFlags::ZERO));
        }
    );

    test_cpu!(test_dey, [0x88 /* DEY */], true, |cpu: &mut CPU| {
        cpu.register_y = 10;
        cpu.run();
        assert_eq!(cpu.register_y, 9);
    });

    test_cpu!(test_lda_immediate, [0xa9, 5 /* LDA, 5 */], |cpu: CPU| {
        assert_eq!(cpu.accumulator, 5);
        assert_eq!(cpu.status, CpuFlags::empty());
    });

    test_cpu!(test_ldy_immediate, [0xa0, 5 /* LDY, 5 */], |cpu: CPU| {
        assert_eq!(cpu.register_y, 5);
        assert_eq!(cpu.status, CpuFlags::empty());
    });

    test_cpu!(test_ldx_immediate, [0xa2, 5 /* LDX, 5 */], |cpu: CPU| {
        assert_eq!(cpu.register_x, 5);
        assert_eq!(cpu.status, CpuFlags::empty());
    });

    test_cpu!(test_cpx_immediate, [0xe0, 5 /* CPX, 5 */], |cpu: CPU| {
        // TODO: This doesnt feel right
        assert_eq!(cpu.status, CpuFlags::empty());
    });

    test_cpu!(
        test_cpx_immediate_zero,
        [0xe0, 0 /* CPX, 0 */],
        |cpu: CPU| {
            assert!(cpu.status.contains(CpuFlags::ZERO));
        }
    );

    test_cpu!(test_read_write_word, |cpu: &mut CPU| {
        cpu.write_word(0x10, 0x1234);
        assert_eq!(cpu.read_word(0x10), 0x1234);
        cpu.write_word(0xfff, 0x5422);
        assert_eq!(cpu.read_word(0xfff), 0x5422);
    });

    test_cpu!(test_read_write_byte, |cpu: &mut CPU| {
        cpu.write_byte(0x10, 0x12);
        assert_eq!(cpu.read_byte(0x10), 0x12);
    });

    test_cpu!(test_update_negative, |cpu: &mut CPU| {
        cpu.update_negative_flag(0b1000_0000);
        assert!(cpu.status.contains(CpuFlags::NEGATIVE));
        cpu.update_negative_flag(0b0111_1111);
        assert!(!cpu.status.contains(CpuFlags::NEGATIVE));
    });

    test_cpu!(test_update_zero, |cpu: &mut CPU| {
        cpu.update_zero_and_negative_flags(0);
        assert!(cpu.status.contains(CpuFlags::ZERO));
        cpu.update_zero_and_negative_flags(1);
        assert!(!cpu.status.contains(CpuFlags::ZERO));
    });

    #[test]
    fn test_nth_bit() {
        let value = 0b1010_1010;
        assert_eq!(CPU::nth_bit(value, 0), false);
        assert_eq!(CPU::nth_bit(value, 1), true);
        assert_eq!(CPU::nth_bit(value, 2), false);
        assert_eq!(CPU::nth_bit(value, 3), true);
        assert_eq!(CPU::nth_bit(value, 4), false);
        assert_eq!(CPU::nth_bit(value, 5), true);
        assert_eq!(CPU::nth_bit(value, 6), false);
        assert_eq!(CPU::nth_bit(value, 7), true);
    }
}
