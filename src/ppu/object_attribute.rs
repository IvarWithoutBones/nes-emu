use crate::util;
use bitflags::bitflags;
use std::ops::{Index, Range};

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
        tracing::debug!("DMA write of ${:02X}", addr);
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

bitflags! {
    /*
        https://www.nesdev.org/wiki/PPU_OAM#Byte_2

        76543210
        ||||||||
        ||||||++- Palette (4 to 7) of sprite
        |||+++--- Unimplemented (read 0)
        ||+------ Priority (0: in front of background; 1: behind background)
        |+------- Flip sprite horizontally
        +-------- Flip sprite vertically
    */
    struct ObjectAttrs: u8 {
        const Palette2       = 0b0000_0001;
        const Palette1       = 0b0000_0010;
        const Priority       = 0b0010_0000;
        const FlipHorizontal = 0b0100_0000;
        const FlipVertical   = 0b1000_0000;
    }
}

pub struct Object {
    pub x: usize,
    pub y: usize,
    pub flip_horizontal: bool,
    pub flip_vertical: bool,
    pub behind_background: bool,
    pub palette_index: usize,
    pub tile_index: usize,
}

impl From<&[u8]> for Object {
    fn from(data: &[u8]) -> Self {
        let attrs = ObjectAttrs::from_bits_truncate(data[2]);
        let flip_horizontal = attrs.contains(ObjectAttrs::FlipHorizontal);
        let flip_vertical = attrs.contains(ObjectAttrs::FlipVertical);
        let behind_background = attrs.contains(ObjectAttrs::Priority);

        // TODO: Ignoring 8x16 sprites for now
        let tile_index = data[1] as usize;
        let palette_index = util::combine_bools(
            attrs.contains(ObjectAttrs::Palette1),
            attrs.contains(ObjectAttrs::Palette2),
        ) as usize;

        let x = data[3] as usize;
        let y = data[0] as usize;

        Self {
            x,
            y,
            flip_horizontal,
            flip_vertical,
            behind_background,
            palette_index,
            tile_index,
        }
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
