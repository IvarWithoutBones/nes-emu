use crate::bus::Memory;
use crate::cpu::CPU;
use std::fmt;

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
