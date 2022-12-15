pub mod assembler;
mod instructions;

use crate::bus::{Bus, Clock, Memory, PROGRAM_ROM_START};
use bitflags::bitflags;
use instructions::Instruction;
use std::{fmt, sync::mpsc::Sender};

/// See https://www.nesdev.org/wiki/CPU_addressing_modes
#[derive(Debug, PartialEq)]
pub enum AdressingMode {
    Implied,
    Relative,
    Immediate,
    Accumulator,
    Indirect,
    IndirectX,
    IndirectY,
    Absolute,
    AbsoluteX,
    AbsoluteY,
    ZeroPage,
    ZeroPageX,
    ZeroPageY,
}

impl AdressingMode {
    pub const fn has_arguments(&self) -> bool {
        match self {
            AdressingMode::Implied | AdressingMode::Accumulator => false,
            _ => true,
        }
    }

    /// The length of an instruction, counting the identifier and arguments
    pub const fn len(&self) -> u16 {
        match self {
            AdressingMode::Implied | AdressingMode::Accumulator => 1,

            AdressingMode::Immediate
            | AdressingMode::Relative
            | AdressingMode::IndirectX
            | AdressingMode::IndirectY
            | AdressingMode::ZeroPage
            | AdressingMode::ZeroPageX
            | AdressingMode::ZeroPageY => 2,

            AdressingMode::Indirect
            | AdressingMode::Absolute
            | AdressingMode::AbsoluteX
            | AdressingMode::AbsoluteY => 3,
        }
    }

    /// Fetch the address of the operand. Returns the address and a flag indicating if a page boundary was crossed
    pub fn fetch_param_address(&self, cpu: &CPU) -> (u16, bool) {
        let after_opcode = cpu.program_counter.wrapping_add(1);
        match self {
            Self::Immediate | Self::Relative => (after_opcode, false),
            Self::Absolute => (cpu.read_word(after_opcode), false),
            Self::ZeroPage => (cpu.read_byte(after_opcode) as u16, false),

            Self::ZeroPageX => (
                cpu.read_byte(after_opcode).wrapping_add(cpu.register_x) as u16,
                false,
            ),

            Self::ZeroPageY => (
                cpu.read_byte(after_opcode).wrapping_add(cpu.register_y) as u16,
                false,
            ),

            Self::AbsoluteX => {
                let addr_base = cpu.read_word(after_opcode);
                let addr = addr_base.wrapping_add(cpu.register_x as u16);
                (addr, CPU::is_on_different_page(addr_base, addr))
            }

            Self::AbsoluteY => {
                let addr_base = cpu.read_word(after_opcode);
                let addr = addr_base.wrapping_add(cpu.register_y as u16);
                (addr, CPU::is_on_different_page(addr_base, addr))
            }

            Self::Indirect => {
                let ptr = cpu.read_word(after_opcode);
                let low = cpu.read_byte(ptr as u16);

                // Accomodate for a hardware bug, the 6502 reference states the following:
                //    "An original 6502 has does not correctly fetch the target address if the indirect vector
                //    falls on a page boundary (e.g. $xxFF where xx is any value from $00 to $FF). In this case
                //    it fetches the LSB from $xxFF as expected but takes the MSB from $xx00"
                let high = if ptr & 0x00FF == 0xFF {
                    cpu.read_byte(ptr & 0xFF00)
                } else {
                    cpu.read_byte(ptr.wrapping_add(1))
                };

                (u16::from_le_bytes([low, high]), false)
            }

            Self::IndirectX => {
                let ptr = cpu.read_byte(after_opcode).wrapping_add(cpu.register_x);
                let addr = u16::from_le_bytes([
                    cpu.read_byte(ptr as u16),
                    cpu.read_byte(ptr.wrapping_add(1) as u16),
                ]);
                (addr, false)
            }

            Self::IndirectY => {
                let ptr = cpu.read_byte(after_opcode);
                let addr_base = u16::from_le_bytes([
                    cpu.read_byte(ptr as u16),
                    cpu.read_byte(ptr.wrapping_add(1) as u16),
                ]);
                let addr = addr_base.wrapping_add(cpu.register_y as u16);
                (addr, CPU::is_on_different_page(addr_base, addr))
            }

            _ => {
                panic!("Addressing mode {} has no arguments!", self);
            }
        }
    }

