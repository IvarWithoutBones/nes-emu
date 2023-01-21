use super::{
    object_attribute::ObjectAttributeMemory,
    palette::{Color, Palette, PaletteEntry, PALETTE_TABLE},
    VideoRam,
};
use crate::{cartridge::Mirroring, util};
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
    pub pattern_table: Option<Vec<u8>>,
}

impl Renderer {
    pub fn new(pixel_sender: Sender<Box<PixelBuffer>>) -> Self {
        Self {
            pixel_sender,
            // pattern_table,
            pixels: Box::new([0; PIXEL_BUFFER_LEN]),
            palette: Palette::default(),
            pattern_table: None,
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

        let base = ((y * WIDTH) + x) * RGB_LEN;
        self.pixels[base..base + RGB_LEN].copy_from_slice([color.0, color.1, color.2].as_ref());
    }

    fn get_tile(&self, bank: usize, tile_index: usize) -> TileData {
        let range = {
            let start = bank + (tile_index * TILE_LEN);
            start..start + TILE_LEN
        };
        self.pattern_table.as_ref().unwrap()[range].try_into().unwrap()
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

    fn draw_nametable(
        &mut self,
        bank: usize,
        nametable: &[u8],
        viewport: Rectangle,
        shift_x: isize,
        shift_y: isize,
    ) {
        const TILES_WIDTH: usize = 32;
        const TILES_HEIGHT: usize = 30;
        const ATTRIBUTE_TABLE_LEN: usize = 64;
        const NAMETABLE_END: usize = TILES_WIDTH * TILES_HEIGHT;
        const ATTRIBUTE_TABLE_END: usize = NAMETABLE_END + ATTRIBUTE_TABLE_LEN;

        let attribute_table = &nametable[NAMETABLE_END..ATTRIBUTE_TABLE_END];

        // TODO: More ergonomic nametable access
        for (i, tile_index) in nametable.iter().enumerate().take(NAMETABLE_END) {
            let tile_x = i % TILES_WIDTH;
            let tile_y = i / TILES_WIDTH;
            let tile = self.get_tile(bank, *tile_index as usize);

            let palette = {
                // The attribute table is an 8x8 byte array containing palette table indices.
                // Each byte represents a 2x2 tile area in the nametable.
                let quad = Quadrant::from((tile_x, tile_y));
                let attr = {
                    let attr_index = {
                        let x = tile_x / 4;
                        let y = tile_y / 4;
                        (y * 8) + x
                    };
                    attribute_table[attr_index]
                };

                let palette_index = (attr >> quad as u8) & 0b11;
                self.palette.background_entry(palette_index as usize)
            };

            self.for_pixel_in_tile(&tile, palette, |renderer, mut x, mut y, color| {
                x += tile_x * 8;
                y += tile_y * 8;

                if viewport.contains(&Point::new(x, y)) {
                    renderer.set_pixel(
                        (x as isize + shift_x) as usize,
                        (y as isize + shift_y) as usize,
                        color,
                    );
                }
            });
        }
    }

    pub fn draw_background(
        &mut self,
        bank: usize,
        scroll_x: u8,
        scroll_y: u8,
        nametable_addr: usize,
        mirroring: &Mirroring,
        vram: &VideoRam,
    ) {
        let scroll_x = scroll_x as usize;
        let scroll_y = scroll_y as usize;

        // TODO: This is not very pretty
        let (first_nametable, second_nametable) = match (&mirroring, nametable_addr) {
            /*
                Vertical mirroring:
                // A B
                // A B
            */
            (Mirroring::Vertical, 0x2000) | (Mirroring::Vertical, 0x2800) => {
                (&vram[0..0x400], &vram[0x400..0x800])
            }

            (Mirroring::Vertical, 0x2400) | (Mirroring::Vertical, 0x2C00) => {
                (&vram[0x400..0x800], &vram[0..0x400])
            }

            /*
                Horizontal mirroring:
                // A A
                // B B
            */
            (Mirroring::Horizontal, 0x2000) | (Mirroring::Horizontal, 0x2400) => {
                (&vram[0..0x400], &vram[0x400..0x800])
            }

            (Mirroring::Horizontal, 0x2800) | (Mirroring::Horizontal, 0x2C00) => {
                (&vram[0x400..0x800], &vram[0..0x400])
            }

            _ => panic!(
                "unimplemented mirroring mode ({}, ${:04X})",
                mirroring, nametable_addr
            ),
        };

        self.draw_nametable(
            bank,
            first_nametable,
            Rectangle::new(Point::new(scroll_x, scroll_y), Point::new(WIDTH, HEIGHT)),
            -(scroll_x as isize),
            -(scroll_y as isize),
        );

        self.draw_nametable(
            bank,
            second_nametable,
            Rectangle::new(Point::new(0, 0), Point::new(scroll_x, HEIGHT)),
            (WIDTH - scroll_x) as isize,
            0,
        );
    }

    pub fn draw_sprites(&mut self, bank: usize, oam: &ObjectAttributeMemory) {
        for object in oam.iter() {
            // TODO: Apply sprite priority properly
            // if object.behind_background {
            //     continue;
            // }

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

// TODO: Should probably use generics for this
struct Point {
    x: usize,
    y: usize,
}

impl Point {
    const fn new(x: usize, y: usize) -> Self {
        Self { x, y }
    }
}

struct Rectangle {
    top_left: Point,
    bottom_right: Point,
}

impl Rectangle {
    const fn new(top_left: Point, bottom_right: Point) -> Self {
        Self {
            top_left,
            bottom_right,
        }
    }

    const fn contains(&self, point: &Point) -> bool {
        point.x >= self.top_left.x
            && point.x < self.bottom_right.x
            && point.y >= self.top_left.y
            && point.y < self.bottom_right.y
    }
}
