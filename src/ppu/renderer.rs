use {
    super::{
        object_attribute::ObjectAttributeMemory,
        palette::{Color, Palette, PaletteEntry, PALETTE_TABLE},
        VideoRam,
    },
    crate::{
        cartridge::{MapperInstance, Mirroring},
        util,
    },
    std::sync::mpsc::Sender,
};

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

    fn for_pixel_in_tile<T>(&mut self, tile: &TileData, palette_entry: PaletteEntry, draw_fn: T)
    where
        T: Fn(&mut Self, usize, usize, Color),
    {
        const BETWEEN_PLANES: usize = 8;
        const PIXELS_PER_ROW: usize = 8;

        for y in 0..PIXELS_PER_ROW {
            let lower_plane = tile[y];
            let upper_plane = tile[y + BETWEEN_PLANES];
            self.draw_line(
                (lower_plane, upper_plane),
                palette_entry,
                |renderer, x, color| draw_fn(renderer, x, y, color),
            );
        }
    }

    fn draw_line<T>(
        &mut self,
        (mut lower_plane, mut upper_plane): (u8, u8),
        palette_entry: PaletteEntry,
        draw_fn: T,
    ) where
        T: Fn(&mut Self, usize, Color),
    {
        for x in (0..8).rev() {
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

    fn draw_nametable_scanline(
        &mut self,
        scanline: usize,
        bank: usize,
        nametable: &[u8],
        viewport: Rectangle,
        scroll_x: isize,
        scroll_y: isize,
    ) {
        const TILES_WIDTH: usize = 32;
        const TILES_HEIGHT: usize = 30;
        const ATTRIBUTE_TABLE_LEN: usize = 64;
        const NAMETABLE_END: usize = TILES_WIDTH * TILES_HEIGHT;
        const ATTRIBUTE_TABLE_END: usize = NAMETABLE_END + ATTRIBUTE_TABLE_LEN;

        let attribute_table = &nametable[NAMETABLE_END..ATTRIBUTE_TABLE_END];

        let y = scanline % 8;
        let tile_y = scanline / 8;

        for tile_x in 0..TILES_WIDTH {
            let tile_idx = nametable[(tile_y * TILES_WIDTH) + tile_x] as u16;
            let tile = self.get_tile(bank, tile_idx as usize);

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

            self.draw_line((tile[y], tile[y + 8]), palette, |renderer, x, color| {
                let pixel_x = tile_x * 8 + x;
                let pixel_y = tile_y * 8 + y;

                if viewport.contains(&Point::new(pixel_x, pixel_y)) {
                    renderer.set_pixel(
                        (pixel_x as isize + scroll_x) as usize,
                        (pixel_y as isize + scroll_y) as usize,
                        color,
                    );
                }
            });
        }
    }

    pub fn draw_background_scanline(
        &mut self,
        scanline: usize,
        bank: usize,
        nametable_addr: usize,
        vram: &VideoRam,
        (scroll_x, scroll_y): (u8, u8),
    ) {
        let mirroring = self.mapper.as_ref().unwrap().borrow().mirroring();
        let (first_nametable, second_nametable) = match (&mirroring, nametable_addr) {
            (Mirroring::Vertical, 0x2000)
            | (Mirroring::Vertical, 0x2800)
            | (Mirroring::Horizontal, 0x2000)
            | (Mirroring::Horizontal, 0x2400) => (&vram[0..0x400], &vram[0x400..0x800]),

            (Mirroring::Vertical, 0x2400)
            | (Mirroring::Vertical, 0x2C00)
            | (Mirroring::Horizontal, 0x2800)
            | (Mirroring::Horizontal, 0x2C00) => (&vram[0x400..0x800], &vram[0..0x400]),

            _ => panic!("unimplemented mirroring mode ({mirroring}, ${nametable_addr:04X})"),
        };

        if scroll_y == 0 {
            self.draw_nametable_scanline(
                scanline,
                bank,
                first_nametable,
                Rectangle::new(
                    Point::new(scroll_x as usize, scroll_y as usize),
                    Point::new(256, 240),
                ),
                -(scroll_x as isize),
                -(scroll_y as isize),
            );

            self.draw_nametable_scanline(
                scanline,
                bank,
                second_nametable,
                Rectangle::new(Point::new(0, 0), Point::new(scroll_x.into(), 240)),
                256 - (scroll_x as isize),
                0,
            );
        } else if (scanline + scroll_y as usize) > 240 {
            self.draw_nametable_scanline(
                (scanline + scroll_y as usize) - 240,
                bank,
                second_nametable,
                Rectangle::new(Point::new(0, 0), Point::new(256, 240)),
                0,
                (239 - scroll_y) as isize,
            );
        } else {
            self.draw_nametable_scanline(
                scanline + scroll_y as usize,
                bank,
                second_nametable,
                Rectangle::new(Point::new(0, 0), Point::new(256, 240)),
                0,
                -(scroll_y as isize),
            );
        }
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
