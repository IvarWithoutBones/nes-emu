use crate::bus::CycleCount;
use tartan_bitfield::bitfield;

#[derive(Debug, PartialEq, Eq)]
pub enum Frame {
    Quarter,
    Half,
}

#[derive(Debug, PartialEq, Eq)]
enum SequencerMode {
    Four,
    Five,
}

impl From<bool> for SequencerMode {
    fn from(value: bool) -> Self {
        if value {
            SequencerMode::Five
        } else {
            SequencerMode::Four
        }
    }
}

bitfield! {
    /// https://www.nesdev.org/wiki/APU_Frame_Counter
    struct Flags(u8) {
        [6] sequencer_mode_bit,
        [7] interrupt_inhibit,
    }
}

impl Flags {
    fn sequencer_mode(&self) -> SequencerMode {
        self.sequencer_mode_bit().into()
    }
}

/// https://www.nesdev.org/wiki/APU_Frame_Counter
#[derive(Debug, Default)]
pub struct FrameCounter {
    pub cycle_count: CycleCount,
    flags: Flags,
}

impl FrameCounter {
    pub fn write(&mut self, data: u8) {
        self.flags = Flags::from(data);
        self.cycle_count = 0;
    }

    pub fn read(&self) -> u8 {
        self.flags.into()
    }

    pub fn tick(&mut self, cpu_cycles: CycleCount) -> Option<Frame> {
        let max = match self.flags.sequencer_mode() {
            SequencerMode::Four => 29_831,
            SequencerMode::Five => 37_283,
        };

        let result = if self.cycle_count >= max {
            // Rollover
            self.cycle_count = 0;
            Some(Frame::Half)
        } else {
            match self.cycle_count {
                7_459 | 22_373 => Some(Frame::Quarter),
                14_915 => Some(Frame::Half),
                _ => None,
            }
        };

        self.cycle_count = self.cycle_count.wrapping_add(cpu_cycles);
        result
    }
}
