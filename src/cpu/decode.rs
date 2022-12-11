#[derive(Copy, Clone)]
enum Group {
    One,
    Two,
    Three,
}

impl Group {
    const fn from(group_bits: u8) -> Option<Self> {
        match group_bits {
            0b01 => Some(Group::One),
            0b10 => Some(Group::Two),
            0b00 => Some(Group::Three),
            _ => None,
        }
    }
}

#[derive(Debug, Copy, Clone)]
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
    fn from(mode_bits: u8, group: Group, instr: Opcode) -> Option<Self> {
        let mode = match group {
            Group::One => match mode_bits {
                0b011 => Some(AdressingMode::Absolute),
                0b111 => Some(AdressingMode::AbsoluteX),
                0b110 => Some(AdressingMode::AbsoluteY),
                0b000 => Some(AdressingMode::IndirectX),
                0b100 => Some(AdressingMode::IndirectY),
                0b001 => Some(AdressingMode::ZeroPage),
                0b101 => Some(AdressingMode::ZeroPageX),
                0b010 => Some(AdressingMode::Immediate),
                _ => None,
            },

            Group::Two => match mode_bits {
                0b011 => Some(AdressingMode::Absolute),
                0b111 => Some(AdressingMode::AbsoluteX),
                0b001 => Some(AdressingMode::ZeroPage),
                0b101 => Some(AdressingMode::ZeroPageX),
                0b000 => Some(AdressingMode::Immediate),
                0b010 => Some(AdressingMode::Accumulator),
                _ => None,
            },

            Group::Three => match mode_bits {
                0b011 => Some(AdressingMode::Absolute),
                0b111 => Some(AdressingMode::AbsoluteX),
                0b001 => Some(AdressingMode::ZeroPage),
                0b101 => Some(AdressingMode::ZeroPageX),
                0b000 => Some(AdressingMode::Immediate),
                _ => None,
            },
        };

        if mode.is_none() {
            return None;
        }
        let mode = mode.unwrap();

        Self::handle_quirks(instr, mode, group)
    }

    const fn handle_quirks(instr: Opcode, mode: Self, group: Group) -> Option<Self> {
        match group {
            Group::One => match (instr, mode) {
                (Opcode::STA, Self::Immediate) => None,
                _ => Some(mode),
            },

            Group::Two => match (instr, mode) {
                (Opcode::STX, Self::AbsoluteY) => None,
                (Opcode::STX, Self::ZeroPageX) => Some(Self::ZeroPageY),
                (Opcode::LDX, Self::ZeroPageX) => Some(Self::ZeroPageY),
                (Opcode::LDX, Self::AbsoluteX) => Some(Self::AbsoluteY),

                (_, Self::Immediate) => match instr {
                    Opcode::ASL
                    | Opcode::ROL
                    | Opcode::LSR
                    | Opcode::ROR
                    | Opcode::STX
                    | Opcode::DEC
                    | Opcode::INC => None,
                    _ => Some(mode),
                },

                (_, Self::Accumulator) => match instr {
                    Opcode::STX | Opcode::LDX | Opcode::DEC | Opcode::INC => None,
                    _ => Some(mode),
                },

                _ => Some(mode),
            },

            Group::Three => match (instr, mode) {
                (_, Self::Immediate) => match instr {
                    Opcode::BIT | Opcode::JMP_ABSOLUTE | Opcode::JMP_INDIRECT | Opcode::STY => None,
                    _ => Some(mode),
                },

                (_, Self::ZeroPage) => match instr {
                    Opcode::JMP_ABSOLUTE | Opcode::JMP_INDIRECT => None,
                    _ => Some(mode),
                },

                (_, Self::ZeroPageX) => match instr {
                    Opcode::BIT
                    | Opcode::JMP_ABSOLUTE
                    | Opcode::JMP_INDIRECT
                    | Opcode::CPY
                    | Opcode::CPX => None,
                    _ => Some(mode),
                },

                (_, Self::AbsoluteX) => match instr {
                    Opcode::BIT
                    | Opcode::JMP_ABSOLUTE
                    | Opcode::JMP_INDIRECT
                    | Opcode::STY
                    | Opcode::CPY
                    | Opcode::CPX => None,
                    _ => Some(mode),
                },

                (_, Self::Absolute) => match instr {
                    Opcode::JMP_ABSOLUTE => Some(Self::Absolute),
                    Opcode::JMP_INDIRECT => Some(Self::Indirect),
                    _ => Some(mode),
                }

                _ => Some(mode),
            },
        }
    }
}

