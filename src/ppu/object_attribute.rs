use super::renderer::PIXELS_PER_TILE;
use crate::util;
use std::ops::{Index, Range};
use tartan_bitfield::bitfield;

/// https://www.nesdev.org/wiki/PPU_OAM
pub struct ObjectAttributeMemory {
    span: tracing::Span,
    pub memory: [u8; Self::MEMORY_SIZE],
    address: u8,
}

impl ObjectAttributeMemory {
    pub const MEMORY_SIZE: usize = 0x100;

    #[tracing::instrument(skip(self, data), parent = &self.span)]
    pub fn write_address(&mut self, data: u8) {
        tracing::trace!("address write of ${:02X}", data);
        self.address = data;
    }

    #[tracing::instrument(skip(self, data), parent = &self.span)]
    pub fn write_data(&mut self, data: u8) {
        tracing::trace!("oam write at ${:02X}: ${:02X}", self.address, data);
        self.memory[self.address as usize] = data;
        self.address = self.address.wrapping_add(1);
    }

    #[tracing::instrument(skip(self), parent = &self.span)]
    pub fn read_data(&self) -> u8 {
        let result = self.memory[self.address as usize];
        tracing::trace!("read at ${:02X}: ${:02X}", self.address, result);
        result
    }

    #[tracing::instrument(skip(self, addr), parent = &self.span)]
    pub fn dma(&self, addr: u8) -> Range<usize> {
        // Convert to a page index: $XX -> $XX00
        let begin = ((addr as u16) << 8) as usize;
        let end = begin + Self::MEMORY_SIZE;
        tracing::debug!(
            "DMA transfer from ${:04X}..${:04X}, starting at DMA ${:02X}",
            begin,
            end,
            self.address
        );
        begin..end
    }

    pub fn iter(&self) -> OamIterator<'_> {
        OamIterator {
            index: 0,
            oam: self,
        }
    }
}

impl Default for ObjectAttributeMemory {
    fn default() -> Self {
        Self {
            span: tracing::span!(tracing::Level::INFO, "ppu:oam"),
            memory: [0; Self::MEMORY_SIZE],
            address: 0,
        }
    }
}

impl Index<usize> for ObjectAttributeMemory {
    type Output = u8;

    fn index(&self, index: usize) -> &Self::Output {
        &self.memory[index]
    }
}

bitfield! {
    /*
        76543210
        ||||||||
        ||||||++- Palette (4 to 7) of sprite
        |||+++--- Unimplemented (read 0)
        ||+------ Priority (0: in front of background; 1: behind background)
        |+------- Flip sprite horizontally
        +-------- Flip sprite vertically
    */
    /// https://www.nesdev.org/wiki/PPU_OAM#Byte_2
    pub struct ObjectAttributes(u8) {
        [0..=1] pub palette: u8,
        [5] pub behind_background,
        [6] pub flip_horizontal,
        [7] pub flip_vertical,
    }
}

pub struct Object {
    pub x: usize,
    pub y: usize,
    pub attrs: ObjectAttributes,
    pub tile_index: usize,
}

impl From<&[u8]> for Object {
    fn from(data: &[u8]) -> Self {
        Self {
            attrs: ObjectAttributes::from(data[2]),
            tile_index: data[1] as _,
            x: data[3] as _,
            y: data[0] as _,
        }
    }
}

impl Object {
    pub const fn bank_8x16(&self) -> usize {
        if util::nth_bit(self.tile_index as _, 0) {
            0x1000
        } else {
            0
        }
    }

    pub const fn tile_index_8x16(&self) -> usize {
        self.tile_index & 0b1111_1110
    }

    pub fn pixel_position(&self, x: usize, y: usize) -> (usize, usize) {
        const LEN: usize = PIXELS_PER_TILE - 1;

        let x = if self.attrs.flip_horizontal() {
            (self.x + LEN) - x
        } else {
            self.x + x
        };

        let y = if self.attrs.flip_vertical() {
            (self.y + LEN) - y
        } else {
            self.y + y
        };

        (x, y)
    }
}

pub struct OamIterator<'a> {
    oam: &'a ObjectAttributeMemory,
    index: usize,
}

impl<'a> Iterator for OamIterator<'a> {
    type Item = Object;

    fn next(&mut self) -> Option<Self::Item> {
        if self.index >= ObjectAttributeMemory::MEMORY_SIZE {
            return None;
        }
        self.index += 4;
        Some(Object::from(&self.oam.memory[self.index - 4..self.index]))
    }
}
