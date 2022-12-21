use bitflags::bitflags;

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

// TODO: should probably contain something to look up the register function
pub const REGISTERS: [(u16, Mutability); 9] = [
    (0x2000, Mutability::Write),     // Control
    (0x2001, Mutability::Write),     // Mask
    (0x2002, Mutability::Read),      // Status
    (0x2003, Mutability::Write),     // ObjectAttributeAddress
    (0x2004, Mutability::ReadWrite), // ObjectAttributeData
    (0x2005, Mutability::Write),     // Scroll
    (0x2006, Mutability::Write),     // Address
    (0x2007, Mutability::ReadWrite), // Data
    (0x4014, Mutability::Write),     // ObjectAttributeDirectMemoryAccess
];

pub fn get_mutability(address: u16) -> Option<&'static Mutability> {
    // TODO: consider mirroring
    REGISTERS
        .iter()
        .find_map(|r| if r.0 == address { Some(&r.1) } else { None })
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
    const VRAM_ADDR_INCREMENT_IF_FLAG: u8 = 32;
    const VRAM_ADDR_INCREMENT_NO_FLAG: u8 = 1;

    pub fn vram_address_increment(&self) -> u8 {
        if self.contains(Self::VramAdressIncrement) {
            Self::VRAM_ADDR_INCREMENT_IF_FLAG
        } else {
            Self::VRAM_ADDR_INCREMENT_NO_FLAG
        }
    }

    fn update(&mut self, value: u8) {
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
    fn update(&mut self, value: u8) {
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

impl Status {
    fn update(&mut self, value: u8) {
        Self::from_bits_truncate(value);
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

/// https://www.nesdev.org/wiki/PPU_registers#OAMDATA
pub struct ObjectAttributeData {
    pub value: u8,
}

impl Default for ObjectAttributeData {
    fn default() -> Self {
        Self { value: 0 }
    }
}

/// https://www.nesdev.org/wiki/PPU_registers#PPUSCROLL
pub struct Scroll {
    pub value: u8,
}

impl Default for Scroll {
    fn default() -> Self {
        Self { value: 0 }
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

    fn update(&mut self, data: u8) {
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

    fn reset_latch(&mut self) {
        self.latch_high = true;
    }
}

// TODO: Move PPUDATA here?

/// https://www.nesdev.org/wiki/PPU_registers#OAMDMA
pub struct ObjectAttributeDirectMemoryAccess {
    pub value: u8,
}

impl Default for ObjectAttributeDirectMemoryAccess {
    fn default() -> Self {
        Self { value: 0 }
    }
}