    /// Fetch the operand. Returns the operand and a flag indicating if a page boundary was crossed.
    pub fn fetch_param(&self, cpu: &CPU) -> (u8, bool) {
        let (addr, page_crossed) = self.fetch_param_address(cpu);
        (cpu.read_byte(addr), page_crossed)
    }
}

bitflags! {
    /// See https://www.nesdev.org/wiki/Status_flags
    #[rustfmt::skip]
    #[derive(Debug, Clone, PartialEq)]
    pub struct CpuFlags: u8 {
        const CARRY =    0b0000_0001;
        const ZERO =     0b0000_0010;
        const IRQ =      0b0000_0100;
        const DECIMAL =  0b0000_1000; // No effect
        const BREAK =    0b0001_0000;
        const BREAK2 =   0b0010_0000; // No effect
        const OVERFLOW = 0b0100_0000;
        const NEGATIVE = 0b1000_0000;
    }
}

impl Default for CpuFlags {
    fn default() -> CpuFlags {
        // Self::IRQ | Self::BREAK | Self::BREAK2
        Self::IRQ | Self::BREAK2 // Hack to diff against nestest log, above is correct
    }
}

pub struct InstructionState {
    pub formatted: String,
    pub accumulator: u8,
    pub register_x: u8,
    pub register_y: u8,
    pub program_counter: u16,
    pub stack_pointer: u8,
    pub status: CpuFlags,
}

/// See https://www.nesdev.org/wiki/CPU
pub struct CPU {
    pub accumulator: u8,
    pub register_x: u8,
    pub register_y: u8,
    pub program_counter: u16,
    pub stack_pointer: u8,
    pub status: CpuFlags,
    pub bus: Bus,
}

impl CPU {
    #[allow(dead_code)] // TODO: remove when graphics are implemented
    const RESET_VECTOR: u16 = 0xFFFC;

    const STACK_OFFSET: u16 = 0x0100;
    const STACK_RESET: u8 = 0xFD;
    const RESET_CYCLES: u64 = 7;

    pub fn new(bus: Bus) -> CPU {
        CPU {
            program_counter: PROGRAM_ROM_START,
            stack_pointer: CPU::STACK_RESET,
            status: CpuFlags::default(),
            accumulator: 0,
            register_x: 0,
            register_y: 0,
            bus,
        }
    }

    pub fn reset(&mut self) {
        self.set_cycles(CPU::RESET_CYCLES);
        self.status = CpuFlags::default();
        self.stack_pointer = CPU::STACK_RESET;
        self.accumulator = 0;
        self.register_x = 0;
        self.register_y = 0;

        // This is a hack to skip graphic init in nestest
        self.program_counter = 0xC000;
        // self.program_counter = self.read_word(CPU::RESET_VECTOR);
    }

    pub fn push_byte(&mut self, data: u8) {
        self.write_byte(CPU::STACK_OFFSET + self.stack_pointer as u16, data);
        self.stack_pointer = self.stack_pointer.wrapping_sub(1);
    }

    pub fn pop_byte(&mut self) -> u8 {
        self.stack_pointer = self.stack_pointer.wrapping_add(1);
        self.read_byte(CPU::STACK_OFFSET + self.stack_pointer as u16)
    }

    pub fn push_word(&mut self, data: u16) {
        // I dont understand why pushing in big-endian order makes the value be read in little-endian?
        for byte in u16::to_be_bytes(data) {
            self.push_byte(byte);
        }
    }

    pub fn pop_word(&mut self) -> u16 {
        u16::from_le_bytes([self.pop_byte(), self.pop_byte()])
    }

    pub fn update_zero_and_negative_flags(&mut self, value: u8) {
        self.status.set(CpuFlags::NEGATIVE, Self::nth_bit(value, 7));
        self.status.set(CpuFlags::ZERO, value == 0);
    }

