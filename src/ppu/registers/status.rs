use bitflags::bitflags;

bitflags! {
    /*
        https://www.nesdev.org/wiki/PPU_registers#PPUSTATUS

        7  bit  0
        ---- ----
        VSO. ....
        |||| ||||
        |||+-++++- PPU open bus. Returns stale PPU bus contents.
        ||+------- Sprite overflow. The intent was for this flag to be set whenever more than eight sprites
        ||         appear on a scanline, but a hardware bug causes the actual behavior to be more complicated
        ||         and generate false positives as well as false negatives; See PPU sprite evaluation.
        ||         This flag is set during sprite evaluation and cleared at dot 1 (the second dot) of the pre-render line.
        |+-------- Sprite 0 Hit. Set when a nonzero pixel of sprite 0 overlaps a nonzero background pixel.
        |          Cleared at dot 1 of the pre-render line. Used for raster timing.
        +--------- Vertical blank has started (0: not in vblank; 1: in vblank).
                   Set at dot 1 of line 241 (the line *after* the post-render line);
                   cleared after reading $2002 and at dot 1 of the pre-render line.
    */
    pub struct Status: u8 {
        const Unused         = 0b0001_1111;
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

    pub fn read(&mut self) -> u8 {
        self.bits()
    }

    pub fn reset_vblank(&mut self) {
        self.remove(Self::VBlankStarted);
    }

    pub fn set_vblank(&mut self) {
        self.insert(Self::VBlankStarted);
    }

    pub fn in_vblank(&mut self) -> bool {
        self.contains(Self::VBlankStarted)
    }
}
