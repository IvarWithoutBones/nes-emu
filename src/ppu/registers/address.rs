/// https://www.nesdev.org/wiki/PPU_registers#PPUADDR
pub struct Address {
    pub value: u16,
    latch_high: bool,
}

impl Default for Address {
    fn default() -> Self {
        Self {
            value: 0,
            // Since this is big-endian we want to write the high byte first
            latch_high: true,
        }
    }
}

impl Address {
    fn mirror(&mut self) {
        const HIGHEST_VALID_ADDR: u16 = 0x4000;
        if self.value >= HIGHEST_VALID_ADDR {
            self.value = self.value % HIGHEST_VALID_ADDR;
        }
    }

    pub fn update(&mut self, data: u8) {
        let mut bytes = u16::to_be_bytes(self.value);
        if self.latch_high {
            bytes[0] = data;
        } else {
            bytes[1] = data;
        }

        self.value = u16::from_be_bytes(bytes);
        self.mirror();
        self.latch_high = !self.latch_high;
    }

    pub fn increment(&mut self, increment: u8) {
        self.value = self.value.wrapping_add(increment as u16);
        self.mirror();
    }

    pub fn reset_latch(&mut self) {
        self.latch_high = true;
    }
}
