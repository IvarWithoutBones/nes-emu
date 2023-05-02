use tartan_bitfield::bitfield;

bitfield! {
    /// Register $4001
    /// https://www.nesdev.org/wiki/APU_Sweep
    struct Flags(u8) {
        [0..=2] shift_counter: u8,
        [3] negate,
        [4..=6] divider_period: u8,
        [7] enabled,
    }
}

#[derive(Default, Debug)]
pub struct Sweep {
    flags: Flags,
    timer: u16,
}

impl Sweep {
    pub const PERIOD_MAX: i16 = 0x800;

    pub fn write(&mut self, data: u8) {
        self.flags = Flags::from(data);
    }

    /// https://www.nesdev.org/wiki/APU_Sweep#Calculating_the_target_period
    pub fn target_period(&self, timer: i16) -> i16 {
        if self.flags.negate() {
            // Substract one for ones compliment, since this is channel one
            timer - (timer >> self.flags.shift_counter()) - 1
        } else {
            timer + (timer >> self.flags.shift_counter())
        }
    }

    pub fn tick(&mut self, period: i16) -> Option<i16> {
        let mut result = None;

        if self.timer == 0 && period >= 8 && self.flags.enabled() && self.flags.shift_counter() > 0
        {
            let target_period = self.target_period(period);
            if target_period < Self::PERIOD_MAX {
                result = Some(target_period);
            }
        }

        if self.timer > 0 {
            self.timer -= 1;
        } else {
            self.timer = self.flags.divider_period() as u16;
        }

        result
    }
}
