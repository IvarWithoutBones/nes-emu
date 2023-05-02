use core::fmt;
use tartan_bitfield::bitfield;

const LENGTH_TABLE: [u8; 32] = [
    0x0A, 0xFE, 0x14, 0x02, 0x28, 0x04, 0x50, 0x06, 0xA0, 0x08, 0x3C, 0x0A, 0x0E, 0x0C, 0x1A, 0x0E,
    0x0C, 0x10, 0x18, 0x12, 0x30, 0x14, 0x60, 0x16, 0xC0, 0x18, 0x48, 0x1A, 0x10, 0x1C, 0x20, 0x1E,
];

#[derive(Debug, Default, PartialEq, PartialOrd)]
pub enum State {
    Enabled,
    #[default]
    Disabled,
    Halted,
}

impl From<bool> for State {
    fn from(value: bool) -> Self {
        if value {
            State::Enabled
        } else {
            State::Disabled
        }
    }
}

bitfield! {
    /// Register $4003
    /// https://www.nesdev.org/wiki/APU_Pulse#Registers
    struct LengthCounterHigh(u8) {
        [0..=3] timer: u8,
        [4..=7] length_counter: u8,
    }
}

/// Registers $4002 and $4003
/// https://www.nesdev.org/wiki/APU_Pulse#Registers
#[derive(Default)]
pub struct LengthCounter {
    // Register $4002
    low: u8,
    // Register $4003
    high: LengthCounterHigh,

    state: State,
}

impl LengthCounter {
    pub fn timer(&self) -> u16 {
        u16::from_le_bytes([self.low, self.high.timer()])
    }

    pub fn set_timer(&mut self, value: u16) {
        let [low, high] = value.to_le_bytes();
        self.low = low;
        self.high.set_timer(high);
    }

    pub fn active(&self) -> bool {
        self.state == State::Enabled && self.timer() > 0
    }

    pub fn set_state(&mut self, state: State) {
        self.state = state;
    }

    pub fn set_high(&mut self, value: u8) {
        self.high = LengthCounterHigh::from(value);
        self.high
            .set_length_counter(LENGTH_TABLE[self.high.length_counter() as usize]);
    }

    pub fn set_low(&mut self, value: u8) {
        self.low = value;
    }

    pub fn tick(&mut self) {
        if self.state == State::Enabled {
            let new = self.high.length_counter().saturating_sub(1);
            self.high.set_length_counter(new);
        }
    }
}

impl fmt::Debug for LengthCounter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("LengthCounter")
            .field("state", &self.state)
            .field("timer", &self.timer())
            .field("length_counter", &self.high.length_counter())
            .finish()
    }
}
