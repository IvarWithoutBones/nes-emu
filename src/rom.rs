#![allow(dead_code)]

use bitflags::bitflags;

bitflags! {
    #[rustfmt::skip]
    struct Flags6: u8 {
        const MIRRORING   = 0b0000_0001;
        const BATTERY_RAM = 0b0000_0010;
        const TRAINER     = 0b0000_0100;
        const FOUR_SCREEN = 0b0000_1000;
        const MAPPER_LOW  = 0b1111_0000;
    }

    #[rustfmt::skip]
    struct Flags7: u8 {
        // TODO: implement other flags
        const MAPPER_HIGH =   0b1111_0000;
    }
}

#[derive(Debug, Copy, Clone)]
struct Header {
    program_rom_size: usize,
    character_rom_size: usize,
    mapper: u8,

    mirroring_vertical: bool,
    battery_ram: bool,
    has_trainer: bool,
    four_screen: bool,
}

impl Header {
    fn new(data: [u8; 16]) -> Header {
        if std::str::from_utf8(&data[0..=3]) != Ok("NES\x1a") {
            panic!("Invalid ROM file");
        }

        let flags_6 = Flags6::from_bits_truncate(data[6]);
        let flags_7 = Flags7::from_bits_truncate(data[7]);

        let mapper = flags_6.bits() & Flags6::MAPPER_LOW.bits()
            | flags_7.bits() & Flags7::MAPPER_HIGH.bits();

        Header {
            program_rom_size: Self::program_rom_size(data[4]),
            character_rom_size: Self::character_rom_size(data[5]),
            mirroring_vertical: flags_6.contains(Flags6::MIRRORING),
            battery_ram: flags_6.contains(Flags6::BATTERY_RAM),
            has_trainer: flags_6.contains(Flags6::TRAINER),
            four_screen: flags_6.contains(Flags6::FOUR_SCREEN),
            mapper,
        }
    }

    const fn into_kb(units: usize, value: usize) -> usize {
        units * (value * 1024)
    }

    const fn program_rom_size(size: u8) -> usize {
        Header::into_kb(16, size as usize)
    }

    const fn character_rom_size(size: u8) -> usize {
        Header::into_kb(8, size as usize)
    }
}

pub struct Rom {
    header: Header,
    pub program_rom: Vec<u8>,
    pub character_rom: Option<Vec<u8>>,
    pub trainer: Option<[u8; Self::TRAINER_SIZE]>,
}

impl Rom {
    const HEADER_END: usize = 16;
    const TRAINER_SIZE: usize = 512;

    pub fn from_file(path: &str) -> Rom {
        let data =
            std::fs::read(path).expect(format!("Unable to read file from path {}", path).as_str());
        let header = Header::new(data[..Self::HEADER_END].try_into().unwrap());

        let trainer: Option<[u8; Self::TRAINER_SIZE]>;
        let trainer_end: usize;
        if header.has_trainer {
            trainer = Some(
                data[Self::HEADER_END..Self::HEADER_END + Self::TRAINER_SIZE]
                    .try_into()
                    .unwrap(),
            );
            trainer_end = Self::HEADER_END + Self::TRAINER_SIZE;
        } else {
            trainer = None;
            trainer_end = Self::TRAINER_SIZE;
        }

        let character_rom: Option<Vec<u8>>;
        if header.character_rom_size != 0 {
            character_rom =
                Some(data[trainer_end..trainer_end + header.character_rom_size].to_vec());
        } else {
            character_rom = None;
        }

        Rom {
            header,
            trainer,
            character_rom,
            program_rom: data[trainer_end..(trainer_end + header.program_rom_size)].to_vec(),
        }
    }
}
