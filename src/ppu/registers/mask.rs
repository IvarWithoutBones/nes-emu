use bitflags::bitflags;

bitflags! {
    /*
        https://www.nesdev.org/wiki/PPU_registers#PPUMASK

        7  bit  0
        ---- ----
        BGRs bMmG
        |||| ||||
        |||| |||+- Greyscale (0: normal color, 1: produce a greyscale display)
        |||| ||+-- 1: Show background in leftmost 8 pixels of screen, 0: Hide
        |||| |+--- 1: Show sprites in leftmost 8 pixels of screen, 0: Hide
        |||| +---- 1: Show background
        |||+------ 1: Show sprites
        ||+------- Emphasize red (green on PAL/Dendy)
        |+-------- Emphasize green (red on PAL/Dendy)
        +--------- Emphasize blue
    */
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

impl From<u8> for Mask {
    fn from(val: u8) -> Self {
        Self::from_bits_truncate(val)
    }
}
