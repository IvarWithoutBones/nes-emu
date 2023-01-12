use crate::bus::{Clock, CycleCount, Memory};
use crate::cpu::{addressing_mode::AdressingMode, Cpu, CpuFlags};

/// A single instruction, with addressing mode.
struct Opcode {
    code: &'static u8,
    cycles: &'static CycleCount,
    mode: &'static AdressingMode,
}

/// A collection of instructions using the same function
pub struct Instruction {
    pub name: &'static str,
    pub function: fn(cpu: &mut Cpu, mode: &AdressingMode),
    opcodes: &'static [Opcode],
}

impl Instruction {
    /// Returns an instruction from a given opcode, or None if this instruction does not contain it
    fn find(&self, code: &u8) -> Option<(&Instruction, &AdressingMode, &CycleCount)> {
        for opcode in self.opcodes {
            if opcode.code == code {
                return Some((self, opcode.mode, opcode.cycles));
            }
        }
        None
    }

    /// Search through all instructions for a given opcode, and return its metadata
    pub fn decode(
        code: &u8,
    ) -> Option<(
        &'static Instruction,
        &'static AdressingMode,
        &'static CycleCount,
    )> {
        INSTRUCTIONS.iter().find_map(|i| i.find(code))
    }

    /// Format the instruction to a human-readable string, used for debugging
    pub fn format(&self, cpu: &mut Cpu, mode: &AdressingMode) -> String {
        let mut args = String::new();
        match *mode {
            AdressingMode::Immediate => {
                args = format!("#${:02X}", mode.fetch_param(cpu).0);
            }

            AdressingMode::Relative => {
                // TODO: this is hacky, instructions seem to work fine though
                args = format!("${:02X}", mode.fetch_param_address(cpu).0.wrapping_add(1));
            }

            AdressingMode::ZeroPage => {
                args = format!("${:02X}", mode.fetch_param_address(cpu).0);
            }

            AdressingMode::Accumulator => {
                args = "A".to_string();
            }

            // TODO: formatting of indirect modes
            _ => {
                if mode.has_arguments() {
                    args = format!("${:04X}", mode.fetch_param_address(cpu).0)
                }
            }
        }
        format!("{0: <3} {1: <6}", self.name, args,)
    }
}

/*
    Instruction helpers
*/

fn branch(cpu: &mut Cpu, mode: &AdressingMode, condition: bool) {
    // TODO: should some of this be moved to the addressing mode?
    let after_opcode = cpu.program_counter.wrapping_add(mode.len());
    if condition {
        let offset = mode.fetch_param(cpu).0;
        cpu.tick(1);

        // Two's complement signed offset to branch backwards
        let new_pc = if offset > (u8::MAX / 2) {
            after_opcode.wrapping_sub(offset.wrapping_neg() as u16)
        } else {
            after_opcode.wrapping_add(offset as u16)
        };

        if Cpu::is_on_different_page(after_opcode, new_pc) {
            cpu.tick(1);
        }

        cpu.program_counter = new_pc;
        return;
    }
    cpu.program_counter = after_opcode;
}

/// https://www.nesdev.org/obelisk-6502-guide/reference.html
mod instruction_impls {
    use super::*;

    pub fn nop(cpu: &mut Cpu, mode: &AdressingMode) {
        if mode.has_arguments() {
            // Some illegal opcodes use this with arguments
            let crossed_page = mode.fetch_param_address(cpu).1;
            cpu.tick_once_if(crossed_page);
        }
    }

    pub fn brk(cpu: &mut Cpu, _mode: &AdressingMode) {
        cpu.push_word(cpu.program_counter.wrapping_add(1));
        cpu.push_byte(cpu.flags.bits());

        cpu.program_counter = cpu.read_word(Cpu::BREAK_VECTOR);
        cpu.flags.insert(CpuFlags::Break);
    }

    pub fn jmp(cpu: &mut Cpu, mode: &AdressingMode) {
        cpu.program_counter = mode.fetch_param_address(cpu).0;
    }

    pub fn inx(cpu: &mut Cpu, _mode: &AdressingMode) {
        cpu.register_x = cpu.register_x.wrapping_add(1);
        cpu.update_zero_and_negative_flags(cpu.register_x);
    }

    pub fn iny(cpu: &mut Cpu, _mode: &AdressingMode) {
        cpu.register_y = cpu.register_y.wrapping_add(1);
        cpu.update_zero_and_negative_flags(cpu.register_y);
    }

    pub fn inc(cpu: &mut Cpu, mode: &AdressingMode) {
        let addr = mode.fetch_param_address(cpu).0;
        let value = cpu.read_byte(addr).wrapping_add(1);
        cpu.write_byte(addr, value);
        cpu.update_zero_and_negative_flags(value);
    }

