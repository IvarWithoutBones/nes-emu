use super::{
    frame_counter::Frame,
    length_counter::{self, LengthCounter},
    sweep::Sweep,
};
use crate::bus::{Device, Memory};
use tartan_bitfield::bitfield;

bitfield! {
    /// Register $4000
    /// https://www.nesdev.org/wiki/APU_Pulse#Registers
    struct Control(u8) {
        [0..=3] envelope_divider_period: u8,
        [4] envelope_volume,
        [5] length_counter_halt,
        [6..=7] duty_cycle: u8,
    }
}

/// https://www.nesdev.org/wiki/APU_Pulse
#[derive(Default, Debug)]
pub struct PulseChannel {
    control: Control,
    sweep: Sweep,
    length_counter: LengthCounter,
    // TODO: this is the sequencer, move it to a better place
    current_step: u8,
    counter: u8,
}

impl PulseChannel {
    pub fn target_period(&mut self) -> i16 {
        self.sweep.target_period(self.length_counter.timer() as _)
    }

    pub fn set_enabled(&mut self, enabled: bool) {
        self.length_counter.set_state(enabled.into());
    }

    pub fn active(&self) -> bool {
        self.length_counter.active()
    }

    pub fn tick(&mut self, frame: Option<Frame>) {
        match frame {
            Some(Frame::Quarter) => {
                // TODO: envelope
            }

            Some(Frame::Half) => {
                self.length_counter.tick();
                let target_period = self.sweep.tick(self.length_counter.timer() as _);
                if let Some(target_period) = target_period {
                    self.length_counter.set_timer(target_period as _);
                }
            }

            None => {
                // Sequencer
                if self.counter == 0 {
                    self.counter = self.length_counter.timer() as u8;
                    self.current_step = (self.current_step + 1) % 8;
                } else {
                    self.counter -= 1;
                }
            }
        }
    }

    pub fn sample(&mut self) -> u8 {
        const WAVEFORMS: [[u8; 8]; 4] = [
            [0, 1, 0, 0, 0, 0, 0, 0],
            [0, 1, 1, 0, 0, 0, 0, 0],
            [0, 1, 1, 1, 1, 0, 0, 0],
            [1, 0, 0, 1, 1, 1, 1, 1],
        ];

        if self.length_counter.active() && self.length_counter.timer() >= 8 {
            WAVEFORMS[self.control.duty_cycle() as usize][self.current_step as usize]
        } else {
            0
        }
    }
}

impl Memory for PulseChannel {
    fn write_byte(&mut self, address: u16, data: u8) {
        // println!("pulse {:04X} {:02X} {self:#?}", address, data);
        match address {
            0x4000 => {
                self.control = Control::from(data);
                if self.control.length_counter_halt() {
                    self.length_counter.set_state(length_counter::State::Halted)
                }
            }

            0x4001 => self.sweep.write(data),
            0x4002 => self.length_counter.set_low(data),
            0x4003 => self.length_counter.set_high(data),

            _ => unreachable!(),
        };
    }

    fn read_byte(&mut self, _address: u16) -> u8 {
        // Registers are write-only, this is open bus
        0
    }
}

impl Device for PulseChannel {
    #[inline]
    fn contains(&self, address: u16) -> bool {
        (0x4000..=0x4003).contains(&address)
    }
}
