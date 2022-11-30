use bitflags::bitflags;

#[derive(Debug, Copy, Clone)]
enum Mirroring {
    Horizontal,
    Vertical,
    FourScreen,
}

// TODO: Implement more flags
bitflags! {
    struct Flags6: u8 {
        const MIRRORING   = 0b0000_0001;
        const TRAINER     = 0b0000_0100;
        const FOUR_SCREEN = 0b0000_1000;
        const MAPPER_LOW  = 0b1111_0000;
    }

    struct Flags7: u8 {
        const MAPPER_HIGH = 0b1111_0000;
    }
}

#[derive(Debug, Copy, Clone)]
#[allow(dead_code)]
pub struct Header {
    mirroring: Mirroring,
    program_rom_pages: usize,
    character_rom_pages: usize,
    has_trainer: bool,
    mapper: u8,
}

impl Header {
    fn new(data: [u8; 16]) -> Result<Self, String> {
        if std::str::from_utf8(&data[0..=3]) != Ok("NES\x1a") {
            return Err("Invalid ROM file".to_string());
        }

        let flags_6 = Flags6::from_bits_truncate(data[6]);
        let flags_7 = Flags7::from_bits_truncate(data[7]);

        let mapper = flags_6.bits() & Flags6::MAPPER_LOW.bits()
            | flags_7.bits() & Flags7::MAPPER_HIGH.bits();

        let mirroring = match (
            flags_6.contains(Flags6::FOUR_SCREEN),
            flags_6.contains(Flags6::MIRRORING),
        ) {
            (true, _) => Mirroring::FourScreen,
            (false, true) => Mirroring::Vertical,
            (false, false) => Mirroring::Horizontal,
        };

        Ok(Header {
            has_trainer: flags_6.contains(Flags6::TRAINER),
            program_rom_pages: data[4] as usize,
            character_rom_pages: data[5] as usize,
            mirroring,
            mapper,
        })
    }
}

pub struct Cartridge {
    pub header: Header,
    pub program_rom: Vec<u8>,
    pub character_rom: Vec<u8>,
}

impl Cartridge {
    const PROGRAM_ROM_PAGE_SIZE: usize = 16 * 1024;
    const CHARACTER_ROM_PAGE_SIZE: usize = 8 * 1024;
    const HEADER_SIZE: usize = 16;
    const TRAINER_SIZE: usize = 512;

    pub fn from_path(path: &str) -> Result<Cartridge, String> {
        let data =
            std::fs::read(path).expect(format!("Unable to read file from path {}", path).as_str());

        let header = Header::new(data[..Self::HEADER_SIZE].try_into().unwrap());
        if header.is_err() {
            return Err(header.err().unwrap());
        }
        let header = header.unwrap();

        let program_rom_size = header.program_rom_pages * Self::PROGRAM_ROM_PAGE_SIZE;
        let character_rom_size = header.character_rom_pages * Self::CHARACTER_ROM_PAGE_SIZE;

        let program_rom_start = Self::HEADER_SIZE
            + if header.has_trainer {
                Self::TRAINER_SIZE
            } else {
                0
            };

        let character_rom_start = program_rom_start + program_rom_size;

        Ok(Cartridge {
            program_rom: data[program_rom_start..(program_rom_start + program_rom_size)].to_vec(),
            character_rom: data[character_rom_start..(character_rom_start + character_rom_size)]
                .to_vec(),
            header,
        })
    }
}