impl Opcode {
    const fn mask_opcode(opcode: u8) -> u8 {
        const OPCODE_MASK: u8 = 0b111_000_00;
        (opcode & OPCODE_MASK) >> 5
    }

    const fn mask_mode(opcode: u8) -> u8 {
        const MODE_MASK: u8 = 0b000_111_00;
        (opcode & MODE_MASK) >> 2
    }

    const fn mask_group(opcode: u8) -> u8 {
        const GROUP_MASK: u8 = 0b000_000_11;
        opcode & GROUP_MASK
    }

    pub fn from(opcode: u8) -> Option<Self> {
        let opcode_bits = Self::mask_opcode(opcode);
        let mode_bits = Self::mask_mode(opcode);
        let group = Group::from(Self::mask_group(opcode));

        if group.is_none() {
            return None;
        }
        let group = group.unwrap();

        let instr = match group {
            Group::One => match opcode_bits {
                0b000 => Some(Self::ORA),
                0b001 => Some(Self::AND),
                0b010 => Some(Self::EOR),
                0b011 => Some(Self::ADC),
                0b100 => Some(Self::STA),
                0b101 => Some(Self::LDA),
                0b110 => Some(Self::CMP),
                0b111 => Some(Self::SBC),
                _ => None,
            },

            Group::Two => match opcode_bits {
                0b000 => Some(Self::ASL),
                0b001 => Some(Self::ROL),
                0b010 => Some(Self::LSR),
                0b011 => Some(Self::ROR),
                0b100 => Some(Self::STX),
                0b101 => Some(Self::LDX),
                0b110 => Some(Self::DEC),
                0b111 => Some(Self::INC),
                _ => None,
            },

            Group::Three => match opcode_bits {
                0b001 => Some(Self::BIT),
                0b010 => Some(Self::JMP_INDIRECT),
                0b011 => Some(Self::JMP_ABSOLUTE),
                0b100 => Some(Self::STY),
                0b101 => Some(Self::LDY),
                0b110 => Some(Self::CPY),
                0b111 => Some(Self::CPX),
                _ => None,
            },
        };

        if instr.is_none() {
            return None;
        }
        let instr = instr.unwrap();

        let mode = AdressingMode::from(mode_bits, group, instr);
        println!("{:?} {:?}", mode, instr);

        Some(instr)
    }
}

#[allow(dead_code, non_camel_case_types)]
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum Opcode {
    ADC,
    AND,
    ASL,
    BCC,
    BCS,
    BEQ,
    BIT,
    BMI,
    BNE,
    BPL,
    BRK,
    BVC,
    BVS,
    CLC,
    CLD,
    CLI,
    CLV,
    CMP,
    CPX,
    CPY,
    DEC,
    DEX,
    DEY,
    EOR,
    INC,
    INX,
    INY,
    JMP_ABSOLUTE,
    JMP_INDIRECT,
    JSR,
    LDA,
    LDX,
    LDY,
    LSR,
    NOP,
    ORA,
    PHA,
    PHP,
    PLA,
    PLP,
    ROL,
    ROR,
    RTI,
    RTS,
    SBC,
    SEC,
    SED,
    SEI,
    STA,
    STX,
    STY,
    TAX,
    TAY,
    TSX,
    TXA,
    TXS,
    TYA,
}
