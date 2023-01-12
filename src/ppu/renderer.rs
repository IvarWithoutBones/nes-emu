use super::Ppu;
use std::ops::Range;
use std::sync::mpsc::Sender;

pub const WIDTH: usize = 256;
pub const HEIGHT: usize = 240;
const RGB_LEN: usize = 3;

pub const fn pixel_buffer_len() -> usize {
    (WIDTH * HEIGHT) * RGB_LEN
}

pub type PixelBuffer = [u8; pixel_buffer_len()];

pub struct Renderer {
    pixel_sender: Sender<Box<PixelBuffer>>,
    pixels: Box<PixelBuffer>,
}

impl Renderer {
    pub fn new(pixel_sender: Sender<Box<PixelBuffer>>) -> Self {
        Self {
            pixels: Box::new([0; pixel_buffer_len()]),
            pixel_sender,
        }
    }

    pub fn update(&mut self) {
        self.pixel_sender
            .send(self.pixels.clone())
            .unwrap_or_else(|e| {
                tracing::error!("failed to send pixel buffer: {}", e);
            });
    }

    fn set_pixel(&mut self, x: usize, y: usize, color: (u8, u8, u8)) {
        if x >= WIDTH || y >= HEIGHT {
            return;
        }

        let base = (y * WIDTH + x) * RGB_LEN;
        self.pixels[base..base + RGB_LEN].copy_from_slice([color.0, color.1, color.2].as_ref());
    }

    fn draw_tile(&mut self, tile: &[u8], palette: [u8; 4], x: usize, y: usize) {
        const BETWEEN_PLANES: usize = 8;
        const PIXELS_PER_ROW: usize = 8;

        for y_offset in 0..PIXELS_PER_ROW {
            let mut lower_plane = tile[y_offset];
            let mut upper_plane = tile[y_offset + BETWEEN_PLANES];

            for x_offset in (0..PIXELS_PER_ROW).rev() {
                let rgb = SYSTEM_PALLETE
                    [palette[Self::to_color_index(upper_plane, lower_plane) as usize] as usize];

                self.set_pixel(
                    (x * PIXELS_PER_ROW) + x_offset,
                    (y * PIXELS_PER_ROW) + y_offset,
                    rgb,
                );

                // Loop over the color indices for each pixel
                upper_plane >>= 1;
                lower_plane >>= 1;
            }
        }
    }

    fn fetch_bg_palette(
        vram: &[u8; Ppu::VRAM_SIZE],
        palette_table: &[u8; Ppu::PALETTE_TABLE_SIZE],
        nametable_offset: usize,
        x: usize,
        y: usize,
    ) -> [u8; 4] {
        // The attribute table is an 8x8 byte array containing palette table indices.
        // Each byte represents a 2x2 tile area in the nametable.
        let attr_index = ((y / 4) * 8) + (x / 4);
        let attr_byte = vram[nametable_offset + attr_index];
        let palette_start = Self::to_palette_start(Quadrant::from((x, y)), attr_byte);

        [
            palette_table[0], // Background color
            palette_table[palette_start],
            palette_table[palette_start + 1],
            palette_table[palette_start + 2],
        ]
    }

    pub fn draw_background(
        &mut self,
        bank: usize,
        chr_rom: &[u8],
        palette_table: &[u8; Ppu::PALETTE_TABLE_SIZE],
        vram: &[u8; Ppu::VRAM_SIZE],
    ) {
        const ROWS_PER_NAMETABLE: usize = 30;
        const TILES_PER_ROW: usize = 32;
        // TODO: Assuming first nametable
        let nametable_offset = ROWS_PER_NAMETABLE * TILES_PER_ROW;

        for i in 0..nametable_offset {
            let y = i / TILES_PER_ROW;
            let x = i % TILES_PER_ROW;

            let tile_index = vram[i] as usize;
            let tile = &chr_rom[Self::to_tile_range(bank, tile_index)];
            let palette = Self::fetch_bg_palette(vram, palette_table, nametable_offset, x, y);

            self.draw_tile(tile, palette, x, y);
            tracing::trace!("drawing tile {} at {},{} ({})", tile_index, x, y, i);
        }
    }

    const fn to_tile_range(bank: usize, tile_index: usize) -> Range<usize> {
        const TILE_LEN: usize = 16;
        let start = bank + (tile_index * TILE_LEN);
        let end = start + TILE_LEN;
        start..end
    }

    const fn to_color_index(upper_plane: u8, lower_plane: u8) -> u8 {
        // Combine the two to get a 2-bit color index (0-3)
        ((1 & upper_plane) << 1) | (1 & lower_plane)
    }

    const fn to_palette_start(quad: Quadrant, attr: u8) -> usize {
        let palette_index = (attr >> quad as u8) & 0b11;
        ((palette_index as usize) * 4) + 1
    }