    pub fn rti(cpu: &mut Cpu, _mode: &AdressingMode) {
        cpu.flags = CpuFlags::from_bits_truncate(cpu.pop_byte());
        // See https://www.nesdev.org/wiki/Status_flags#The_B_flag
        cpu.flags.remove(CpuFlags::Break);
        cpu.flags.insert(CpuFlags::Break2);
        cpu.program_counter = cpu.pop_word();
    }

    pub fn adc(cpu: &mut Cpu, mode: &AdressingMode) {
        let (value, page_crossed) = mode.fetch_param(cpu);
        let (data, overflow1) = cpu.accumulator.overflowing_add(value);
        let (result, overflow2) = data.overflowing_add(cpu.flags.contains(CpuFlags::Carry) as u8);

        cpu.flags.set(CpuFlags::Carry, overflow1 || overflow2);
        cpu.update_zero_and_negative_flags(result);
        cpu.flags.set(
            CpuFlags::Overflow,
            (((cpu.accumulator ^ result) & (value ^ result)) & 0x80) != 0,
        );

        cpu.accumulator = result;
        cpu.tick_once_if(page_crossed);
    }

    pub fn sbc(cpu: &mut Cpu, mode: &AdressingMode) {
        let (value, page_crossed) = mode.fetch_param(cpu);
        let (data, overflow1) = cpu.accumulator.overflowing_sub(value);
        let (result, overflow2) = data.overflowing_sub(!cpu.flags.contains(CpuFlags::Carry) as u8);

        cpu.flags.set(CpuFlags::Carry, !(overflow1 || overflow2));
        cpu.update_zero_and_negative_flags(result);
        cpu.flags.set(
            CpuFlags::Overflow,
            (((cpu.accumulator ^ result) & !(value ^ result)) & 0x80) != 0,
        );

        cpu.accumulator = result;
        cpu.tick_once_if(page_crossed);
    }

    pub fn cmp(cpu: &mut Cpu, mode: &AdressingMode) {
        let (value, page_crossed) = mode.fetch_param(cpu);
        cpu.flags.set(CpuFlags::Carry, cpu.accumulator >= value);
        // Subtract so that we set the ZERO flag if the values are equal
        cpu.update_zero_and_negative_flags(cpu.accumulator.wrapping_sub(value));
        cpu.tick_once_if(page_crossed);
    }

    pub fn cpx(cpu: &mut Cpu, mode: &AdressingMode) {
        let value = mode.fetch_param(cpu).0;
        cpu.flags.set(CpuFlags::Carry, cpu.register_x >= value);
        // Subtract so that we set the ZERO flag if the values are equal
        cpu.update_zero_and_negative_flags(cpu.register_x.wrapping_sub(value));
    }

    pub fn cpy(cpu: &mut Cpu, mode: &AdressingMode) {
        let value = mode.fetch_param(cpu).0;
        cpu.flags.set(CpuFlags::Carry, cpu.register_y >= value);
        // Subtract so that we set the ZERO flag if the values are equal
        cpu.update_zero_and_negative_flags(cpu.register_y.wrapping_sub(value));
    }

    pub fn dec(cpu: &mut Cpu, mode: &AdressingMode) {
        let addr = mode.fetch_param_address(cpu).0;
        let value = cpu.read_byte(addr).wrapping_sub(1);
        cpu.write_byte(addr, value);
        cpu.update_zero_and_negative_flags(value);
    }

    pub fn dey(cpu: &mut Cpu, _mode: &AdressingMode) {
        cpu.register_y = cpu.register_y.wrapping_sub(1);
        cpu.update_zero_and_negative_flags(cpu.register_y);
    }

    pub fn dex(cpu: &mut Cpu, _mode: &AdressingMode) {
        cpu.register_x = cpu.register_x.wrapping_sub(1);
        cpu.update_zero_and_negative_flags(cpu.register_x);
    }

    pub fn bcs(cpu: &mut Cpu, mode: &AdressingMode) {
        branch(cpu, mode, cpu.flags.contains(CpuFlags::Carry));
    }

    pub fn bcc(cpu: &mut Cpu, mode: &AdressingMode) {
        branch(cpu, mode, !cpu.flags.contains(CpuFlags::Carry));
    }

    pub fn beq(cpu: &mut Cpu, mode: &AdressingMode) {
        branch(cpu, mode, cpu.flags.contains(CpuFlags::Zero));
    }

    pub fn bne(cpu: &mut Cpu, mode: &AdressingMode) {
        branch(cpu, mode, !cpu.flags.contains(CpuFlags::Zero));
    }

    pub fn bmi(cpu: &mut Cpu, mode: &AdressingMode) {
        branch(cpu, mode, cpu.flags.contains(CpuFlags::Negative));
    }

    pub fn bpl(cpu: &mut Cpu, mode: &AdressingMode) {
        branch(cpu, mode, !cpu.flags.contains(CpuFlags::Negative));
    }