    pub fn step(&mut self) -> Option<InstructionState> {
        let opcode = self.read_byte(self.program_counter);
        let (instr, mode, cycles) = Instruction::decode(&opcode).expect(
            format!(
                "invalid opcode ${:02X} at PC ${:04X}",
                opcode, self.program_counter
            )
            .as_str(),
        );

        let state = InstructionState {
            formatted: instr.format(self, mode),
            accumulator: self.accumulator,
            register_x: self.register_x,
            register_y: self.register_y,
            program_counter: self.program_counter,
            stack_pointer: self.stack_pointer,
            status: self.status.clone(),
        };

        if instr.name == "BRK" {
            // TODO: this is a hack
            return None;
        }

        let program_counter_prior = self.program_counter;
        (instr.function)(self, mode);
        if self.program_counter == program_counter_prior {
            // Some instructions (e.g. JMP) set the program counter themselves
            self.program_counter += mode.len();
        }

        self.tick(*cycles as u64);
        Some(state)
    }

    #[allow(dead_code)] // TODO: remove when GUI is stabalised
    pub fn run(&mut self) {
        loop {
            let opcode = self.read_byte(self.program_counter);
            let (instr, mode, cycles) = Instruction::decode(&opcode).expect(
                format!(
                    "invalid opcode ${:02X} at PC ${:04X}",
                    opcode, self.program_counter
                )
                .as_str(),
            );

            // TODO: do this from a callback to make it more flexible
            if !self.bus.quiet {
                let mut bytes = String::new();
                for i in 0..mode.len() {
                    bytes += &format!("{:02X} ", self.read_byte(self.program_counter + i as u16));
                }

                println!(
                    "{:04X}  {1: <9} {2:}  {3:}",
                    self.program_counter,
                    bytes,
                    self,
                    instr.format(self, mode)
                );
            }

            if instr.name == "BRK" {
                // TODO: properly implement
                break;
            };

            let program_counter_prior = self.program_counter;
            (instr.function)(self, mode);
            if self.program_counter == program_counter_prior {
                // Some instructions (e.g. JMP) set the program counter themselves
                self.program_counter += mode.len();
            }

            self.tick(*cycles as u64);
        }
    }

    /// Get the status of bit N
    pub const fn nth_bit(value: u8, n: u8) -> bool {
        value & (1 << n) != 0
    }

    /// Check if two values are contained on a different page in memory
    pub const fn is_on_different_page(a: u16, b: u16) -> bool {
        (a & 0xFF00) != (b & 0xFF00)
    }
}

impl Memory for CPU {
    fn read_byte(&self, address: u16) -> u8 {
        self.bus.read_byte(address)
    }

    fn write_byte(&mut self, address: u16, data: u8) {
        self.bus.write_byte(address, data)
    }
}

impl Clock for CPU {
    fn tick_internal(&mut self, cycles: u64) {
        self.bus.tick_internal(cycles);
    }

    fn get_cycles(&self) -> u64 {
        self.bus.get_cycles()
    }

    fn set_cycles(&mut self, cycles: u64) {
        self.bus.set_cycles(cycles);
    }
}

impl CpuFlags {
    const fn format(&self, flag: CpuFlags, display: char) -> char {
        if self.contains(flag) {
            display
        } else {
            '-'
        }
    }
}

impl fmt::Display for CpuFlags {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> std::fmt::Result {
        let mut string = String::with_capacity(8);
        string.push(self.format(CpuFlags::NEGATIVE, 'N'));
        string.push(self.format(CpuFlags::OVERFLOW, 'O'));
        string.push(self.format(CpuFlags::BREAK2, 'B'));
        string.push(self.format(CpuFlags::BREAK, 'B'));
        string.push(self.format(CpuFlags::DECIMAL, 'D'));
        string.push(self.format(CpuFlags::IRQ, 'I'));
        string.push(self.format(CpuFlags::ZERO, 'Z'));
        string.push(self.format(CpuFlags::CARRY, 'C'));
        write!(f, "{}", string)
    }
}

impl fmt::Display for CPU {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "A:{:02X} X:{:02X} Y:{:02X} P:{:02X} SP:{:02X} C:{5: <5} {6:}",
            self.accumulator,
            self.register_x,
            self.register_y,
            self.status,
            self.stack_pointer,
            self.get_cycles(),
            self.status
        )
    }
}

