mod addressing_mode;
mod assembler;
mod instructions;

use crate::bus::{Bus, Clock, Memory, PROGRAM_ROM_START};
use bitflags::bitflags;
use instructions::Instruction;
use std::fmt;

/// Passed to the GUI for the debugger. Could maybe be used for savestates in the future?
pub struct CpuState {
    pub formatted: String,
    pub accumulator: u8,
    pub register_x: u8,
    pub register_y: u8,
    pub program_counter: u16,
    pub stack_pointer: u8,
    pub status: CpuFlags,
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
        Self::IRQ | Self::BREAK | Self::BREAK2
        // Self::IRQ | Self::BREAK2 // Hack to diff against nestest log, above is correct
    }
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
    const SPAN_NAME: &'static str = "cpu";

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
        let _span = tracing::span!(tracing::Level::INFO, CPU::SPAN_NAME).entered();
        self.set_cycles(CPU::RESET_CYCLES);
        self.status = CpuFlags::default();
        self.stack_pointer = CPU::STACK_RESET;
        self.accumulator = 0;
        self.register_x = 0;
        self.register_y = 0;

        // This is a hack to skip graphic init in nestest
        self.program_counter = 0xC000;
        // self.program_counter = self.read_word(CPU::RESET_VECTOR);
        tracing::info!("resetting. PC={:04X}", self.program_counter);
    }

    /// Get the status of bit N
    pub const fn nth_bit(value: u8, n: u8) -> bool {
        value & (1 << n) != 0
    }

    /// Check if two values are contained on a different page in memory
    pub const fn is_on_different_page(a: u16, b: u16) -> bool {
        (a & 0xFF00) != (b & 0xFF00)
    }

    pub fn push_byte(&mut self, data: u8) {
        self.write_byte(CPU::STACK_OFFSET + self.stack_pointer as u16, data);
        self.stack_pointer = self.stack_pointer.wrapping_sub(1);
        tracing::trace!("pushed ${:02X} to the stack", data);
    }

    pub fn pop_byte(&mut self) -> u8 {
        self.stack_pointer = self.stack_pointer.wrapping_add(1);
        let data = self.read_byte(CPU::STACK_OFFSET + self.stack_pointer as u16);
        tracing::trace!("popped ${:02X} off the stack", data);
        data
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

    pub fn step(&mut self) -> Option<CpuState> {
        let _span = tracing::span!(tracing::Level::INFO, CPU::SPAN_NAME).entered();
        let opcode = self.read_byte(self.program_counter);
        let (instr, mode, cycles) = Instruction::decode(&opcode).expect(
            format!(
                "invalid opcode ${:02X} at PC ${:04X}",
                opcode, self.program_counter
            )
            .as_str(),
        );

        let state = CpuState {
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

        tracing::debug!("{}  {}", self, state.formatted);

        self.tick(*cycles as u64);
        Some(state)
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
            "{:04X}  A:{:02X} X:{:02X} Y:{:02X} P:{:02X} SP:{:02X} C:{6: <5} {7:}",
            self.program_counter,
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

#[cfg(test)]
mod test {
    use super::*;

    fn run_cpu(cpu: &mut CPU) {
        loop {
            if let Some(state) = cpu.step() {
                // Unfortunately tracing doesn't seem to want to cooperate with tests.
                // It'll print the logs even for passing tests, which clutters the output.
                println!("{}  {}", cpu, state.formatted);
            } else {
                break;
            }
        }
    }

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
                let mut cpu = CPU::new(Bus::new_dummy($asm.to_vec()));
                run_cpu(&mut cpu);
                $callback(cpu);
            }
        };

        ($test_name: ident, $asm:expr, $dont_execute:expr, $callback:expr) => {
            #[test]
            fn $test_name() {
                let mut cpu = CPU::new(Bus::new_dummy($asm.to_vec()));
                $callback(&mut cpu);
            }
        };

        ($test_name: ident, $callback:expr) => {
            #[test]
            fn $test_name() {
                let mut cpu = CPU::new(Bus::new_dummy([].to_vec()));
                $callback(&mut cpu);
            }
        };
    }

    test_cpu!(cpu_init, |cpu: &mut CPU| {
        assert_eq!(cpu.accumulator, 0);
        assert_eq!(cpu.register_x, 0);
        assert_eq!(cpu.register_y, 0);
        assert_eq!(cpu.program_counter, PROGRAM_ROM_START);
        assert_eq!(cpu.status, CpuFlags::default());
    });

    test_cpu!(jmp, [0x4C, 0x10, 0x00 /* JMP 0x0010 */], |cpu: CPU| {
        assert_eq!(cpu.program_counter, 0x0010);
    });

    test_cpu!(jsr, [0x20, 0x10, 0x00 /* JSR 0x0010 */], |cpu: CPU| {
        assert_eq!(cpu.program_counter, 0x0010);
    });

    test_cpu!(
        jsr_ldx,
        [0x20, 0x53, 0x12 /* JSR 0x1253 */,],
        true,
        |cpu: &mut CPU| {
            cpu.write_byte(0x1253, 0xA2); // LDX
            cpu.write_byte(0x1254, 0x23); // #0x23
            run_cpu(cpu);
            assert_eq!(cpu.program_counter, 0x1255);
            assert_eq!(cpu.register_x, 0x23);
        }
    );

    test_cpu!(
        jsr_rts,
        [0x20, 0x53, 0x12 /* JSR 0x1253 */,],
        true,
        |cpu: &mut CPU| {
            cpu.write_byte(0x1253, 0xA0); // LDY
            cpu.write_byte(0x1254, 0xFF); // #0xFF
            cpu.write_byte(0x1255, 0x60); // RTS
            run_cpu(cpu);
            assert_eq!(cpu.program_counter, 0x8003);
            assert_eq!(cpu.register_y, 0xFF);
        }
    );

    test_cpu!(
        jmp_indirect,
        [0x6C, 0xff, 0x00 /* JMP 0x00ff */],
        true,
        |cpu: &mut CPU| {
            cpu.write_byte(0x00ff, 0x00);
            run_cpu(cpu);
            assert_eq!(cpu.program_counter, 0x0000)
        }
    );

    test_cpu!(and, [0x29, 234 /*  AND, 234 */], true, |cpu: &mut CPU| {
        cpu.accumulator = 0b1010;
        run_cpu(cpu);
        assert_eq!(cpu.accumulator, 0b1010 & 234);
    });

    test_cpu!(ora, [0x09, 123 /* ORA, 123 */], true, |cpu: &mut CPU| {
        cpu.accumulator = 0b1010;
        run_cpu(cpu);
        assert_eq!(cpu.accumulator, 0b1010 | 123);
    });

    test_cpu!(eor, [0x49, 123 /* EOR, 123 */], true, |cpu: &mut CPU| {
        cpu.accumulator = 0b1010;
        run_cpu(cpu);
        assert_eq!(cpu.accumulator, 0b1010 ^ 123);
    });

    test_cpu!(inx, [0xe8 /* INX */], true, |cpu: &mut CPU| {
        cpu.register_x = 5;
        run_cpu(cpu);
        assert_eq!(cpu.register_x, 6);
    });

    test_cpu!(iny, [0xc8 /* INY */], true, |cpu: &mut CPU| {
        cpu.register_y = 0xff;
        run_cpu(cpu);
        assert_eq!(cpu.register_y, 0);
    });

    test_cpu!(inc, [0xe6, 0x10 /* INC, 0x10 */], true, |cpu: &mut CPU| {
        cpu.write_byte(0x10, 1);
        run_cpu(cpu);
        assert_eq!(cpu.read_byte(0x10), 2);
        assert!(!cpu.status.contains(CpuFlags::ZERO));
        assert!(!cpu.status.contains(CpuFlags::NEGATIVE));
    });

    test_cpu!(tax, [0xaa /* TAX */], true, |cpu: &mut CPU| {
        cpu.accumulator = 5;
        run_cpu(cpu);
        assert_eq!(cpu.register_x, 5);
    });

    test_cpu!(
        multiple_ops,
        [0xa9, 0xc0, /* LDA, 0xc0 */ 0xaa, /* TAX */ 0xe8 /* TAX */],
        |cpu: CPU| { assert_eq!(cpu.register_x, 0xc1) }
    );

    test_cpu!(dec, [0xc6, 0x10 /* DEC, 0x10 */], true, |cpu: &mut CPU| {
        cpu.write_byte(0x10, 0x1);
        run_cpu(cpu);
        assert_eq!(cpu.read_byte(0x10), 0);
        assert!(cpu.status.contains(CpuFlags::ZERO));
    });

    test_cpu!(dey, [0x88 /* DEY */], true, |cpu: &mut CPU| {
        cpu.register_y = 10;
        run_cpu(cpu);
        assert_eq!(cpu.register_y, 9);
    });

    test_cpu!(dex, [0xCA /* DEX */], true, |cpu: &mut CPU| {
        cpu.register_x = 0;
        run_cpu(cpu);
        assert_eq!(cpu.register_x, 0xff);
    });

    test_cpu!(cpx, [0xe0, 10 /* CPX, 10 */], true, |cpu: &mut CPU| {
        cpu.register_x = 10;
        run_cpu(cpu);
        assert!(cpu.status.contains(CpuFlags::CARRY));
        assert!(cpu.status.contains(CpuFlags::ZERO));
        assert!(!cpu.status.contains(CpuFlags::NEGATIVE));
    });

    test_cpu!(cpy, [0xc0, 10 /* CPY, 10 */], true, |cpu: &mut CPU| {
        cpu.register_y = 9;
        run_cpu(cpu);
        assert!(!cpu.status.contains(CpuFlags::CARRY));
        assert!(!cpu.status.contains(CpuFlags::ZERO));
        assert!(cpu.status.contains(CpuFlags::NEGATIVE));
    });

    test_cpu!(cmp, [0xc5, 0x56 /* CMP, 0x56 */], true, |cpu: &mut CPU| {
        cpu.write_byte(0x56, 10);
        cpu.accumulator = 10;
        run_cpu(cpu);
        assert!(cpu.status.contains(CpuFlags::CARRY));
        assert!(cpu.status.contains(CpuFlags::ZERO));
        assert!(!cpu.status.contains(CpuFlags::NEGATIVE));
    });

    test_cpu!(lda, [0xa9, 5 /* LDA, 5 */], |cpu: CPU| {
        assert_eq!(cpu.accumulator, 5);
        assert_eq!(cpu.status, CpuFlags::default());
    });

    test_cpu!(
        lda_from_memory,
        [0xa5, 0x55 /* LDA, 0x55 */],
        true,
        |cpu: &mut CPU| {
            cpu.write_byte(0x55, 10);
            run_cpu(cpu);
            assert_eq!(cpu.accumulator, 10);
        }
    );

    test_cpu!(ldy, [0xa0, 5 /* LDY, 5 */], |cpu: CPU| {
        assert_eq!(cpu.register_y, 5);
        assert_eq!(cpu.status, CpuFlags::default());
    });

    test_cpu!(ldx, [0xa2, 5 /* LDX, 5 */], |cpu: CPU| {
        assert_eq!(cpu.register_x, 5);
        assert_eq!(cpu.status, CpuFlags::default());
    });

    test_cpu!(read_write_word, |cpu: &mut CPU| {
        cpu.write_word(0x10, 0x1234);
        assert_eq!(cpu.read_word(0x10), 0x1234);
        cpu.write_word(0xfff, 0x5422);
        assert_eq!(cpu.read_word(0xfff), 0x5422);
    });

    test_cpu!(read_write_byte, |cpu: &mut CPU| {
        cpu.write_byte(0x10, 0x12);
        assert_eq!(cpu.read_byte(0x10), 0x12);
    });

    test_cpu!(update_zero_and_negative, |cpu: &mut CPU| {
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
                run_cpu(cpu);
                assert!(!cpu.status.contains($flag));
            });
        };
    }

    test_status_clear!(clv, 0xb8, CpuFlags::OVERFLOW);

    test_status_set!(sec, 0x38, CpuFlags::CARRY);
    test_status_clear!(clc, 0x18, CpuFlags::CARRY);

    test_status_clear!(cld, 0xd8, CpuFlags::DECIMAL);
    test_status_set!(sed, 0xf8, CpuFlags::DECIMAL);

    test_status_set!(sei, 0x78, CpuFlags::IRQ);
    test_status_clear!(cli, 0x58, CpuFlags::IRQ);
}
