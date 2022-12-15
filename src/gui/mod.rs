use crate::cpu::{CpuFlags, InstructionState};
use eframe::egui;
use std::sync::mpsc::Receiver;

type InstructionBox = Box<InstructionState>;

pub struct Gui {
    instruction_receiver: Receiver<InstructionBox>,
    instructions: Vec<InstructionBox>,
    selected_instruction_index: Option<usize>,
}

impl Gui {
    pub fn new(receiver: Receiver<InstructionBox>) -> Self {
        Self {
            instruction_receiver: receiver,
            instructions: Vec::new(),
            selected_instruction_index: None,
        }
    }

    pub fn run(window_title: &str, receiver: Receiver<InstructionBox>) {
        let options = eframe::NativeOptions::default();
        eframe::run_native(
            window_title,
            options,
            Box::new(|_cc| Box::new(Self::new(receiver))),
        );
    }

    fn disassembly_ui(&mut self, ui: &mut egui::Ui) {
        ui.heading(egui::RichText::new("Disassembly").strong());
        ui.separator();

        let jump_to_bottom = ui
            .button("Scroll to bottom")
            .on_hover_text("Jump to the latest instruction");
        if jump_to_bottom.clicked() {
            self.selected_instruction_index = None;
        }
        ui.separator();

        egui::ScrollArea::vertical()
            .auto_shrink([false, true])
            .stick_to_bottom(true)
            .show(ui, |ui| {
                for (index, instr) in self.instructions.iter().enumerate() {
                    egui::Grid::new(index).show(ui, |ui| {
                        ui.horizontal(|ui| {
                            let pc_label = ui
                                .selectable_label(false, &format!("{:04X}", instr.program_counter))
                                .on_hover_text("Program counter");
                            let instr_label = ui
                                .selectable_label(false, instr.formatted.clone())
                                .on_hover_text(format!("{}", instr.status));

                            if instr_label.clicked() || pc_label.clicked() {
                                self.selected_instruction_index = Some(index);
                            }
                        });
                    });
                }

                if jump_to_bottom.clicked() {
                    ui.scroll_to_cursor(Some(egui::Align::BOTTOM));
                }
            });
    }

    fn registers_ui(&self, ui: &mut egui::Ui, instr: &InstructionState) {
        ui.label(egui::RichText::new("Registers").strong());
        egui::Grid::new("registers").striped(true).show(ui, |ui| {
            let label = |ui: &mut egui::Ui, text: &str, num: u8| {
                ui.label(text);
                ui.label(format!("${:02X}", num));
                ui.end_row();
            };

            ui.label("Program counter");
            ui.label(format!("${:04X}", instr.program_counter));
            ui.end_row();

            label(ui, "Stack pointer", instr.stack_pointer);
            label(ui, "Accumulator", instr.accumulator);
            label(ui, "X register", instr.register_x);
            label(ui, "Y register", instr.register_y);
        });
    }

    fn flags_ui(&self, ui: &mut egui::Ui, instr: &InstructionState) {
        ui.label(egui::RichText::new("Flags").strong());
        egui::Grid::new("flags").striped(true).show(ui, |ui| {
            let mut label = |text: &str, enabled: bool| {
                ui.label(text);
                ui.label(if enabled { "true" } else { "false" });
                ui.end_row();
            };

            label("Negative", instr.status.contains(CpuFlags::NEGATIVE));
            label("Overflow", instr.status.contains(CpuFlags::OVERFLOW));
            label("Break", instr.status.contains(CpuFlags::BREAK));
            label("Break2", instr.status.contains(CpuFlags::BREAK2));
            label("Decimal", instr.status.contains(CpuFlags::DECIMAL));
            label("Interrupts disabled", instr.status.contains(CpuFlags::IRQ));
            label("Zero", instr.status.contains(CpuFlags::ZERO));
            label("Carry", instr.status.contains(CpuFlags::CARRY));
        });
    }

    fn selected_or_last_instr(&self) -> &InstructionState {
        let selected_index = if self.selected_instruction_index.is_some() {
            self.selected_instruction_index.unwrap()
        } else {
            self.instructions.len() - 1
        };
        self.instructions.get(selected_index).unwrap()
    }

    fn update_instruction_cache(&mut self) {
        if let Ok(state) = self.instruction_receiver.try_recv() {
            // TODO: truncate the cache if it gets too big.
            self.instructions.push(state);
        };
    }
}

impl eframe::App for Gui {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.update_instruction_cache();

        egui::SidePanel::left("disassembly")
            // Make sure the margins are consistent
            .frame(egui::Frame::central_panel(&egui::Style::default()))
            .show(ctx, |ui| self.disassembly_ui(ui));

        egui::CentralPanel::default().show(ctx, |_ui| {
            let instr = self.selected_or_last_instr();

            egui::SidePanel::left("flags")
                // Make sure the margins are consistent
                .frame(egui::Frame::central_panel(&egui::Style::default()))
                .show(ctx, |ui| self.flags_ui(ui, instr));

            egui::CentralPanel::default().show(ctx, |ui| self.registers_ui(ui, instr));
        });

        // Calling this here will request another frame immediately after this one
        ctx.request_repaint();
    }
}
