use std::ops::RangeInclusive;
use std::sync::mpsc::Sender;

use super::Ppu;

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

    fn set_pixel(pixels: &mut Box<PixelBuffer>, x: usize, y: usize, color: (u8, u8, u8)) {
        if x >= WIDTH || y >= HEIGHT {
            return;
        }

        let base = (y * WIDTH + x) * RGB_LEN;
        pixels[base..base + RGB_LEN].copy_from_slice([color.0, color.1, color.2].as_ref());
    }

    pub const fn to_tile_range(bank: usize, tile_index: usize) -> RangeInclusive<usize> {
        let start = bank + tile_index * 16;
        let end = start + 15;
        start..=end
    }

    fn render_tile(&mut self, tile: &[u8], x_offset: usize, y_offset: usize) {
        const TILE_SIZE: usize = 8;
        const OFFSET_BETWEEN_PLANES: usize = 8;

        for y in 0..TILE_SIZE {
            // A row of 8x8 pixels
            let mut upper_plane = tile[y];
            let mut lower_plane = tile[y + OFFSET_BETWEEN_PLANES];

            for x in (0..TILE_SIZE).rev() {
                // Combine the highes bit of the upper and lower plane to get the color index (0-3)
                let value = (1 & upper_plane) << 1 | (1 & lower_plane);

                // TODO: use palette correctly
                let rgb = match value {
                    0 => SYSTEM_PALLETE[0x01],
                    1 => SYSTEM_PALLETE[0x23],
                    2 => SYSTEM_PALLETE[0x27],
                    3 => SYSTEM_PALLETE[0x30],
                    _ => unreachable!(),
                };

                // Shift the planes to the right to get the next pixel
                upper_plane = upper_plane >> 1;
                lower_plane = lower_plane >> 1;

                Self::set_pixel(&mut self.pixels, x_offset + x, y_offset + y, rgb);
            }
        }
    }

    // For the debugger in the future
    #[allow(dead_code)]
    pub fn show_tiles_in_bank(&mut self, character_rom: &Vec<u8>, bank: usize) {
        assert!(bank <= 1);
        const TILES_PER_BANK: usize = 256;
        const TILES_PER_ROW: usize = 20;

        let mut y_offset = 0;
        let mut x_offset = 0;
        for tile_index in 0..TILES_PER_BANK {
            // Scroll to the next row if needed
            if tile_index != 0 && tile_index % TILES_PER_ROW == 0 {
                y_offset += 10;
                x_offset = 0;
            }

            let tile = &character_rom[Self::to_tile_range(bank, tile_index)];
            self.render_tile(tile, x_offset, y_offset);
            x_offset += 10;
        }
    }

    pub fn render_bg(&mut self, bank: usize, chr_rom: &Vec<u8>, vram: &[u8; Ppu::VRAM_SIZE]) {
        // TODO: Assuming first nametable
        for i in 0..0x03c0 {
            let tile_index = vram[i] as usize;
            let tile = &chr_rom[Self::to_tile_range(bank, tile_index)];
            let x = (i % 32) * 8;
            let y = (i / 32) * 8;
            // tracing::info!("rendering tile {} at {},{}", tile_index, x, y);
            self.render_tile(tile, x, y);
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
