use tartan_bitfield::bitfield;

bitfield! {
    /// https://www.nesdev.org/wiki/PPU_registers#PPUMASK
    pub struct Mask(u8) {
        [0] pub greyscale,
        [1] pub show_leftmost_background,
        [2] pub show_leftmost_sprites,
        [3] pub show_background,
        [4] pub show_sprites,
        [5] pub emphasize_red,
        [6] pub emphasize_green,
        [7] pub emphasize_blue,
    }
}
