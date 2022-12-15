use crate::cpu::InstructionState;
use eframe::egui;
use std::sync::mpsc::Receiver;

pub struct Gui {
    receiver: Receiver<Option<InstructionState>>,
    instructions: Vec<InstructionState>,
}

impl Gui {
    pub fn new(receiver: Receiver<Option<InstructionState>>) -> Self {
        Self {
            receiver,
            instructions: Vec::new(),
        }
    }

    pub fn run(window_title: &str, receiver: Receiver<Option<InstructionState>>) {
        let options = eframe::NativeOptions::default();
        eframe::run_native(
            window_title,
            options,
            Box::new(|_cc| Box::new(Self::new(receiver))),
        );
    }

    fn disassembly_ui(&self, ui: &mut egui::Ui) {
        ui.spacing();
        ui.heading(egui::RichText::new("Disassembly").strong());
        ui.separator();

        let text_style = egui::TextStyle::Body;
        let row_height = ui.text_style_height(&text_style);
        egui::ScrollArea::vertical()
            .auto_shrink([false, true])
            .stick_to_bottom(true)
            .show_rows(ui, row_height, self.instructions.len(), |ui, row_range| {
                for row in row_range {
                    if let Some(instruction) = self.instructions.get(row) {
                        ui.label(instruction.formatted.clone());
                    }
                }
            });
    }

    fn registers_ui(&self, ui: &mut egui::Ui) {
        ui.heading(egui::RichText::new("Registers").strong());
        ui.separator();

        // TODO: dont hardcode latest
        let instr = self.instructions.last();
        if instr.is_none() {
            return;
        }
        let instr = instr.unwrap();

        egui::Grid::new("registers").show(ui, |ui| {
            grid_label(ui, "A", instr.accumulator);
            grid_label(ui, "X", instr.register_x);
            grid_label(ui, "Y", instr.register_y);
            grid_label(ui, "SP", instr.stack_pointer);
        });
    }

    fn update_instruction_cache(&mut self) {
        while let Ok(Some(instruction)) = self.receiver.try_recv() {
            self.instructions.push(instruction);
        }
    }
}

impl eframe::App for Gui {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.update_instruction_cache();

        egui::SidePanel::left("disassembly").show(ctx, |ui| self.disassembly_ui(ui));
        egui::CentralPanel::default().show(ctx, |ui| self.registers_ui(ui));

        // Calling this here will request another frame immediately after this one
        ctx.request_repaint();
    }
}

fn grid_label(ui: &mut egui::Ui, first: &str, second: u8) {
    ui.label(first);
    ui.label(format!("{:02X}", second));
    ui.end_row();
}