    pub fn bvs(cpu: &mut Cpu, mode: &AdressingMode) {
        branch(cpu, mode, cpu.flags.contains(CpuFlags::Overflow));
    }

    pub fn bvc(cpu: &mut Cpu, mode: &AdressingMode) {
        branch(cpu, mode, !cpu.flags.contains(CpuFlags::Overflow));
    }

    pub fn php(cpu: &mut Cpu, _mode: &AdressingMode) {
        let mut status = cpu.flags.clone();
        // See https://www.nesdev.org/wiki/Status_flags#The_B_flag
        status.insert(CpuFlags::Break);
        status.insert(CpuFlags::Break2);
        cpu.push_byte(status.bits());
    }

    pub fn plp(cpu: &mut Cpu, _mode: &AdressingMode) {
        cpu.flags = CpuFlags::from_bits_truncate(cpu.pop_byte());
        // See https://www.nesdev.org/wiki/Status_flags#The_B_flag
        cpu.flags.remove(CpuFlags::Break);
        cpu.flags.insert(CpuFlags::Break2);
    }

    pub fn pha(cpu: &mut Cpu, _mode: &AdressingMode) {
        cpu.push_byte(cpu.accumulator);
    }

    pub fn pla(cpu: &mut Cpu, _mode: &AdressingMode) {
        cpu.accumulator = cpu.pop_byte();
        cpu.update_zero_and_negative_flags(cpu.accumulator);
    }

    pub fn tax(cpu: &mut Cpu, _mode: &AdressingMode) {
        cpu.register_x = cpu.accumulator;
        cpu.update_zero_and_negative_flags(cpu.register_x);
    }

    pub fn txa(cpu: &mut Cpu, _mode: &AdressingMode) {
        cpu.accumulator = cpu.register_x;
        cpu.update_zero_and_negative_flags(cpu.accumulator);
    }

    pub fn tay(cpu: &mut Cpu, _mode: &AdressingMode) {
        cpu.register_y = cpu.accumulator;
        cpu.update_zero_and_negative_flags(cpu.register_y);
    }

    pub fn tya(cpu: &mut Cpu, _mode: &AdressingMode) {
        cpu.accumulator = cpu.register_y;
        cpu.update_zero_and_negative_flags(cpu.accumulator);
    }

    pub fn clv(cpu: &mut Cpu, _mode: &AdressingMode) {
        cpu.flags.remove(CpuFlags::Overflow);
    }

    pub fn clc(cpu: &mut Cpu, _mode: &AdressingMode) {
        cpu.flags.remove(CpuFlags::Carry);
    }

    pub fn cld(cpu: &mut Cpu, _mode: &AdressingMode) {
        cpu.flags.remove(CpuFlags::Decimal);
    }

    pub fn sec(cpu: &mut Cpu, _mode: &AdressingMode) {
        cpu.flags.insert(CpuFlags::Carry);
    }

    pub fn sed(cpu: &mut Cpu, _mode: &AdressingMode) {
        cpu.flags.insert(CpuFlags::Decimal);
    }

    pub fn sei(cpu: &mut Cpu, _mode: &AdressingMode) {
        cpu.flags.insert(CpuFlags::InterruptsDisabled);
    }

    pub fn cli(cpu: &mut Cpu, _mode: &AdressingMode) {
        cpu.flags.remove(CpuFlags::InterruptsDisabled);
    }

    pub fn jsr(cpu: &mut Cpu, mode: &AdressingMode) {
        let addr = mode.fetch_param_address(cpu).0;
        cpu.push_word(cpu.program_counter.wrapping_add(mode.len()).wrapping_sub(1));
        cpu.program_counter = addr;
    }

    pub fn rts(cpu: &mut Cpu, _mode: &AdressingMode) {
        cpu.program_counter = cpu.pop_word().wrapping_add(1);
    }

    pub fn lsr(cpu: &mut Cpu, mode: &AdressingMode) {
        let result = if mode == &AdressingMode::Accumulator {
            cpu.flags
                .set(CpuFlags::Carry, Cpu::nth_bit(cpu.accumulator, 0));
            cpu.accumulator >>= 1;
            cpu.accumulator
        } else {
            let addr = mode.fetch_param_address(cpu).0;
            let mut value = cpu.read_byte(addr);

            cpu.flags.set(CpuFlags::Carry, Cpu::nth_bit(value, 0));
            value >>= 1;
            cpu.write_byte(addr, value);
            value
        };

        cpu.update_zero_and_negative_flags(result);
    }

    pub fn asl(cpu: &mut Cpu, mode: &AdressingMode) {
        let result = if mode == &AdressingMode::Accumulator {
            cpu.flags
                .set(CpuFlags::Carry, Cpu::nth_bit(cpu.accumulator, 7));
            cpu.accumulator <<= 1;
            cpu.accumulator
        } else {
            let addr = mode.fetch_param_address(cpu).0;
            let mut value = cpu.read_byte(addr);

            cpu.flags.set(CpuFlags::Carry, Cpu::nth_bit(value, 7));
            value <<= 1;
            cpu.write_byte(addr, value);
            value
        };

        cpu.update_zero_and_negative_flags(result);
    }

