use super::{
    nametable::{Nametable, TILES_PER_ROW},
    object_attribute::ObjectAttributeMemory,
    palette::{Color, Palette, PaletteEntry, PALETTE_TABLE},
};
use crate::{cartridge::MapperInstance, util};
use std::sync::mpsc::Sender;

pub const WIDTH: usize = 256;
pub const HEIGHT: usize = 240;

const RGB_LEN: usize = 3;
const PIXEL_BUFFER_LEN: usize = (WIDTH * HEIGHT) * RGB_LEN;
pub type PixelBuffer = [u8; PIXEL_BUFFER_LEN];

const TILE_LEN: usize = 16;
type TileData = [u8; TILE_LEN];
pub const PIXELS_PER_TILE: usize = 8;

const BETWEEN_PLANES: usize = 8;

pub struct Renderer {
    pixel_sender: Sender<Box<PixelBuffer>>,
    pixels: Box<PixelBuffer>,
    pub palette: Palette,
    mapper: Option<MapperInstance>,
}

impl Renderer {
    pub fn new(pixel_sender: Sender<Box<PixelBuffer>>) -> Self {
        Self {
            pixel_sender,
            pixels: Box::new([0; PIXEL_BUFFER_LEN]),
            palette: Palette::default(),
            mapper: None,
        }
    }

    pub fn reset(&mut self) {
        self.pixels = Box::new([0; PIXEL_BUFFER_LEN]);
        self.palette = Palette::default();
        self.update(); // Clear the screen
    }

    pub fn unload_mapper(&mut self) {
        self.mapper = None;
        self.reset();
    }

    pub fn load_mapper(&mut self, mapper: MapperInstance) {
        self.mapper = Some(mapper);
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

        let base = ((y * WIDTH) + x) * RGB_LEN;
        self.pixels[base..base + RGB_LEN].copy_from_slice([color.0, color.1, color.2].as_ref());
    }

    fn get_tile(&self, bank: usize, tile_index: usize) -> TileData {
        let range = {
            let start = bank + (tile_index * TILE_LEN);
            start..start + TILE_LEN
        };
        self.mapper
            .as_ref()
            .unwrap()
            .borrow_mut()
            .read_ppu_range(range)
            .try_into()
            .unwrap()
    }

    fn for_pixels_in_line<T>(
        &mut self,
        (mut upper_plane, mut lower_plane): (u8, u8),
        palette_entry: PaletteEntry,
        draw_fn: T,
    ) where
        T: Fn(&mut Self, usize, Color),
    {
        for x in (0..PIXELS_PER_TILE).rev() {
            let color = {
                let index = util::combine_bools(
                    util::nth_bit(upper_plane, 0),
                    util::nth_bit(lower_plane, 0),
                );
                Palette::get(palette_entry, index.into())
            };

            draw_fn(self, x, color);

            upper_plane >>= 1;
            lower_plane >>= 1;
        }
    }

    fn for_pixels_in_tile<T>(&mut self, tile: &TileData, palette_entry: PaletteEntry, draw_fn: T)
    where
        T: Fn(&mut Self, usize, usize, Color),
    {
        for y in 0..PIXELS_PER_TILE {
            let upper_plane = tile[y + BETWEEN_PLANES];
            let lower_plane = tile[y];
            self.for_pixels_in_line(
                (upper_plane, lower_plane),
                palette_entry,
                |renderer, x, color| draw_fn(renderer, x, y, color),
            );
        }
    }

