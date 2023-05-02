use super::{Cartridge, Mapper, Mirroring, PROGRAM_ROM_PAGE_SIZE, PROGRAM_ROM_START};
use crate::util;
use tartan_bitfield::bitfield;

enum ProgramRomBank {
    Consecutive,
    FixFirst,
    FixLast,
}

enum CharacterRomBank {
    Consecutive,
    Split,
}

bitfield! {
    /*
        4bit0
        -----
        CPPMM
        |||||
        |||++- Mirroring (0: one-screen, lower bank; 1: one-screen, upper bank;
        |||               2: vertical; 3: horizontal)
        |++--- PRG ROM bank mode (0, 1: switch 32 KB at $8000, ignoring low bit of bank number;
        |                         2: fix first bank at $8000 and switch 16 KB bank at $C000;
        |                         3: fix last bank at $C000 and switch 16 KB bank at $8000)
        +----- CHR ROM bank mode (0: switch 8 KB at a time; 1: switch two separate 4 KB banks)
    */
    /// https://www.nesdev.org/wiki/MMC1#Control_(internal,_$8000-$9FFF)
    pub struct ControlRegister(u8) {
        [0..=1] pub mirroring_mode: u8,
        [2..=3] pub program_rom_bank_mode: u8,
        [4] pub character_rom_bank_mode,
    }
}

impl ControlRegister {
    fn mirroring(&self) -> Mirroring {
        match self.mirroring_mode() {
            0 | 1 => Mirroring::OneScreen,
            2 => Mirroring::Vertical,
            3 => Mirroring::Horizontal,
            _ => unreachable!(),
        }
    }

    fn program_rom_bank(&self) -> ProgramRomBank {
        match self.program_rom_bank_mode() {
            0 | 1 => ProgramRomBank::Consecutive,
            2 => ProgramRomBank::FixFirst,
            3 => ProgramRomBank::FixLast,
            _ => unreachable!(),
        }
    }

    fn character_rom_bank(&self) -> CharacterRomBank {
        if self.character_rom_bank_mode() {
            CharacterRomBank::Split
        } else {
            CharacterRomBank::Consecutive
        }
    }
}

/// https://www.nesdev.org/wiki/MMC1
#[allow(clippy::upper_case_acronyms)]
pub struct MMC1 {
    cartridge: Cartridge,

    shift_count: u8,
    shift_register: u8,

    control: ControlRegister,
    character_bank_0: u8,
    character_bank_1: u8,
    program_bank: u8,
}

impl MMC1 {
    pub fn new(cartridge: Cartridge) -> Self {
        Self {
            cartridge,
            shift_count: 0,
            shift_register: 0,
            control: ControlRegister::default(),
            character_bank_0: 0,
            character_bank_1: 0,
            program_bank: 0,
        }
    }

    fn reset_shift(&mut self) {
        self.shift_count = 0;
        self.shift_register = 0;
        self.control = ControlRegister(u8::from(self.control) | 0x0C);
    }

    fn read_shift(&self) -> u8 {
        self.shift_register & 0b0001_1111
    }

    const fn bank(&self, index: u8) -> usize {
        index as usize * PROGRAM_ROM_PAGE_SIZE
    }

    fn ppu_bank(&self, address: u16) -> usize {
        match address {
            (0..=0x0FFF) => match self.control.character_rom_bank() {
                CharacterRomBank::Consecutive => self.character_bank_0 & 0b1111_1110,
                CharacterRomBank::Split => self.character_bank_0,
            },

            (0x1000..=0x1FFF) => match self.control.character_rom_bank() {
                CharacterRomBank::Consecutive => self.character_bank_1 & 0b1111_1110,
                CharacterRomBank::Split => self.character_bank_1,
            },

            _ => unreachable!(),
        }
        .into()
    }
}

impl Mapper for MMC1 {
    fn mirroring(&self) -> Mirroring {
        self.control.mirroring()
    }

    fn read_cpu(&mut self, address: u16) -> u8 {
        const LAST_BANK: u16 = PROGRAM_ROM_START + PROGRAM_ROM_PAGE_SIZE as u16;

        if (PROGRAM_ROM_START..LAST_BANK).contains(&address) {
            let bank = self.bank(match self.control.program_rom_bank() {
                ProgramRomBank::Consecutive => self.program_bank & 0b1111_1110,
                ProgramRomBank::FixFirst => self.program_bank,
                ProgramRomBank::FixLast => 0,
            });
            self.cartridge.program_rom[bank + (address - PROGRAM_ROM_START) as usize]
        } else {
            let bank = self.bank(match self.control.program_rom_bank() {
                ProgramRomBank::Consecutive => (self.program_bank & 0b1111_1110) | 1,
                ProgramRomBank::FixFirst => self.program_bank,
                ProgramRomBank::FixLast => self.cartridge.header.program_rom_pages as u8 - 1,
            });
            self.cartridge.program_rom[bank + (address - LAST_BANK) as usize]
        }
    }

    fn write_cpu(&mut self, address: u16, value: u8) {
        if util::nth_bit(value, 7) {
            self.reset_shift()
        }

        self.shift_register |= (util::nth_bit(value, 0) as u8) << self.shift_count;
        self.shift_count += 1;

        if self.shift_count == 5 {
            self.shift_count = 0;
            match address {
                (0x8000..=0x9FFF) => {
                    self.control = ControlRegister(self.read_shift());
                }

                (0xA000..=0xBFFF) => {
                    self.character_bank_0 = self.read_shift();
                }

                (0xC000..=0xDFFF) => {
                    self.character_bank_1 = self.read_shift();
                }

                (0xE000..=0xFFFF) => {
                    // TODO: Highest bit selects wether program RAM is enabled
                    self.program_bank = self.read_shift() & 0b0000_1111;
                }

                _ => unreachable!(),
            }

            self.shift_register = 0;
        }
    }

    fn read_ppu(&mut self, address: u16) -> u8 {
        self.cartridge.character_rom[(self.ppu_bank(address)) + address as usize]
    }

    fn write_ppu(&mut self, address: u16, value: u8) {
        let bank = self.ppu_bank(address);
        self.cartridge.character_rom[bank + address as usize] = value;
    }
}
