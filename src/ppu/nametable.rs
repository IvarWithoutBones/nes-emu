use super::{renderer::PIXELS_PER_TILE, VideoRam};
use crate::cartridge::Mirroring;
use std::ops::{Index, Range};

pub const TILES_PER_ROW: usize = 32;
pub const TILES_PER_COLUMN: usize = 30;

const ATTRIBUTE_TABLE_LEN: usize = 64;
const TILE_TABLE_LEN: usize = TILES_PER_COLUMN * TILES_PER_ROW;

pub const NAMETABLE_LEN: usize = TILE_TABLE_LEN + ATTRIBUTE_TABLE_LEN;

#[repr(u16)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NametableAddr {
    TopLeft = 0,
    TopRight = 0x400,
    BottomLeft = 0x800,
    BottomRight = 0xC00,
}

impl From<u16> for NametableAddr {
    fn from(value: u16) -> Self {
        match value {
            0 => Self::TopLeft,
            1 => Self::TopRight,
            2 => Self::BottomLeft,
            3 => Self::BottomRight,
            _ => unreachable!(),
        }
    }
}

impl NametableAddr {
    const VRAM_BASE: u16 = 0x2000;

    pub fn mirror_vram_index(mut addr: u16, mirroring: Mirroring) -> u16 {
        addr -= Self::VRAM_BASE;
        let nametable = Self::from(addr / NAMETABLE_LEN as u16);
        (nametable.mirror(mirroring) as u16) + (addr % NAMETABLE_LEN as u16)
    }

    /// https://www.nesdev.org/wiki/Mirroring#Nametable_Mirroring
    /// Only two nametables are stored in VRAM, the other two are mirrored.
    /// This function normalizes the address to a location in VRAM
    fn mirror(self, mirroring: Mirroring) -> Self {
        match (mirroring, &self) {
            (Mirroring::Vertical, Self::TopLeft)
            | (Mirroring::Vertical, Self::BottomLeft)
            | (Mirroring::Horizontal, Self::TopLeft)
            | (Mirroring::Horizontal, Self::TopRight) => Self::TopLeft,

            (Mirroring::Vertical, NametableAddr::TopRight)
            | (Mirroring::Vertical, NametableAddr::BottomRight)
            | (Mirroring::Horizontal, NametableAddr::BottomLeft)
            | (Mirroring::Horizontal, NametableAddr::BottomRight) => Self::TopRight,

            _ => {
                let address = self as u16;
                panic!("unsupported mirroring {mirroring} for nametable {address:04X}")
            }
        }
    }
}

pub struct Nametable<'a>(&'a [u8]);

impl<'a> Nametable<'a> {
    pub fn from(vram: &'a VideoRam, address: NametableAddr, mirroring: Mirroring) -> (Self, Self) {
        let first = Self(&vram[NametableAddr::TopLeft as usize..NametableAddr::TopRight as usize]);
        let second =
            Self(&vram[NametableAddr::TopRight as usize..NametableAddr::BottomLeft as usize]);

        // The mirrored address gives us the first nametable in VRAM, swap them if needed
        if address.mirror(mirroring) == NametableAddr::TopLeft {
            (first, second)
        } else {
            (second, first)
        }
    }

    pub fn get_tile_index(&self, x: usize, y: usize) -> u8 {
        self[(y * TILES_PER_ROW) + x]
    }

    pub fn get_palette_index(&self, x: usize, y: usize) -> u8 {
        let attr = {
            let coarse_x = x / 4;
            let coarse_y = y / 4;
            let index = (coarse_y * PIXELS_PER_TILE) + coarse_x;
            self[TILE_TABLE_LEN + index]
        };
        Quadrant::from(x, y).into_palette_index(attr)
    }
}

impl Index<usize> for Nametable<'_> {
    type Output = u8;

    fn index(&self, index: usize) -> &Self::Output {
        &self.0[index]
    }
}

impl Index<Range<usize>> for Nametable<'_> {
    type Output = [u8];

    fn index(&self, index: Range<usize>) -> &Self::Output {
        &self.0[index]
    }
}

/// https://www.nesdev.org/wiki/PPU_attribute_tables
#[repr(u8)]
#[derive(Debug)]
enum Quadrant {
    TopLeft = 0,
    TopRight = 2,
    BottomLeft = 4,
    BottomRight = 6,
}

impl Quadrant {
    const fn from(mut x: usize, mut y: usize) -> Self {
        // Normalize to a 2x2 grid
        x = (x % 4) / 2;
        y = (y % 4) / 2;

        match (x, y) {
            (0, 0) => Quadrant::TopLeft,
            (1, 0) => Quadrant::TopRight,
            (0, 1) => Quadrant::BottomLeft,
            (1, 1) => Quadrant::BottomRight,
            (_, _) => unreachable!(),
        }
    }

    const fn into_palette_index(self, value: u8) -> u8 {
        (value >> self as u8) & 0b11
    }
}
