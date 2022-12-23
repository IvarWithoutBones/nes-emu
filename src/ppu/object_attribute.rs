use std::ops::RangeInclusive;

/// https://www.nesdev.org/wiki/PPU_OAM
pub struct ObjectAttributeMemory {
    span: tracing::Span,
    memory: [u8; Self::MEMORY_SIZE],
    address: u8,
}

impl Default for ObjectAttributeMemory {
    fn default() -> Self {
        Self {
            span: tracing::span!(tracing::Level::INFO, "oam"),
            memory: [0; Self::MEMORY_SIZE],
            address: 0,
        }
    }
}

impl ObjectAttributeMemory {
    const MEMORY_SIZE: usize = 0xFF;

    #[tracing::instrument(skip(self, data), parent = &self.span)]
    pub fn write_address(&mut self, data: u8) {
        tracing::trace!("address write of ${:02X}", data);
        self.address = data;
    }

    #[tracing::instrument(skip(self, data), parent = &self.span)]
    pub fn write_data(&mut self, data: u8) {
        tracing::trace!("oam write at ${:02X}: ${:02X}", self.address, data);
        self.memory[self.address as usize] = data;
        self.address = self.address.wrapping_add(1);
    }

    #[tracing::instrument(skip(self), parent = &self.span)]
    pub fn read_data(&self) -> u8 {
        let result = self.memory[self.address as usize];
        tracing::trace!("read at ${:02X}: ${:02X}", self.address, result);
        result
    }

    #[tracing::instrument(skip(self, addr, func), parent = &self.span)]
    pub fn write_dma<F>(&mut self, addr: u8, func: F)
    where
        F: FnOnce(RangeInclusive<usize>) -> [u8; Self::MEMORY_SIZE],
    {
        // Convert to a page index: $XX -> $XX00
        let begin = ((addr * 16) * 16) as usize;
        let end = begin + Self::MEMORY_SIZE;
        tracing::trace!("DMA transfer at ${:02X}", self.address);

        let buffer = func(begin..=end);
        for byte in buffer {
            self.memory[self.address as usize] = byte;
            self.address = self.address.wrapping_add(1);
        }
    }
}