    fn draw_background_scanline(
        &mut self,
        scanline: usize,
        bank: usize,
        nametable: &Nametable,
        viewport: Rectangle,
        (scroll_x, scroll_y): (isize, isize),
    ) {
        // Position within the tile
        let y_offset = scanline % PIXELS_PER_TILE;
        // Loop over each tile in the scanline
        let tile_y = scanline / PIXELS_PER_TILE;

        for tile_x in 0..TILES_PER_ROW {
            let tile = {
                let index = nametable.get_tile_index(tile_x, tile_y);
                self.get_tile(bank, index as usize)
            };

            let palette = {
                let index = nametable.get_palette_index(tile_x, tile_y);
                self.palette.background_entry(index as usize)
            };

            let upper_plane = tile[y_offset + BETWEEN_PLANES];
            let lower_plane = tile[y_offset];
            self.for_pixels_in_line(
                (upper_plane, lower_plane),
                palette,
                |renderer, x_offset, color| {
                    let pixel_x = (tile_x * PIXELS_PER_TILE) + x_offset;
                    let pixel_y = (tile_y * PIXELS_PER_TILE) + y_offset;

                    if viewport.contains(pixel_x, pixel_y) {
                        renderer.set_pixel(
                            (pixel_x as isize + scroll_x) as usize,
                            (pixel_y as isize + scroll_y) as usize,
                            color,
                        );
                    }
                },
            );
        }
    }

    pub fn draw_sprites(&mut self, maybe_bank: Option<usize>, oam: &ObjectAttributeMemory) {
        // TODO: Apply sprite priority properly
        for object in oam.iter() {
            let (bank, tile_index) = {
                if let Some(bank) = maybe_bank {
                    (bank, object.tile_index)
                } else {
                    (object.bank_8x16(), object.tile_index_8x16())
                }
            };

            let tile = self.get_tile(bank, tile_index);
            let palette = self.palette.sprite_entry(object.attrs.palette() as _);

            self.for_pixels_in_tile(&tile, palette, |renderer, x, y, color| {
                if color == PALETTE_TABLE[0] {
                    // Transparant
                    return;
                }

                let (x, y) = object.pixel_position(x, y);
                renderer.set_pixel(x, y, color);
            });

            if maybe_bank.is_none() {
                // 8x16 sprites are drawn effectively in two tiles, stacked on top
                let tile = self.get_tile(bank, tile_index + 1);
                let palette = self.palette.sprite_entry(object.attrs.palette() as _);

                self.for_pixels_in_tile(&tile, palette, |renderer, x, y, color| {
                    if color == PALETTE_TABLE[0] {
                        // Transparant
                        return;
                    }

                    let (x, y) = object.pixel_position(x, y + BETWEEN_PLANES);
                    renderer.set_pixel(x, y, color);
                });
            }
        }
    }

    pub fn draw_scanline(
        &mut self,
        scanline: usize,
        bank: usize,
        (first_nametable, second_nametable): (&Nametable, &Nametable),
        (scroll_x, scroll_y): (u8, u8),
    ) {
        if scroll_y == 0 {
            self.draw_background_scanline(
                scanline,
                bank,
                first_nametable,
                Rectangle::new((scroll_x as usize, scroll_y as usize), (WIDTH, HEIGHT)),
                (-(scroll_x as isize), -(scroll_y as isize)),
            );

            self.draw_background_scanline(
                scanline,
                bank,
                second_nametable,
                Rectangle::new((0, 0), (scroll_x.into(), HEIGHT)),
                ((WIDTH as isize) - (scroll_x as isize), 0),
            );
        } else if (scanline + scroll_y as usize) > HEIGHT {
            self.draw_background_scanline(
                (scanline + scroll_y as usize) - HEIGHT,
                bank,
                second_nametable,
                Rectangle::new((0, 0), (WIDTH, HEIGHT)),
                (0, (HEIGHT as u8 - scroll_y) as isize),
            );
        } else {
            self.draw_background_scanline(
                scanline + scroll_y as usize,
                bank,
                second_nametable,
                Rectangle::new((0, 0), (WIDTH, HEIGHT)),
                (0, -(scroll_y as isize)),
            );
        }
    }
}

struct Rectangle {
    top_left_x: usize,
    top_left_y: usize,
    bottom_right_x: usize,
    bottom_right_y: usize,
}

impl Rectangle {
    const fn new(
        (top_left_x, top_left_y): (usize, usize),
        (bottom_right_x, bottom_right_y): (usize, usize),
    ) -> Self {
        Self {
            top_left_x,
            top_left_y,
            bottom_right_x,
            bottom_right_y,
        }
    }

    const fn contains(&self, x: usize, y: usize) -> bool {
        x >= self.top_left_x
            && x < self.bottom_right_x
            && y >= self.top_left_y
            && y < self.bottom_right_y
    }
}
