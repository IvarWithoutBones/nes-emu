use crate::bus::Memory;
use crate::cpu::{CpuFlags, CPU};
use std::fmt;

pub type Instruction = (
    &'static str,
    fn(cpu: &mut CPU, mode: &AdressingMode) -> u16,
    &'static [(u8, &'static AdressingMode)],
);

/// Retrieve an instruction based on an identifier
pub fn parse_instruction(identifier: u8) -> Option<(&'static Instruction, &'static AdressingMode)> {
    for instr in INSTRUCTIONS.iter() {
        for (opcode, mode) in instr.2 {
            if *opcode == identifier {
                return Some((instr, mode));
            }
        }
    }
    None
}

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

#[allow(dead_code)]
pub fn instruction_identifier(
    instr: &'static Instruction,
    mode: &'static AdressingMode,
) -> Option<u8> {
    for (opcode, m) in instr.2 {
        if m == &mode {
            return Some(*opcode);
        }
    }
    None
}

pub fn format_instruction(
    cpu: &mut CPU,
    instr: &'static Instruction,
    mode: &AdressingMode,
) -> String {
    let mut bytes = String::from(format!("{:02X} ", cpu.read_byte(cpu.program_counter)));
    let mut args = String::new();

    for i in 1..mode.opcode_len() {
        bytes += &format!("{:02X} ", cpu.read_byte(cpu.program_counter + i as u16));
    }

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

    format!(
        "{:04X} {1: <9}{2: <3} {3: <6}",
        cpu.program_counter,
        bytes,
        instruction_name(instr),
        args,
    )
}

/// Get the next program counter based on the adressing mode
const fn consume_opcode(pc: u16, mode: &AdressingMode) -> u16 {
    pc + mode.opcode_len()
}

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

impl AdressingMode {
    pub const fn has_arguments(&self) -> bool {
        match self {
            AdressingMode::Implied | AdressingMode::Accumulator => false,
            _ => true,
        }
    }

