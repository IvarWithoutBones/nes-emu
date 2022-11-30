use bitflags::bitflags;

/// See https://www.nesdev.org/wiki/CPU_addressing_modes
#[derive(Debug)]
enum AdressingMode {
    Immediate,

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
    #[derive(Debug, Clone, PartialEq)]
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
    stack_pointer: u8,
    status: CpuFlags,
    memory: [u8; CPU::RAM_SIZE],
}

impl CPU {
    const RAM_SIZE: usize = 0xFFFF;
    const PROGRAM_ROM_START: u16 = 0x8000;
    const STACK_OFFSET: u16 = 0x0100;
    const STACK_RESET: u8 = 0xFD;

    pub fn new() -> CPU {
        CPU {
            program_counter: 0,
            stack_pointer: CPU::STACK_RESET,
            status: CpuFlags::empty(),
            memory: [0; CPU::RAM_SIZE],
            accumulator: 0,
            register_x: 0,
            register_y: 0,
        }
    }

    pub fn load_program(&mut self, program: Vec<u8>) {
        self.reset();
        self.memory[CPU::PROGRAM_ROM_START as usize
            ..(CPU::PROGRAM_ROM_START + program.len() as u16) as usize]
            .copy_from_slice(&program[..]);
    }

    fn reset(&mut self) {
        self.status = CpuFlags::empty();
        self.accumulator = 0;
        self.register_x = 0;
        self.register_y = 0;
        self.program_counter = CPU::PROGRAM_ROM_START;
        self.memory = [0; CPU::RAM_SIZE];
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

    #[allow(dead_code)] // Used in unit tests
    fn write_word(&mut self, address: u16, data: u16) {
        self.memory[address as usize..(address + 2) as usize]
            .copy_from_slice(u16::to_le_bytes(data).as_ref());
    }

    fn stack_push_byte(&mut self, data: u8) {
        self.write_byte(CPU::STACK_OFFSET + self.stack_pointer as u16, data);
        self.stack_pointer = self.stack_pointer.wrapping_sub(1);
    }

    fn stack_pop_byte(&mut self) -> u8 {
        self.stack_pointer = self.stack_pointer.wrapping_add(1);
        self.read_byte(CPU::STACK_OFFSET + self.stack_pointer as u16)
    }

    fn stack_push_word(&mut self, data: u16) {
        let bytes = u16::to_le_bytes(data);
        self.stack_push_byte(bytes[0]);
        self.stack_push_byte(bytes[1]);
    }

    fn stack_pop_word(&mut self) -> u16 {
        u16::from_le_bytes([self.stack_pop_byte(), self.stack_pop_byte()])
    }

    fn update_negative_flag(&mut self, value: u8) {
        self.status.set(CpuFlags::NEGATIVE, CPU::nth_bit(value, 7));
    }

    fn update_zero_and_negative_flags(&mut self, value: u8) {
        self.update_negative_flag(value);
        self.status.set(CpuFlags::ZERO, value == 0);
    }

    fn param_from_adressing_mode(&self, mode: &AdressingMode) -> (u16, u16) {
        let addr = match mode {
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

    /*
      Opcodes
    */

    fn rti(&mut self) {
        self.status = CpuFlags::from_bits_truncate(self.stack_pop_byte());
        self.program_counter = self.stack_pop_word();
    }

    fn adc(&mut self, mode: &AdressingMode) {
        let (addr, next_pc) = self.param_from_adressing_mode(mode);
        let value = self.read_byte(addr);

        let result = value
            .wrapping_add(self.accumulator)
            .wrapping_add(self.status.contains(CpuFlags::CARRY) as u8);

        self.status.set(CpuFlags::CARRY, result < value);
        self.status.set(
            CpuFlags::OVERFLOW,
            CPU::nth_bit(self.accumulator, 7) == CPU::nth_bit(value, 7)
                && CPU::nth_bit(self.accumulator, 7) != CPU::nth_bit(result, 7),
        );

        self.program_counter = next_pc;
    }

    fn sdc(&mut self, mode: &AdressingMode) {
        let (addr, next_pc) = self.param_from_adressing_mode(mode);
        let value = self.read_byte(addr);

        let result = value
            .wrapping_sub(self.accumulator)
            .wrapping_sub(self.status.contains(CpuFlags::CARRY) as u8);

        self.status.set(CpuFlags::CARRY, result > value);
        self.status.set(
            CpuFlags::OVERFLOW,
            CPU::nth_bit(self.accumulator, 7) == CPU::nth_bit(value, 7)
                && CPU::nth_bit(self.accumulator, 7) != CPU::nth_bit(result, 7),
        );

        self.program_counter = next_pc;
    }

    fn branch(&mut self, condition: bool) {
        if condition {
            let offset = self.read_byte(self.program_counter) as i8;
            self.program_counter = self.program_counter.wrapping_add(offset as u16);
        }
        self.program_counter += 1; // TODO: this isnt the cleanest
    }

    fn pha(&mut self) {
        self.stack_push_byte(self.accumulator);
    }

    fn pla(&mut self) {
        self.accumulator = self.stack_pop_byte();
        self.update_zero_and_negative_flags(self.accumulator);
    }

    fn php(&mut self) {
        let mut status = self.status.clone();
        // See https://www.nesdev.org/wiki/Status_flags#The_B_flag
        status.insert(CpuFlags::BREAK);
        status.insert(CpuFlags::BREAK2);
        self.stack_push_byte(status.bits());
    }

    fn plp(&mut self) {
        self.status = CpuFlags::from_bits_truncate(self.stack_pop_byte());
        // See https://www.nesdev.org/wiki/Status_flags#The_B_flag
        self.status.remove(CpuFlags::BREAK);
        self.status.insert(CpuFlags::BREAK2);
    }

    fn jsr(&mut self, mode: &AdressingMode) {
        let (addr, new_pc) = self.param_from_adressing_mode(mode);
        self.stack_push_word(new_pc - 1);
        self.program_counter = addr;
    }

    fn rts(&mut self) {
        self.program_counter = self.stack_pop_word() + 1;
    }

    fn sta(&mut self, mode: &AdressingMode) {
        let (addr, next_pc) = self.param_from_adressing_mode(mode);
        self.write_byte(addr, self.accumulator);
        self.program_counter = next_pc;
    }

    fn stx(&mut self, mode: &AdressingMode) {
        let (addr, next_pc) = self.param_from_adressing_mode(mode);
        self.write_byte(addr, self.register_x);
        self.program_counter = next_pc;
    }

    fn sty(&mut self, mode: &AdressingMode) {
        let (addr, next_pc) = self.param_from_adressing_mode(mode);
        self.write_byte(addr, self.register_y);
        self.program_counter = next_pc;
    }

    fn bit(&mut self, mode: &AdressingMode) {
        let (addr, new_pc) = self.param_from_adressing_mode(mode);
        let value = self.read_byte(addr);

        self.status
            .set(CpuFlags::ZERO, (self.accumulator & value) == 0);
        self.status.set(CpuFlags::NEGATIVE, CPU::nth_bit(value, 7));
        self.status.set(CpuFlags::OVERFLOW, CPU::nth_bit(value, 6));

        self.program_counter = new_pc;
    }

    fn jmp(&mut self, mode: &Option<AdressingMode>) {
        let mut address: u16 = self.read_word(self.program_counter);

        // Indirect mode, not used by any other opcode
        if mode.is_none() {
            address = if address & 0x00FF == 0x00FF {
                // An original 6502 has does not correctly fetch the target address if the indirect vector falls on a
                // page boundary (e.g. $xxFF). In this case fetches the LSB from $xxFF as expected but takes the MSB from $xx00.
                u16::from_le_bytes([
                    self.read_byte(address as u16),
                    self.read_byte((address as u16) & 0xFF00),
                ])
            } else {
                self.read_word(address)
            };
        }

        self.program_counter = address;
    }

    fn cmp(&mut self, mode: &AdressingMode, compare_with: u8) {
        let (addr, new_pc) = self.param_from_adressing_mode(mode);
        let value = self.read_byte(addr);

        self.status.set(CpuFlags::CARRY, compare_with >= value);
        // Subtract so that we set the ZERO flag if the values are equal
        self.update_zero_and_negative_flags(compare_with.wrapping_sub(value));
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

    fn rol_accumulator(&mut self) {
        self.status
            .set(CpuFlags::CARRY, CPU::nth_bit(self.accumulator, 7));
        self.accumulator = self.accumulator.rotate_left(1);
        self.update_zero_and_negative_flags(self.accumulator);
    }

    fn rol(&mut self, mode: &AdressingMode) {
        let (addr, new_pc) = self.param_from_adressing_mode(mode);
        let mut value = self.read_byte(addr);

        self.status.set(CpuFlags::CARRY, CPU::nth_bit(value, 7));
        value = value.rotate_left(1);
        self.write_byte(addr, value);

        self.update_zero_and_negative_flags(value);
        self.program_counter = new_pc;
    }

    fn ror_accumulator(&mut self) {
        self.status
            .set(CpuFlags::CARRY, CPU::nth_bit(self.accumulator, 0));
        self.accumulator = self.accumulator.rotate_right(1);
        self.update_zero_and_negative_flags(self.accumulator);
    }

    fn ror(&mut self, mode: &AdressingMode) {
        let (addr, new_pc) = self.param_from_adressing_mode(mode);
        let mut value = self.read_byte(addr);

        self.status.set(CpuFlags::CARRY, CPU::nth_bit(value, 0));
        value = value.rotate_right(1);
        self.write_byte(addr, value);

        self.update_zero_and_negative_flags(value);
        self.program_counter = new_pc;
    }

    fn asl_accumulator(&mut self) {
        self.status
            .set(CpuFlags::CARRY, CPU::nth_bit(self.accumulator, 7));
        self.accumulator <<= 1;
        self.update_zero_and_negative_flags(self.accumulator);
    }

    fn asl(&mut self, mode: &AdressingMode) {
        let (addr, new_pc) = self.param_from_adressing_mode(mode);
        let mut value = self.read_byte(addr);

        self.status.set(CpuFlags::CARRY, CPU::nth_bit(value, 7));
        value <<= 1;
        self.write_byte(addr, value);

        self.update_zero_and_negative_flags(value);
        self.program_counter = new_pc;
    }

    fn lsr_accumulator(&mut self) {
        self.status
            .set(CpuFlags::CARRY, CPU::nth_bit(self.accumulator, 0));
        self.accumulator >>= 1;
        self.update_zero_and_negative_flags(self.accumulator);
    }

    fn lsr(&mut self, mode: &AdressingMode) {
        let (addr, new_pc) = self.param_from_adressing_mode(mode);
        let mut value = self.read_byte(addr);

        self.status.set(CpuFlags::CARRY, CPU::nth_bit(value, 0));
        value >>= 1;
        self.write_byte(addr, value);

        self.update_zero_and_negative_flags(value);
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

    fn txa(&mut self) {
        self.accumulator = self.register_x;
        self.update_zero_and_negative_flags(self.accumulator)
    }

    fn tya(&mut self) {
        self.accumulator = self.register_y;
        self.update_zero_and_negative_flags(self.accumulator)
    }

    fn tay(&mut self) {
        self.register_y = self.accumulator;
        self.update_zero_and_negative_flags(self.register_y)
    }

    fn sed(&mut self) {
        self.status.insert(CpuFlags::DECIMAL);
    }

    fn sec(&mut self) {
        self.status.insert(CpuFlags::CARRY);
    }

    fn sei(&mut self) {
        self.status.insert(CpuFlags::IRQ);
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
        self.status.remove(CpuFlags::DECIMAL);
    }

    pub fn run(&mut self) {
        loop {
            let opcode = self.read_byte(self.program_counter);
            self.program_counter += 1;

            match opcode {
                0xEA => continue, // NOP
                0x00 => return,   // BRK

                0x40 => self.rti(),

                0x69 => self.adc(&AdressingMode::Immediate),
                0x65 => self.adc(&AdressingMode::ZeroPage),
                0x75 => self.adc(&AdressingMode::ZeroPageX),
                0x6D => self.adc(&AdressingMode::Absolute),
                0x7D => self.adc(&AdressingMode::AbsoluteX),
                0x79 => self.adc(&AdressingMode::AbsoluteY),
                0x61 => self.adc(&AdressingMode::IndirectX),
                0x71 => self.adc(&AdressingMode::IndirectY),

                0xE9 => self.sdc(&AdressingMode::Immediate),
                0xE5 => self.sdc(&AdressingMode::ZeroPage),
                0xF5 => self.sdc(&AdressingMode::ZeroPageX),
                0xED => self.sdc(&AdressingMode::Absolute),
                0xFD => self.sdc(&AdressingMode::AbsoluteX),
                0xF9 => self.sdc(&AdressingMode::AbsoluteY),
                0xE1 => self.sdc(&AdressingMode::IndirectX),
                0xF1 => self.sdc(&AdressingMode::IndirectY),

                // BCS
                0xB0 => self.branch(self.status.contains(CpuFlags::CARRY)),
                // BCC
                0x90 => self.branch(!self.status.contains(CpuFlags::CARRY)),
                // BEQ
                0xF0 => self.branch(self.status.contains(CpuFlags::ZERO)),
                // BNE
                0xD0 => self.branch(!self.status.contains(CpuFlags::ZERO)),
                // BMI
                0x30 => self.branch(self.status.contains(CpuFlags::NEGATIVE)),
                // BPL
                0x10 => self.branch(!self.status.contains(CpuFlags::NEGATIVE)),
                // BVS
                0x70 => self.branch(self.status.contains(CpuFlags::OVERFLOW)),
                // BVC
                0x50 => self.branch(!self.status.contains(CpuFlags::OVERFLOW)),

                0x08 => self.php(),
                0x28 => self.plp(),

                0x48 => self.pha(),
                0x68 => self.pla(),

                0xAA => self.tax(),
                0x9A => self.txa(),

                0xA8 => self.tay(),
                0x98 => self.tya(),

                0xB8 => self.clv(),

                0x78 => self.sei(),
                0x58 => self.cli(),

                0xf8 => self.sed(),
                0xD8 => self.cld(),

                0x38 => self.sec(),
                0x18 => self.clc(),

                0xCA => self.dex(),
                0x88 => self.dey(),

                0x20 => self.jsr(&AdressingMode::Absolute),
                0x60 => self.rts(),

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

                0x4A => self.lsr_accumulator(),
                0x46 => self.lsr(&AdressingMode::ZeroPage),
                0x56 => self.lsr(&AdressingMode::ZeroPageX),
                0x4E => self.lsr(&AdressingMode::Absolute),
                0x5E => self.lsr(&AdressingMode::AbsoluteX),

                0x0A => self.asl_accumulator(),
                0x06 => self.asl(&AdressingMode::ZeroPage),
                0x16 => self.asl(&AdressingMode::ZeroPageX),
                0x0E => self.asl(&AdressingMode::Absolute),
                0x1E => self.asl(&AdressingMode::AbsoluteX),

                0x6A => self.ror_accumulator(),
                0x66 => self.ror(&AdressingMode::ZeroPage),
                0x76 => self.ror(&AdressingMode::ZeroPageX),
                0x6E => self.ror(&AdressingMode::Absolute),
                0x7E => self.ror(&AdressingMode::AbsoluteX),

                0x2A => self.rol_accumulator(),
                0x26 => self.rol(&AdressingMode::ZeroPage),
                0x36 => self.rol(&AdressingMode::ZeroPageX),
                0x2E => self.rol(&AdressingMode::Absolute),
                0x3E => self.rol(&AdressingMode::AbsoluteX),

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

                0xC9 => self.cmp(&AdressingMode::Immediate, self.accumulator),
                0xC5 => self.cmp(&AdressingMode::ZeroPage, self.accumulator),
                0xD5 => self.cmp(&AdressingMode::ZeroPageX, self.accumulator),
                0xCD => self.cmp(&AdressingMode::Absolute, self.accumulator),
                0xDD => self.cmp(&AdressingMode::AbsoluteX, self.accumulator),
                0xD9 => self.cmp(&AdressingMode::AbsoluteY, self.accumulator),
                0xC1 => self.cmp(&AdressingMode::IndirectX, self.accumulator),
                0xD1 => self.cmp(&AdressingMode::IndirectY, self.accumulator),

                0xE0 => self.cmp(&AdressingMode::Immediate, self.register_x),
                0xE4 => self.cmp(&AdressingMode::ZeroPage, self.register_x),
                0xEC => self.cmp(&AdressingMode::Absolute, self.register_x),

                0xC0 => self.cmp(&AdressingMode::Immediate, self.register_y),
                0xC4 => self.cmp(&AdressingMode::ZeroPage, self.register_y),
                0xCC => self.cmp(&AdressingMode::Absolute, self.register_y),

                0xA9 => self.lda(&AdressingMode::Immediate),
                0xA5 => self.lda(&AdressingMode::ZeroPage),
                0xB5 => self.lda(&AdressingMode::ZeroPageX),
                0xAD => self.lda(&AdressingMode::Absolute),
                0xBD => self.lda(&AdressingMode::AbsoluteX),
                0xB9 => self.lda(&AdressingMode::AbsoluteY),
                0xA1 => self.lda(&AdressingMode::IndirectX),
                0xB1 => self.lda(&AdressingMode::IndirectY),

                0x85 => self.sta(&AdressingMode::ZeroPage),
                0x95 => self.sta(&AdressingMode::ZeroPageX),
                0x8D => self.sta(&AdressingMode::Absolute),
                0x9D => self.sta(&AdressingMode::AbsoluteX),
                0x99 => self.sta(&AdressingMode::AbsoluteY),
                0x81 => self.sta(&AdressingMode::IndirectX),
                0x91 => self.sta(&AdressingMode::IndirectY),

                0xA2 => self.ldx(&AdressingMode::Immediate),
                0xA6 => self.ldx(&AdressingMode::ZeroPage),
                0xB6 => self.ldx(&AdressingMode::ZeroPageY),
                0xAE => self.ldx(&AdressingMode::Absolute),
                0xBE => self.ldx(&AdressingMode::AbsoluteY),

                0x86 => self.stx(&AdressingMode::ZeroPage),
                0x96 => self.stx(&AdressingMode::ZeroPageY),
                0x8E => self.stx(&AdressingMode::Absolute),

                0xA0 => self.ldy(&AdressingMode::Immediate),
                0xA4 => self.ldy(&AdressingMode::ZeroPage),
                0xB4 => self.ldy(&AdressingMode::ZeroPageX),
                0xAC => self.ldy(&AdressingMode::Absolute),
                0xBC => self.ldy(&AdressingMode::AbsoluteX),

                0x84 => self.sty(&AdressingMode::ZeroPage),
                0x94 => self.sty(&AdressingMode::ZeroPageX),
                0x8C => self.sty(&AdressingMode::Absolute),

                0x4c => self.jmp(&Some(AdressingMode::Absolute)),
                0x6c => self.jmp(&None),

                0x24 => self.bit(&AdressingMode::ZeroPage),
                0x2C => self.bit(&AdressingMode::Absolute),

                _ => todo!("opcode {:02x} not implemented", opcode),
            }
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

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

    test_cpu!(test_cpu_init, |cpu: &mut CPU| {
        assert_eq!(cpu.accumulator, 0);
        assert_eq!(cpu.register_x, 0);
        assert_eq!(cpu.register_y, 0);
        assert_eq!(cpu.program_counter, 0);
        assert_eq!(cpu.status, CpuFlags::empty());
    });

    test_cpu!(test_jmp, [0x4C, 0x00, 0xff /* JMP 0xff00 */], |cpu: CPU| {
        assert_eq!(cpu.program_counter, 0xff01); // Plus one because of BRK instruction decoding
    });

    test_cpu!(
        test_jmp_indirect,
        [0x6C, 0x00, 0xff /* JMP 0xff00 */],
        true,
        |cpu: &mut CPU| {
            cpu.write_byte(0xff00, 0x00);
            cpu.run();
            assert_eq!(cpu.program_counter, 0x0001); // Plus one because of BRK instruction decoding
        }
    );

    test_cpu!(
        test_and,
        [0x29, 234 /*  AND, 234 */],
        true,
        |cpu: &mut CPU| {
            cpu.accumulator = 0b1010;
            cpu.run();
            assert_eq!(cpu.accumulator, 0b1010 & 234);
        }
    );

    test_cpu!(
        test_ora,
        [0x09, 123 /* ORA, 123 */],
        true,
        |cpu: &mut CPU| {
            cpu.accumulator = 0b1010;
            cpu.run();
            assert_eq!(cpu.accumulator, 0b1010 | 123);
        }
    );

    test_cpu!(
        test_eor,
        [0x49, 123 /* EOR, 123 */],
        true,
        |cpu: &mut CPU| {
            cpu.accumulator = 0b1010;
            cpu.run();
            assert_eq!(cpu.accumulator, 0b1010 ^ 123);
        }
    );

    test_cpu!(test_inx, [0xe8 /* INX */], true, |cpu: &mut CPU| {
        cpu.register_x = 5;
        cpu.run();
        assert_eq!(cpu.register_x, 6);
    });

    test_cpu!(test_iny, [0xc8 /* INY */], true, |cpu: &mut CPU| {
        cpu.register_y = 0xff;
        cpu.run();
        assert_eq!(cpu.register_y, 0);
    });

    test_cpu!(
        test_inc,
        [0xe6, 0x10 /* INC, 0x10 */],
        true,
        |cpu: &mut CPU| {
            cpu.write_byte(0x10, 1);
            cpu.run();
            assert_eq!(cpu.read_byte(0x10), 2);
            assert!(!cpu.status.contains(CpuFlags::ZERO));
            assert!(!cpu.status.contains(CpuFlags::NEGATIVE));
        }
    );

    test_cpu!(test_tax, [0xaa /* TAX */], true, |cpu: &mut CPU| {
        cpu.accumulator = 5;
        cpu.run();
        assert_eq!(cpu.register_x, 5);
    });

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

    test_cpu!(test_dex, [0xCA /* DEX */], true, |cpu: &mut CPU| {
        cpu.register_x = 0;
        cpu.run();
        assert_eq!(cpu.register_x, 0xff);
    });

    test_cpu!(test_cpx, [0xe0, 10 /* CPX, 10 */], true, |cpu: &mut CPU| {
        cpu.register_x = 10;
        cpu.run();
        assert!(cpu.status.contains(CpuFlags::CARRY));
        assert!(cpu.status.contains(CpuFlags::ZERO));
        assert!(!cpu.status.contains(CpuFlags::NEGATIVE));
    });

    test_cpu!(test_cpy, [0xc0, 10 /* CPY, 10 */], true, |cpu: &mut CPU| {
        cpu.register_y = 9;
        cpu.run();
        assert!(!cpu.status.contains(CpuFlags::CARRY));
        assert!(!cpu.status.contains(CpuFlags::ZERO));
        assert!(cpu.status.contains(CpuFlags::NEGATIVE));
    });

    test_cpu!(
        test_cmp,
        [0xc5, 0x56 /* CMP, 0x56 */],
        true,
        |cpu: &mut CPU| {
            cpu.write_byte(0x56, 10);
            cpu.accumulator = 10;
            cpu.run();
            assert!(cpu.status.contains(CpuFlags::CARRY));
            assert!(cpu.status.contains(CpuFlags::ZERO));
            assert!(!cpu.status.contains(CpuFlags::NEGATIVE));
        }
    );

    test_cpu!(test_lda, [0xa9, 5 /* LDA, 5 */], |cpu: CPU| {
        assert_eq!(cpu.accumulator, 5);
        assert_eq!(cpu.status, CpuFlags::empty());
    });

    test_cpu!(
        test_lda_from_memory,
        [0xa5, 0x55 /* LDA, 0x55 */],
        true,
        |cpu: &mut CPU| {
            cpu.write_byte(0x55, 10);
            cpu.run();
            assert_eq!(cpu.accumulator, 10);
        }
    );

    test_cpu!(test_ldy, [0xa0, 5 /* LDY, 5 */], |cpu: CPU| {
        assert_eq!(cpu.register_y, 5);
        assert_eq!(cpu.status, CpuFlags::empty());
    });

    test_cpu!(test_ldx, [0xa2, 5 /* LDX, 5 */], |cpu: CPU| {
        assert_eq!(cpu.register_x, 5);
        assert_eq!(cpu.status, CpuFlags::empty());
    });

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

    macro_rules! test_status_set {
        ($test_name: ident, $asm: expr, $flag: expr) => {
            test_cpu!($test_name, [$asm], |cpu: CPU| {
                assert!(cpu.status.contains($flag));
            });
        };
    }

    macro_rules! test_status_clear {
        ($test_name: ident, $asm: expr, $flag: expr) => {
            test_cpu!($test_name, [$asm], true, |cpu: &mut CPU| {
                cpu.status.insert($flag);
                cpu.run();
                assert!(!cpu.status.contains($flag));
            });
        };
    }

    test_status_clear!(test_clv, 0xb8, CpuFlags::OVERFLOW);

    test_status_set!(test_sec, 0x38, CpuFlags::CARRY);
    test_status_clear!(test_clc, 0x18, CpuFlags::CARRY);

    test_status_clear!(test_cld, 0xd8, CpuFlags::DECIMAL);
    test_status_set!(test_sed, 0xf8, CpuFlags::DECIMAL);

    test_status_set!(test_sei, 0x78, CpuFlags::IRQ);
    test_status_clear!(test_cli, 0x58, CpuFlags::IRQ);
}
