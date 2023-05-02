use crate::{bus::Device, util};
use std::sync::mpsc::Receiver;
use tartan_bitfield::bitfield;

pub const fn format_button_index(val: u8) -> &'static str {
    match val {
        0 => "A",
        1 => "B",
        2 => "Select",
        3 => "Start",
        4 => "Up",
        5 => "Down",
        6 => "Left",
        7 => "Right",
        _ => "Unknown",
    }
}

bitfield! {
    /// https://www.nesdev.org/wiki/Standard_controller#Report
    pub struct Buttons(u8) {
        [0] pub a,
        [1] pub b,
        [2] pub select,
        [3] pub start,
        [4] pub up,
        [5] pub down,
        [6] pub left,
        [7] pub right,
    }
}

/// https://www.nesdev.org/wiki/Standard_controller
pub struct Controller {
    span: tracing::Span,
    button_receiver: Receiver<Buttons>,
    buttons: Buttons,
    strobe: bool,
    index: u8,
}

impl Controller {
    pub fn new(button_receiver: Receiver<Buttons>) -> Self {
        let span = tracing::span!(tracing::Level::INFO, "controller");
        Self {
            span,
            button_receiver,
            buttons: Buttons::default(),
            strobe: false,
            index: 0,
        }
    }

    #[tracing::instrument(skip(self), parent = &self.span)]
    pub fn write(&mut self, data: u8) {
        self.strobe = util::nth_bit(data, 0);
        tracing::debug!("strobe: {}", self.strobe);
        if self.strobe {
            self.index = 0;
        }
    }

    #[tracing::instrument(skip(self), parent = &self.span)]
    pub fn read(&mut self) -> u8 {
        tracing::debug!(
            "reading controller button {}",
            format_button_index(self.index)
        );

        let result = util::nth_bit(self.buttons.into(), self.index) as u8;
        if !self.strobe {
            self.index = self.index.wrapping_add(1);
        }
        result
    }

    pub fn update(&mut self) {
        while let Some(buttons) = self.button_receiver.try_iter().last() {
            self.buttons = buttons;
        }
    }
}

impl Device for Controller {
    fn contains(&self, address: u16) -> bool {
        // TODO: Second controller
        address == 0x4016
    }
}