    // The length of an instruction, counting the identifier and arguments
    pub const fn opcode_len(&self) -> u16 {
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

    pub fn fetch_param_address(&self, cpu: &CPU) -> u16 {
        let after_opcode = cpu.program_counter.wrapping_add(1);

        match self {
            AdressingMode::Immediate | AdressingMode::Relative => after_opcode,
            AdressingMode::Absolute => cpu.read_word(after_opcode),
            AdressingMode::ZeroPage => cpu.read_byte(after_opcode) as u16,

            AdressingMode::ZeroPageX => {
                cpu.read_byte(after_opcode).wrapping_add(cpu.register_x) as u16
            }

            AdressingMode::ZeroPageY => {
                cpu.read_byte(after_opcode).wrapping_add(cpu.register_y) as u16
            }

            AdressingMode::AbsoluteX => cpu
                .read_word(after_opcode)
                .wrapping_add(cpu.register_x as u16),

            AdressingMode::AbsoluteY => cpu
                .read_word(after_opcode)
                .wrapping_add(cpu.register_y as u16),

            AdressingMode::Indirect => {
                let ptr = cpu.read_word(after_opcode);
                let low = cpu.read_byte(ptr as u16);

                // Accomodate for a hardware bug, the 6502 reference states the following:
                //    "An original 6502 has does not correctly fetch the target address if the indirect vector
                //    falls on a page boundary (e.g. $xxFF where xx is any value from $00 to $FF). In this case
                //    it fetches the LSB from $xxFF as expected but takes the MSB from $xx00"
                let high = if ptr & 0x00FF != 0xFF {
                    cpu.read_byte(ptr.wrapping_add(1))
                } else {
                    cpu.read_byte(ptr & 0xFF00)
                };

                u16::from_le_bytes([low, high])
            }

            AdressingMode::IndirectX => {
                let ptr = cpu.read_byte(after_opcode).wrapping_add(cpu.register_x);

                u16::from_le_bytes([
                    cpu.read_byte(ptr as u16),
                    cpu.read_byte(ptr.wrapping_add(1) as u16),
                ])
            }

            AdressingMode::IndirectY => {
                let ptr = cpu.read_byte(after_opcode);
                u16::from_le_bytes([
                    cpu.read_byte(ptr as u16),
                    cpu.read_byte(ptr.wrapping_add(1) as u16),
                ])
                .wrapping_add(cpu.register_y as u16)
            }

            AdressingMode::Implied | AdressingMode::Accumulator => {
                panic!("Addressing mode {} has no arguments!", self);
            }
        }
    }
}

#[rustfmt::skip]
/// See https://www.nesdev.org/obelisk-6502-guide/reference.html
pub const INSTRUCTIONS: [Instruction; 57] = [
    ("BRK", opcodes::brk, &[(0x00, &AdressingMode::Implied)]),
    ("RTI", opcodes::rti, &[(0x40, &AdressingMode::Implied)]),

    ("BCS", opcodes::bcs, &[(0xB0, &AdressingMode::Relative)]),
    ("BCC", opcodes::bcc, &[(0x90, &AdressingMode::Relative)]),
    ("BEQ", opcodes::beq, &[(0xF0, &AdressingMode::Relative)]),
    ("BNE", opcodes::bne, &[(0xD0, &AdressingMode::Relative)]),
    ("BMI", opcodes::bmi, &[(0x30, &AdressingMode::Relative)]),
    ("BPL", opcodes::bpl, &[(0x10, &AdressingMode::Relative)]),
    ("BVS", opcodes::bvs, &[(0x70, &AdressingMode::Relative)]),
    ("BVC", opcodes::bvc, &[(0x50, &AdressingMode::Relative)]),

    ("CLV", opcodes::clv, &[(0xB8, &AdressingMode::Implied)]),
    ("CLC", opcodes::clc, &[(0x18, &AdressingMode::Implied)]),
    ("CLD", opcodes::cld, &[(0xD8, &AdressingMode::Implied)]),
    ("CLI", opcodes::cli, &[(0x58, &AdressingMode::Implied)]),
    ("SEC", opcodes::sec, &[(0x38, &AdressingMode::Implied)]),
    ("SED", opcodes::sed, &[(0xF8, &AdressingMode::Implied)]),
    ("SEI", opcodes::sei, &[(0x78, &AdressingMode::Implied)]),

    ("TAX", opcodes::tax, &[(0xAA, &AdressingMode::Implied)]),
    ("TAY", opcodes::tay, &[(0xA8, &AdressingMode::Implied)]),
    ("TXA", opcodes::txa, &[(0x8A, &AdressingMode::Implied)]),
    ("TYA", opcodes::tya, &[(0x98, &AdressingMode::Implied)]),

    ("JSR", opcodes::jsr, &[(0x20, &AdressingMode::Absolute)]),
    ("RTS", opcodes::rts, &[(0x60, &AdressingMode::Implied)]),
    ("PHP", opcodes::php, &[(0x08, &AdressingMode::Implied)]),
    ("PLP", opcodes::plp, &[(0x28, &AdressingMode::Implied)]),
    ("PLP", opcodes::plp, &[(0x28, &AdressingMode::Implied)]),
    ("PHA", opcodes::pha, &[(0x48, &AdressingMode::Implied)]),
    ("PLA", opcodes::pla, &[(0x68, &AdressingMode::Implied)]),
    ("TSX", opcodes::tsx, &[(0xBA, &AdressingMode::Implied)]),
    ("TXS", opcodes::txs, &[(0x9A, &AdressingMode::Implied)]),

    ("NOP", opcodes::nop, &[
        (0x80, &AdressingMode::Immediate),
        (0x0C, &AdressingMode::Absolute),
        (0x1C, &AdressingMode::AbsoluteX),
        (0x3C, &AdressingMode::AbsoluteX),
        (0x5C, &AdressingMode::AbsoluteX),
        (0x7C, &AdressingMode::AbsoluteX),
        (0xDC, &AdressingMode::AbsoluteX),
        (0xFC, &AdressingMode::AbsoluteX),
        (0xEA, &AdressingMode::Implied),
        (0x1A, &AdressingMode::Implied),
        (0x3A, &AdressingMode::Implied),
        (0x5A, &AdressingMode::Implied),
        (0x7A, &AdressingMode::Implied),
        (0xDA, &AdressingMode::Implied),
        (0xFA, &AdressingMode::Implied),
        (0xFA, &AdressingMode::Implied),
        (0x04, &AdressingMode::ZeroPage),
        (0x44, &AdressingMode::ZeroPage),
        (0x64, &AdressingMode::ZeroPage),
        (0x14, &AdressingMode::ZeroPageX),
        (0x34, &AdressingMode::ZeroPageX),
        (0x54, &AdressingMode::ZeroPageX),
        (0x74, &AdressingMode::ZeroPageX),
        (0xD4, &AdressingMode::ZeroPageX),
        (0xF4, &AdressingMode::ZeroPageX),
    ]),

    ("BIT", opcodes::bit, &[
        (0x24, &AdressingMode::ZeroPage),
        (0x2C, &AdressingMode::Absolute),
    ]),

    ("JMP",opcodes::jmp, &[
        (0x4C, &AdressingMode::Absolute),
        (0x6C, &AdressingMode::Indirect),
    ]),

    ("INX", opcodes::inx, &[(0xE8, &AdressingMode::Implied)]),
    ("INY", opcodes::iny, &[(0xC8, &AdressingMode::Implied)]),
    ("INC", opcodes::inc, &[
        (0xE6, &AdressingMode::ZeroPage),
        (0xF6, &AdressingMode::ZeroPageX),
        (0xEE, &AdressingMode::Absolute),
        (0xFE, &AdressingMode::AbsoluteX),
    ]),

    ("DEX", opcodes::dex, &[(0xCA, &AdressingMode::Implied)]),
    ("DEY", opcodes::dey, &[(0x88, &AdressingMode::Implied)]),
    ("DEC", opcodes::dec, &[
        (0xC6, &AdressingMode::ZeroPage),
        (0xD6, &AdressingMode::ZeroPageX),
        (0xCE, &AdressingMode::Absolute),
        (0xDE, &AdressingMode::AbsoluteX),
    ]),

    ("ADC", opcodes::adc, &[
        (0x69, &AdressingMode::Immediate),
        (0x65, &AdressingMode::ZeroPage),
        (0x75, &AdressingMode::ZeroPageX),
        (0x6D, &AdressingMode::Absolute),
        (0x7D, &AdressingMode::AbsoluteX),
        (0x79, &AdressingMode::AbsoluteY),
        (0x61, &AdressingMode::IndirectX),
        (0x71, &AdressingMode::IndirectY),
    ]),

    ("SBC", opcodes::sbc, &[
        (0xE9, &AdressingMode::Immediate),
        (0xE5, &AdressingMode::ZeroPage),
        (0xF5, &AdressingMode::ZeroPageX),
        (0xED, &AdressingMode::Absolute),
        (0xFD, &AdressingMode::AbsoluteX),
        (0xF9, &AdressingMode::AbsoluteY),
        (0xE1, &AdressingMode::IndirectX),
        (0xF1, &AdressingMode::IndirectY),
    ]),

    ("LSR", opcodes::lsr, &[
        (0x4A, &AdressingMode::Accumulator),
        (0x46, &AdressingMode::ZeroPage),
        (0x56, &AdressingMode::ZeroPageX),
        (0x4E, &AdressingMode::Absolute),
        (0x5E, &AdressingMode::AbsoluteX),
    ]),

    ("ASL", opcodes::asl, &[
        (0x0A, &AdressingMode::Accumulator),
        (0x06, &AdressingMode::ZeroPage),
        (0x16, &AdressingMode::ZeroPageX),
        (0x0E, &AdressingMode::Absolute),
        (0x1E, &AdressingMode::AbsoluteX),
    ]),

    ("ROL", opcodes::rol, &[
        (0x2A, &AdressingMode::Accumulator),
        (0x26, &AdressingMode::ZeroPage),
        (0x36, &AdressingMode::ZeroPageX),
        (0x2E, &AdressingMode::Absolute),
        (0x3E, &AdressingMode::AbsoluteX),
    ]),

    ("ROR", opcodes::ror, &[
        (0x6A, &AdressingMode::Accumulator),
        (0x66, &AdressingMode::ZeroPage),
        (0x76, &AdressingMode::ZeroPageX),
        (0x6E, &AdressingMode::Absolute),
        (0x7E, &AdressingMode::AbsoluteX),
    ]),

    ("AND", opcodes::and, &[
        (0x29, &AdressingMode::Immediate),
        (0x25, &AdressingMode::ZeroPage),
        (0x35, &AdressingMode::ZeroPageX),
        (0x2D, &AdressingMode::Absolute),
        (0x3D, &AdressingMode::AbsoluteX),
        (0x39, &AdressingMode::AbsoluteY),
        (0x21, &AdressingMode::IndirectX),
        (0x31, &AdressingMode::IndirectY),
    ]),

    ("EOR", opcodes::eor, &[
        (0x49, &AdressingMode::Immediate),
        (0x45, &AdressingMode::ZeroPage),
        (0x55, &AdressingMode::ZeroPageX),
        (0x4D, &AdressingMode::Absolute),
        (0x5D, &AdressingMode::AbsoluteX),
        (0x59, &AdressingMode::AbsoluteY),
        (0x41, &AdressingMode::IndirectX),
        (0x51, &AdressingMode::IndirectY),
    ]),

    ("ORA", opcodes::ora, &[
        (0x09, &AdressingMode::Immediate),
        (0x05, &AdressingMode::ZeroPage),
        (0x15, &AdressingMode::ZeroPageX),
        (0x0D, &AdressingMode::Absolute),
        (0x1D, &AdressingMode::AbsoluteX),
        (0x19, &AdressingMode::AbsoluteY),
        (0x01, &AdressingMode::IndirectX),
        (0x11, &AdressingMode::IndirectY),
    ]),

    ("CMP", opcodes::cmp, &[
        (0xC9, &AdressingMode::Immediate),
        (0xC5, &AdressingMode::ZeroPage),
        (0xD5, &AdressingMode::ZeroPageX),
        (0xCD, &AdressingMode::Absolute),
        (0xDD, &AdressingMode::AbsoluteX),
        (0xD9, &AdressingMode::AbsoluteY),
        (0xC1, &AdressingMode::IndirectX),
        (0xD1, &AdressingMode::IndirectY),
    ]),

    ("CPX", opcodes::cpx, &[
        (0xE0, &AdressingMode::Immediate),
        (0xE4, &AdressingMode::ZeroPage),
        (0xEC, &AdressingMode::Absolute),
    ]),

    ("CPY", opcodes::cpy, &[
        (0xC0, &AdressingMode::Immediate),
        (0xC4, &AdressingMode::ZeroPage),
        (0xCC, &AdressingMode::Absolute),
    ]),

    ("LDA", opcodes::lda, &[
        (0xA9, &AdressingMode::Immediate),
        (0xA5, &AdressingMode::ZeroPage),
        (0xB5, &AdressingMode::ZeroPageX),
        (0xAD, &AdressingMode::Absolute),
        (0xBD, &AdressingMode::AbsoluteX),
        (0xB9, &AdressingMode::AbsoluteY),
        (0xA1, &AdressingMode::IndirectX),
        (0xB1, &AdressingMode::IndirectY),
    ]),

    ("LDX", opcodes::ldx, &[
        (0xA2, &AdressingMode::Immediate),
        (0xA6, &AdressingMode::ZeroPage),
        (0xB6, &AdressingMode::ZeroPageY),
        (0xAE, &AdressingMode::Absolute),
        (0xBE, &AdressingMode::AbsoluteY),
    ]),

    ("LDY", opcodes::ldy, &[
        (0xA0, &AdressingMode::Immediate),
        (0xA4, &AdressingMode::ZeroPage),
        (0xB4, &AdressingMode::ZeroPageX),
        (0xAC, &AdressingMode::Absolute),
        (0xBC, &AdressingMode::AbsoluteX),
    ]),

    ("STA", opcodes::sta, &[
        (0x85, &AdressingMode::ZeroPage),
        (0x95, &AdressingMode::ZeroPageX),
        (0x8D, &AdressingMode::Absolute),
        (0x9D, &AdressingMode::AbsoluteX),
        (0x99, &AdressingMode::AbsoluteY),
        (0x81, &AdressingMode::IndirectX),
        (0x91, &AdressingMode::IndirectY),
    ]),

    ("STX", opcodes::stx, &[
        (0x86, &AdressingMode::ZeroPage),
        (0x96, &AdressingMode::ZeroPageY),
        (0x8E, &AdressingMode::Absolute),
    ]),

    ("STY", opcodes::sty, &[
        (0x84, &AdressingMode::ZeroPage),
        (0x94, &AdressingMode::ZeroPageX),
        (0x8C, &AdressingMode::Absolute),
    ]),
];

mod opcodes {
    use super::*;

