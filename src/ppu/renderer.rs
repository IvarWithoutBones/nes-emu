use super::{
    object_attribute::ObjectAttributeMemory,
    palette::{Color, Palette, PaletteEntry, PALETTE_TABLE},
    VideoRam,
};
use crate::util;
use std::sync::mpsc::Sender;

pub const WIDTH: usize = 256;
pub const HEIGHT: usize = 240;
const RGB_LEN: usize = 3;

const PIXEL_BUFFER_LEN: usize = (WIDTH * HEIGHT) * RGB_LEN;
pub type PixelBuffer = [u8; PIXEL_BUFFER_LEN];

const TILE_LEN: usize = 16;
type TileData = [u8; TILE_LEN];

pub struct Renderer {
    pixel_sender: Sender<Box<PixelBuffer>>,
    pixels: Box<PixelBuffer>,
    pub palette: Palette,
    pub pattern_table: Vec<u8>,
}

impl Renderer {
    pub fn new(pattern_table: Vec<u8>, pixel_sender: Sender<Box<PixelBuffer>>) -> Self {
        Self {
            pixel_sender,
            pattern_table,
            pixels: Box::new([0; PIXEL_BUFFER_LEN]),
            palette: Palette::default(),
        }
    }

    pub fn update(&mut self) {
        self.pixel_sender
            .send(self.pixels.clone())
            .unwrap_or_else(|e| {
                tracing::error!("failed to send pixel buffer: {}", e);
            });
    }

    fn set_pixel(&mut self, x: usize, y: usize, color: Color) {
        if x >= WIDTH || y >= HEIGHT {
            return;
        }

        let base = (y * WIDTH + x) * RGB_LEN;
        self.pixels[base..base + RGB_LEN].copy_from_slice([color.0, color.1, color.2].as_ref());
    }

    fn get_tile(&self, bank: usize, tile_index: usize) -> TileData {
        let range = {
            let start = bank + (tile_index * TILE_LEN);
            start..start + TILE_LEN
        };
        self.pattern_table[range].try_into().unwrap()
    }

    fn for_pixel_in_tile<T>(&mut self, tile: &TileData, palette_entry: PaletteEntry, draw_fn: T)
    where
        T: Fn(&mut Self, usize, usize, Color),
    {
        const BETWEEN_PLANES: usize = 8;
        const PIXELS_PER_ROW: usize = 8;

        for y in 0..PIXELS_PER_ROW {
            let mut lower_plane = tile[y];
            let mut upper_plane = tile[y + BETWEEN_PLANES];

            for x in (0..PIXELS_PER_ROW).rev() {
                let index = util::combine_bools(
                    util::nth_bit(upper_plane, 0),
                    util::nth_bit(lower_plane, 0),
                ) as usize;

                let color = Palette::get(palette_entry, index);
                draw_fn(self, x, y, color);

                // Loop over the color indices for each pixel
                upper_plane >>= 1;
                lower_plane >>= 1;
            }
        }
    }

    pub fn draw_background(&mut self, bank: usize, mut nametable_end: usize, vram: &VideoRam) {
        const TILES_PER_ROW: usize = 32;

        // The last 64 bytes of the nametable are used for attribute tables
        nametable_end -= 64;
        for i in 0..nametable_end {
            let tile_x = i % TILES_PER_ROW;
            let tile_y = i / TILES_PER_ROW;

            let tile_index = vram[i] as usize;
            let tile = self.get_tile(bank, tile_index);

            // TODO: More ergonomic nametable access
            let palette: PaletteEntry = {
                // The attribute table is an 8x8 byte array containing palette table indices.
                // Each byte represents a 2x2 tile area in the nametable.
                let quad = Quadrant::from((tile_x, tile_y));
                let attr_index = ((tile_y / 4) * 8) + (tile_x / 4);
                let attr = vram[nametable_end + attr_index];

                let index = (attr >> quad as u8) & 0b11;
                self.palette.background_entry(index as usize)
            };

            self.for_pixel_in_tile(&tile, palette, |renderer, x, y, color| {
                renderer.set_pixel((tile_x * 8) + x, (tile_y * 8) + y, color);
            });

            tracing::trace!("drawing tile {} at {},{}", tile_index, tile_x, tile_y,);
        }
    }

    pub fn draw_sprites(&mut self, bank: usize, oam: &ObjectAttributeMemory) {
        for object in oam.iter() {
            if object.behind_background {
                continue;
            }

            let tile = self.get_tile(bank, object.tile_index);
            let palette = self.palette.sprite_entry(object.palette_index);

            self.for_pixel_in_tile(&tile, palette, |renderer, x, y, color| {
                if color == PALETTE_TABLE[0] {
                    // Transparant
                    return;
                }

                match (object.flip_horizontal, object.flip_vertical) {
                    (false, false) => renderer.set_pixel(object.x + x, object.y + y, color),
                    (true, false) => renderer.set_pixel((object.x + 7) - x, object.y + y, color),
                    (false, true) => renderer.set_pixel(object.x + x, (object.y + 7) - y, color),
                    (true, true) => {
                        renderer.set_pixel((object.x + 7) - x, (object.y + 7) - y, color)
                    }
                }
            });
        }
    }
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
