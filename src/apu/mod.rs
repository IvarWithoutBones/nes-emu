#![allow(dead_code)]

use self::{frame_counter::FrameCounter, pulse::PulseChannel};
use crate::bus::{Clock, CycleCount, Device, Memory};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use tartan_bitfield::bitfield;

mod frame_counter;
mod length_counter;
mod pulse;
mod sweep;

bitfield! {
    /// https://www.nesdev.org/wiki/APU#Status_($4015)
    struct Status(u8) {
        [0] pulse_1,
        [1] pulse_2,
        [2] triangle,
        [3] noise,
        [4] delta_modulation,
        [6] frame_interrupt,
        [7] delta_modulation_interrupt,
    }
}

impl Status {
    pub fn write(&mut self, data: u8) {
        self.0 = Self::from(data)
            .with_frame_interrupt(self.frame_interrupt())
            .with_delta_modulation_interrupt(false)
            .into();
    }

    pub fn read(&mut self) -> u8 {
        self.set_frame_interrupt(false);
        self.0
    }
}

/// https://www.nesdev.org/wiki/APU
#[derive(Default)]
pub struct Apu {
    frame_counter: FrameCounter,
    pulse_channel: PulseChannel,
    status: Status,
}

impl Apu {
    pub fn new() -> Self {
        Self::default()
    }
}

impl Clock for Apu {
    fn tick_impl(&mut self, cpu_cycles: CycleCount) {
        let maybe_frame = self.frame_counter.tick(cpu_cycles);
        self.pulse_channel.tick(maybe_frame);

        if self.frame_counter.cycle_count % 40 == 0 {
            let _sample = self.pulse_channel.sample() as f64;
        }
    }
}

impl Memory for Apu {
    fn read_byte(&mut self, address: u16) -> u8 {
        match address {
            0x4017 => self.frame_counter.read(),
            0x4015 => self.status.read(),

            _ if self.pulse_channel.contains(address) => self.pulse_channel.read_byte(address),

            _ => {
                tracing::warn!("stub: APU read at ${address:04X}");
                0
            }
        }
    }

    fn write_byte(&mut self, address: u16, data: u8) {
        match address {
            0x4017 => self.frame_counter.write(data),
            0x4015 => {
                self.status.write(data);
                self.pulse_channel.set_enabled(self.status.pulse_1());
            }

            _ if self.pulse_channel.contains(address) => {
                self.pulse_channel.write_byte(address, data)
            }

            _ => tracing::warn!("stub: APU write at ${address:04X} = {data:02X}"),
        }
    }
}

impl Device for Apu {
    fn contains(&self, address: u16) -> bool {
        // 0x4014 is PPU OAM DMA, weirdly shoved inbetween
        address != 0x4014 && (0x4000..=0x4017).contains(&address)
    }
}
