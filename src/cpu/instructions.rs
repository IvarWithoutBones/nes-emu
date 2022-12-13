use crate::bus::Memory;
use crate::cpu::{AdressingMode, CpuFlags, CPU};

// This should really be a struct instead. That would remove the need of the functions below.
pub type Instruction = (
    &'static str,
    fn(cpu: &mut CPU, mode: &AdressingMode) -> u16,
    &'static [(u8, u8, &'static AdressingMode)],
);

/// Returns the instruction, addressing mode, and the number of cycles it takes to execute.
pub fn decode_instruction(
    identifier: u8,
) -> Option<(&'static Instruction, &'static AdressingMode, &'static u8)> {
    for instr in INSTRUCTIONS.iter() {
        for (opcode, cycles, mode) in instr.2 {
            if *opcode == identifier {
                return Some((instr, mode, cycles));
            }
        }
    }
    None
}

/// Execute the function associated with an instruction.
pub fn execute_instruction(
    cpu: &mut CPU,
    instr: &'static Instruction,
    mode: &AdressingMode,
) -> u16 {
    (instr.1)(cpu, mode)
}

pub fn instruction_name(instr: &'static Instruction) -> &'static str {
    instr.0
}

/// Format an instruction to a human-readable string, for debugging purposes.
pub fn format_instruction(cpu: &CPU, instr: &'static Instruction, mode: &AdressingMode) -> String {
    let mut args = String::new();

    match mode {
        &AdressingMode::Immediate => {
            args = format!("#${:02X}", cpu.read_byte(mode.fetch_param_address(cpu)));
        }

        &AdressingMode::Relative => {
            // TODO: this is hacky, instruction seems to work fine though
            args = format!("${:02X}", mode.fetch_param_address(cpu).wrapping_add(1));
        }

        &AdressingMode::ZeroPage => {
            args = format!("${:02X}", mode.fetch_param_address(cpu));
        }

        &AdressingMode::Accumulator => {
            args = "A".to_string();
        }

        _ => {
            if mode.has_arguments() {
                args = format!("${:04X}", mode.fetch_param_address(cpu))
            }
        }
    }

    format!("{0: <3} {1: <6}", instruction_name(instr), args,)
}

/// Get program counter after a instruction.
const fn consume_opcode(pc: u16, mode: &AdressingMode) -> u16 {
    pc + mode.opcode_len()
}

/// See https://www.nesdev.org/obelisk-6502-guide/reference.html
#[rustfmt::skip]
pub const INSTRUCTIONS: [Instruction; 64] = [
    ("BRK", opcodes::brk, &[(0x00, 7, &AdressingMode::Implied)]),
    ("RTI", opcodes::rti, &[(0x40, 6, &AdressingMode::Implied)]),

    // TODO: page boundary cycles
    ("BCS", opcodes::bcs, &[(0xB0, 2, &AdressingMode::Relative)]),
    ("BCC", opcodes::bcc, &[(0x90, 2, &AdressingMode::Relative)]),
    ("BEQ", opcodes::beq, &[(0xF0, 2, &AdressingMode::Relative)]),
    ("BNE", opcodes::bne, &[(0xD0, 2, &AdressingMode::Relative)]),
    ("BMI", opcodes::bmi, &[(0x30, 2, &AdressingMode::Relative)]),
    ("BPL", opcodes::bpl, &[(0x10, 2, &AdressingMode::Relative)]),
    ("BVS", opcodes::bvs, &[(0x70, 2, &AdressingMode::Relative)]),
    ("BVC", opcodes::bvc, &[(0x50, 2, &AdressingMode::Relative)]),

    ("CLV", opcodes::clv, &[(0xB8, 2, &AdressingMode::Implied)]),
    ("CLC", opcodes::clc, &[(0x18, 2, &AdressingMode::Implied)]),
    ("CLD", opcodes::cld, &[(0xD8, 2, &AdressingMode::Implied)]),
    ("CLI", opcodes::cli, &[(0x58, 2, &AdressingMode::Implied)]),
    ("SEC", opcodes::sec, &[(0x38, 2, &AdressingMode::Implied)]),
    ("SED", opcodes::sed, &[(0xF8, 2, &AdressingMode::Implied)]),
    ("SEI", opcodes::sei, &[(0x78, 2, &AdressingMode::Implied)]),

    ("TAX", opcodes::tax, &[(0xAA, 2, &AdressingMode::Implied)]),
    ("TAY", opcodes::tay, &[(0xA8, 2, &AdressingMode::Implied)]),
    ("TXA", opcodes::txa, &[(0x8A, 2, &AdressingMode::Implied)]),
    ("TYA", opcodes::tya, &[(0x98, 2, &AdressingMode::Implied)]),

    ("JSR", opcodes::jsr, &[(0x20, 6, &AdressingMode::Absolute)]),
    ("RTS", opcodes::rts, &[(0x60, 6, &AdressingMode::Implied)]),
    ("PHP", opcodes::php, &[(0x08, 3, &AdressingMode::Implied)]),
    ("PLP", opcodes::plp, &[(0x28, 4, &AdressingMode::Implied)]),
    ("PHA", opcodes::pha, &[(0x48, 3, &AdressingMode::Implied)]),
    ("PLA", opcodes::pla, &[(0x68, 4, &AdressingMode::Implied)]),
    ("TSX", opcodes::tsx, &[(0xBA, 2, &AdressingMode::Implied)]),
    ("TXS", opcodes::txs, &[(0x9A, 2, &AdressingMode::Implied)]),

    // TODO: page boundary cycles
    ("NOP", opcodes::nop, &[
        (0x80, 2, &AdressingMode::Immediate),
        (0x0C, 4, &AdressingMode::Absolute),
        (0x1C, 4, &AdressingMode::AbsoluteX),
        (0x3C, 4, &AdressingMode::AbsoluteX),
        (0x5C, 4, &AdressingMode::AbsoluteX),
        (0x7C, 4, &AdressingMode::AbsoluteX),
        (0xDC, 4, &AdressingMode::AbsoluteX),
        (0xFC, 2, &AdressingMode::AbsoluteX),
        (0xEA, 2, &AdressingMode::Implied),
        (0x1A, 2, &AdressingMode::Implied),
        (0x3A, 2, &AdressingMode::Implied),
        (0x5A, 2, &AdressingMode::Implied),
        (0x7A, 2, &AdressingMode::Implied),
        (0xDA, 2, &AdressingMode::Implied),
        (0xFA, 2, &AdressingMode::Implied),
        (0xFA, 2, &AdressingMode::Implied),
        (0x04, 3, &AdressingMode::ZeroPage),
        (0x44, 3, &AdressingMode::ZeroPage),
        (0x64, 3, &AdressingMode::ZeroPage),
        (0x14, 4, &AdressingMode::ZeroPageX),
        (0x34, 4, &AdressingMode::ZeroPageX),
        (0x54, 4, &AdressingMode::ZeroPageX),
        (0x74, 4, &AdressingMode::ZeroPageX),
        (0xD4, 4, &AdressingMode::ZeroPageX),
        (0xF4, 4, &AdressingMode::ZeroPageX),
    ]),

    ("BIT", opcodes::bit, &[
        (0x24, 3, &AdressingMode::ZeroPage),
        (0x2C, 4, &AdressingMode::Absolute),
    ]),

    ("JMP",opcodes::jmp, &[
        (0x4C, 3, &AdressingMode::Absolute),
        (0x6C, 5, &AdressingMode::Indirect),
    ]),

    ("INX", opcodes::inx, &[(0xE8, 2, &AdressingMode::Implied)]),
    ("INY", opcodes::iny, &[(0xC8, 2, &AdressingMode::Implied)]),
    ("INC", opcodes::inc, &[
        (0xE6, 5, &AdressingMode::ZeroPage),
        (0xF6, 6, &AdressingMode::ZeroPageX),
        (0xEE, 6, &AdressingMode::Absolute),
        (0xFE, 7, &AdressingMode::AbsoluteX),
    ]),

    ("DEX", opcodes::dex, &[(0xCA, 2, &AdressingMode::Implied)]),
    ("DEY", opcodes::dey, &[(0x88, 2, &AdressingMode::Implied)]),
    ("DEC", opcodes::dec, &[
        (0xC6, 5, &AdressingMode::ZeroPage),
        (0xD6, 6, &AdressingMode::ZeroPageX),
        (0xCE, 6, &AdressingMode::Absolute),
        (0xDE, 7, &AdressingMode::AbsoluteX),
    ]),

    // TODO: page boundary cycles
    ("ADC", opcodes::adc, &[
        (0x69, 2, &AdressingMode::Immediate),
        (0x65, 3, &AdressingMode::ZeroPage),
        (0x75, 4, &AdressingMode::ZeroPageX),
        (0x6D, 4, &AdressingMode::Absolute),
        (0x7D, 4, &AdressingMode::AbsoluteX),
        (0x79, 4, &AdressingMode::AbsoluteY),
        (0x61, 6, &AdressingMode::IndirectX),
        (0x71, 5, &AdressingMode::IndirectY),
    ]),

    // TODO: page boundary cycles
    ("SBC", opcodes::sbc, &[
        (0xE9, 2, &AdressingMode::Immediate),
        (0xEB, 2, &AdressingMode::Immediate), // Undocumented
        (0xE5, 3, &AdressingMode::ZeroPage),
        (0xF5, 4, &AdressingMode::ZeroPageX),
        (0xED, 4, &AdressingMode::Absolute),
        (0xFD, 4, &AdressingMode::AbsoluteX),
        (0xF9, 4, &AdressingMode::AbsoluteY),
        (0xE1, 6, &AdressingMode::IndirectX),
        (0xF1, 5, &AdressingMode::IndirectY),
    ]),

    ("LSR", opcodes::lsr, &[
        (0x4A, 2, &AdressingMode::Accumulator),
        (0x46, 5, &AdressingMode::ZeroPage),
        (0x56, 6, &AdressingMode::ZeroPageX),
        (0x4E, 6, &AdressingMode::Absolute),
        (0x5E, 7, &AdressingMode::AbsoluteX),
    ]),

    ("ASL", opcodes::asl, &[
        (0x0A, 2, &AdressingMode::Accumulator),
        (0x06, 5, &AdressingMode::ZeroPage),
        (0x16, 6, &AdressingMode::ZeroPageX),
        (0x0E, 6, &AdressingMode::Absolute),
        (0x1E, 7, &AdressingMode::AbsoluteX),
    ]),

    ("ROL", opcodes::rol, &[
        (0x2A, 2, &AdressingMode::Accumulator),
        (0x26, 5, &AdressingMode::ZeroPage),
        (0x36, 6, &AdressingMode::ZeroPageX),
        (0x2E, 6, &AdressingMode::Absolute),
        (0x3E, 7, &AdressingMode::AbsoluteX),
    ]),

    ("ROR", opcodes::ror, &[
        (0x6A, 2, &AdressingMode::Accumulator),
        (0x66, 5, &AdressingMode::ZeroPage),
        (0x76, 6, &AdressingMode::ZeroPageX),
        (0x6E, 6, &AdressingMode::Absolute),
        (0x7E, 7, &AdressingMode::AbsoluteX),
    ]),

    // TODO: page boundary cycles
    ("AND", opcodes::and, &[
        (0x29, 2, &AdressingMode::Immediate),
        (0x25, 3, &AdressingMode::ZeroPage),
        (0x35, 4, &AdressingMode::ZeroPageX),
        (0x2D, 4, &AdressingMode::Absolute),
        (0x3D, 4, &AdressingMode::AbsoluteX),
        (0x39, 4, &AdressingMode::AbsoluteY),
        (0x21, 6, &AdressingMode::IndirectX),
        (0x31, 5, &AdressingMode::IndirectY),
    ]),

    // TODO: page boundary cycles
    ("EOR", opcodes::eor, &[
        (0x49, 2, &AdressingMode::Immediate),
        (0x45, 3, &AdressingMode::ZeroPage),
        (0x55, 4, &AdressingMode::ZeroPageX),
        (0x4D, 4, &AdressingMode::Absolute),
        (0x5D, 4, &AdressingMode::AbsoluteX),
        (0x59, 4, &AdressingMode::AbsoluteY),
        (0x41, 6, &AdressingMode::IndirectX),
        (0x51, 5, &AdressingMode::IndirectY),
    ]),

    // TODO: page boundary cycles
    ("ORA", opcodes::ora, &[
        (0x09, 2, &AdressingMode::Immediate),
        (0x05, 3, &AdressingMode::ZeroPage),
        (0x15, 4, &AdressingMode::ZeroPageX),
        (0x0D, 4, &AdressingMode::Absolute),
        (0x1D, 4, &AdressingMode::AbsoluteX),
        (0x19, 4, &AdressingMode::AbsoluteY),
        (0x01, 6, &AdressingMode::IndirectX),
        (0x11, 5, &AdressingMode::IndirectY),
    ]),

    // TODO: page boundary cycles
    ("CMP", opcodes::cmp, &[
        (0xC9, 2, &AdressingMode::Immediate),
        (0xC5, 3, &AdressingMode::ZeroPage),
        (0xD5, 4, &AdressingMode::ZeroPageX),
        (0xCD, 4, &AdressingMode::Absolute),
        (0xDD, 4, &AdressingMode::AbsoluteX),
        (0xD9, 4, &AdressingMode::AbsoluteY),
        (0xC1, 6, &AdressingMode::IndirectX),
        (0xD1, 5, &AdressingMode::IndirectY),
    ]),

    ("CPX", opcodes::cpx, &[
        (0xE0, 2, &AdressingMode::Immediate),
        (0xE4, 3, &AdressingMode::ZeroPage),
        (0xEC, 4, &AdressingMode::Absolute),
    ]),

    ("CPY", opcodes::cpy, &[
        (0xC0, 2, &AdressingMode::Immediate),
        (0xC4, 3, &AdressingMode::ZeroPage),
        (0xCC, 4, &AdressingMode::Absolute),
    ]),

    // TODO: page boundary cycles
    ("LDA", opcodes::lda, &[
        (0xA9, 2, &AdressingMode::Immediate),
        (0xA5, 3, &AdressingMode::ZeroPage),
        (0xB5, 4, &AdressingMode::ZeroPageX),
        (0xAD, 4, &AdressingMode::Absolute),
        (0xBD, 4, &AdressingMode::AbsoluteX),
        (0xB9, 4, &AdressingMode::AbsoluteY),
        (0xA1, 6, &AdressingMode::IndirectX),
        (0xB1, 5, &AdressingMode::IndirectY),
    ]),

    // TODO: page boundary cycles
    ("LDX", opcodes::ldx, &[
        (0xA2, 2, &AdressingMode::Immediate),
        (0xA6, 3, &AdressingMode::ZeroPage),
        (0xB6, 4, &AdressingMode::ZeroPageY),
        (0xAE, 4, &AdressingMode::Absolute),
        (0xBE, 4, &AdressingMode::AbsoluteY),
    ]),

    // TODO: page boundary cycles
    ("LDY", opcodes::ldy, &[
        (0xA0, 2, &AdressingMode::Immediate),
        (0xA4, 3, &AdressingMode::ZeroPage),
        (0xB4, 4, &AdressingMode::ZeroPageX),
        (0xAC, 4, &AdressingMode::Absolute),
        (0xBC, 4, &AdressingMode::AbsoluteX),
    ]),

    ("STA", opcodes::sta, &[
        (0x85, 3, &AdressingMode::ZeroPage),
        (0x95, 4, &AdressingMode::ZeroPageX),
        (0x8D, 4, &AdressingMode::Absolute),
        (0x9D, 5, &AdressingMode::AbsoluteX),
        (0x99, 5, &AdressingMode::AbsoluteY),
        (0x81, 6, &AdressingMode::IndirectX),
        (0x91, 6, &AdressingMode::IndirectY),
    ]),

    ("STX", opcodes::stx, &[
        (0x86, 3, &AdressingMode::ZeroPage),
        (0x96, 4, &AdressingMode::ZeroPageY),
        (0x8E, 4, &AdressingMode::Absolute),
    ]),

    ("STY", opcodes::sty, &[
        (0x84, 3, &AdressingMode::ZeroPage),
        (0x94, 4, &AdressingMode::ZeroPageX),
        (0x8C, 4, &AdressingMode::Absolute),
    ]),

    // Unofficial opcodes

    // TODO: page boundary cycles
    ("LAX", opcodes::lax, &[
        (0xA7, 3, &AdressingMode::ZeroPage),
        (0xB7, 4, &AdressingMode::ZeroPageY),
        (0xAF, 4, &AdressingMode::Absolute),
        (0xBF, 4, &AdressingMode::AbsoluteY),
        (0xA3, 6, &AdressingMode::IndirectX),
        (0xB3, 5, &AdressingMode::IndirectY),
    ]),

    ("SAX", opcodes::sax, &[
        (0x87, 3, &AdressingMode::ZeroPage),
        (0x97, 4, &AdressingMode::ZeroPageY),
        (0x8F, 4, &AdressingMode::Absolute),
        (0x83, 6, &AdressingMode::IndirectX),
    ]),

    ("DCP", opcodes::dcp, &[
        (0xC7, 5, &AdressingMode::ZeroPage),
        (0xD7, 6, &AdressingMode::ZeroPageX),
        (0xCF, 6, &AdressingMode::Absolute),
        (0xDF, 7, &AdressingMode::AbsoluteX),
        (0xDB, 7, &AdressingMode::AbsoluteY),
        (0xC3, 8, &AdressingMode::IndirectX),
        (0xD3, 8, &AdressingMode::IndirectY),
    ]),

    ("ISB", opcodes::isb, &[
        (0xE7, 5, &AdressingMode::ZeroPage),
        (0xF7, 6, &AdressingMode::ZeroPageX),
        (0xEF, 6, &AdressingMode::Absolute),
        (0xFF, 7, &AdressingMode::AbsoluteX),
        (0xFB, 7, &AdressingMode::AbsoluteY),
        (0xE3, 8, &AdressingMode::IndirectX),
        (0xF3, 8, &AdressingMode::IndirectY),
    ]),

    ("SLO", opcodes::slo, &[
        (0x07, 5, &AdressingMode::ZeroPage),
        (0x17, 6, &AdressingMode::ZeroPageX),
        (0x0F, 6, &AdressingMode::Absolute),
        (0x1F, 7, &AdressingMode::AbsoluteX),
        (0x1B, 7, &AdressingMode::AbsoluteY),
        (0x03, 8, &AdressingMode::IndirectX),
        (0x13, 8, &AdressingMode::IndirectY),
    ]),

    ("RLA", opcodes::rla, &[
        (0x27, 5, &AdressingMode::ZeroPage),
        (0x37, 6, &AdressingMode::ZeroPageX),
        (0x2F, 6, &AdressingMode::Absolute),
        (0x3F, 7, &AdressingMode::AbsoluteX),
        (0x3B, 7, &AdressingMode::AbsoluteY),
        (0x23, 8, &AdressingMode::IndirectX),
        (0x33, 8, &AdressingMode::IndirectY),
    ]),

    ("SRE", opcodes::sre, &[
        (0x47, 5, &AdressingMode::ZeroPage),
        (0x57, 6, &AdressingMode::ZeroPageX),
        (0x4F, 6, &AdressingMode::Absolute),
        (0x5F, 7, &AdressingMode::AbsoluteX),
        (0x5B, 7, &AdressingMode::AbsoluteY),
        (0x43, 8, &AdressingMode::IndirectX),
        (0x53, 8, &AdressingMode::IndirectY),
    ]),

    ("RRA", opcodes::rra, &[
        (0x67, 5, &AdressingMode::ZeroPage),
        (0x77, 6, &AdressingMode::ZeroPageX),
        (0x6F, 6, &AdressingMode::Absolute),
        (0x7F, 7, &AdressingMode::AbsoluteX),
        (0x7B, 7, &AdressingMode::AbsoluteY),
        (0x63, 8, &AdressingMode::IndirectX),
        (0x73, 8, &AdressingMode::IndirectY),
    ])
];

mod opcodes {
    use super::*;

    pub fn nop(cpu: &mut CPU, mode: &AdressingMode) -> u16 {
        consume_opcode(cpu.program_counter, mode)
    }

    pub fn brk(cpu: &mut CPU, mode: &AdressingMode) -> u16 {
        // TODO: this is not correct, should execute code from the BRK vector
        consume_opcode(cpu.program_counter, mode)
    }

    pub fn jmp(cpu: &mut CPU, mode: &AdressingMode) -> u16 {
        mode.fetch_param_address(cpu)
    }

    pub fn inx(cpu: &mut CPU, mode: &AdressingMode) -> u16 {
        cpu.register_x = cpu.register_x.wrapping_add(1);
        cpu.update_zero_and_negative_flags(cpu.register_x);
        consume_opcode(cpu.program_counter, mode)
    }
    pub fn iny(cpu: &mut CPU, mode: &AdressingMode) -> u16 {
        cpu.register_y = cpu.register_y.wrapping_add(1);
        cpu.update_zero_and_negative_flags(cpu.register_y);
        consume_opcode(cpu.program_counter, mode)
    }

    pub fn inc(cpu: &mut CPU, mode: &AdressingMode) -> u16 {
        let addr = mode.fetch_param_address(cpu);
        let value = cpu.read_byte(addr).wrapping_add(1);

        cpu.write_byte(addr, value);
        cpu.update_zero_and_negative_flags(value);

        consume_opcode(cpu.program_counter, mode)
    }

    pub fn rti(cpu: &mut CPU, _mode: &AdressingMode) -> u16 {
        cpu.status = CpuFlags::from_bits_truncate(cpu.pop_byte());
        // See https://www.nesdev.org/wiki/Status_flags#The_B_flag
        cpu.status.remove(CpuFlags::BREAK);
        cpu.status.insert(CpuFlags::BREAK2);
        cpu.pop_word()
    }

    pub fn adc(cpu: &mut CPU, mode: &AdressingMode) -> u16 {
        let addr = mode.fetch_param_address(cpu);
        let value = cpu.read_byte(addr);

        let (data, overflow1) = cpu.accumulator.overflowing_add(value);
        let (result, overflow2) = data.overflowing_add(cpu.status.contains(CpuFlags::CARRY) as u8);

        cpu.status.set(CpuFlags::CARRY, overflow1 || overflow2);
        cpu.update_zero_and_negative_flags(result);
        cpu.status.set(
            CpuFlags::OVERFLOW,
            (((cpu.accumulator ^ result) & (value ^ result)) & 0x80) != 0,
        );

        cpu.accumulator = result;
        consume_opcode(cpu.program_counter, mode)
    }

    pub fn sbc(cpu: &mut CPU, mode: &AdressingMode) -> u16 {
        let addr = mode.fetch_param_address(cpu);
        let value = cpu.read_byte(addr);

        let (data, overflow1) = cpu.accumulator.overflowing_sub(value);
        let (result, overflow2) = data.overflowing_sub(!cpu.status.contains(CpuFlags::CARRY) as u8);

        cpu.status.set(CpuFlags::CARRY, !(overflow1 || overflow2));
        cpu.update_zero_and_negative_flags(result);
        cpu.status.set(
            CpuFlags::OVERFLOW,
            (((cpu.accumulator ^ result) & !(value ^ result)) & 0x80) != 0,
        );

        cpu.accumulator = result;
        consume_opcode(cpu.program_counter, mode)
    }

    pub fn cmp(cpu: &mut CPU, mode: &AdressingMode) -> u16 {
        let addr = mode.fetch_param_address(cpu);
        let value = cpu.read_byte(addr);

        cpu.status.set(CpuFlags::CARRY, cpu.accumulator >= value);
        // Subtract so that we set the ZERO flag if the values are equal
        cpu.update_zero_and_negative_flags(cpu.accumulator.wrapping_sub(value));
        consume_opcode(cpu.program_counter, mode)
    }

    pub fn cpx(cpu: &mut CPU, mode: &AdressingMode) -> u16 {
        let addr = mode.fetch_param_address(cpu);
        let value = cpu.read_byte(addr);

        cpu.status.set(CpuFlags::CARRY, cpu.register_x >= value);
        // Subtract so that we set the ZERO flag if the values are equal
        cpu.update_zero_and_negative_flags(cpu.register_x.wrapping_sub(value));
        consume_opcode(cpu.program_counter, mode)
    }

    pub fn cpy(cpu: &mut CPU, mode: &AdressingMode) -> u16 {
        let addr = mode.fetch_param_address(cpu);
        let value = cpu.read_byte(addr);

        cpu.status.set(CpuFlags::CARRY, cpu.register_y >= value);
        // Subtract so that we set the ZERO flag if the values are equal
        cpu.update_zero_and_negative_flags(cpu.register_y.wrapping_sub(value));
        consume_opcode(cpu.program_counter, mode)
    }

    pub fn dec(cpu: &mut CPU, mode: &AdressingMode) -> u16 {
        let addr = mode.fetch_param_address(cpu);
        let value = cpu.read_byte(addr).wrapping_sub(1);

        cpu.write_byte(addr, value);
        cpu.update_zero_and_negative_flags(value);
        consume_opcode(cpu.program_counter, mode)
    }

    pub fn dey(cpu: &mut CPU, mode: &AdressingMode) -> u16 {
        cpu.register_y = cpu.register_y.wrapping_sub(1);
        cpu.update_zero_and_negative_flags(cpu.register_y);
        consume_opcode(cpu.program_counter, mode)
    }

    pub fn dex(cpu: &mut CPU, mode: &AdressingMode) -> u16 {
        cpu.register_x = cpu.register_x.wrapping_sub(1);
        cpu.update_zero_and_negative_flags(cpu.register_x);
        consume_opcode(cpu.program_counter, mode)
    }

    pub fn bcs(cpu: &mut CPU, mode: &AdressingMode) -> u16 {
        cpu.branch(mode, cpu.status.contains(CpuFlags::CARRY))
    }

    pub fn bcc(cpu: &mut CPU, mode: &AdressingMode) -> u16 {
        cpu.branch(mode, !cpu.status.contains(CpuFlags::CARRY))
    }

    pub fn beq(cpu: &mut CPU, mode: &AdressingMode) -> u16 {
        cpu.branch(mode, cpu.status.contains(CpuFlags::ZERO))
    }

    pub fn bne(cpu: &mut CPU, mode: &AdressingMode) -> u16 {
        cpu.branch(mode, !cpu.status.contains(CpuFlags::ZERO))
    }

    pub fn bmi(cpu: &mut CPU, mode: &AdressingMode) -> u16 {
        cpu.branch(mode, cpu.status.contains(CpuFlags::NEGATIVE))
    }

    pub fn bpl(cpu: &mut CPU, mode: &AdressingMode) -> u16 {
        cpu.branch(mode, !cpu.status.contains(CpuFlags::NEGATIVE))
    }

    pub fn bvs(cpu: &mut CPU, mode: &AdressingMode) -> u16 {
        cpu.branch(mode, cpu.status.contains(CpuFlags::OVERFLOW))
    }

    pub fn bvc(cpu: &mut CPU, mode: &AdressingMode) -> u16 {
        cpu.branch(mode, !cpu.status.contains(CpuFlags::OVERFLOW))
    }

    pub fn php(cpu: &mut CPU, mode: &AdressingMode) -> u16 {
        let mut status = cpu.status.clone();
        // See https://www.nesdev.org/wiki/Status_flags#The_B_flag
        status.insert(CpuFlags::BREAK);
        status.insert(CpuFlags::BREAK2);
        cpu.push_byte(status.bits());
        consume_opcode(cpu.program_counter, mode)
    }

    pub fn plp(cpu: &mut CPU, mode: &AdressingMode) -> u16 {
        cpu.status = CpuFlags::from_bits_truncate(cpu.pop_byte());
        // See https://www.nesdev.org/wiki/Status_flags#The_B_flag
        cpu.status.remove(CpuFlags::BREAK);
        cpu.status.insert(CpuFlags::BREAK2);
        consume_opcode(cpu.program_counter, mode)
    }

    pub fn pha(cpu: &mut CPU, mode: &AdressingMode) -> u16 {
        cpu.push_byte(cpu.accumulator);
        consume_opcode(cpu.program_counter, mode)
    }

    pub fn pla(cpu: &mut CPU, mode: &AdressingMode) -> u16 {
        cpu.accumulator = cpu.pop_byte();
        cpu.update_zero_and_negative_flags(cpu.accumulator);
        consume_opcode(cpu.program_counter, mode)
    }

    pub fn tax(cpu: &mut CPU, mode: &AdressingMode) -> u16 {
        cpu.register_x = cpu.accumulator;
        cpu.update_zero_and_negative_flags(cpu.register_x);
        consume_opcode(cpu.program_counter, mode)
    }

    pub fn txa(cpu: &mut CPU, mode: &AdressingMode) -> u16 {
        cpu.accumulator = cpu.register_x;
        cpu.update_zero_and_negative_flags(cpu.accumulator);
        consume_opcode(cpu.program_counter, mode)
    }

    pub fn tay(cpu: &mut CPU, mode: &AdressingMode) -> u16 {
        cpu.register_y = cpu.accumulator;
        cpu.update_zero_and_negative_flags(cpu.register_y);
        consume_opcode(cpu.program_counter, mode)
    }

    pub fn tya(cpu: &mut CPU, mode: &AdressingMode) -> u16 {
        cpu.accumulator = cpu.register_y;
        cpu.update_zero_and_negative_flags(cpu.accumulator);
        consume_opcode(cpu.program_counter, mode)
    }

    pub fn clv(cpu: &mut CPU, mode: &AdressingMode) -> u16 {
        cpu.status.remove(CpuFlags::OVERFLOW);
        consume_opcode(cpu.program_counter, mode)
    }

    pub fn clc(cpu: &mut CPU, mode: &AdressingMode) -> u16 {
        cpu.status.remove(CpuFlags::CARRY);
        consume_opcode(cpu.program_counter, mode)
    }

    pub fn cld(cpu: &mut CPU, mode: &AdressingMode) -> u16 {
        cpu.status.remove(CpuFlags::DECIMAL);
        consume_opcode(cpu.program_counter, mode)
    }

    pub fn sec(cpu: &mut CPU, mode: &AdressingMode) -> u16 {
        cpu.status.insert(CpuFlags::CARRY);
        consume_opcode(cpu.program_counter, mode)
    }

    pub fn sed(cpu: &mut CPU, mode: &AdressingMode) -> u16 {
        cpu.status.insert(CpuFlags::DECIMAL);
        consume_opcode(cpu.program_counter, mode)
    }

    pub fn sei(cpu: &mut CPU, mode: &AdressingMode) -> u16 {
        cpu.status.insert(CpuFlags::IRQ);
        consume_opcode(cpu.program_counter, mode)
    }

    pub fn cli(cpu: &mut CPU, mode: &AdressingMode) -> u16 {
        cpu.status.remove(CpuFlags::IRQ);
        consume_opcode(cpu.program_counter, mode)
    }

    pub fn jsr(cpu: &mut CPU, mode: &AdressingMode) -> u16 {
        let addr = mode.fetch_param_address(cpu);
        cpu.push_word(consume_opcode(cpu.program_counter, mode) - 1);
        addr
    }

    pub fn rts(cpu: &mut CPU, _: &AdressingMode) -> u16 {
        let addr = cpu.pop_word();
        addr + 1
    }

    pub fn lsr(cpu: &mut CPU, mode: &AdressingMode) -> u16 {
        let result = if mode == &AdressingMode::Accumulator {
            cpu.status
                .set(CpuFlags::CARRY, CPU::nth_bit(cpu.accumulator, 0));
            cpu.accumulator >>= 1;
            cpu.accumulator
        } else {
            let addr = mode.fetch_param_address(cpu);
            let mut value = cpu.read_byte(addr);
            cpu.status.set(CpuFlags::CARRY, CPU::nth_bit(value, 0));
            value >>= 1;
            cpu.write_byte(addr, value);
            value
        };

        cpu.update_zero_and_negative_flags(result);
        consume_opcode(cpu.program_counter, mode)
    }

    pub fn asl(cpu: &mut CPU, mode: &AdressingMode) -> u16 {
        let result = if mode == &AdressingMode::Accumulator {
            cpu.status
                .set(CpuFlags::CARRY, CPU::nth_bit(cpu.accumulator, 7));
            cpu.accumulator <<= 1;
            cpu.accumulator
        } else {
            let addr = mode.fetch_param_address(cpu);
            let mut value = cpu.read_byte(addr);

            cpu.status.set(CpuFlags::CARRY, CPU::nth_bit(value, 7));
            value <<= 1;
            cpu.write_byte(addr, value);
            value
        };

        cpu.update_zero_and_negative_flags(result);
        consume_opcode(cpu.program_counter, mode)
    }

    pub fn ror(cpu: &mut CPU, mode: &AdressingMode) -> u16 {
        let carry = cpu.status.contains(CpuFlags::CARRY);
        let rotate_right = |value: u8| (value >> 1) | ((carry as u8) << 7);

        let result = if mode == &AdressingMode::Accumulator {
            cpu.status
                .set(CpuFlags::CARRY, CPU::nth_bit(cpu.accumulator, 0));
            cpu.accumulator = rotate_right(cpu.accumulator);
            cpu.accumulator
        } else {
            let addr = mode.fetch_param_address(cpu);
            let mut value = cpu.read_byte(addr);

            cpu.status.set(CpuFlags::CARRY, CPU::nth_bit(value, 0));
            value = rotate_right(value);
            cpu.write_byte(addr, value);
            value
        };

        cpu.update_zero_and_negative_flags(result);
        consume_opcode(cpu.program_counter, mode)
    }

    pub fn rol(cpu: &mut CPU, mode: &AdressingMode) -> u16 {
        let carry = cpu.status.contains(CpuFlags::CARRY);
        let rotate_left = |value: u8| (value << 1) | carry as u8;

        let result = if mode == &AdressingMode::Accumulator {
            cpu.status
                .set(CpuFlags::CARRY, CPU::nth_bit(cpu.accumulator, 7));
            cpu.accumulator = rotate_left(cpu.accumulator);
            cpu.accumulator
        } else {
            let addr = mode.fetch_param_address(cpu);
            let mut value = cpu.read_byte(addr);

            cpu.status.set(CpuFlags::CARRY, CPU::nth_bit(value, 7));
            value = rotate_left(value);
            cpu.write_byte(addr, value);
            value
        };

        cpu.update_zero_and_negative_flags(result);
        consume_opcode(cpu.program_counter, mode)
    }

    pub fn and(cpu: &mut CPU, mode: &AdressingMode) -> u16 {
        let addr = mode.fetch_param_address(cpu);
        let value = cpu.read_byte(addr);

        cpu.accumulator &= value;
        cpu.update_zero_and_negative_flags(cpu.accumulator);

        consume_opcode(cpu.program_counter, mode)
    }

    pub fn eor(cpu: &mut CPU, mode: &AdressingMode) -> u16 {
        let addr = mode.fetch_param_address(cpu);
        let value = cpu.read_byte(addr);

        cpu.accumulator ^= value;
        cpu.update_zero_and_negative_flags(cpu.accumulator);

        consume_opcode(cpu.program_counter, mode)
    }

    pub fn ora(cpu: &mut CPU, mode: &AdressingMode) -> u16 {
        let addr = mode.fetch_param_address(cpu);
        let value = cpu.read_byte(addr);

        cpu.accumulator |= value;
        cpu.update_zero_and_negative_flags(cpu.accumulator);
        consume_opcode(cpu.program_counter, mode)
    }

    pub fn lda(cpu: &mut CPU, mode: &AdressingMode) -> u16 {
        let addr = mode.fetch_param_address(cpu);
        let value = cpu.read_byte(addr);

        cpu.accumulator = value;
        cpu.update_zero_and_negative_flags(cpu.accumulator);

        consume_opcode(cpu.program_counter, mode)
    }

    pub fn ldx(cpu: &mut CPU, mode: &AdressingMode) -> u16 {
        let addr = mode.fetch_param_address(cpu);
        let value = cpu.read_byte(addr);

        cpu.register_x = value;
        cpu.update_zero_and_negative_flags(cpu.register_x);

        consume_opcode(cpu.program_counter, mode)
    }

    pub fn ldy(cpu: &mut CPU, mode: &AdressingMode) -> u16 {
        let addr = mode.fetch_param_address(cpu);
        let value = cpu.read_byte(addr);

        cpu.register_y = value;
        cpu.update_zero_and_negative_flags(cpu.register_y);

        consume_opcode(cpu.program_counter, mode)
    }

    pub fn sta(cpu: &mut CPU, mode: &AdressingMode) -> u16 {
        let addr = mode.fetch_param_address(cpu);
        cpu.write_byte(addr, cpu.accumulator);
        consume_opcode(cpu.program_counter, mode)
    }

    pub fn stx(cpu: &mut CPU, mode: &AdressingMode) -> u16 {
        let addr = mode.fetch_param_address(cpu);
        cpu.write_byte(addr, cpu.register_x);
        consume_opcode(cpu.program_counter, mode)
    }

    pub fn sty(cpu: &mut CPU, mode: &AdressingMode) -> u16 {
        let addr = mode.fetch_param_address(cpu);
        cpu.write_byte(addr, cpu.register_y);
        consume_opcode(cpu.program_counter, mode)
    }

    pub fn bit(cpu: &mut CPU, mode: &AdressingMode) -> u16 {
        let addr = mode.fetch_param_address(cpu);
        let value = cpu.read_byte(addr);

        cpu.status
            .set(CpuFlags::ZERO, (cpu.accumulator & value) == 0);
        cpu.status.set(CpuFlags::NEGATIVE, CPU::nth_bit(value, 7));
        cpu.status.set(CpuFlags::OVERFLOW, CPU::nth_bit(value, 6));

        consume_opcode(cpu.program_counter, mode)
    }

    pub fn tsx(cpu: &mut CPU, mode: &AdressingMode) -> u16 {
        cpu.register_x = cpu.stack_pointer;
        cpu.update_zero_and_negative_flags(cpu.register_x);
        consume_opcode(cpu.program_counter, mode)
    }

    pub fn txs(cpu: &mut CPU, mode: &AdressingMode) -> u16 {
        cpu.stack_pointer = cpu.register_x;
        consume_opcode(cpu.program_counter, mode)
    }

    // Unofficial opcodes

    pub fn lax(cpu: &mut CPU, mode: &AdressingMode) -> u16 {
        let addr = mode.fetch_param_address(cpu);
        let value = cpu.read_byte(addr);

        cpu.accumulator = value;
        cpu.register_x = value;
        cpu.update_zero_and_negative_flags(cpu.accumulator);

        consume_opcode(cpu.program_counter, mode)
    }

    pub fn sax(cpu: &mut CPU, mode: &AdressingMode) -> u16 {
        let addr = mode.fetch_param_address(cpu);
        let result = cpu.accumulator & cpu.register_x;

        cpu.write_byte(addr, result);
        consume_opcode(cpu.program_counter, mode)
    }

    pub fn dcp(cpu: &mut CPU, mode: &AdressingMode) -> u16 {
        let addr = mode.fetch_param_address(cpu);
        let value = cpu.read_byte(addr).wrapping_sub(1);

        cpu.write_byte(addr, value);
        cpu.update_zero_and_negative_flags(value);

        cpu.status.set(CpuFlags::CARRY, cpu.accumulator >= value);
        // Subtract so that we set the ZERO flag if the values are equal
        cpu.update_zero_and_negative_flags(cpu.accumulator.wrapping_sub(value));

        consume_opcode(cpu.program_counter, mode)
    }

    pub fn isb(cpu: &mut CPU, mode: &AdressingMode) -> u16 {
        let addr = mode.fetch_param_address(cpu);
        let value = cpu.read_byte(addr).wrapping_add(1);

        let (data, overflow1) = cpu.accumulator.overflowing_sub(value);
        let (result, overflow2) = data.overflowing_sub(!cpu.status.contains(CpuFlags::CARRY) as u8);

        cpu.status.set(CpuFlags::CARRY, !(overflow1 || overflow2));
        cpu.update_zero_and_negative_flags(result);
        cpu.status.set(
            CpuFlags::OVERFLOW,
            (((cpu.accumulator ^ result) & !(value ^ result)) & 0x80) != 0,
        );

        cpu.accumulator = result;
        cpu.write_byte(addr, value);

        consume_opcode(cpu.program_counter, mode)
    }

    pub fn slo(cpu: &mut CPU, mode: &AdressingMode) -> u16 {
        let addr = mode.fetch_param_address(cpu);
        let mut value = cpu.read_byte(addr);

        cpu.status.set(CpuFlags::CARRY, CPU::nth_bit(value, 7));
        value <<= 1;

        cpu.write_byte(addr, value);
        cpu.accumulator |= value;

        cpu.update_zero_and_negative_flags(cpu.accumulator);
        consume_opcode(cpu.program_counter, mode)
    }

    pub fn rla(cpu: &mut CPU, mode: &AdressingMode) -> u16 {
        let addr = mode.fetch_param_address(cpu);
        let mut value = cpu.read_byte(addr);

        let carry = cpu.status.contains(CpuFlags::CARRY);
        let rotate_left = |value: u8| (value << 1) | carry as u8;

        cpu.status.set(CpuFlags::CARRY, CPU::nth_bit(value, 7));
        value = rotate_left(value);
        cpu.write_byte(addr, value);

        cpu.accumulator &= value;
        cpu.update_zero_and_negative_flags(cpu.accumulator);
        consume_opcode(cpu.program_counter, mode)
    }

    pub fn sre(cpu: &mut CPU, mode: &AdressingMode) -> u16 {
        let addr = mode.fetch_param_address(cpu);
        let mut value = cpu.read_byte(addr);

        cpu.status.set(CpuFlags::CARRY, CPU::nth_bit(value, 0));
        value >>= 1;
        cpu.write_byte(addr, value);

        cpu.accumulator ^= value;
        cpu.update_zero_and_negative_flags(cpu.accumulator);

        consume_opcode(cpu.program_counter, mode)
    }

    pub fn rra(cpu: &mut CPU, mode: &AdressingMode) -> u16 {
        let addr = mode.fetch_param_address(cpu);
        let mut value = cpu.read_byte(addr);

        let carry = cpu.status.contains(CpuFlags::CARRY);
        let rotate_right = |value: u8| (value >> 1) | ((carry as u8) << 7);

        cpu.status.set(CpuFlags::CARRY, CPU::nth_bit(value, 0));
        value = rotate_right(value);
        cpu.write_byte(addr, value);

        let (data, overflow1) = cpu.accumulator.overflowing_add(value);
        let (result, overflow2) = data.overflowing_add(cpu.status.contains(CpuFlags::CARRY) as u8);

        cpu.status.set(CpuFlags::CARRY, overflow1 || overflow2);
        cpu.update_zero_and_negative_flags(result);
        cpu.status.set(
            CpuFlags::OVERFLOW,
            (((cpu.accumulator ^ result) & (value ^ result)) & 0x80) != 0,
        );

        cpu.accumulator = result;
        consume_opcode(cpu.program_counter, mode)
    }
}
