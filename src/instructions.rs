use crate::bus::Memory;
use crate::cpu::{CpuFlags, CPU};
use lazy_static::lazy_static;

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

/// Pretty-print an instruction to the console
pub fn print_instruction(program_counter: u16, name: &'static str, params: u16) {
    println!("{:#06x}: {} {:#04x}", program_counter, name, params);
}

type InstructionContainer = Box<dyn opcodes::Instruction + Send + Sync>;

/// Match the opcode to the correct instruction
pub fn into_instruction(
    identifier: u8,
) -> Option<(&'static InstructionContainer, &'static AdressingMode)> {
    for instruction in INSTRUCTION_SET.iter() {
        for (opcode, mode) in instruction.opcodes() {
            if *opcode == identifier {
                return Some((instruction, mode));
            }
        }
    }
    None
}

lazy_static! {
    static ref INSTRUCTION_SET: Vec<InstructionContainer> = vec![
        Box::new(opcodes::Adc),
        Box::new(opcodes::And),
        Box::new(opcodes::Asl),
        Box::new(opcodes::Bcc),
        Box::new(opcodes::Bcs),
        Box::new(opcodes::Beq),
        Box::new(opcodes::Bit),
        Box::new(opcodes::Bmi),
        Box::new(opcodes::Bne),
        Box::new(opcodes::Bpl),
        Box::new(opcodes::Brk),
        Box::new(opcodes::Bvc),
        Box::new(opcodes::Bvs),
        Box::new(opcodes::Clc),
        Box::new(opcodes::Cld),
        Box::new(opcodes::Cli),
        Box::new(opcodes::Clv),
        Box::new(opcodes::Cmp),
        Box::new(opcodes::Cpx),
        Box::new(opcodes::Cpy),
        Box::new(opcodes::Dec),
        Box::new(opcodes::Dex),
        Box::new(opcodes::Dey),
        Box::new(opcodes::Eor),
        Box::new(opcodes::Inc),
        Box::new(opcodes::Inx),
        Box::new(opcodes::Iny),
        Box::new(opcodes::Jmp),
        Box::new(opcodes::Jsr),
        Box::new(opcodes::Lda),
        Box::new(opcodes::Ldx),
        Box::new(opcodes::Ldy),
        Box::new(opcodes::Lsr),
        Box::new(opcodes::Nop),
        Box::new(opcodes::Ora),
        Box::new(opcodes::Pha),
        Box::new(opcodes::Php),
        Box::new(opcodes::Pla),
        Box::new(opcodes::Plp),
        Box::new(opcodes::Rol),
        Box::new(opcodes::Ror),
        Box::new(opcodes::Rti),
        Box::new(opcodes::Rts),
        Box::new(opcodes::Sbc),
        Box::new(opcodes::Sec),
        Box::new(opcodes::Sed),
        Box::new(opcodes::Sei),
        Box::new(opcodes::Sta),
        Box::new(opcodes::Stx),
        Box::new(opcodes::Sty),
        Box::new(opcodes::Tax),
        Box::new(opcodes::Tay),
        Box::new(opcodes::Tsx),
        Box::new(opcodes::Txa),
        Box::new(opcodes::Txs),
        Box::new(opcodes::Tya),
    ];
}

pub mod opcodes {
    use super::*;

    /// Get the next program counter based on the adressing mode
    const fn consume_params(pc: u16, mode: &AdressingMode) -> u16 {
        match mode {
            AdressingMode::Implied | AdressingMode::Accumulator => pc,

            AdressingMode::Immediate
            | AdressingMode::Relative
            | AdressingMode::Indirect
            | AdressingMode::IndirectX
            | AdressingMode::IndirectY
            | AdressingMode::ZeroPage
            | AdressingMode::ZeroPageX
            | AdressingMode::ZeroPageY => pc + 1,

            AdressingMode::Absolute | AdressingMode::AbsoluteX | AdressingMode::AbsoluteY => pc + 2,
        }
    }

    /// Get the memory address of an parameter, based on the adressing mode
    fn get_params(cpu: &CPU, mode: &AdressingMode) -> u16 {
        match mode {
            AdressingMode::Immediate => cpu.program_counter,
            AdressingMode::Absolute => cpu.read_word(cpu.program_counter),
            AdressingMode::ZeroPage => cpu.read_byte(cpu.program_counter) as u16,

            AdressingMode::Relative => {
                // TODO: is this correct?
                let offset = cpu.read_byte(cpu.program_counter) as i8;
                cpu.program_counter.wrapping_add(offset as u16)
            }

            AdressingMode::ZeroPageX => cpu
                .read_byte(cpu.program_counter)
                .wrapping_add(cpu.register_x) as u16,

            AdressingMode::ZeroPageY => cpu
                .read_byte(cpu.program_counter)
                .wrapping_add(cpu.register_y) as u16,

            AdressingMode::AbsoluteX => cpu
                .read_word(cpu.program_counter)
                .wrapping_add(cpu.register_x as u16),

            AdressingMode::AbsoluteY => cpu
                .read_word(cpu.program_counter)
                .wrapping_add(cpu.register_y as u16),

            AdressingMode::Indirect => {
                // TODO: ignoring page boundary bug
                let ptr = cpu.read_word(cpu.program_counter);

                u16::from_le_bytes([
                    cpu.read_byte(ptr as u16),
                    cpu.read_byte((ptr as u16).wrapping_add(1)),
                ])
            }

            AdressingMode::IndirectX => {
                let ptr = cpu
                    .read_word(cpu.program_counter)
                    .wrapping_add(cpu.register_x.into());

                u16::from_le_bytes([
                    cpu.read_byte(ptr as u16),
                    cpu.read_byte((ptr as u16).wrapping_add(1)),
                ])
            }

            AdressingMode::IndirectY => {
                let ptr = cpu.read_word(cpu.program_counter);

                u16::from_le_bytes([
                    cpu.read_byte(ptr as u16),
                    cpu.read_byte((ptr as u16).wrapping_add(1)),
                ])
                .wrapping_add(cpu.register_y as u16)
            }

            AdressingMode::Implied | AdressingMode::Accumulator => {
                panic!("Addressing mode has no parameters")
            }
        }
    }

