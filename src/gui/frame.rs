use eframe::egui;

pub struct Frame {
    pixels: [u8; Self::pixels_len()],
    texture: Option<egui::TextureHandle>,
    pixel_index: usize, // For testing
}

impl Frame {
    const WIDTH: usize = 256;
    const HEIGHT: usize = 240;

    const fn pixels_len() -> usize {
        Self::WIDTH * Self::HEIGHT * 3
    }

    pub fn new() -> Self {
        Self {
            pixels: [0; Self::pixels_len()],
            texture: None,
            pixel_index: 0,
        }
    }

    pub fn widget(&mut self) -> impl egui::Widget + '_ {
        move |ui: &mut egui::Ui| {
            // For testing
            self.pixel_index += 1;
            for x in 0..Self::WIDTH {
                for y in 0..Self::HEIGHT {
                    self.update_pixel(x, y, [(x % 256) as u8, (y % 256) as u8, 230]);
                }
            }

            // TODO: this is highly inefficient, should only update upon change
            self.update_texture(ui.ctx());

            // Retain the original aspect ratio
            let width = (ui.available_height() * Self::WIDTH as f32) / Self::HEIGHT as f32;
            let height = (ui.available_width() * Self::HEIGHT as f32) / Self::WIDTH as f32;
            ui.horizontal(|ui| {
                if let Some(texture) = &self.texture {
                    ui.image(texture, [width, height]);
                }
            })
            .response
        }
    }

    fn update_texture(&mut self, ctx: &egui::Context) {
        self.texture = Some(ctx.load_texture(
            "pixel-frame",
            egui::ColorImage::from_rgb([Self::WIDTH, Self::HEIGHT], &self.pixels),
            egui::TextureOptions::NEAREST,
        ));
    }

    fn update_pixel(&mut self, x: usize, y: usize, color: [u8; 3]) {
        if x >= Self::WIDTH || y >= Self::HEIGHT {
            return;
        }

        let index = (y * Self::WIDTH + x) * 3;
        self.pixels[index..index + 3].copy_from_slice(&color);
    }
}