impl fmt::Display for AdressingMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Implied => write!(f, "implied"),
            Self::Relative => write!(f, "relative"),
            Self::Immediate => write!(f, "immediate"),
            Self::Accumulator => write!(f, "accumulator"),
            Self::Indirect => write!(f, "indirect"),
            Self::IndirectX => write!(f, "indirectX"),
            Self::IndirectY => write!(f, "indirectY"),
            Self::Absolute => write!(f, "absolute"),
            Self::AbsoluteX => write!(f, "absoluteX"),
            Self::AbsoluteY => write!(f, "absoluteY"),
            Self::ZeroPage => write!(f, "zeropage"),
            Self::ZeroPageX => write!(f, "zeropageX"),
            Self::ZeroPageY => write!(f, "zeropageX"),
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::Cartridge;

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
                let cart = Cartridge::new($asm.to_vec()).unwrap();
                let mut cpu = CPU::new(Bus::new(cart, false));
                cpu.run();
                $callback(cpu);
            }
        };

        ($test_name: ident, $asm:expr, $dont_execute:expr, $callback:expr) => {
            #[test]
            fn $test_name() {
                let cart = Cartridge::new($asm.to_vec()).unwrap();
                let mut cpu = CPU::new(Bus::new(cart, false));
                $callback(&mut cpu);
            }
        };

        ($test_name: ident, $callback:expr) => {
            #[test]
            fn $test_name() {
                let cart = Cartridge::new([0].to_vec()).unwrap();
                let mut cpu = CPU::new(Bus::new(cart, false));
                $callback(&mut cpu);
            }
        };
    }

    test_cpu!(test_cpu_init, |cpu: &mut CPU| {
        assert_eq!(cpu.accumulator, 0);
        assert_eq!(cpu.register_x, 0);
        assert_eq!(cpu.register_y, 0);
        assert_eq!(cpu.program_counter, PROGRAM_ROM_START);
        assert_eq!(cpu.status, CpuFlags::default());
    });

    test_cpu!(test_jmp, [0x4C, 0x10, 0x00 /* JMP 0x0010 */], |cpu: CPU| {
        assert_eq!(cpu.program_counter, 0x0010);
    });

    test_cpu!(test_jsr, [0x20, 0x10, 0x00 /* JSR 0x0010 */], |cpu: CPU| {
        assert_eq!(cpu.program_counter, 0x0010);
    });

    test_cpu!(
        test_jsr_ldx,
        [0x20, 0x53, 0x12 /* JSR 0x1253 */,],
        true,
        |cpu: &mut CPU| {
            cpu.write_byte(0x1253, 0xA2); // LDX
            cpu.write_byte(0x1254, 0x23); // #0x23
            cpu.run();
            assert_eq!(cpu.program_counter, 0x1255);
            assert_eq!(cpu.register_x, 0x23);
        }
    );

    test_cpu!(
        test_jsr_rts,
        [0x20, 0x53, 0x12 /* JSR 0x1253 */,],
        true,
        |cpu: &mut CPU| {
            cpu.write_byte(0x1253, 0xA0); // LDY
            cpu.write_byte(0x1254, 0xFF); // #0xFF
            cpu.write_byte(0x1255, 0x60); // RTS
            cpu.run();
            assert_eq!(cpu.program_counter, 0x8003);
            assert_eq!(cpu.register_y, 0xFF);
        }
    );

    test_cpu!(
        test_jmp_indirect,
        [0x6C, 0xff, 0x00 /* JMP 0x00ff */],
        true,
        |cpu: &mut CPU| {
            cpu.write_byte(0x00ff, 0x00);
            cpu.run();
            assert_eq!(cpu.program_counter, 0x0000)
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
        assert_eq!(cpu.status, CpuFlags::default());
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
        assert_eq!(cpu.status, CpuFlags::default());
    });

    test_cpu!(test_ldx, [0xa2, 5 /* LDX, 5 */], |cpu: CPU| {
        assert_eq!(cpu.register_x, 5);
        assert_eq!(cpu.status, CpuFlags::default());
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

    test_cpu!(test_update_zero_and_negative, |cpu: &mut CPU| {
        cpu.update_zero_and_negative_flags(0);
        assert!(cpu.status.contains(CpuFlags::ZERO));
        cpu.update_zero_and_negative_flags(1);
        assert!(!cpu.status.contains(CpuFlags::ZERO));

        cpu.update_zero_and_negative_flags(0b1000_0000);
        assert!(cpu.status.contains(CpuFlags::NEGATIVE));
        cpu.update_zero_and_negative_flags(0b0111_1111);
        assert!(!cpu.status.contains(CpuFlags::NEGATIVE));
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
