#![allow(dead_code)] // TODO: remove

mod registers;

use crate::cartridge::Mirroring;
use registers::{AddressRegister, ControlRegister};

pub const WRITE_ONLY_REGISTERS: [u16; 6] =
    [0x2000, 0x2001, 0x2003, 0x2005, 0x2006, 0x4014];

/// https://www.nesdev.org/wiki/PPU
pub struct Ppu {
    palette_table: [u8; Self::PALETTE_TABLE_SIZE],
    object_attribute_data: [u8; Self::OBJECT_ATTRIBUTE_TABLE_SIZE],
    vram: [u8; Self::VRAM_SIZE],
    data_buffer: u8,

    address: AddressRegister,
    control: ControlRegister,

    mirroring: Mirroring,
    character_rom: Vec<u8>,
}

impl Ppu {
    const PALETTE_TABLE_SIZE: usize = 0x20;
    const OBJECT_ATTRIBUTE_TABLE_SIZE: usize = 0x100;
    const VRAM_SIZE: usize = 0x800;

    const PATTERN_TABLE_START: u16 = 0;
    const PATTERN_TABLE_END: u16 = 0x1FFF;

    const NAMETABLE_START: u16 = 0x2000;
    const NAMETABLE_MIRRORS_END: u16 = 0x3EFF;

    const PALETTE_RAM_START: u16 = 0x3F00;
    const PALETTE_RAM_MIRRORS_END: u16 = 0x3FFF;

    pub fn new(character_rom: Vec<u8>, mirroring: Mirroring) -> Self {
        Self {
            palette_table: [0; Self::PALETTE_TABLE_SIZE],
            object_attribute_data: [0; Self::OBJECT_ATTRIBUTE_TABLE_SIZE],
            vram: [0; Self::VRAM_SIZE],
            data_buffer: 0,
            address: AddressRegister::default(),
            control: ControlRegister::default(),
            mirroring,
            character_rom,
        }
    }

    fn increment_vram_address(&mut self) {
        self.address
            .increment(self.control.vram_address_increment());
    }

    /// https://www.nesdev.org/wiki/Mirroring#Nametable_Mirroring
    fn mirror_nametable_addr(&self, addr: u16) -> u16 {
        // TODO: no idea how this works
        let mirrored_vram = addr & 0b10111111111111; // mirror down 0x3000-0x3eff to 0x2000 - 0x2eff
        let vram_index = mirrored_vram - 0x2000; // to vram vector
        let name_table = vram_index / 0x400; // to the name table index
        match (&self.mirroring, name_table) {
            (Mirroring::Vertical, 2) | (Mirroring::Vertical, 3) => vram_index - 0x800,
            (Mirroring::Horizontal, 2) => vram_index - 0x400,
            (Mirroring::Horizontal, 1) => vram_index - 0x400,
            (Mirroring::Horizontal, 3) => vram_index - 0x800,
            _ => vram_index,
        }
    }

    pub fn read_data(&mut self) -> u8 {
        let addr = self.address.value;
        self.increment_vram_address();

        match addr {
            Self::PATTERN_TABLE_START..=Self::PATTERN_TABLE_END => {
                tracing::debug!(addr, "pattern table read");
                let result = self.data_buffer;
                self.data_buffer = self.character_rom[addr as usize];
                result
            }

            Self::NAMETABLE_START..=Self::NAMETABLE_MIRRORS_END => {
                tracing::debug!(addr, "nametable read");
                let result = self.data_buffer;
                self.data_buffer = self.vram[self.mirror_nametable_addr(addr) as usize];
                result
            }

            Self::PALETTE_RAM_START..=Self::PALETTE_RAM_MIRRORS_END => {
                tracing::debug!(addr, "palette ram read");
                self.palette_table[(addr - Self::PALETTE_RAM_START) as usize]
            }

            _ => {
                tracing::error!(addr, "invalid data read");
                panic!()
            }
        }
    }
}