    pub trait Instruction {
        /// The opcodes used by this instruction together with the corresponding addressing mode
        fn opcodes(&self) -> &'static [(u8, AdressingMode)];

        /// The name of the instruction, used for assembly output
        fn name(&self) -> &'static str;

        /// Execute the opcode, returning the next program counter
        fn execute(&self, cpu: &mut CPU, mode: &AdressingMode) -> u16;
    }

    pub struct Nop;
    impl Instruction for Nop {
        fn name(&self) -> &'static str {
            "NOP"
        }

        fn opcodes(&self) -> &'static [(u8, AdressingMode)] {
            &[(0xEA, AdressingMode::Implied)]
        }

        fn execute(&self, cpu: &mut CPU, mode: &AdressingMode) -> u16 {
            consume_params(cpu.program_counter, mode)
        }
    }

    pub struct Brk;
    impl Instruction for Brk {
        fn name(&self) -> &'static str {
            "BRK"
        }

        fn opcodes(&self) -> &'static [(u8, AdressingMode)] {
            &[(0x00, AdressingMode::Implied)]
        }

        fn execute(&self, cpu: &mut CPU, mode: &AdressingMode) -> u16 {
            // Let the callee handle this
            consume_params(cpu.program_counter, mode)
        }
    }

    pub struct Jmp;
    impl Instruction for Jmp {
        fn name(&self) -> &'static str {
            "JMP"
        }

        fn opcodes(&self) -> &'static [(u8, AdressingMode)] {
            &[
                (0x4C, AdressingMode::Absolute),
                (0x6C, AdressingMode::Indirect),
            ]
        }

        fn execute(&self, cpu: &mut CPU, mode: &AdressingMode) -> u16 {
            get_params(cpu, mode)
        }
    }

    pub struct Inx;
    impl Instruction for Inx {
        fn name(&self) -> &'static str {
            "INX"
        }

        fn opcodes(&self) -> &'static [(u8, AdressingMode)] {
            &[(0xE8, AdressingMode::Implied)]
        }

        fn execute(&self, cpu: &mut CPU, mode: &AdressingMode) -> u16 {
            cpu.register_x = cpu.register_x.wrapping_add(1);
            consume_params(cpu.program_counter, mode)
        }
    }

    pub struct Iny;
    impl Instruction for Iny {
        fn name(&self) -> &'static str {
            "INY"
        }

        fn opcodes(&self) -> &'static [(u8, AdressingMode)] {
            &[(0xC8, AdressingMode::Implied)]
        }

        fn execute(&self, cpu: &mut CPU, mode: &AdressingMode) -> u16 {
            cpu.register_y = cpu.register_y.wrapping_add(1);
            consume_params(cpu.program_counter, mode)
        }
    }

    pub struct Inc;
    impl Instruction for Inc {
        fn name(&self) -> &'static str {
            "INC"
        }

        fn opcodes(&self) -> &'static [(u8, AdressingMode)] {
            &[
                (0xE6, AdressingMode::ZeroPage),
                (0xF6, AdressingMode::ZeroPageX),
                (0xEE, AdressingMode::Absolute),
                (0xFE, AdressingMode::AbsoluteX),
            ]
        }

        fn execute(&self, cpu: &mut CPU, mode: &AdressingMode) -> u16 {
            let addr = get_params(cpu, mode);
            let value = cpu.read_byte(addr).wrapping_add(1); // Should this be a word?

            cpu.write_byte(addr, value);
            cpu.update_zero_and_negative_flags(value);

            consume_params(cpu.program_counter, mode)
        }
    }

    pub struct Rti;
    impl Instruction for Rti {
        fn name(&self) -> &'static str {
            "RTI"
        }

        fn opcodes(&self) -> &'static [(u8, AdressingMode)] {
            &[(0x40, AdressingMode::Implied)]
        }

        fn execute(&self, cpu: &mut CPU, _mode: &AdressingMode) -> u16 {
            cpu.status = CpuFlags::from_bits_truncate(cpu.stack_pop_byte());
            cpu.stack_pop_word()
        }
    }

    pub struct Adc;
    impl Instruction for Adc {
        fn name(&self) -> &'static str {
            "ADC"
        }

        fn opcodes(&self) -> &'static [(u8, AdressingMode)] {
            &[
                (0xC9, AdressingMode::Immediate),
                (0x65, AdressingMode::ZeroPage),
                (0x75, AdressingMode::ZeroPageX),
                (0x6D, AdressingMode::Absolute),
                (0x7D, AdressingMode::AbsoluteX),
                (0x79, AdressingMode::AbsoluteY),
                (0x61, AdressingMode::IndirectX),
                (0x71, AdressingMode::IndirectY),
            ]
        }

        fn execute(&self, cpu: &mut CPU, mode: &AdressingMode) -> u16 {
            let addr = get_params(cpu, mode);
            let value = cpu.read_byte(addr);

            let result = value
                .wrapping_add(cpu.accumulator)
                .wrapping_add(cpu.status.contains(CpuFlags::CARRY) as u8);

            cpu.status.set(CpuFlags::CARRY, result < value);
            cpu.status.set(
                CpuFlags::OVERFLOW,
                CPU::nth_bit(cpu.accumulator, 7) == CPU::nth_bit(value, 7)
                    && CPU::nth_bit(cpu.accumulator, 7) != CPU::nth_bit(result, 7),
            );

            cpu.accumulator = result;
            consume_params(cpu.program_counter, mode)
        }
    }

    pub struct Sbc;
    impl Instruction for Sbc {
        fn name(&self) -> &'static str {
            "SDC"
        }

        fn opcodes(&self) -> &'static [(u8, AdressingMode)] {
            &[
                (0xE9, AdressingMode::Immediate),
                (0xE5, AdressingMode::ZeroPage),
                (0xF5, AdressingMode::ZeroPageX),
                (0xED, AdressingMode::Absolute),
                (0xFD, AdressingMode::AbsoluteX),
                (0xF9, AdressingMode::AbsoluteY),
                (0xE1, AdressingMode::IndirectX),
                (0xF1, AdressingMode::IndirectY),
            ]
        }

        fn execute(&self, cpu: &mut CPU, mode: &AdressingMode) -> u16 {
            let addr = get_params(cpu, mode);
            let value = cpu.read_byte(addr);

            let result = value
                .wrapping_add(cpu.accumulator)
                .wrapping_add(cpu.status.contains(CpuFlags::CARRY) as u8);

            cpu.status.set(CpuFlags::CARRY, result < value);
            cpu.status.set(
                CpuFlags::OVERFLOW,
                CPU::nth_bit(cpu.accumulator, 7) == CPU::nth_bit(value, 7)
                    && CPU::nth_bit(cpu.accumulator, 7) != CPU::nth_bit(result, 7),
            );

            cpu.accumulator = result;
            consume_params(cpu.program_counter, mode)
        }
    }

    pub struct Cmp;
    impl Instruction for Cmp {
        fn name(&self) -> &'static str {
            "CMP"
        }

        fn opcodes(&self) -> &'static [(u8, AdressingMode)] {
            &[
                (0xC9, AdressingMode::Immediate),
                (0xC5, AdressingMode::ZeroPage),
                (0xD5, AdressingMode::ZeroPageX),
                (0xCD, AdressingMode::Absolute),
                (0xDD, AdressingMode::AbsoluteX),
                (0xD9, AdressingMode::AbsoluteY),
                (0xC1, AdressingMode::IndirectX),
                (0xD1, AdressingMode::IndirectY),
            ]
        }

        fn execute(&self, cpu: &mut CPU, mode: &AdressingMode) -> u16 {
            let addr = get_params(cpu, mode);
            let value = cpu.read_byte(addr);

            cpu.status.set(CpuFlags::CARRY, cpu.accumulator >= value);
            // Subtract so that we set the ZERO flag if the values are equal
            cpu.update_zero_and_negative_flags(cpu.accumulator.wrapping_sub(value));
            consume_params(cpu.program_counter, mode)
        }
    }

    pub struct Cpx;
    impl Instruction for Cpx {
        fn name(&self) -> &'static str {
            "CPX"
        }

        fn opcodes(&self) -> &'static [(u8, AdressingMode)] {
            &[
                (0xE0, AdressingMode::Immediate),
                (0xE4, AdressingMode::ZeroPage),
                (0xEC, AdressingMode::Absolute),
            ]
        }

        fn execute(&self, cpu: &mut CPU, mode: &AdressingMode) -> u16 {
            let addr = get_params(cpu, mode);
            let value = cpu.read_byte(addr);

            cpu.status.set(CpuFlags::CARRY, cpu.register_x >= value);
            // Subtract so that we set the ZERO flag if the values are equal
            cpu.update_zero_and_negative_flags(cpu.register_x.wrapping_sub(value));
            consume_params(cpu.program_counter, mode)
        }
    }

    pub struct Cpy;
    impl Instruction for Cpy {
        fn name(&self) -> &'static str {
            "CPY"
        }

        fn opcodes(&self) -> &'static [(u8, AdressingMode)] {
            &[
                (0xC0, AdressingMode::Immediate),
                (0xC4, AdressingMode::ZeroPage),
                (0xCC, AdressingMode::Absolute),
            ]
        }

        fn execute(&self, cpu: &mut CPU, mode: &AdressingMode) -> u16 {
            let addr = get_params(cpu, mode);
            let value = cpu.read_byte(addr);

            cpu.status.set(CpuFlags::CARRY, cpu.register_y >= value);
            // Subtract so that we set the ZERO flag if the values are equal
            cpu.update_zero_and_negative_flags(cpu.register_y.wrapping_sub(value));
            consume_params(cpu.program_counter, mode)
        }
    }

    pub struct Dec;
    impl Instruction for Dec {
        fn name(&self) -> &'static str {
            "DEC"
        }

        fn opcodes(&self) -> &'static [(u8, AdressingMode)] {
            &[
                (0xC6, AdressingMode::ZeroPage),
                (0xD6, AdressingMode::ZeroPageX),
                (0xCE, AdressingMode::Absolute),
                (0xDE, AdressingMode::AbsoluteX),
            ]
        }

        fn execute(&self, cpu: &mut CPU, mode: &AdressingMode) -> u16 {
            let addr = get_params(cpu, mode);
            let value = cpu.read_byte(addr).wrapping_sub(1); // Should this be a word?

            cpu.write_byte(addr, value);
            cpu.update_zero_and_negative_flags(value);
            consume_params(cpu.program_counter, mode)
        }
    }

    pub struct Dey;
    impl Instruction for Dey {
        fn name(&self) -> &'static str {
            "DEY"
        }

        fn opcodes(&self) -> &'static [(u8, AdressingMode)] {
            &[(0x88, AdressingMode::Implied)]
        }

        fn execute(&self, cpu: &mut CPU, mode: &AdressingMode) -> u16 {
            cpu.register_y = cpu.register_y.wrapping_sub(1);
            cpu.update_zero_and_negative_flags(cpu.register_y);
            consume_params(cpu.program_counter, mode)
        }
    }

    pub struct Dex;
    impl Instruction for Dex {
        fn name(&self) -> &'static str {
            "DEX"
        }

        fn opcodes(&self) -> &'static [(u8, AdressingMode)] {
            &[(0xCA, AdressingMode::Implied)]
        }

        fn execute(&self, cpu: &mut CPU, mode: &AdressingMode) -> u16 {
            cpu.register_x = cpu.register_x.wrapping_sub(1);
            cpu.update_zero_and_negative_flags(cpu.register_x);
            consume_params(cpu.program_counter, mode)
        }
    }

    pub struct Bcs;
    impl Instruction for Bcs {
        fn name(&self) -> &'static str {
            "BCS"
        }

        fn opcodes(&self) -> &'static [(u8, AdressingMode)] {
            &[(0xB0, AdressingMode::Relative)]
        }

        fn execute(&self, cpu: &mut CPU, mode: &AdressingMode) -> u16 {
            if cpu.status.contains(CpuFlags::CARRY) {
                get_params(cpu, mode)
            } else {
                consume_params(cpu.program_counter, mode)
            }
        }
    }

    pub struct Bcc;
    impl Instruction for Bcc {
        fn name(&self) -> &'static str {
            "BCC"
        }

        fn opcodes(&self) -> &'static [(u8, AdressingMode)] {
            &[(0x90, AdressingMode::Relative)]
        }

        fn execute(&self, cpu: &mut CPU, mode: &AdressingMode) -> u16 {
            if !cpu.status.contains(CpuFlags::CARRY) {
                get_params(cpu, mode)
            } else {
                consume_params(cpu.program_counter, mode)
            }
        }
    }

    pub struct Beq;
    impl Instruction for Beq {
        fn name(&self) -> &'static str {
            "BEQ"
        }

        fn opcodes(&self) -> &'static [(u8, AdressingMode)] {
            &[(0xF0, AdressingMode::Relative)]
        }

        fn execute(&self, cpu: &mut CPU, mode: &AdressingMode) -> u16 {
            if cpu.status.contains(CpuFlags::ZERO) {
                get_params(cpu, mode)
            } else {
                consume_params(cpu.program_counter, mode)
            }
        }
    }

    pub struct Bne;
    impl Instruction for Bne {
        fn name(&self) -> &'static str {
            "BNE"
        }

        fn opcodes(&self) -> &'static [(u8, AdressingMode)] {
            &[(0xD0, AdressingMode::Relative)]
        }

        fn execute(&self, cpu: &mut CPU, mode: &AdressingMode) -> u16 {
            if !cpu.status.contains(CpuFlags::ZERO) {
                get_params(cpu, mode)
            } else {
                consume_params(cpu.program_counter, mode)
            }
        }
    }

    pub struct Bmi;
    impl Instruction for Bmi {
        fn name(&self) -> &'static str {
            "BMI"
        }

        fn opcodes(&self) -> &'static [(u8, AdressingMode)] {
            &[(0x30, AdressingMode::Relative)]
        }

        fn execute(&self, cpu: &mut CPU, mode: &AdressingMode) -> u16 {
            if cpu.status.contains(CpuFlags::NEGATIVE) {
                get_params(cpu, mode)
            } else {
                consume_params(cpu.program_counter, mode)
            }
        }
    }

    pub struct Bpl;
    impl Instruction for Bpl {
        fn name(&self) -> &'static str {
            "BLP"
        }

        fn opcodes(&self) -> &'static [(u8, AdressingMode)] {
            &[(0x10, AdressingMode::Relative)]
        }

        fn execute(&self, cpu: &mut CPU, mode: &AdressingMode) -> u16 {
            if !cpu.status.contains(CpuFlags::NEGATIVE) {
                get_params(cpu, mode)
            } else {
                consume_params(cpu.program_counter, mode)
            }
        }
    }

    pub struct Bvs;
    impl Instruction for Bvs {
        fn name(&self) -> &'static str {
            "BVS"
        }

        fn opcodes(&self) -> &'static [(u8, AdressingMode)] {
            &[(0x70, AdressingMode::Relative)]
        }

        fn execute(&self, cpu: &mut CPU, mode: &AdressingMode) -> u16 {
            if cpu.status.contains(CpuFlags::OVERFLOW) {
                get_params(cpu, mode)
            } else {
                consume_params(cpu.program_counter, mode)
            }
        }
    }

    pub struct Bvc;
    impl Instruction for Bvc {
        fn name(&self) -> &'static str {
            "BVC"
        }

        fn opcodes(&self) -> &'static [(u8, AdressingMode)] {
            &[(0x50, AdressingMode::Relative)]
        }

        fn execute(&self, cpu: &mut CPU, mode: &AdressingMode) -> u16 {
            if !cpu.status.contains(CpuFlags::OVERFLOW) {
                get_params(cpu, mode)
            } else {
                consume_params(cpu.program_counter, mode)
            }
        }
    }

    pub struct Php;
    impl Instruction for Php {
        fn name(&self) -> &'static str {
            "PHP"
        }

        fn opcodes(&self) -> &'static [(u8, AdressingMode)] {
            &[(0x08, AdressingMode::Implied)]
        }

        fn execute(&self, cpu: &mut CPU, mode: &AdressingMode) -> u16 {
            // See https://www.nesdev.org/wiki/Status_flags#The_B_flag
            let mut status = cpu.status.clone();
            status.insert(CpuFlags::BREAK);
            status.insert(CpuFlags::BREAK2);
            cpu.stack_push_byte(status.bits());
            consume_params(cpu.program_counter, mode)
        }
    }

    pub struct Plp;
    impl Instruction for Plp {
        fn name(&self) -> &'static str {
            "PLP"
        }

        fn opcodes(&self) -> &'static [(u8, AdressingMode)] {
            &[(0x28, AdressingMode::Implied)]
        }

        fn execute(&self, cpu: &mut CPU, mode: &AdressingMode) -> u16 {
            // See https://www.nesdev.org/wiki/Status_flags#The_B_flag
            cpu.status = CpuFlags::from_bits_truncate(cpu.stack_pop_byte());
            cpu.status.remove(CpuFlags::BREAK);
            cpu.status.insert(CpuFlags::BREAK2);
            consume_params(cpu.program_counter, mode)
        }
    }

    pub struct Pha;
    impl Instruction for Pha {
        fn name(&self) -> &'static str {
            "PHA"
        }

        fn opcodes(&self) -> &'static [(u8, AdressingMode)] {
            &[(0x48, AdressingMode::Implied)]
        }

        fn execute(&self, cpu: &mut CPU, mode: &AdressingMode) -> u16 {
            cpu.stack_push_byte(cpu.accumulator);
            consume_params(cpu.program_counter, mode)
        }
    }

    pub struct Pla;
    impl Instruction for Pla {
        fn name(&self) -> &'static str {
            "PLA"
        }

        fn opcodes(&self) -> &'static [(u8, AdressingMode)] {
            &[(0x68, AdressingMode::Implied)]
        }

        fn execute(&self, cpu: &mut CPU, mode: &AdressingMode) -> u16 {
            cpu.stack_push_byte(cpu.accumulator);
            cpu.update_zero_and_negative_flags(cpu.accumulator);
            consume_params(cpu.program_counter, mode)
        }
    }

    pub struct Tax;
    impl Instruction for Tax {
        fn name(&self) -> &'static str {
            "TAX"
        }

        fn opcodes(&self) -> &'static [(u8, AdressingMode)] {
            &[(0xAA, AdressingMode::Implied)]
        }

        fn execute(&self, cpu: &mut CPU, mode: &AdressingMode) -> u16 {
            cpu.register_x = cpu.accumulator;
            cpu.update_zero_and_negative_flags(cpu.register_x);
            consume_params(cpu.program_counter, mode)
        }
    }

    pub struct Txa;
    impl Instruction for Txa {
        fn name(&self) -> &'static str {
            "TXA"
        }

        fn opcodes(&self) -> &'static [(u8, AdressingMode)] {
            &[(0x8A, AdressingMode::Implied)]
        }

        fn execute(&self, cpu: &mut CPU, mode: &AdressingMode) -> u16 {
            cpu.accumulator = cpu.register_x;
            cpu.update_zero_and_negative_flags(cpu.accumulator);
            consume_params(cpu.program_counter, mode)
        }
    }

    pub struct Tay;
    impl Instruction for Tay {
        fn name(&self) -> &'static str {
            "TAY"
        }

        fn opcodes(&self) -> &'static [(u8, AdressingMode)] {
            &[(0xA8, AdressingMode::Implied)]
        }

        fn execute(&self, cpu: &mut CPU, mode: &AdressingMode) -> u16 {
            cpu.register_y = cpu.accumulator;
            cpu.update_zero_and_negative_flags(cpu.register_y);
            consume_params(cpu.program_counter, mode)
        }
    }

    pub struct Tya;
    impl Instruction for Tya {
        fn name(&self) -> &'static str {
            "TYA"
        }

        fn opcodes(&self) -> &'static [(u8, AdressingMode)] {
            &[(0x98, AdressingMode::Implied)]
        }

        fn execute(&self, cpu: &mut CPU, mode: &AdressingMode) -> u16 {
            cpu.accumulator = cpu.register_y;
            cpu.update_zero_and_negative_flags(cpu.accumulator);
            consume_params(cpu.program_counter, mode)
        }
    }

    pub struct Clv;
    impl Instruction for Clv {
        fn name(&self) -> &'static str {
            "CLV"
        }

        fn opcodes(&self) -> &'static [(u8, AdressingMode)] {
            &[(0xB8, AdressingMode::Implied)]
        }

        fn execute(&self, cpu: &mut CPU, mode: &AdressingMode) -> u16 {
            cpu.status.remove(CpuFlags::OVERFLOW);
            consume_params(cpu.program_counter, mode)
        }
    }

    pub struct Clc;
    impl Instruction for Clc {
        fn name(&self) -> &'static str {
            "CLC"
        }

        fn opcodes(&self) -> &'static [(u8, AdressingMode)] {
            &[(0x18, AdressingMode::Implied)]
        }

        fn execute(&self, cpu: &mut CPU, mode: &AdressingMode) -> u16 {
            cpu.status.remove(CpuFlags::CARRY);
            consume_params(cpu.program_counter, mode)
        }
    }

    pub struct Cld;
    impl Instruction for Cld {
        fn name(&self) -> &'static str {
            "CLD"
        }

        fn opcodes(&self) -> &'static [(u8, AdressingMode)] {
            &[(0xD8, AdressingMode::Implied)]
        }

        fn execute(&self, cpu: &mut CPU, mode: &AdressingMode) -> u16 {
            cpu.status.remove(CpuFlags::DECIMAL);
            consume_params(cpu.program_counter, mode)
        }
    }

    pub struct Sec;
    impl Instruction for Sec {
        fn name(&self) -> &'static str {
            "SEC"
        }

        fn opcodes(&self) -> &'static [(u8, AdressingMode)] {
            &[(0x38, AdressingMode::Implied)]
        }

        fn execute(&self, cpu: &mut CPU, mode: &AdressingMode) -> u16 {
            cpu.status.insert(CpuFlags::CARRY);
            consume_params(cpu.program_counter, mode)
        }
    }

    pub struct Sed;
    impl Instruction for Sed {
        fn name(&self) -> &'static str {
            "SED"
        }

        fn opcodes(&self) -> &'static [(u8, AdressingMode)] {
            &[(0xF8, AdressingMode::Implied)]
        }

        fn execute(&self, cpu: &mut CPU, mode: &AdressingMode) -> u16 {
            cpu.status.insert(CpuFlags::DECIMAL);
            consume_params(cpu.program_counter, mode)
        }
    }

    pub struct Sei;
    impl Instruction for Sei {
        fn name(&self) -> &'static str {
            "SEI"
        }

        fn opcodes(&self) -> &'static [(u8, AdressingMode)] {
            &[(0x78, AdressingMode::Implied)]
        }

        fn execute(&self, cpu: &mut CPU, mode: &AdressingMode) -> u16 {
            cpu.status.insert(CpuFlags::IRQ);
            consume_params(cpu.program_counter, mode)
        }
    }

    pub struct Cli;
    impl Instruction for Cli {
        fn name(&self) -> &'static str {
            "CLI"
        }

        fn opcodes(&self) -> &'static [(u8, AdressingMode)] {
            &[(0x58, AdressingMode::Implied)]
        }

        fn execute(&self, cpu: &mut CPU, mode: &AdressingMode) -> u16 {
            cpu.status.remove(CpuFlags::IRQ);
            consume_params(cpu.program_counter, mode)
        }
    }

    pub struct Jsr;
    impl Instruction for Jsr {
        fn name(&self) -> &'static str {
            "JSR"
        }

        fn opcodes(&self) -> &'static [(u8, AdressingMode)] {
            &[(0x20, AdressingMode::Absolute)]
        }

        fn execute(&self, cpu: &mut CPU, mode: &AdressingMode) -> u16 {
            let addr = get_params(cpu, mode);
            let new_pc = consume_params(cpu.program_counter, mode);
            cpu.stack_push_word(new_pc - 1);
            addr
        }
    }

    pub struct Rts;
    impl Instruction for Rts {
        fn name(&self) -> &'static str {
            "RTS"
        }

        fn opcodes(&self) -> &'static [(u8, AdressingMode)] {
            &[(0x60, AdressingMode::Implied)]
        }

        fn execute(&self, cpu: &mut CPU, mode: &AdressingMode) -> u16 {
            let addr = cpu.stack_pop_word();
            consume_params(addr + 1, mode)
        }
    }

    pub struct Lsr;
    impl Instruction for Lsr {
        fn name(&self) -> &'static str {
            "LSR"
        }

        fn opcodes(&self) -> &'static [(u8, AdressingMode)] {
            &[
                (0x4A, AdressingMode::Accumulator),
                (0x46, AdressingMode::ZeroPage),
                (0x56, AdressingMode::ZeroPageX),
                (0x4E, AdressingMode::Absolute),
                (0x5E, AdressingMode::AbsoluteX),
            ]
        }

        fn execute(&self, cpu: &mut CPU, mode: &AdressingMode) -> u16 {
            let addr = get_params(cpu, mode);

            let result = if mode == &AdressingMode::Accumulator {
                cpu.status
                    .set(CpuFlags::CARRY, CPU::nth_bit(cpu.accumulator, 0));
                cpu.accumulator >>= 1;
                cpu.accumulator
            } else {
                let mut value = cpu.read_byte(addr);
                cpu.status.set(CpuFlags::CARRY, CPU::nth_bit(value, 0));
                value >>= 1;
                cpu.write_byte(addr, value);
                value
            };

            cpu.update_zero_and_negative_flags(result);
            consume_params(cpu.program_counter, mode)
        }
    }

    pub struct Asl;
    impl Instruction for Asl {
        fn name(&self) -> &'static str {
            "ASL"
        }

        fn opcodes(&self) -> &'static [(u8, AdressingMode)] {
            &[
                (0x0A, AdressingMode::Accumulator),
                (0x06, AdressingMode::ZeroPage),
                (0x16, AdressingMode::ZeroPageX),
                (0x0E, AdressingMode::Absolute),
                (0x1E, AdressingMode::AbsoluteX),
            ]
        }

        fn execute(&self, cpu: &mut CPU, mode: &AdressingMode) -> u16 {
            let addr = get_params(cpu, mode);

            let result = if mode == &AdressingMode::Accumulator {
                cpu.status
                    .set(CpuFlags::CARRY, CPU::nth_bit(cpu.accumulator, 7));
                cpu.accumulator <<= 1;
                cpu.accumulator
            } else {
                let mut value = cpu.read_byte(addr);
                cpu.status.set(CpuFlags::CARRY, CPU::nth_bit(value, 7));
                value <<= 1;
                cpu.write_byte(addr, value);
                value
            };

            cpu.update_zero_and_negative_flags(result);
            consume_params(cpu.program_counter, mode)
        }
    }

    pub struct Ror;
    impl Instruction for Ror {
        fn name(&self) -> &'static str {
            "ROR"
        }

        fn opcodes(&self) -> &'static [(u8, AdressingMode)] {
            &[
                (0x6A, AdressingMode::Accumulator),
                (0x66, AdressingMode::ZeroPage),
                (0x76, AdressingMode::ZeroPageX),
                (0x6E, AdressingMode::Absolute),
                (0x7E, AdressingMode::AbsoluteX),
            ]
        }

        fn execute(&self, cpu: &mut CPU, mode: &AdressingMode) -> u16 {
            let addr = get_params(cpu, mode);

            let result = if mode == &AdressingMode::Accumulator {
                cpu.status
                    .set(CpuFlags::CARRY, CPU::nth_bit(cpu.accumulator, 0));
                cpu.accumulator = cpu.accumulator.rotate_right(1);
                cpu.accumulator
            } else {
                let mut value = cpu.read_byte(addr);
                cpu.status.set(CpuFlags::CARRY, CPU::nth_bit(value, 0));
                value = value.rotate_right(1);
                cpu.write_byte(addr, value);
                value
            };

            cpu.update_zero_and_negative_flags(result);
            consume_params(cpu.program_counter, mode)
        }
    }

    pub struct Rol;
    impl Instruction for Rol {
        fn name(&self) -> &'static str {
            "ROL"
        }

        fn opcodes(&self) -> &'static [(u8, AdressingMode)] {
            &[
                (0x2A, AdressingMode::Accumulator),
                (0x26, AdressingMode::ZeroPage),
                (0x36, AdressingMode::ZeroPageX),
                (0x2E, AdressingMode::Absolute),
                (0x3E, AdressingMode::AbsoluteX),
            ]
        }

        fn execute(&self, cpu: &mut CPU, mode: &AdressingMode) -> u16 {
            let addr = get_params(cpu, mode);

            let result = if mode == &AdressingMode::Accumulator {
                cpu.status
                    .set(CpuFlags::CARRY, CPU::nth_bit(cpu.accumulator, 7));
                cpu.accumulator = cpu.accumulator.rotate_left(1);
                cpu.accumulator
            } else {
                let mut value = cpu.read_byte(addr);
                cpu.status.set(CpuFlags::CARRY, CPU::nth_bit(value, 7));
                value = value.rotate_left(1);
                cpu.write_byte(addr, value);
                value
            };

            cpu.update_zero_and_negative_flags(result);
            consume_params(cpu.program_counter, mode)
        }
    }

    pub struct And;
    impl Instruction for And {
        fn name(&self) -> &'static str {
            "AND"
        }

        fn opcodes(&self) -> &'static [(u8, AdressingMode)] {
            &[
                (0x29, AdressingMode::Immediate),
                (0x25, AdressingMode::ZeroPage),
                (0x35, AdressingMode::ZeroPageX),
                (0x2D, AdressingMode::Absolute),
                (0x3D, AdressingMode::AbsoluteX),
                (0x39, AdressingMode::AbsoluteY),
                (0x21, AdressingMode::IndirectX),
                (0x31, AdressingMode::IndirectY),
            ]
        }

        fn execute(&self, cpu: &mut CPU, mode: &AdressingMode) -> u16 {
            let addr = get_params(cpu, mode);
            let value = cpu.read_byte(addr);

            cpu.accumulator &= value;
            cpu.update_zero_and_negative_flags(cpu.accumulator);

            consume_params(cpu.program_counter, mode)
        }
    }

    pub struct Eor;
    impl Instruction for Eor {
        fn name(&self) -> &'static str {
            "EOR"
        }

        fn opcodes(&self) -> &'static [(u8, AdressingMode)] {
            &[
                (0x49, AdressingMode::Immediate),
                (0x45, AdressingMode::ZeroPage),
                (0x55, AdressingMode::ZeroPageX),
                (0x4D, AdressingMode::Absolute),
                (0x5D, AdressingMode::AbsoluteX),
                (0x59, AdressingMode::AbsoluteY),
                (0x41, AdressingMode::IndirectX),
                (0x51, AdressingMode::IndirectY),
            ]
        }

        fn execute(&self, cpu: &mut CPU, mode: &AdressingMode) -> u16 {
            let addr = get_params(cpu, mode);
            let value = cpu.read_byte(addr);

            cpu.accumulator ^= value;
            cpu.update_zero_and_negative_flags(cpu.accumulator);

            consume_params(cpu.program_counter, mode)
        }
    }

    pub struct Ora;
    impl Instruction for Ora {
        fn name(&self) -> &'static str {
            "ORA"
        }

        fn opcodes(&self) -> &'static [(u8, AdressingMode)] {
            &[
                (0x09, AdressingMode::Immediate),
                (0x05, AdressingMode::ZeroPage),
                (0x15, AdressingMode::ZeroPageX),
                (0x0D, AdressingMode::Absolute),
                (0x1D, AdressingMode::AbsoluteX),
                (0x19, AdressingMode::AbsoluteY),
                (0x01, AdressingMode::IndirectX),
                (0x11, AdressingMode::IndirectY),
            ]
        }

        fn execute(&self, cpu: &mut CPU, mode: &AdressingMode) -> u16 {
            let addr = get_params(cpu, mode);
            let value = cpu.read_byte(addr);

            cpu.accumulator |= value;
            cpu.update_zero_and_negative_flags(cpu.accumulator);

            consume_params(cpu.program_counter, mode)
        }
    }

    pub struct Lda;
    impl Instruction for Lda {
        fn name(&self) -> &'static str {
            "LDA"
        }

        fn opcodes(&self) -> &'static [(u8, AdressingMode)] {
            &[
                (0xA9, AdressingMode::Immediate),
                (0xA5, AdressingMode::ZeroPage),
                (0xB5, AdressingMode::ZeroPageX),
                (0xAD, AdressingMode::Absolute),
                (0xBD, AdressingMode::AbsoluteX),
                (0xB9, AdressingMode::AbsoluteY),
                (0xA1, AdressingMode::IndirectX),
                (0xB1, AdressingMode::IndirectY),
            ]
        }

        fn execute(&self, cpu: &mut CPU, mode: &AdressingMode) -> u16 {
            let addr = get_params(cpu, mode);
            let value = cpu.read_byte(addr);

            cpu.accumulator = value;
            cpu.update_zero_and_negative_flags(cpu.accumulator);

            consume_params(cpu.program_counter, mode)
        }
    }

    pub struct Ldx;
    impl Instruction for Ldx {
        fn name(&self) -> &'static str {
            "LDX"
        }

        fn opcodes(&self) -> &'static [(u8, AdressingMode)] {
            &[
                (0xA2, AdressingMode::Immediate),
                (0xA6, AdressingMode::ZeroPage),
                (0xB6, AdressingMode::ZeroPageY),
                (0xAE, AdressingMode::Absolute),
                (0xBE, AdressingMode::AbsoluteY),
            ]
        }

        fn execute(&self, cpu: &mut CPU, mode: &AdressingMode) -> u16 {
            let addr = get_params(cpu, mode);
            let value = cpu.read_byte(addr);

            cpu.register_x = value;
            cpu.update_zero_and_negative_flags(cpu.register_x);

            consume_params(cpu.program_counter, mode)
        }
    }

    pub struct Ldy;
    impl Instruction for Ldy {
        fn name(&self) -> &'static str {
            "LDY"
        }

        fn opcodes(&self) -> &'static [(u8, AdressingMode)] {
            &[
                (0xA0, AdressingMode::Immediate),
                (0xA4, AdressingMode::ZeroPage),
                (0xB4, AdressingMode::ZeroPageX),
                (0xAC, AdressingMode::Absolute),
                (0xBC, AdressingMode::AbsoluteX),
            ]
        }

        fn execute(&self, cpu: &mut CPU, mode: &AdressingMode) -> u16 {
            let addr = get_params(cpu, mode);
            let value = cpu.read_byte(addr);

            cpu.register_y = value;
            cpu.update_zero_and_negative_flags(cpu.register_y);

            consume_params(cpu.program_counter, mode)
        }
    }

    pub struct Sta;
    impl Instruction for Sta {
        fn name(&self) -> &'static str {
            "STA"
        }

        fn opcodes(&self) -> &'static [(u8, AdressingMode)] {
            &[
                (0x85, AdressingMode::ZeroPage),
                (0x95, AdressingMode::ZeroPageX),
                (0x8D, AdressingMode::Absolute),
                (0x9D, AdressingMode::AbsoluteX),
                (0x99, AdressingMode::AbsoluteY),
                (0x81, AdressingMode::IndirectX),
                (0x91, AdressingMode::IndirectY),
            ]
        }

        fn execute(&self, cpu: &mut CPU, mode: &AdressingMode) -> u16 {
            let addr = get_params(cpu, mode);
            cpu.write_byte(addr, cpu.accumulator);
            consume_params(cpu.program_counter, mode)
        }
    }

    pub struct Stx;
    impl Instruction for Stx {
        fn name(&self) -> &'static str {
            "STX"
        }

        fn opcodes(&self) -> &'static [(u8, AdressingMode)] {
            &[
                (0x86, AdressingMode::ZeroPage),
                (0x96, AdressingMode::ZeroPageY),
                (0x8E, AdressingMode::Absolute),
            ]
        }

        fn execute(&self, cpu: &mut CPU, mode: &AdressingMode) -> u16 {
            let addr = get_params(cpu, mode);
            cpu.write_byte(addr, cpu.register_x);
            consume_params(cpu.program_counter, mode)
        }
    }

    pub struct Sty;
    impl Instruction for Sty {
        fn name(&self) -> &'static str {
            "STY"
        }

        fn opcodes(&self) -> &'static [(u8, AdressingMode)] {
            &[
                (0x84, AdressingMode::ZeroPage),
                (0x94, AdressingMode::ZeroPageX),
                (0x8C, AdressingMode::Absolute),
            ]
        }

        fn execute(&self, cpu: &mut CPU, mode: &AdressingMode) -> u16 {
            let addr = get_params(cpu, mode);
            cpu.write_byte(addr, cpu.register_y);
            consume_params(cpu.program_counter, mode)
        }
    }

    pub struct Bit;
    impl Instruction for Bit {
        fn name(&self) -> &'static str {
            "BIT"
        }

        fn opcodes(&self) -> &'static [(u8, AdressingMode)] {
            &[
                (0x24, AdressingMode::ZeroPage),
                (0x2C, AdressingMode::Absolute),
            ]
        }

        fn execute(&self, cpu: &mut CPU, mode: &AdressingMode) -> u16 {
            let addr = get_params(cpu, mode);
            let value = cpu.read_byte(addr);

            cpu.status
                .set(CpuFlags::ZERO, (cpu.accumulator & value) == 0);
            cpu.status.set(CpuFlags::NEGATIVE, CPU::nth_bit(value, 7));
            cpu.status.set(CpuFlags::OVERFLOW, CPU::nth_bit(value, 6));

            consume_params(cpu.program_counter, mode)
        }
    }

    pub struct Tsx;
    impl Instruction for Tsx {
        fn name(&self) -> &'static str {
            "TSX"
        }

        fn opcodes(&self) -> &'static [(u8, AdressingMode)] {
            &[(0xBA, AdressingMode::Implied)]
        }

        fn execute(&self, cpu: &mut CPU, mode: &AdressingMode) -> u16 {
            cpu.register_x = cpu.stack_pointer;
            cpu.update_zero_and_negative_flags(cpu.register_x);
            consume_params(cpu.program_counter, mode)
        }
    }

    pub struct Txs;
    impl Instruction for Txs {
        fn name(&self) -> &'static str {
            "TXS"
        }

        fn opcodes(&self) -> &'static [(u8, AdressingMode)] {
            &[(0x9A, AdressingMode::Implied)]
        }

        fn execute(&self, cpu: &mut CPU, mode: &AdressingMode) -> u16 {
            cpu.stack_pointer = cpu.register_x;
            consume_params(cpu.program_counter, mode)
        }
    }
}