    pub fn ror(cpu: &mut Cpu, mode: &AdressingMode) {
        let carry = cpu.flags.contains(CpuFlags::Carry);
        let rotate_right = |value: u8| (value >> 1) | ((carry as u8) << 7);

        let result = if mode == &AdressingMode::Accumulator {
            cpu.flags
                .set(CpuFlags::Carry, Cpu::nth_bit(cpu.accumulator, 0));
            cpu.accumulator = rotate_right(cpu.accumulator);
            cpu.accumulator
        } else {
            let addr = mode.fetch_param_address(cpu).0;
            let mut value = cpu.read_byte(addr);

            cpu.flags.set(CpuFlags::Carry, Cpu::nth_bit(value, 0));
            value = rotate_right(value);
            cpu.write_byte(addr, value);
            value
        };

        cpu.update_zero_and_negative_flags(result);
    }

    pub fn rol(cpu: &mut Cpu, mode: &AdressingMode) {
        let carry = cpu.flags.contains(CpuFlags::Carry);
        let rotate_left = |value: u8| (value << 1) | carry as u8;

        let result = if mode == &AdressingMode::Accumulator {
            cpu.flags
                .set(CpuFlags::Carry, Cpu::nth_bit(cpu.accumulator, 7));
            cpu.accumulator = rotate_left(cpu.accumulator);
            cpu.accumulator
        } else {
            let addr = mode.fetch_param_address(cpu).0;
            let mut value = cpu.read_byte(addr);

            cpu.flags.set(CpuFlags::Carry, Cpu::nth_bit(value, 7));
            value = rotate_left(value);
            cpu.write_byte(addr, value);
            value
        };

        cpu.update_zero_and_negative_flags(result);
    }

    pub fn and(cpu: &mut Cpu, mode: &AdressingMode) {
        let (value, page_crossed) = mode.fetch_param(cpu);
        cpu.accumulator &= value;
        cpu.update_zero_and_negative_flags(cpu.accumulator);
        cpu.tick_once_if(page_crossed);
    }

    pub fn eor(cpu: &mut Cpu, mode: &AdressingMode) {
        let (value, page_crossed) = mode.fetch_param(cpu);
        cpu.accumulator ^= value;
        cpu.update_zero_and_negative_flags(cpu.accumulator);
        cpu.tick_once_if(page_crossed);
    }

    pub fn ora(cpu: &mut Cpu, mode: &AdressingMode) {
        let (value, page_crossed) = mode.fetch_param(cpu);
        cpu.accumulator |= value;
        cpu.update_zero_and_negative_flags(cpu.accumulator);
        cpu.tick_once_if(page_crossed);
    }

    pub fn lda(cpu: &mut Cpu, mode: &AdressingMode) {
        let (value, page_crossed) = mode.fetch_param(cpu);
        cpu.accumulator = value;
        cpu.update_zero_and_negative_flags(cpu.accumulator);
        cpu.tick_once_if(page_crossed);
    }

    pub fn ldx(cpu: &mut Cpu, mode: &AdressingMode) {
        let (value, page_crossed) = mode.fetch_param(cpu);
        cpu.register_x = value;
        cpu.update_zero_and_negative_flags(cpu.register_x);
        cpu.tick_once_if(page_crossed);
    }

    pub fn ldy(cpu: &mut Cpu, mode: &AdressingMode) {
        let (value, page_crossed) = mode.fetch_param(cpu);
        cpu.register_y = value;
        cpu.update_zero_and_negative_flags(cpu.register_y);
        cpu.tick_once_if(page_crossed);
    }

    pub fn sta(cpu: &mut Cpu, mode: &AdressingMode) {
        let addr = mode.fetch_param_address(cpu).0;
        cpu.write_byte(addr, cpu.accumulator);
    }

    pub fn stx(cpu: &mut Cpu, mode: &AdressingMode) {
        let addr = mode.fetch_param_address(cpu).0;
        cpu.write_byte(addr, cpu.register_x);
    }

    pub fn sty(cpu: &mut Cpu, mode: &AdressingMode) {
        let addr = mode.fetch_param_address(cpu).0;
        cpu.write_byte(addr, cpu.register_y);
    }

    pub fn bit(cpu: &mut Cpu, mode: &AdressingMode) {
        let value = mode.fetch_param(cpu).0;
        cpu.flags
            .set(CpuFlags::Zero, (cpu.accumulator & value) == 0);
        cpu.flags.set(CpuFlags::Negative, Cpu::nth_bit(value, 7));
        cpu.flags.set(CpuFlags::Overflow, Cpu::nth_bit(value, 6));
    }

