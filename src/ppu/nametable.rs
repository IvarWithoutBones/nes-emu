use {
    super::VideoRam,
    crate::cartridge::Mirroring,
    std::ops::{Index, Range},
};

const NAMETABLE_LEN: u16 = 0x400;
const ATTRIBUTE_TABLE_LEN: u16 = 64;

#[repr(u16)]
#[derive(Clone)]
pub enum NametableAddr {
    One = 0,
    Two = 0x400,
    Three = 0x800,
    Four = 0xC00,
}

impl NametableAddr {
    const VRAM_BASE: u16 = 0x2000;

    /// https://www.nesdev.org/wiki/Mirroring#Nametable_Mirroring
    pub fn mirror(addr: u16, mirroring: Mirroring) -> u16 {
        let vram_index = {
            let mirrored = addr & 0b10111111111111; // mirror down 0x3000-0x3eff to 0x2000 - 0x2eff
            mirrored - Self::VRAM_BASE
        };
        let nametable = Self::from(vram_index / NAMETABLE_LEN);

        match (mirroring, nametable) {
            (Mirroring::Vertical, Self::Three)
            | (Mirroring::Vertical, Self::Four)
            | (Mirroring::Horizontal, Self::Four) => vram_index - Self::Three as u16,

            (Mirroring::Horizontal, Self::Three) | (Mirroring::Horizontal, Self::Two) => {
                vram_index - Self::Two as u16
            }

            _ => vram_index,
        }
    }
}

impl From<u16> for NametableAddr {
    fn from(value: u16) -> Self {
        match value {
            0 => Self::One,
            1 => Self::Two,
            2 => Self::Three,
            3 => Self::Four,
            _ => unreachable!(),
        }
    }
}

pub struct Nametable<'a>(&'a [u8]);

impl<'a> Nametable<'a> {
    pub fn from(vram: &'a VideoRam, address: NametableAddr, mirroring: Mirroring) -> (Self, Self) {
        let first = Self(&vram[NametableAddr::One as usize..NametableAddr::Two as usize]);
        let second = Self(&vram[NametableAddr::Two as usize..NametableAddr::Three as usize]);

        // TODO: use NametableAddr::mirror?
        match (mirroring, address.clone()) {
            (Mirroring::Vertical, NametableAddr::One)
            | (Mirroring::Vertical, NametableAddr::Three)
            | (Mirroring::Horizontal, NametableAddr::One)
            | (Mirroring::Horizontal, NametableAddr::Two) => (first, second),

            (Mirroring::Vertical, NametableAddr::Two)
            | (Mirroring::Vertical, NametableAddr::Four)
            | (Mirroring::Horizontal, NametableAddr::Three)
            | (Mirroring::Horizontal, NametableAddr::Four) => (second, first),

            _ => {
                let address = address as u16;
                panic!("unsupported mirroring {mirroring} for nametable {address:04X}")
            }
        }
    }

    pub fn get_attribute(&self, index: usize) -> u8 {
        // TODO: move more of the logic into this method
        self.0[(NAMETABLE_LEN - ATTRIBUTE_TABLE_LEN) as usize + index]
    }
}

impl Index<usize> for Nametable<'_> {
    type Output = u8;

    fn index(&self, index: usize) -> &Self::Output {
        &self.0[index]
    }
}

impl Index<Range<usize>> for Nametable<'_> {
    type Output = [u8];

    fn index(&self, index: Range<usize>) -> &Self::Output {
        &self.0[index]
    }
}
