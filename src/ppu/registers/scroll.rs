/// https://www.nesdev.org/wiki/PPU_registers#PPUSCROLL
pub struct Scroll {
    pub x: u8,
    pub y: u8,
    horizontal_latch: bool,
}

impl Default for Scroll {
    fn default() -> Self {
        Self {
            x: 0,
            y: 0,
            horizontal_latch: true,
        }
    }
}

impl Scroll {
    pub fn update(&mut self, data: u8) {
        if self.horizontal_latch {
            self.x = data;
        } else {
            self.y = data;
        }

        self.horizontal_latch = !self.horizontal_latch;
    }

    pub fn reset_latch(&mut self) {
        self.horizontal_latch = true;
    }
}
