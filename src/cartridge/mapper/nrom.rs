use super::{Cartridge, Mapper, Mirroring, PROGRAM_ROM_PAGE_SIZE, PROGRAM_ROM_START};

/// https://www.nesdev.org/wiki/NROM
#[allow(clippy::upper_case_acronyms)]
pub struct NROM {
    cartridge: Cartridge,
}

impl NROM {
    pub fn new(cartridge: Cartridge) -> Self {
        Self { cartridge }
    }
}

impl Mapper for NROM {
    fn mirroring(&self) -> Mirroring {
        self.cartridge.header.mirroring
    }

    fn read_cpu(&mut self, mut address: u16) -> u8 {
        address -= PROGRAM_ROM_START;
        if self.cartridge.header.program_rom_pages == 1 {
            address %= PROGRAM_ROM_PAGE_SIZE as u16;
        }
        self.cartridge.program_rom[address as usize]
    }

    fn read_ppu(&mut self, address: u16) -> u8 {
        self.cartridge.character_rom[address as usize]
    }

    fn write_ppu(&mut self, _address: u16, _value: u8) {
        unreachable!("cartridge's character rom is read-only");
    }

    fn write_cpu(&mut self, _address: u16, _value: u8) {
        unreachable!("cartridge's program rom is read-only");
    }
}
