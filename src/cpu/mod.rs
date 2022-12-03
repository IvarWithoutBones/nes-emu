mod instructions;
pub mod assembler;

use crate::bus::{Bus, Memory, PROGRAM_ROM_START};
use bitflags::bitflags;
use instructions::{execute_instruction, format_instruction, instruction_name, parse_instruction};
use std::fmt;

bitflags! {
    #[rustfmt::skip]
    #[derive(Debug, Clone, PartialEq)]
    pub struct CpuFlags: u8 {
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
    const STACK_OFFSET: u16 = 0x0100;
    const STACK_RESET: u8 = 0xFD;

    pub fn new(bus: Bus) -> CPU {
        CPU {
            program_counter: PROGRAM_ROM_START,
            stack_pointer: CPU::STACK_RESET,
            status: CpuFlags::empty(),
            accumulator: 0,
            register_x: 0,
            register_y: 0,
            bus,
        }
    }

    pub fn reset(&mut self) {
        self.status = CpuFlags::empty();
        self.stack_pointer = CPU::STACK_RESET;
        self.accumulator = 0;
        self.register_x = 0;
        self.register_y = 0;
        self.program_counter = PROGRAM_ROM_START as u16;
    }

    /*
      Helpers
    */

    pub const fn nth_bit(value: u8, n: u8) -> bool {
        value & (1 << n) != 0
    }

    pub fn stack_push_byte(&mut self, data: u8) {
        self.write_byte(CPU::STACK_OFFSET + self.stack_pointer as u16, data);
        self.stack_pointer = self.stack_pointer.wrapping_sub(1);
    }

    pub fn stack_pop_byte(&mut self) -> u8 {
        self.stack_pointer = self.stack_pointer.wrapping_add(1);
        self.read_byte(CPU::STACK_OFFSET + self.stack_pointer as u16)
    }

    pub fn stack_push_word(&mut self, data: u16) {
        let bytes = u16::to_le_bytes(data);
        self.stack_push_byte(bytes[0]);
        self.stack_push_byte(bytes[1]);
    }

    pub fn stack_pop_word(&mut self) -> u16 {
        u16::from_le_bytes([self.stack_pop_byte(), self.stack_pop_byte()])
    }

    pub fn update_zero_and_negative_flags(&mut self, value: u8) {
        self.status.set(CpuFlags::NEGATIVE, CPU::nth_bit(value, 7));
        self.status.set(CpuFlags::ZERO, value == 0);
    }

    pub fn run(&mut self) {
        loop {
            let opcode = self.read_byte(self.program_counter);
            let (instr, mode) =
                parse_instruction(opcode).expect(format!("Invalid opcode {}", opcode).as_str());

            if !self.bus.quiet {
                let instr_str = format_instruction(self, instr, mode);
                println!("{0: <24}\t{1:}", instr_str, self);
            }

            if instruction_name(instr) == "BRK" {
                break;
            };

            self.program_counter = execute_instruction(self, instr, mode);
        }
    }
}

impl fmt::Display for CPU {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "A: {}, X: {}, Y: {}, SP: {:#x}",
            self.accumulator, self.register_x, self.register_y, self.stack_pointer
        )
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
                let mut cpu = CPU::new(Bus::new(cart, true));
                cpu.run();
                $callback(cpu);
            }
        };

        ($test_name: ident, $asm:expr, $dont_execute:expr, $callback:expr) => {
            #[test]
            fn $test_name() {
                let cart = Cartridge::new($asm.to_vec()).unwrap();
                let mut cpu = CPU::new(Bus::new(cart, true));
                $callback(&mut cpu);
            }
        };

        ($test_name: ident, $callback:expr) => {
            #[test]
            fn $test_name() {
                let cart = Cartridge::new([0].to_vec()).unwrap();
                let mut cpu = CPU::new(Bus::new(cart, true));
                $callback(&mut cpu);
            }
        };
    }

    test_cpu!(test_cpu_init, |cpu: &mut CPU| {
        assert_eq!(cpu.accumulator, 0);
        assert_eq!(cpu.register_x, 0);
        assert_eq!(cpu.register_y, 0);
        assert_eq!(cpu.program_counter, PROGRAM_ROM_START);
        assert_eq!(cpu.status, CpuFlags::empty());
    });

    test_cpu!(test_jmp, [0x4C, 0x10, 0x00 /* JMP 0x0010 */], |cpu: CPU| {
        assert_eq!(cpu.program_counter, 0x0010);
    });

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
