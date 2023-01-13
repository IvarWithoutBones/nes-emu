use crate::controller;
use eframe::egui;
use std::sync::mpsc::Sender;

pub struct Input {
    span: tracing::Span,
    pub button_sender: Sender<controller::Buttons>,
}

impl Input {
    pub fn new(button_sender: Sender<controller::Buttons>) -> Self {
        let span = tracing::span!(tracing::Level::INFO, "input");
        Self {
            span,
            button_sender,
        }
    }

    // TODO: This is very, very ugly
    #[tracing::instrument(skip(self, ctx), parent = &self.span)]
    pub fn update(&self, ctx: &egui::Context) {
        let mut button = controller::Buttons::default();

        if ctx.input().key_down(egui::Key::Z) {
            button |= controller::Buttons::A;
        }

        if ctx.input().key_down(egui::Key::X) {
            button |= controller::Buttons::B;
        }

        if ctx.input().key_down(egui::Key::Space) {
            button |= controller::Buttons::Select;
        }

        if ctx.input().key_down(egui::Key::Enter) {
            button |= controller::Buttons::Start;
        }

        if ctx.input().key_down(egui::Key::ArrowRight) {
            button |= controller::Buttons::Right;
        }

        if ctx.input().key_down(egui::Key::ArrowLeft) {
            button |= controller::Buttons::Left;
        }

        if ctx.input().key_down(egui::Key::ArrowUp) {
            button |= controller::Buttons::Up;
        }

        if ctx.input().key_down(egui::Key::ArrowDown) {
            button |= controller::Buttons::Down;
        }

        if let Err(e) = self.button_sender.send(button) {
            tracing::error!("failed to send button to controller: {}", e);
        }
    }
}
