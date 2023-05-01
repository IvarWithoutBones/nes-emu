use super::{Cartridge, Mapper, Mirroring, PROGRAM_ROM_PAGE_SIZE, PROGRAM_ROM_START};

const LAST_BANK_START: u16 = PROGRAM_ROM_START + PROGRAM_ROM_PAGE_SIZE as u16;
const FIRST_BANK_END: u16 = LAST_BANK_START - 1;

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
        match address {
            // Variable bank of program ROM
            (PROGRAM_ROM_START..=FIRST_BANK_END) => {
                address -= PROGRAM_ROM_START;
                let bank = self.bank(self.bank_select as usize);
                self.cartridge.program_rom[bank + address as usize]
            }

            // Last 16KB of program ROM
            (LAST_BANK_START..=0xFFFF) => {
                address -= LAST_BANK_START;
                let bank = self.bank(self.cartridge.header.program_rom_pages - 1);
                self.cartridge.program_rom[bank + address as usize]
            }

            _ => panic!("invalid address: ${address:04X}"),
        }
    }

    fn write_cpu(&mut self, _address: u16, value: u8) {
        // Select a program ROM bank mapped to $8000-$BFFF
        self.bank_select = value & 0b0000_0111;
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
