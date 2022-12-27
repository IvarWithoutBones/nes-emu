use crate::ppu::renderer::{PixelBuffer, HEIGHT, WIDTH};
use eframe::egui;
use std::sync::mpsc::Receiver;

/// The screen to show pixels generated by the PPU.
pub struct Screen {
    texture: Option<egui::TextureHandle>,
    receiver: Receiver<Box<PixelBuffer>>,
}

impl Screen {
    pub fn new(receiver: Receiver<Box<PixelBuffer>>) -> Self {
        Self {
            texture: None,
            receiver,
        }
    }

    pub fn widget(&mut self) -> impl egui::Widget + '_ {
        move |ui: &mut egui::Ui| {
            while let Ok(buf) = self.receiver.try_recv() {
                self.update_texture(ui.ctx(), buf);
            }

            // TODO: Retain the original aspect ratio
            let width = ui.available_width();
            let height = ui.available_height();
            ui.horizontal(|ui| {
                if let Some(texture) = &self.texture {
                    ui.image(texture, [width, height]);
                }
            })
            .response
        }
    }

    /// Update the internal texture with the current pixel buffer. Should only be called when the buffer has changed.
    fn update_texture(&mut self, ctx: &egui::Context, pixels: Box<PixelBuffer>) {
        self.texture = Some(ctx.load_texture(
            "screen-with-pixels",
            egui::ColorImage::from_rgb([WIDTH, HEIGHT], &*pixels),
            egui::TextureOptions::NEAREST,
        ));
    }
}
