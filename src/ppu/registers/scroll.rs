/// https://www.nesdev.org/wiki/PPU_registers#PPUSCROLL
pub struct Scroll {
    horizontal: u8,
    vertical: u8,
    horizontal_latch: bool,
}

impl Default for Scroll {
    fn default() -> Self {
        Self {
            horizontal: 0,
            vertical: 0,
            horizontal_latch: true,
        }
    }
}

impl Scroll {
    pub fn update(&mut self, data: u8) {
        if self.horizontal_latch {
            self.horizontal = data;
        } else {
            self.vertical = data;
        }

        self.horizontal_latch = !self.horizontal_latch;
    }
}

