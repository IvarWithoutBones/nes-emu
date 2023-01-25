use super::{Cartridge, Mapper, Mirroring, PROGRAM_ROM_PAGE_SIZE, PROGRAM_ROM_START};

/// https://www.nesdev.org/wiki/UxROM
pub struct UxROM {
    cartridge: Cartridge,
    bank_select: u8,
}

impl UxROM {
    pub fn new(cartridge: Cartridge) -> Self {
        Self {
            cartridge,
            bank_select: 0,
        }
    }

    const fn bank(&self, index: usize) -> usize {
        index * PROGRAM_ROM_PAGE_SIZE
    }
}

impl Mapper for UxROM {
    fn mirroring(&self) -> Mirroring {
        self.cartridge.header.mirroring
    }

    fn read_cpu(&mut self, mut address: u16) -> u8 {
        const LAST_BANK: u16 = PROGRAM_ROM_START + PROGRAM_ROM_PAGE_SIZE as u16;
        if (PROGRAM_ROM_START..LAST_BANK).contains(&address) {
            // Variable bank of program ROM
            let bank = self.bank(self.bank_select as usize);
            address -= PROGRAM_ROM_START;
            self.cartridge.program_rom[bank + address as usize]
        } else if (LAST_BANK..=0xFFFF).contains(&address) {
            // Last 16KB of program ROM
            let bank = self.bank(self.cartridge.header.program_rom_pages - 1);
            address -= LAST_BANK;
            self.cartridge.program_rom[bank + address as usize]
        } else {
            panic!("invalid address: ${:04X}", address);
        }
    }

    fn write_cpu(&mut self, _address: u16, value: u8) {
        const BANK_SELECT_MASK: u8 = 0b0000_1111;
        self.bank_select = value & BANK_SELECT_MASK;
    }

    fn read_ppu(&mut self, address: u16) -> u8 {
        self.cartridge.character_rom[address as usize]
    }

    fn write_ppu(&mut self, address: u16, value: u8) {
        self.cartridge.character_rom[address as usize] = value;
        tracing::warn!(
            "writing to normally read-only character rom: ${:04X} = ${:02X}",
            address,
            value
        );
    }
}
