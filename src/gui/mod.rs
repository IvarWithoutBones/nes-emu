use eframe::egui;
use std::sync::mpsc::Receiver;

#[derive(Default)]
pub struct Gui {
    receiver: Option<Receiver<String>>,
    instructions: Vec<String>,
}

impl Gui {
    pub fn new(receiver: Receiver<String>) -> Self {
        Self {
            receiver: Some(receiver),
            instructions: Vec::new(),
        }
    }

    pub fn run(window_title: &str, receiver: Receiver<String>) {
        let options = eframe::NativeOptions::default();
        eframe::run_native(
            window_title,
            options,
            Box::new(|_cc| Box::new(Self::new(receiver))),
        );
    }

    fn disassembly_ui(&self, ui: &mut egui::Ui) {
        ui.heading(egui::RichText::new("Disassembly").strong());
        ui.separator();

        let text_style = egui::TextStyle::Body;
        let row_height = ui.text_style_height(&text_style);
        egui::ScrollArea::vertical()
            .auto_shrink([false, true])
            .stick_to_bottom(true)
            .show_rows(ui, row_height, self.instructions.len(), |ui, row_range| {
                for row in row_range {
                    ui.label(self.instructions.get(row).unwrap());
                }
            });
    }

    fn registers_ui(&self, ui: &mut egui::Ui) {
        ui.heading(egui::RichText::new("Registers").strong());
        ui.separator();

        egui::Grid::new("registers").show(ui, |ui| {
            grid_label(ui, "A", "00");
            grid_label(ui, "X", "00");
            grid_label(ui, "Y", "00");
            grid_label(ui, "SP", "00");
        });
    }
}

impl eframe::App for Gui {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if let Some(receiver) = &self.receiver {
            while let Ok(instruction) = receiver.try_recv() {
                self.instructions.push(instruction);
            }
        }

        egui::SidePanel::left("disassembly").show(ctx, |ui| self.disassembly_ui(ui));
        egui::CentralPanel::default().show(ctx, |ui| self.registers_ui(ui));

        // Calling this here will request another frame immediately after this one
        ctx.request_repaint();
    }
}

fn grid_label(ui: &mut egui::Ui, first: &str, second: &str) {
    ui.label(first);
    ui.label(second);
    ui.end_row();
}
