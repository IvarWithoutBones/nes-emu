mod address;
mod control;
mod mask;
mod scroll;
mod status;

pub use address::Address;
pub use control::Control;
pub use mask::Mask;
pub use scroll::Scroll;
pub use status::Status;

use std::ops::RangeInclusive;

#[derive(Debug)]
pub enum Mutability {
    Read,
    Write,
    ReadWrite,
}

impl Mutability {
    pub fn readable(&self) -> bool {
        match self {
            Self::ReadWrite | Self::Read => true,
            Self::Write => false,
        }
    }

    pub fn writable(&self) -> bool {
        match self {
            Self::ReadWrite | Self::Write => true,
            Self::Read => false,
        }
    }
}

#[derive(Debug, PartialEq)]
pub enum Register {
    Control,
    Mask,
    Status,
    ObjectAttributeAddress,
    ObjectAttributeData,
    Scroll,
    Address,
    Data,
    ObjectAttributeDirectMemoryAccess,
}

const REGISTERS: [(u16, Register, Mutability); 8] = [
    (0, Register::Control, Mutability::Write),
    (1, Register::Mask, Mutability::Write),
    (2, Register::Status, Mutability::Read),
    (3, Register::ObjectAttributeAddress, Mutability::Write),
    (4, Register::ObjectAttributeData, Mutability::ReadWrite),
    (5, Register::Scroll, Mutability::Write),
    (6, Register::Address, Mutability::Write),
    (7, Register::Data, Mutability::ReadWrite),
];

pub fn get_register(address: u16) -> Option<(&'static Register, &'static Mutability)> {
    const REGISTERS_RANGE: RangeInclusive<u16> = 0x2000..=0x3FFF;
    if !REGISTERS_RANGE.contains(&address) {
        // TODO: Remove this when I/O registers are properly implemented
        if address == 0x4014 {
            return Some((
                &Register::ObjectAttributeDirectMemoryAccess,
                &Mutability::Write,
            ));
        } else {
            return None;
        }
    }

    // Registers are mirrored every 8 bytes
    let mirrored = address % 8;
    REGISTERS.iter().find_map(|r| {
        if r.0 == mirrored {
            Some((&r.1, &r.2))
        } else {
            None
        }
    })
}