    // For the debugger in the future
    // #[allow(dead_code)]
    // pub fn show_tiles_in_bank(&mut self, character_rom: &Vec<u8>, bank: usize) {
    //     assert!(bank <= 1);
    //     const TILES_PER_BANK: usize = 256;
    //     const TILES_PER_ROW: usize = 20;
    //
    //     let mut y_offset = 0;
    //     let mut x_offset = 0;
    //     for tile_index in 0..TILES_PER_BANK {
    //         // Scroll to the next row if needed
    //         if tile_index != 0 && tile_index % TILES_PER_ROW == 0 {
    //             y_offset += 10;
    //             x_offset = 0;
    //         }
    //
    //         let tile = &character_rom[Self::to_tile_range(bank, tile_index)];
    //         self.draw_tile(tile, x_offset, y_offset);
    //         x_offset += 10;
    //     }
    // }
}

/// https://www.nesdev.org/wiki/PPU_attribute_tables
#[repr(u8)]
enum Quadrant {
    TopLeft = 0,
    TopRight = 2,
    BottomLeft = 4,
    BottomRight = 6,
}

impl From<(usize, usize)> for Quadrant {
    fn from(mut location: (usize, usize)) -> Self {
        // Normalize the location to an 8x8 grid
        location.0 = (location.0 % 4) / 2;
        location.1 = (location.1 % 4) / 2;

        match location {
            (0, 0) => Quadrant::TopLeft,
            (1, 0) => Quadrant::TopRight,
            (0, 1) => Quadrant::BottomLeft,
            (1, 1) => Quadrant::BottomRight,
            (_, _) => unreachable!(),
        }
    }
}

pub static SYSTEM_PALLETE: [(u8, u8, u8); 64] = [
    (0x80, 0x80, 0x80),
    (0x00, 0x3D, 0xA6),
    (0x00, 0x12, 0xB0),
    (0x44, 0x00, 0x96),
    (0xA1, 0x00, 0x5E),
    (0xC7, 0x00, 0x28),
    (0xBA, 0x06, 0x00),
    (0x8C, 0x17, 0x00),
    (0x5C, 0x2F, 0x00),
    (0x10, 0x45, 0x00),
    (0x05, 0x4A, 0x00),
    (0x00, 0x47, 0x2E),
    (0x00, 0x41, 0x66),
    (0x00, 0x00, 0x00),
    (0x05, 0x05, 0x05),
    (0x05, 0x05, 0x05),
    (0xC7, 0xC7, 0xC7),
    (0x00, 0x77, 0xFF),
    (0x21, 0x55, 0xFF),
    (0x82, 0x37, 0xFA),
    (0xEB, 0x2F, 0xB5),
    (0xFF, 0x29, 0x50),
    (0xFF, 0x22, 0x00),
    (0xD6, 0x32, 0x00),
    (0xC4, 0x62, 0x00),
    (0x35, 0x80, 0x00),
    (0x05, 0x8F, 0x00),
    (0x00, 0x8A, 0x55),
    (0x00, 0x99, 0xCC),
    (0x21, 0x21, 0x21),
    (0x09, 0x09, 0x09),
    (0x09, 0x09, 0x09),
    (0xFF, 0xFF, 0xFF),
    (0x0F, 0xD7, 0xFF),
    (0x69, 0xA2, 0xFF),
    (0xD4, 0x80, 0xFF),
    (0xFF, 0x45, 0xF3),
    (0xFF, 0x61, 0x8B),
    (0xFF, 0x88, 0x33),
    (0xFF, 0x9C, 0x12),
    (0xFA, 0xBC, 0x20),
    (0x9F, 0xE3, 0x0E),
    (0x2B, 0xF0, 0x35),
    (0x0C, 0xF0, 0xA4),
    (0x05, 0xFB, 0xFF),
    (0x5E, 0x5E, 0x5E),
    (0x0D, 0x0D, 0x0D),
    (0x0D, 0x0D, 0x0D),
    (0xFF, 0xFF, 0xFF),
    (0xA6, 0xFC, 0xFF),
    (0xB3, 0xEC, 0xFF),
    (0xDA, 0xAB, 0xEB),
    (0xFF, 0xA8, 0xF9),
    (0xFF, 0xAB, 0xB3),
    (0xFF, 0xD2, 0xB0),
    (0xFF, 0xEF, 0xA6),
    (0xFF, 0xF7, 0x9C),
    (0xD7, 0xE8, 0x95),
    (0xA6, 0xED, 0xAF),
    (0xA2, 0xF2, 0xDA),
    (0x99, 0xFF, 0xFC),
    (0xDD, 0xDD, 0xDD),
    (0x11, 0x11, 0x11),
    (0x11, 0x11, 0x11),
];
