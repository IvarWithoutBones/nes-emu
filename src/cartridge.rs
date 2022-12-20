use bitflags::bitflags;
use std::fmt;

#[derive(Debug, Copy, Clone)]
pub enum Mirroring {
    Horizontal,
    Vertical,
    FourScreen,
}

impl fmt::Display for Mirroring {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Horizontal => write!(f, "horizontal"),
            Self::Vertical => write!(f, "vertical"),
            Self::FourScreen => write!(f, "four-screen"),
        }
    }
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
pub struct Header {
    pub mirroring: Mirroring,
    program_rom_pages: usize,
    character_rom_pages: usize,
    has_trainer: bool,
    mapper: u8,
}

const HEADER_SIZE: usize = 16;

impl Header {
    const SIGNATURE: [u8; 4] = [b'N', b'E', b'S', 0x1A];

    fn new(data: [u8; 16]) -> Result<Self, String> {
        if &data[0..=3] != Self::SIGNATURE {
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

    /// Dummy header for testing
    const fn generate() -> [u8; HEADER_SIZE] {
        [
            Self::SIGNATURE[0],
            Self::SIGNATURE[1],
            Self::SIGNATURE[2],
            Self::SIGNATURE[3],
            1, // Program ROM pages
            0, // Character ROM pages
            0, // Flags 6
            0, // Flags 7
            0, // Flags 8
            0, // Flags 9
            0, // Flags 10
            0, // Zero
            0, // Zero
            0, // Zero
            0, // Zero
            0, // Zero
        ]
    }
}

#[derive(Clone)]
pub struct Cartridge {
    pub header: Header,
    pub program_rom: Vec<u8>,
    pub character_rom: Vec<u8>,
}

impl Cartridge {
    pub const SPAN_NAME: &'static str = "cartridge";

    const PROGRAM_ROM_PAGE_SIZE: usize = 16 * 1024;
    const CHARACTER_ROM_PAGE_SIZE: usize = 8 * 1024;
    const TRAINER_SIZE: usize = 512;

    pub fn from_bytes(data: &Vec<u8>) -> Result<Cartridge, String> {
        let _span = tracing::span!(tracing::Level::INFO, Cartridge::SPAN_NAME).entered();
        let header = Header::new(data[..HEADER_SIZE].try_into().unwrap())?;
        let program_rom_size = header.program_rom_pages * Self::PROGRAM_ROM_PAGE_SIZE;
        let character_rom_size = header.character_rom_pages * Self::CHARACTER_ROM_PAGE_SIZE;

        let program_rom_start = HEADER_SIZE
            + if header.has_trainer {
                Self::TRAINER_SIZE
            } else {
                0
            };

        let character_rom_start = program_rom_start + program_rom_size;

        tracing::debug!(header.has_trainer);
        tracing::info!(
            "{} program ROM page(s), {} bytes",
            header.program_rom_pages,
            program_rom_size
        );
        tracing::info!(
            "{} character ROM page(s), {} bytes",
            header.character_rom_pages,
            character_rom_size
        );
        tracing::info!("{} mirroring", header.mirroring);
        tracing::info!("mapper {}\n", header.mapper);

        Ok(Cartridge {
            program_rom: data[program_rom_start..(program_rom_start + program_rom_size)].to_vec(),
            character_rom: data[character_rom_start..(character_rom_start + character_rom_size)]
                .to_vec(),
            header,
        })
    }

    /// Generate a dummy cartridge with the given program, used for tests
    #[allow(dead_code)]
    pub fn new_dummy(data: Vec<u8>) -> Result<Cartridge, String> {
        let header = Header::generate().to_vec();
        let mut program: Vec<u8> = vec![header, data].concat();
        let len = program.len();
        program.resize(Self::PROGRAM_ROM_PAGE_SIZE + len, 0xEA); // NOP
        program[len] = 0x00; // BRK
        Self::from_bytes(&program)
    }
}
