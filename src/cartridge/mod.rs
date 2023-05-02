mod mapper;

pub use mapper::MapperInstance;
use std::{fmt, path::PathBuf};
use tartan_bitfield::bitfield;

// TODO: Nicer page abstraction
pub const PROGRAM_ROM_START: u16 = 0x8000;
pub const PROGRAM_ROM_PAGE_SIZE: usize = 16 * 1024;
pub const CHARACTER_ROM_PAGE_SIZE: usize = 8 * 1024;
const TRAINER_SIZE: usize = 512;

#[derive(Debug, Copy, Clone)]
pub enum Mirroring {
    Horizontal,
    Vertical,
    FourScreen,
    OneScreen,
}

impl fmt::Display for Mirroring {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Horizontal => write!(f, "horizontal"),
            Self::Vertical => write!(f, "vertical"),
            Self::FourScreen => write!(f, "four-screen"),
            Self::OneScreen => write!(f, "one-screen"),
        }
    }
}

bitfield! {
    /// Flags 6 and 7 of the iNES header
    pub struct Flags(u16) {
        // https://www.nesdev.org/wiki/INES#Flags_6
        [0] pub mirroring,
        [1] pub persistent_memory,
        [2] pub trainer,
        [3] pub four_screen,
        [4..=7] pub mapper_id_low: u8,

        // https://www.nesdev.org/wiki/INES#Flags_7
        [8..=11] pub mapper_id_high: u8,
        [11..=13] pub ines: u8,
        [14] pub playchoice_10,
        [15] pub vs_unisystem,
    }
}

impl Flags {
    pub fn mapper_id(&self) -> u8 {
        self.mapper_id_low() | self.mapper_id_high()
    }

    pub fn ines_version(&self) -> Option<u8> {
        match self.ines() {
            0 => Some(1),
            2 => Some(2),
            _ => None,
        }
    }
}

#[derive(Debug, Copy, Clone)]
pub struct Header {
    pub mirroring: Mirroring,
    pub program_rom_pages: usize,
    character_rom_pages: usize,
    has_trainer: bool,
    mapper_id: u8,
}

const HEADER_SIZE: usize = 16;

/// https://www.nesdev.org/wiki/INES
impl Header {
    const SIGNATURE: [u8; 4] = [b'N', b'E', b'S', 0x1A];

    fn new(data: [u8; 16]) -> Result<Self, String> {
        if data[0..=3] != Self::SIGNATURE {
            return Err("Invalid ROM file".to_string());
        }

        let flags = Flags::from(u16::from_le_bytes([data[6], data[7]]));

        if flags.ines_version() != Some(1) {
            return Err(format!(
                "Unsupported iNES version: {}",
                if let Some(version) = flags.ines_version() {
                    version.to_string()
                } else {
                    "unknown".to_string()
                }
            ));
        }

        let mirroring = if flags.four_screen() {
            Mirroring::FourScreen
        } else {
            match flags.mirroring() {
                true => Mirroring::Vertical,
                false => Mirroring::Horizontal,
            }
        };

        Ok(Header {
            has_trainer: flags.trainer(),
            program_rom_pages: data[4] as usize,
            character_rom_pages: data[5] as usize,
            mapper_id: flags.mapper_id(),
            mirroring,
        })
    }
}

pub struct Cartridge {
    pub header: Header,
    pub program_rom: Vec<u8>,
    pub character_rom: Vec<u8>,
}

impl Cartridge {
    pub const SPAN_NAME: &'static str = "cartridge";

    pub fn from_bytes(data: &[u8]) -> Result<Cartridge, String> {
        let _span = tracing::span!(tracing::Level::INFO, Cartridge::SPAN_NAME).entered();
        let header = Header::new(data[..HEADER_SIZE].try_into().unwrap())?;
        let program_rom_size = header.program_rom_pages * PROGRAM_ROM_PAGE_SIZE;
        let character_rom_size = header.character_rom_pages * CHARACTER_ROM_PAGE_SIZE;

        let program_rom_start = HEADER_SIZE + if header.has_trainer { TRAINER_SIZE } else { 0 };
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
        tracing::info!("mapper {}\n", header.mapper_id);

        let program_rom = data[program_rom_start..(program_rom_start + program_rom_size)].to_vec();
        let mut character_rom =
            data[character_rom_start..(character_rom_start + character_rom_size)].to_vec();

        // Character RAM is used when there is none provided by the cartridge
        // TODO: handle this from the mapper
        if header.character_rom_pages == 0 {
            character_rom.resize(CHARACTER_ROM_PAGE_SIZE, 0);
        }

        Ok(Cartridge {
            program_rom,
            character_rom,
            header,
        })
    }
}

impl From<PathBuf> for Cartridge {
    fn from(path: PathBuf) -> Self {
        let data = std::fs::read(&path).unwrap_or_else(|err| {
            tracing::error!("failed to read file \"{}\": \"{}\"", path.display(), err);
            std::process::exit(1);
        });

        Cartridge::from_bytes(&data).unwrap_or_else(|err| {
            tracing::error!(
                "failed to load cartridge from \"{}\": \"{}\"",
                path.display(),
                err
            );
            std::process::exit(1);
        })
    }
}
