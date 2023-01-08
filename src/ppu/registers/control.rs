use bitflags::bitflags;

bitflags! {
    /*
        https://www.nesdev.org/wiki/PPU_registers#PPUCTRL

        7  bit  0
        ---- ----
        VPHB SINN
        |||| ||||
        |||| ||++- Base nametable address
        |||| ||    (0 = $2000; 1 = $2400; 2 = $2800; 3 = $2C00)
        |||| |+--- VRAM address increment per CPU read/write of PPUDATA
        |||| |     (0: add 1, going across; 1: add 32, going down)
        |||| +---- Sprite pattern table address for 8x8 sprites
        ||||       (0: $0000; 1: $1000; ignored in 8x16 mode)
        |||+------ Background pattern table address (0: $0000; 1: $1000)
        ||+------- Sprite size (0: 8x8 pixels; 1: 8x16 pixels – see PPU OAM#Byte 1)
        |+-------- PPU parent/child select
        |          (0: read backdrop from EXT pins; 1: output color on EXT pins)
        +--------- Generate an NMI at the start of the vertical blanking interval (0: off; 1: on)
    */
    pub struct Control: u8 {
        const NametableAddress             = 0b0000_0011;
        const VramAdressIncrement          = 0b0000_0100;
        const SpritePatternTableAddress    = 0b0000_1000;
        const BackgroundPatternBank        = 0b0001_0000;
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
    pub fn update(&mut self, value: u8) {
        Self::from_bits_truncate(value);
    }

    pub fn vram_address_increment(&self) -> u8 {
        if self.contains(Self::VramAdressIncrement) {
            32
        } else {
            1
        }
    }

    pub fn nmi_at_vblank(&self) -> bool {
        self.contains(Self::NonMaskableInterruptAtVBlank)
    }

    pub fn bg_pattern_bank_addr(&self) -> usize {
        if self.contains(Self::BackgroundPatternBank) {
            0x1000
        } else {
            0
        }
    }

    pub fn base_nametable_addr(&self) -> usize {
        match self.bits() & Self::NametableAddress.bits() {
            0 => 0x2000,
            1 => 0x2400,
            2 => 0x2800,
            3 => 0x2C00,
            _ => unreachable!(),
        }
    }
}