    pub fn tsx(cpu: &mut Cpu, _mode: &AdressingMode) {
        cpu.register_x = cpu.stack_pointer;
        cpu.update_zero_and_negative_flags(cpu.register_x);
    }

    pub fn txs(cpu: &mut Cpu, _mode: &AdressingMode) {
        cpu.stack_pointer = cpu.register_x;
    }

    /*
        Unofficial/undocumented opcodes
    */

    pub fn sax(cpu: &mut Cpu, mode: &AdressingMode) {
        let addr = mode.fetch_param_address(cpu).0;
        let result = cpu.accumulator & cpu.register_x;
        cpu.write_byte(addr, result);
    }

    pub fn lax(cpu: &mut Cpu, mode: &AdressingMode) {
        lda(cpu, mode);
        tax(cpu, mode);
    }

    pub fn dcp(cpu: &mut Cpu, mode: &AdressingMode) {
        dec(cpu, mode);
        cmp(cpu, mode);
    }

    pub fn isc(cpu: &mut Cpu, mode: &AdressingMode) {
        inc(cpu, mode);
        sbc(cpu, mode);
    }

    pub fn slo(cpu: &mut Cpu, mode: &AdressingMode) {
        asl(cpu, mode);
        ora(cpu, mode);
    }

    pub fn rla(cpu: &mut Cpu, mode: &AdressingMode) {
        rol(cpu, mode);
        and(cpu, mode);
    }

    pub fn sre(cpu: &mut Cpu, mode: &AdressingMode) {
        lsr(cpu, mode);
        eor(cpu, mode);
    }

    pub fn rra(cpu: &mut Cpu, mode: &AdressingMode) {
        ror(cpu, mode);
        adc(cpu, mode);
    }

    pub fn alr(cpu: &mut Cpu, mode: &AdressingMode) {
        and(cpu, mode);
        lsr(cpu, &AdressingMode::Accumulator);
    }
}

// A macro to make defining instructions less verbose
macro_rules! instr {
    ($name: expr, $function: expr, $($opcodes: tt),*) => {
        Instruction {
            name: $name,
            function: $function,
            opcodes: &[$(Opcode {
                code: &$opcodes.0,
                cycles: &$opcodes.1,
                mode: &$opcodes.2,
            }),*],
        }
    };
}

