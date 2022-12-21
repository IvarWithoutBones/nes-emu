use bitflags::bitflags;

bitflags! {
    /// https://www.nesdev.org/wiki/PPU_registers#PPUCTRL
    pub struct ControlRegister: u8 {
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

impl Default for ControlRegister {
    fn default() -> Self {
        Self::from_bits_truncate(0)
    }
}

impl ControlRegister {
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

/// https://www.nesdev.org/wiki/PPU_registers#PPUADDR
pub struct AddressRegister {
    pub value: u16,
    latch_high: bool,
}

impl Default for AddressRegister {
    fn default() -> Self {
        Self {
            value: 0,
            // Since this is big-endian we want to write the high byte first
            latch_high: true,
        }
    }
}

impl AddressRegister {
    const HIGHEST_VALID_ADDR: u16 = 0x3FFF;

    fn mirror(&mut self) {
        if self.value > Self::HIGHEST_VALID_ADDR {
            self.value = self.value % (Self::HIGHEST_VALID_ADDR + 1);
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

bitflags! {
    /// https://www.nesdev.org/wiki/PPU_registers#PPUMASK
    pub struct MaskRegister: u8 {
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

impl Default for MaskRegister {
    fn default() -> Self {
        Self::from_bits_truncate(0)
    }
}

impl MaskRegister {
    fn update(&mut self, value: u8) {
        Self::from_bits_truncate(value);
    }
}

bitflags! {
    /// https://www.nesdev.org/wiki/PPU_registers#PPUSTATUS
    pub struct StatusRegister: u8 {
        // TODO: does masking multiple bits work?
        const OpenBus        = 0b0001_1111;
        const SpriteOverflow = 0b0010_0000;
        const SpriteZeroHit  = 0b0100_0000;
        const VBlankStarted  = 0b1000_0000;
    }
}

impl Default for StatusRegister {
    fn default() -> Self {
        Self::from_bits_truncate(0)
    }
}

impl StatusRegister {
    fn update(&mut self, value: u8) {
        Self::from_bits_truncate(value);
    }
}

/// https://www.nesdev.org/wiki/PPU_registers#OAMDMA
pub struct OamDmaRegister {
    pub value: u8,
}

impl Default for OamDmaRegister {
    fn default() -> Self {
        Self { value: 0 }
    }
}

/// https://www.nesdev.org/wiki/PPU_registers#OAMADDR
pub struct OamAddressRegister {
    pub value: u8,
}

impl Default for OamAddressRegister {
    fn default() -> Self {
        Self { value: 0 }
    }
}
