mod mmc1;
mod nrom;
mod uxrom;

pub use super::{Cartridge, Mirroring, PROGRAM_ROM_PAGE_SIZE, PROGRAM_ROM_START};
use {
    crate::bus::Device,
    std::{cell::RefCell, ops::Range, rc::Rc},
};

// Rc is used to allow the mapper to be shared between the bus and the PPU, which can
// mutate it hence the RefCell. Trait objects are not sized, which a Box fixes.
pub type MapperInstance = Rc<RefCell<Box<dyn Mapper>>>;

pub trait Mapper {
    fn mirroring(&self) -> Mirroring;

    fn read_cpu(&mut self, address: u16) -> u8;
    fn write_cpu(&mut self, address: u16, data: u8);

    fn read_ppu(&mut self, address: u16) -> u8;
    fn write_ppu(&mut self, address: u16, data: u8);

    fn read_cpu_range(&mut self, range: Range<usize>) -> Vec<u8> {
        range.map(|address| self.read_cpu(address as u16)).collect()
    }

    fn write_cpu_range(&mut self, range: Range<usize>, data: &[u8]) {
        range
            .zip(data.iter())
            .for_each(|(address, data)| self.write_cpu(address as u16, *data));
    }

    fn read_ppu_range(&mut self, range: Range<usize>) -> Vec<u8> {
        range.map(|address| self.read_ppu(address as u16)).collect()
    }

    fn write_ppu_range(&mut self, range: Range<usize>, data: &[u8]) {
        range
            .zip(data.iter())
            .for_each(|(address, data)| self.write_ppu(address as u16, *data));
    }
}

impl<T> Device for T
where
    T: Mapper + ?Sized,
{
    fn contains(&self, address: u16) -> bool {
        address >= PROGRAM_ROM_START
    }
}

impl From<Cartridge> for Box<dyn Mapper> {
    fn from(cart: Cartridge) -> Self {
        match cart.header.mapper_id {
            0 => Box::new(nrom::NROM::new(cart)),
            1 => Box::new(mmc1::MMC1::new(cart)),
            2 => Box::new(uxrom::UxROM::new(cart)),
            _ => panic!("mapper {} not implemented", cart.header.mapper_id),
        }
    }
}