#[rustfmt::skip]
const INSTRUCTIONS: [Instruction; 65] = [
    instr!("BRK", instruction_impls::brk, (0x00, 7, &AdressingMode::Implied)),
    instr!("RTI", instruction_impls::rti, (0x40, 6, &AdressingMode::Implied)),

    instr!("BCS", instruction_impls::bcs, (0xB0, 2, &AdressingMode::Relative)),
    instr!("BCC", instruction_impls::bcc, (0x90, 2, &AdressingMode::Relative)),
    instr!("BEQ", instruction_impls::beq, (0xF0, 2, &AdressingMode::Relative)),
    instr!("BNE", instruction_impls::bne, (0xD0, 2, &AdressingMode::Relative)),
    instr!("BMI", instruction_impls::bmi, (0x30, 2, &AdressingMode::Relative)),
    instr!("BPL", instruction_impls::bpl, (0x10, 2, &AdressingMode::Relative)),
    instr!("BVS", instruction_impls::bvs, (0x70, 2, &AdressingMode::Relative)),
    instr!("BVC", instruction_impls::bvc, (0x50, 2, &AdressingMode::Relative)),

    instr!("CLV", instruction_impls::clv, (0xB8, 2, &AdressingMode::Implied)),
    instr!("CLC", instruction_impls::clc, (0x18, 2, &AdressingMode::Implied)),
    instr!("CLD", instruction_impls::cld, (0xD8, 2, &AdressingMode::Implied)),
    instr!("CLI", instruction_impls::cli, (0x58, 2, &AdressingMode::Implied)),
    instr!("SEC", instruction_impls::sec, (0x38, 2, &AdressingMode::Implied)),
    instr!("SED", instruction_impls::sed, (0xF8, 2, &AdressingMode::Implied)),
    instr!("SEI", instruction_impls::sei, (0x78, 2, &AdressingMode::Implied)),

    instr!("TAX", instruction_impls::tax, (0xAA, 2, &AdressingMode::Implied)),
    instr!("TAY", instruction_impls::tay, (0xA8, 2, &AdressingMode::Implied)),
    instr!("TXA", instruction_impls::txa, (0x8A, 2, &AdressingMode::Implied)),
    instr!("TYA", instruction_impls::tya,
        (0x98, 2, &AdressingMode::Implied),
        (0x89, 2, &AdressingMode::Implied)
    ),

    instr!("JSR", instruction_impls::jsr, (0x20, 6, &AdressingMode::Absolute)),
    instr!("RTS", instruction_impls::rts, (0x60, 6, &AdressingMode::Implied)),
    instr!("PHP", instruction_impls::php, (0x08, 3, &AdressingMode::Implied)),
    instr!("PLP", instruction_impls::plp, (0x28, 4, &AdressingMode::Implied)),
    instr!("PHA", instruction_impls::pha, (0x48, 3, &AdressingMode::Implied)),
    instr!("PLA", instruction_impls::pla, (0x68, 4, &AdressingMode::Implied)),
    instr!("TSX", instruction_impls::tsx, (0xBA, 2, &AdressingMode::Implied)),
    instr!("TXS", instruction_impls::txs, (0x9A, 2, &AdressingMode::Implied)),

    instr!("NOP", instruction_impls::nop,
        (0x80, 2, &AdressingMode::Immediate),
        (0x0C, 4, &AdressingMode::Absolute),
        (0x1C, 4, &AdressingMode::AbsoluteX),
        (0x3C, 4, &AdressingMode::AbsoluteX),
        (0x5C, 4, &AdressingMode::AbsoluteX),
        (0x7C, 4, &AdressingMode::AbsoluteX),
        (0xDC, 4, &AdressingMode::AbsoluteX),
        (0xFC, 4, &AdressingMode::AbsoluteX),
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
        (0xF4, 4, &AdressingMode::ZeroPageX)
    ),

    instr!("BIT", instruction_impls::bit,
        (0x24, 3, &AdressingMode::ZeroPage),
        (0x2C, 4, &AdressingMode::Absolute)
    ),

    instr!("JMP", instruction_impls::jmp,
        (0x4C, 3, &AdressingMode::Absolute),
        (0x6C, 5, &AdressingMode::Indirect)
    ),

    instr!("INX", instruction_impls::inx, (0xE8, 2, &AdressingMode::Implied)),
    instr!("INY", instruction_impls::iny, (0xC8, 2, &AdressingMode::Implied)),
    instr!("INC", instruction_impls::inc,
        (0xE6, 5, &AdressingMode::ZeroPage),
        (0xF6, 6, &AdressingMode::ZeroPageX),
        (0xEE, 6, &AdressingMode::Absolute),
        (0xFE, 7, &AdressingMode::AbsoluteX)
    ),

    instr!("DEX", instruction_impls::dex, (0xCA, 2, &AdressingMode::Implied)),
    instr!("DEY", instruction_impls::dey, (0x88, 2, &AdressingMode::Implied)),
    instr!("DEC", instruction_impls::dec,
        (0xC6, 5, &AdressingMode::ZeroPage),
        (0xD6, 6, &AdressingMode::ZeroPageX),
        (0xCE, 6, &AdressingMode::Absolute),
        (0xDE, 7, &AdressingMode::AbsoluteX)
    ),

    instr!("ADC", instruction_impls::adc,
        (0x69, 2, &AdressingMode::Immediate),
        (0x65, 3, &AdressingMode::ZeroPage),
        (0x75, 4, &AdressingMode::ZeroPageX),
        (0x6D, 4, &AdressingMode::Absolute),
        (0x7D, 4, &AdressingMode::AbsoluteX),
        (0x79, 4, &AdressingMode::AbsoluteY),
        (0x61, 6, &AdressingMode::IndirectX),
        (0x71, 5, &AdressingMode::IndirectY)
    ),

    instr!("SBC", instruction_impls::sbc,
        (0xE9, 2, &AdressingMode::Immediate),
        (0xEB, 2, &AdressingMode::Immediate), // Undocumented
        (0xE5, 3, &AdressingMode::ZeroPage),
        (0xF5, 4, &AdressingMode::ZeroPageX),
        (0xED, 4, &AdressingMode::Absolute),
        (0xFD, 4, &AdressingMode::AbsoluteX),
        (0xF9, 4, &AdressingMode::AbsoluteY),
        (0xE1, 6, &AdressingMode::IndirectX),
        (0xF1, 5, &AdressingMode::IndirectY)
    ),

    instr!("LSR", instruction_impls::lsr,
        (0x4A, 2, &AdressingMode::Accumulator),
        (0x46, 5, &AdressingMode::ZeroPage),
        (0x56, 6, &AdressingMode::ZeroPageX),
        (0x4E, 6, &AdressingMode::Absolute),
        (0x5E, 7, &AdressingMode::AbsoluteX)
    ),

    instr!("ASL", instruction_impls::asl,
        (0x0A, 2, &AdressingMode::Accumulator),
        (0x06, 5, &AdressingMode::ZeroPage),
        (0x16, 6, &AdressingMode::ZeroPageX),
        (0x0E, 6, &AdressingMode::Absolute),
        (0x1E, 7, &AdressingMode::AbsoluteX)
    ),

    instr!("ROL", instruction_impls::rol,
        (0x2A, 2, &AdressingMode::Accumulator),
        (0x26, 5, &AdressingMode::ZeroPage),
        (0x36, 6, &AdressingMode::ZeroPageX),
        (0x2E, 6, &AdressingMode::Absolute),
        (0x3E, 7, &AdressingMode::AbsoluteX)
    ),

    instr!("ROR", instruction_impls::ror,
        (0x6A, 2, &AdressingMode::Accumulator),
        (0x66, 5, &AdressingMode::ZeroPage),
        (0x76, 6, &AdressingMode::ZeroPageX),
        (0x6E, 6, &AdressingMode::Absolute),
        (0x7E, 7, &AdressingMode::AbsoluteX)
    ),

    instr!("AND", instruction_impls::and,
        (0x29, 2, &AdressingMode::Immediate),
        (0x25, 3, &AdressingMode::ZeroPage),
        (0x35, 4, &AdressingMode::ZeroPageX),
        (0x2D, 4, &AdressingMode::Absolute),
        (0x3D, 4, &AdressingMode::AbsoluteX),
        (0x39, 4, &AdressingMode::AbsoluteY),
        (0x21, 6, &AdressingMode::IndirectX),
        (0x31, 5, &AdressingMode::IndirectY)
    ),

    instr!("EOR", instruction_impls::eor,
        (0x49, 2, &AdressingMode::Immediate),
        (0x45, 3, &AdressingMode::ZeroPage),
        (0x55, 4, &AdressingMode::ZeroPageX),
        (0x4D, 4, &AdressingMode::Absolute),
        (0x5D, 4, &AdressingMode::AbsoluteX),
        (0x59, 4, &AdressingMode::AbsoluteY),
        (0x41, 6, &AdressingMode::IndirectX),
        (0x51, 5, &AdressingMode::IndirectY)
    ),

    instr!("ORA", instruction_impls::ora,
        (0x09, 2, &AdressingMode::Immediate),
        (0x05, 3, &AdressingMode::ZeroPage),
        (0x15, 4, &AdressingMode::ZeroPageX),
        (0x0D, 4, &AdressingMode::Absolute),
        (0x1D, 4, &AdressingMode::AbsoluteX),
        (0x19, 4, &AdressingMode::AbsoluteY),
        (0x01, 6, &AdressingMode::IndirectX),
        (0x11, 5, &AdressingMode::IndirectY)
    ),

    instr!("CMP", instruction_impls::cmp,
        (0xC9, 2, &AdressingMode::Immediate),
        (0xC5, 3, &AdressingMode::ZeroPage),
        (0xD5, 4, &AdressingMode::ZeroPageX),
        (0xCD, 4, &AdressingMode::Absolute),
        (0xDD, 4, &AdressingMode::AbsoluteX),
        (0xD9, 4, &AdressingMode::AbsoluteY),
        (0xC1, 6, &AdressingMode::IndirectX),
        (0xD1, 5, &AdressingMode::IndirectY)
    ),

    instr!("CPX", instruction_impls::cpx,
        (0xE0, 2, &AdressingMode::Immediate),
        (0xE4, 3, &AdressingMode::ZeroPage),
        (0xEC, 4, &AdressingMode::Absolute)
    ),

    instr!("CPY", instruction_impls::cpy,
        (0xC0, 2, &AdressingMode::Immediate),
        (0xC4, 3, &AdressingMode::ZeroPage),
        (0xCC, 4, &AdressingMode::Absolute)
    ),

    instr!("LDA", instruction_impls::lda,
        (0xA9, 2, &AdressingMode::Immediate),
        (0xA5, 3, &AdressingMode::ZeroPage),
        (0xB5, 4, &AdressingMode::ZeroPageX),
        (0xAD, 4, &AdressingMode::Absolute),
        (0xBD, 4, &AdressingMode::AbsoluteX),
        (0xB9, 4, &AdressingMode::AbsoluteY),
        (0xA1, 6, &AdressingMode::IndirectX),
        (0xB1, 5, &AdressingMode::IndirectY)
    ),

    instr!("LDX", instruction_impls::ldx,
        (0xA2, 2, &AdressingMode::Immediate),
        (0xA6, 3, &AdressingMode::ZeroPage),
        (0xB6, 4, &AdressingMode::ZeroPageY),
        (0xAE, 4, &AdressingMode::Absolute),
        (0xBE, 4, &AdressingMode::AbsoluteY)
    ),

    instr!("LDY", instruction_impls::ldy,
        (0xA0, 2, &AdressingMode::Immediate),
        (0xA4, 3, &AdressingMode::ZeroPage),
        (0xB4, 4, &AdressingMode::ZeroPageX),
        (0xAC, 4, &AdressingMode::Absolute),
        (0xBC, 4, &AdressingMode::AbsoluteX)
    ),

    instr!("STA", instruction_impls::sta,
        (0x85, 3, &AdressingMode::ZeroPage),
        (0x95, 4, &AdressingMode::ZeroPageX),
        (0x8D, 4, &AdressingMode::Absolute),
        (0x9D, 5, &AdressingMode::AbsoluteX),
        (0x99, 5, &AdressingMode::AbsoluteY),
        (0x81, 6, &AdressingMode::IndirectX),
        (0x91, 6, &AdressingMode::IndirectY)
    ),

    instr!("STX", instruction_impls::stx,
        (0x86, 3, &AdressingMode::ZeroPage),
        (0x96, 4, &AdressingMode::ZeroPageY),
        (0x8E, 4, &AdressingMode::Absolute)
    ),

    instr!("STY", instruction_impls::sty,
        (0x84, 3, &AdressingMode::ZeroPage),
        (0x94, 4, &AdressingMode::ZeroPageX),
        (0x8C, 4, &AdressingMode::Absolute)
    ),

    // Unofficial opcodes

    instr!("LAX", instruction_impls::lax,
        (0xA7, 3, &AdressingMode::ZeroPage),
        (0xB7, 4, &AdressingMode::ZeroPageY),
        (0xAF, 4, &AdressingMode::Absolute),
        (0xBF, 4, &AdressingMode::AbsoluteY),
        (0xA3, 6, &AdressingMode::IndirectX),
        (0xB3, 5, &AdressingMode::IndirectY)
    ),

    instr!("SAX", instruction_impls::sax,
        (0x87, 3, &AdressingMode::ZeroPage),
        (0x97, 4, &AdressingMode::ZeroPageY),
        (0x8F, 4, &AdressingMode::Absolute),
        (0x83, 6, &AdressingMode::IndirectX)
    ),

    instr!("DCP", instruction_impls::dcp,
        (0xC7, 5, &AdressingMode::ZeroPage),
        (0xD7, 6, &AdressingMode::ZeroPageX),
        (0xCF, 6, &AdressingMode::Absolute),
        (0xDF, 7, &AdressingMode::AbsoluteX),
        (0xDB, 7, &AdressingMode::AbsoluteY),
        (0xC3, 8, &AdressingMode::IndirectX),
        (0xD3, 8, &AdressingMode::IndirectY)
    ),

    instr!("ISC", instruction_impls::isc,
        (0xE7, 5, &AdressingMode::ZeroPage),
        (0xF7, 6, &AdressingMode::ZeroPageX),
        (0xEF, 6, &AdressingMode::Absolute),
        (0xFF, 7, &AdressingMode::AbsoluteX),
        (0xFB, 7, &AdressingMode::AbsoluteY),
        (0xE3, 8, &AdressingMode::IndirectX),
        (0xF3, 8, &AdressingMode::IndirectY)
    ),

    instr!("SLO", instruction_impls::slo,
        (0x07, 5, &AdressingMode::ZeroPage),
        (0x17, 6, &AdressingMode::ZeroPageX),
        (0x0F, 6, &AdressingMode::Absolute),
        (0x1F, 7, &AdressingMode::AbsoluteX),
        (0x1B, 7, &AdressingMode::AbsoluteY),
        (0x03, 8, &AdressingMode::IndirectX),
        (0x13, 8, &AdressingMode::IndirectY)
    ),

    instr!("RLA", instruction_impls::rla,
        (0x27, 5, &AdressingMode::ZeroPage),
        (0x37, 6, &AdressingMode::ZeroPageX),
        (0x2F, 6, &AdressingMode::Absolute),
        (0x3F, 7, &AdressingMode::AbsoluteX),
        (0x3B, 7, &AdressingMode::AbsoluteY),
        (0x23, 8, &AdressingMode::IndirectX),
        (0x33, 8, &AdressingMode::IndirectY)
    ),

    instr!("SRE", instruction_impls::sre,
        (0x47, 5, &AdressingMode::ZeroPage),
        (0x57, 6, &AdressingMode::ZeroPageX),
        (0x4F, 6, &AdressingMode::Absolute),
        (0x5F, 7, &AdressingMode::AbsoluteX),
        (0x5B, 7, &AdressingMode::AbsoluteY),
        (0x43, 8, &AdressingMode::IndirectX),
        (0x53, 8, &AdressingMode::IndirectY)
    ),

    instr!("RRA", instruction_impls::rra,
        (0x67, 5, &AdressingMode::ZeroPage),
        (0x77, 6, &AdressingMode::ZeroPageX),
        (0x6F, 6, &AdressingMode::Absolute),
        (0x7F, 7, &AdressingMode::AbsoluteX),
        (0x7B, 7, &AdressingMode::AbsoluteY),
        (0x63, 8, &AdressingMode::IndirectX),
        (0x73, 8, &AdressingMode::IndirectY)
    ),

    instr!("ALR", instruction_impls::alr,
        (0x4B, 2, &AdressingMode::Immediate)
    )
];
