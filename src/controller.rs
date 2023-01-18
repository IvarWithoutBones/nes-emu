use crate::util;
use bitflags::bitflags;
use std::sync::mpsc::Receiver;

bitflags! {
    #[derive(Default, Clone)]
    pub struct Buttons: u8 {
        const A      = 0b0000_0001;
        const B      = 0b0000_0010;
        const Select = 0b0000_0100;
        const Start  = 0b0000_1000;
        const Up     = 0b0001_0000;
        const Down   = 0b0010_0000;
        const Left   = 0b0100_0000;
        const Right  = 0b1000_0000;
    }
}

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
        tracing::info!(
            "reading controller button {}",
            Buttons::format_index(self.index)
        );

        let result = util::nth_bit(self.buttons.bits(), self.index) as u8;
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

impl Buttons {
    pub fn format_index(val: u8) -> &'static str {
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
}
