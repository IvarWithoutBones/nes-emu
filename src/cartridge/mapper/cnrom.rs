use super::{Cartridge, Mapper, Mirroring, PROGRAM_ROM_START};

/// https://www.nesdev.org/wiki/INES_Mapper_003
pub struct CnROM {
    cartridge: Cartridge,
    bank_select: u8,
}

impl CnROM {
    pub fn new(cartridge: Cartridge) -> Self {
        Self {
            cartridge,
            bank_select: 0,
        }
    }
}

impl Mapper for CnROM {
    fn mirroring(&self) -> Mirroring {
        self.cartridge.header.mirroring
    }

    fn read_cpu(&mut self, address: u16) -> u8 {
        self.cartridge.program_rom[(address - PROGRAM_ROM_START) as usize]
    }

    fn write_cpu(&mut self, _address: u16, value: u8) {
        // Select a character ROM bank mapped to $0000-$1FFF
        self.bank_select = value;
    }

    fn read_ppu(&mut self, address: u16) -> u8 {
        let address = (self.bank_select as usize * 0x2000) + address as usize;
        self.cartridge.character_rom[address]
    }

    fn write_ppu(&mut self, _address: u16, _value: u8) {
        unimplemented!()
    }
}
