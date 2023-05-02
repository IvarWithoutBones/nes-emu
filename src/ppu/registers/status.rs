use tartan_bitfield::bitfield;

bitfield! {
    /*
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
    /// https://www.nesdev.org/wiki/PPU_registers#PPUSTATUS
    pub struct Status(u8) {
        [5] pub sprite_overflow,
        [6] pub sprite_zero_hit,
        [7] pub vblank_started,
    }
}