    pub fn nop(cpu: &mut CPU, mode: &AdressingMode) -> u16 {
        consume_opcode(cpu.program_counter, mode)
    }

    pub fn brk(cpu: &mut CPU, mode: &AdressingMode) -> u16 {
        // Let the callee handle this
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
        let value = cpu.read_byte(addr).wrapping_add(1); // Should this be a word?

        cpu.write_byte(addr, value);
        cpu.update_zero_and_negative_flags(value);

        consume_opcode(cpu.program_counter, mode)
    }

    pub fn rti(cpu: &mut CPU, _mode: &AdressingMode) -> u16 {
        cpu.status = CpuFlags::from_bits_truncate(cpu.stack_pop_byte());
        // See https://www.nesdev.org/wiki/Status_flags#The_B_flag
        cpu.status.remove(CpuFlags::BREAK);
        cpu.status.insert(CpuFlags::BREAK2);
        cpu.stack_pop_word()
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
        let value = cpu.read_byte(addr).wrapping_sub(1); // Should this be a word?

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
        cpu.stack_push_byte(status.bits());
        consume_opcode(cpu.program_counter, mode)
    }

    pub fn plp(cpu: &mut CPU, mode: &AdressingMode) -> u16 {
        cpu.status = CpuFlags::from_bits_truncate(cpu.stack_pop_byte());
        // See https://www.nesdev.org/wiki/Status_flags#The_B_flag
        cpu.status.remove(CpuFlags::BREAK);
        cpu.status.insert(CpuFlags::BREAK2);
        consume_opcode(cpu.program_counter, mode)
    }

    pub fn pha(cpu: &mut CPU, mode: &AdressingMode) -> u16 {
        cpu.stack_push_byte(cpu.accumulator);
        consume_opcode(cpu.program_counter, mode)
    }

    pub fn pla(cpu: &mut CPU, mode: &AdressingMode) -> u16 {
        cpu.accumulator = cpu.stack_pop_byte();
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
        cpu.stack_push_word(consume_opcode(cpu.program_counter, mode) - 1);
        addr
    }

    pub fn rts(cpu: &mut CPU, _: &AdressingMode) -> u16 {
        let addr = cpu.stack_pop_word();
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
}
