use crate::cpu::{CpuFlags, CpuState};
use eframe::egui;
use std::sync::mpsc::Receiver;

type CpuStateBox = Box<CpuState>;

pub struct Gui {
    cpu_state_receiver: Receiver<CpuStateBox>,
    cpu_states: Vec<CpuStateBox>,
    selected_cpu_state_index: Option<usize>,
}

impl Gui {
    const SPAN_NAME: &'static str = "gui";

    const MAX_CPU_STATES: usize = 300;
    const CPU_STATES_BUFFER: usize = 100;

    pub fn new(receiver: Receiver<CpuStateBox>) -> Self {
        Self {
            cpu_state_receiver: receiver,
            cpu_states: Vec::new(),
            selected_cpu_state_index: None,
        }
    }

    pub fn run(window_title: &str, receiver: Receiver<CpuStateBox>) {
        let _span = tracing::span!(tracing::Level::INFO, Gui::SPAN_NAME).entered();
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
            self.selected_cpu_state_index = None;
        }
        ui.separator();

        egui::ScrollArea::vertical()
            .auto_shrink([false, true])
            .stick_to_bottom(true)
            .show(ui, |ui| {
                for (index, state) in self.cpu_states.iter().enumerate() {
                    egui::Grid::new(index).show(ui, |ui| {
                        ui.horizontal(|ui| {
                            let pc_label = ui
                                .selectable_label(false, &format!("{:04X}", state.program_counter))
                                .on_hover_text("Program counter");
                            let state_label = ui
                                .selectable_label(false, state.formatted.clone())
                                .on_hover_text(format!("{}", state.status));

                            if state_label.clicked() || pc_label.clicked() {
                                self.selected_cpu_state_index = Some(index);
                            }
                        });
                    });
                }

                if jump_to_bottom.clicked() {
                    ui.scroll_to_cursor(Some(egui::Align::BOTTOM));
                }
            });
    }

    fn registers_ui(&self, ui: &mut egui::Ui, state: &CpuState) {
        ui.label(egui::RichText::new("Registers").strong());
        egui::Grid::new("registers").striped(true).show(ui, |ui| {
            let label = |ui: &mut egui::Ui, text: &str, num: u8| {
                ui.label(text);
                ui.label(format!("${:02X}", num));
                ui.end_row();
            };

            ui.label("Program counter");
            ui.label(format!("${:04X}", state.program_counter));
            ui.end_row();

            label(ui, "Stack pointer", state.stack_pointer);
            label(ui, "Accumulator", state.accumulator);
            label(ui, "X register", state.register_x);
            label(ui, "Y register", state.register_y);
        });
    }

    fn flags_ui(&self, ui: &mut egui::Ui, state: &CpuState) {
        ui.label(egui::RichText::new("Flags").strong());
        egui::Grid::new("flags").striped(true).show(ui, |ui| {
            let mut label = |text: &str, enabled: bool| {
                ui.label(text);
                ui.label(if enabled { "true" } else { "false" });
                ui.end_row();
            };

            label("Negative", state.status.contains(CpuFlags::NEGATIVE));
            label("Overflow", state.status.contains(CpuFlags::OVERFLOW));
            label("Break", state.status.contains(CpuFlags::BREAK));
            label("Break2", state.status.contains(CpuFlags::BREAK2));
            label("Decimal", state.status.contains(CpuFlags::DECIMAL));
            label("Interrupts disabled", state.status.contains(CpuFlags::IRQ));
            label("Zero", state.status.contains(CpuFlags::ZERO));
            label("Carry", state.status.contains(CpuFlags::CARRY));
        });
    }

    fn selected_or_last_cpu_state(&self) -> &CpuState {
        let selected_index = if self.selected_cpu_state_index.is_some() {
            self.selected_cpu_state_index.unwrap()
        } else {
            self.cpu_states.len() - 1
        };
        self.cpu_states.get(selected_index).unwrap()
    }

    fn update_cpu_state_cache(&mut self) {
        while let Ok(state) = self.cpu_state_receiver.try_recv() {
            // trim the cache if it gets too big, so we don't run out of memory
            if self.cpu_states.len() < (Self::MAX_CPU_STATES + Self::CPU_STATES_BUFFER) {
                self.cpu_states.push(state);
            } else {
                self.cpu_states.drain(0..Self::CPU_STATES_BUFFER);
            }
        }
    }
}

impl eframe::App for Gui {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.update_cpu_state_cache();

        egui::SidePanel::left("disassembly")
            // Make sure the margins are consistent
            .frame(egui::Frame::central_panel(&egui::Style::default()))
            .show(ctx, |ui| self.disassembly_ui(ui));

        egui::CentralPanel::default().show(ctx, |_ui| {
            let state = self.selected_or_last_cpu_state();

            egui::SidePanel::left("flags")
                // Make sure the margins are consistent
                .frame(egui::Frame::central_panel(&egui::Style::default()))
                .show(ctx, |ui| self.flags_ui(ui, state));

            egui::CentralPanel::default().show(ctx, |ui| self.registers_ui(ui, state));
        });

        // Calling this here will request another frame immediately after this one
        ctx.request_repaint();
    }
}
