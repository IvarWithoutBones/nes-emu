use super::{Cartridge, Mapper, Mirroring};
use crate::util;

/// https://www.nesdev.org/wiki/MMC3
pub struct MMC3 {
    cartridge: Cartridge,

    program_ram: [u8; 0x2000],
    program_ram_enabled: bool,
    program_ram_write_protected: bool,

    registers: [u8; 8],
    bank_index: u8,
    program_rom_bank_mode: bool,
    character_rom_bank_mode: bool,

    // TODO: implement the interrupt mechanism, these are a placeholder
    interrupt_enable: bool,
    interrupt_flag: bool,
    interrupt_reset: bool,
    interrupt_counter: u8,
}

impl MMC3 {
    pub const fn new(cartridge: Cartridge) -> Self {
        Self {
            cartridge,

            program_ram: [0; 0x2000],
            program_ram_enabled: false,
            program_ram_write_protected: false,

            registers: [0; 8],
            bank_index: 0,
            program_rom_bank_mode: false,
            character_rom_bank_mode: false,

            interrupt_enable: false,
            interrupt_reset: false,
            interrupt_flag: false,
            interrupt_counter: 0,
        }
    }

    const fn bank(&self, index: usize) -> usize {
        index * 0x2000
    }

    fn total_program_rom_banks(&self) -> usize {
        self.cartridge.program_rom.len() / 0x2000
    }
}

impl Mapper for MMC3 {
    fn mirroring(&self) -> Mirroring {
        self.cartridge.header.mirroring
    }

    fn read_cpu(&mut self, address: u16) -> u8 {
        match (address, self.program_rom_bank_mode) {
            (0x6000..=0x7FFF, _) => {
                if self.program_ram_enabled {
                    let address = address - 0x6000;
                    self.program_ram[address as usize]
                } else {
                    // Open bus
                    0
                }
            }

            (0x8000..=0x9FFF, false) => {
                let bank = self.bank(self.registers[6] as usize);
                let address = address - 0x8000;
                self.cartridge.program_rom[bank + address as usize]
            }

            (0x8000..=0x9FFF, true) => {
                let bank = self.bank(self.total_program_rom_banks() - 2);
                let address = address - 0x8000;
                self.cartridge.program_rom[bank + address as usize]
            }

            (0xA000..=0xBFFF, _) => {
                let bank = self.bank(self.registers[7] as usize);
                let address = address - 0xA000;
                self.cartridge.program_rom[bank + address as usize]
            }

            (0xC000..=0xDFFF, false) => {
                let bank = self.bank(self.total_program_rom_banks() - 2);
                let address = address - 0xC000;
                self.cartridge.program_rom[bank + address as usize]
            }

            (0xC000..=0xDFFF, true) => {
                let bank = self.bank(self.registers[6] as usize);
                let address = address - 0xC000;
                self.cartridge.program_rom[bank + address as usize]
            }

            (0xE000..=0xFFFF, _) => {
                let bank = self.bank(self.total_program_rom_banks() - 1);
                let address = address - 0xE000;
                self.cartridge.program_rom[bank + address as usize]
            }

            _ => panic!(
                "MMC3: Unhandled read at address: {address:#04X}, bank mode = {}",
                self.program_rom_bank_mode
            ),
        }
    }

    fn write_cpu(&mut self, address: u16, value: u8) {
        match (address, address % 2 == 0) {
            (0x6000..=0x7FFF, _) => {
                if self.program_ram_enabled && !self.program_ram_write_protected {
                    let address = address - 0x6000;
                    self.program_ram[address as usize] = value;
                }
            }

            (0x8000..=0x9FFF, true) => {
                self.bank_index = value & 0b0000_0111;
                self.program_rom_bank_mode = util::nth_bit(value, 6);
                self.character_rom_bank_mode = util::nth_bit(value, 7);
            }

            (0x8000..=0x9FFF, false) => {
                let value = match self.bank_index {
                    (0..=1) => value & 0b1111_1110,
                    (6..=7) => value & 0b0011_1111,
                    _ => value,
                };

                self.registers[self.bank_index as usize] = value;
            }

            (0xA000..=0xBFFF, true) => {
                self.cartridge.header.mirroring = if util::nth_bit(value, 0) {
                    Mirroring::Horizontal
                } else {
                    Mirroring::Vertical
                };
            }

            (0xA000..=0xBFFF, false) => {
                self.program_ram_write_protected = util::nth_bit(value, 6);
                self.program_ram_enabled = util::nth_bit(value, 7);
            }

            (0xC000..=0xDFFF, true) => {
                self.interrupt_counter = value;
            }

            (0xC000..=0xDFFF, false) => {
                self.interrupt_reset = true;
            }

            (0xE000..=0xFFFF, true) => {
                self.interrupt_enable = false;
                self.interrupt_flag = false;
            }

            (0xE000..=0xFFFF, false) => {
                self.interrupt_enable = true;
            }

            _ => todo!("MMC3: Unhandled write at address: {address:#04X}"),
        }
    }

    fn read_ppu(&mut self, address: u16) -> u8 {
        let bank = match (address, self.character_rom_bank_mode) {
            (0x0000..=0x03FF, false) => self.registers[0] & !1,
            (0x0000..=0x03FF, true) => self.registers[2],
            (0x0400..=0x07FF, false) => self.registers[0] | 1,
            (0x0400..=0x07FF, true) => self.registers[3],
            (0x0800..=0x0BFF, false) => self.registers[1] & !1,
            (0x0800..=0x0BFF, true) => self.registers[4],
            (0x0C00..=0x0FFF, false) => self.registers[1] | 1,
            (0x0C00..=0x0FFF, true) => self.registers[5],

            (0x1000..=0x13FF, false) => self.registers[2],
            (0x1000..=0x13FF, true) => self.registers[0] & !1,
            (0x1400..=0x17FF, false) => self.registers[3],
            (0x1400..=0x17FF, true) => self.registers[0] | 1,
            (0x1800..=0x1BFF, false) => self.registers[4],
            (0x1800..=0x1BFF, true) => self.registers[1] & !1,
            (0x1C00..=0x1FFF, false) => self.registers[5],
            (0x1C00..=0x1FFF, true) => self.registers[1] | 1,
            _ => panic!(),
        } as usize;

        let address = (bank * 0x400) + (address as usize % 0x400);
        self.cartridge.character_rom[address]
    }

    fn write_ppu(&mut self, _address: u16, _value: u8) {
        unimplemented!()
    }
}
