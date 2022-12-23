use bitflags::bitflags;
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

bitflags! {
    /// https://www.nesdev.org/wiki/PPU_registers#PPUCTRL
    pub struct Control: u8 {
        const BaseNametableAddress1        = 0b0000_0001;
        const BaseNametableAddress2        = 0b0000_0010;
        const VramAdressIncrement          = 0b0000_0100;
        const SpritePatternTableAddress    = 0b0000_1000;
        const BackgroundPatternTable       = 0b0001_0000;
        const SpriteSize                   = 0b0010_0000;
        const ParentChildSelect            = 0b0100_0000;
        const NonMaskableInterruptAtVBlank = 0b1000_0000;
    }
}

impl Default for Control {
    fn default() -> Self {
        Self::from_bits_truncate(0)
    }
}

impl Control {
    pub fn vram_address_increment(&self) -> u8 {
        const VRAM_ADDR_INCREMENT_IF_FLAG: u8 = 32;
        const VRAM_ADDR_INCREMENT_NO_FLAG: u8 = 1;

        if self.contains(Self::VramAdressIncrement) {
            VRAM_ADDR_INCREMENT_IF_FLAG
        } else {
            VRAM_ADDR_INCREMENT_NO_FLAG
        }
    }

    pub fn update(&mut self, value: u8) {
        Self::from_bits_truncate(value);
    }
}

bitflags! {
    /// https://www.nesdev.org/wiki/PPU_registers#PPUMASK
    pub struct Mask: u8 {
        const Greyscale              = 0b0000_0001;
        const ShowLeftmostBackground = 0b0000_0010;
        const ShowLeftmostSprites    = 0b0000_0100;
        const ShowBackground         = 0b0000_1000;
        const ShowSprites            = 0b0001_0000;
        const EmphasizeRed           = 0b0010_0000;
        const EmphasizeGreen         = 0b0100_0000;
        const EmphasizeBlue          = 0b1000_0000;
    }
}

impl Default for Mask {
    fn default() -> Self {
        Self::from_bits_truncate(0)
    }
}

impl Mask {
    pub fn update(&mut self, value: u8) {
        Self::from_bits_truncate(value);
    }
}

bitflags! {
    /// https://www.nesdev.org/wiki/PPU_registers#PPUSTATUS
    pub struct Status: u8 {
        // TODO: does masking multiple bits work?
        const OpenBus        = 0b0001_1111;
        const SpriteOverflow = 0b0010_0000;
        const SpriteZeroHit  = 0b0100_0000;
        const VBlankStarted  = 0b1000_0000;
    }
}

impl Default for Status {
    fn default() -> Self {
        Self::from_bits_truncate(0)
    }
}

impl std::fmt::Display for Status {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut string = String::with_capacity(3);
        string.push(self.format(Self::VBlankStarted, 'V'));
        string.push(self.format(Self::SpriteZeroHit, 'Z'));
        string.push(self.format(Self::SpriteOverflow, 'O'));
        write!(f, "{}", string)
    }
}

impl Status {
    fn format(&self, flag: Self, display: char) -> char {
        if self.contains(flag) {
            display
        } else {
            '-'
        }
    }

    pub fn read(&self) -> u8 {
        let _span = tracing::span!(tracing::Level::INFO, "status").entered();
        tracing::trace!("{}", self);
        self.bits()
    }
}

/// https://www.nesdev.org/wiki/PPU_registers#OAMADDR
pub struct ObjectAttributeAddress {
    pub value: u8,
}

impl Default for ObjectAttributeAddress {
    fn default() -> Self {
        Self { value: 0 }
    }
}

impl ObjectAttributeAddress {
    pub fn update(&mut self, data: u8) {
        self.value = data;
    }

    pub fn increment(&mut self) {
        self.value = self.value.wrapping_add(1);
    }
}

/// https://www.nesdev.org/wiki/PPU_registers#PPUSCROLL
pub struct Scroll {
    horizontal: u8,
    vertical: u8,
    horizontal_latch: bool,
}

impl Default for Scroll {
    fn default() -> Self {
        Self {
            horizontal: 0,
            vertical: 0,
            horizontal_latch: true,
        }
    }
}

impl Scroll {
    pub fn update(&mut self, data: u8) {
        if self.horizontal_latch {
            self.horizontal = data;
        } else {
            self.vertical = data;
        }

        self.horizontal_latch = !self.horizontal_latch;
    }
}

/// https://www.nesdev.org/wiki/PPU_registers#PPUADDR
pub struct Address {
    pub value: u16,
    latch_high: bool,
}

impl Default for Address {
    fn default() -> Self {
        Self {
            value: 0,
            // Since this is big-endian we want to write the high byte first
            latch_high: true,
        }
    }
}

impl Address {
    fn mirror(&mut self) {
        const HIGHEST_VALID_ADDR: u16 = 0x4000;
        if self.value >= HIGHEST_VALID_ADDR {
            self.value = self.value % HIGHEST_VALID_ADDR;
        }
    }

    pub fn update(&mut self, data: u8) {
        let mut bytes = u16::to_be_bytes(self.value);
        if self.latch_high {
            bytes[0] = data;
        } else {
            bytes[1] = data;
        }

        self.value = u16::from_be_bytes(bytes);
        self.mirror();
        self.latch_high = !self.latch_high;
    }

    pub fn increment(&mut self, increment: u8) {
        self.value = self.value.wrapping_add(increment as u16);
        self.mirror();
    }

    // fn reset_latch(&mut self) {
    //     self.latch_high = true;
    // }
}

/// https://www.nesdev.org/wiki/PPU_registers#PPUDATA
pub struct Data {
    buffer: u8,
}

impl Default for Data {
    fn default() -> Self {
        Self { buffer: 0 }
    }
}

impl Data {
    /// Updates the internal data buffer, used for reading. Returns the previous contents.
    pub fn update_buffer(&mut self, value: u8) -> u8 {
        let result = self.buffer;
        self.buffer = value;
        result
    }
}
