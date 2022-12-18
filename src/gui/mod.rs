pub mod step_state;

use crate::bus::CPU_RAM_SIZE;
use crate::cpu::{CpuFlags, CpuState};
use crate::gui::step_state::StepState;
use eframe::egui;
use egui_memory_editor::MemoryEditor;
use std::{
    cell::RefCell,
    sync::mpsc::{Receiver, Sender},
};

type CpuStateBox = Box<CpuState>;

pub struct Gui {
    cpu_state_receiver: Receiver<CpuStateBox>,
    cpu_states: Vec<CpuStateBox>,
    selected_cpu_state_index: Option<usize>,

    step_sender: Sender<StepState>,
    step_state: StepState,

    memory_viewer: RefCell<egui_memory_editor::MemoryEditor>,
}

impl Gui {
    const SPAN_NAME: &'static str = "gui";
    const MAX_CPU_STATES: usize = 300;
    const CPU_STATES_BUFFER: usize = 100;

    pub fn new(cpu_state_receiver: Receiver<CpuStateBox>, step_sender: Sender<StepState>) -> Self {
        let mut mem_viewer_options =
            egui_memory_editor::option_data::MemoryEditorOptions::default();
        mem_viewer_options.show_ascii = false;
        mem_viewer_options.is_resizable_column = false;
        mem_viewer_options.is_options_collapsed = true;
        let memory_viewer = MemoryEditor::new()
            .with_options(mem_viewer_options)
            .with_address_range("CPU RAM", 0..CPU_RAM_SIZE);

        Self {
            cpu_state_receiver,
            cpu_states: Vec::new(),
            selected_cpu_state_index: None,

            step_sender,
            step_state: StepState::default(),

            memory_viewer: RefCell::new(memory_viewer),
        }
    }

    pub fn run(
        window_title: &str,
        cpu_state_receiver: Receiver<CpuStateBox>,
        step_sender: Sender<StepState>,
    ) {
        let _span = tracing::span!(tracing::Level::INFO, Gui::SPAN_NAME).entered();
        let options = eframe::NativeOptions::default();
        eframe::run_native(
            window_title,
            options,
            Box::new(|_cc| Box::new(Self::new(cpu_state_receiver, step_sender))),
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

        ui.horizontal_top(|ui| {
            let step = ui
                .button("Step")
                .on_hover_text("Step the CPU one instruction");
            if step.clicked() {
                self.step_state.step();
                self.step_sender
                    .send(self.step_state.clone())
                    .unwrap_or_else(|err| {
                        tracing::error!("failed to send step state: {}", err);
                    });
            }

            let toggle = ui
                .button("Toggle")
                .on_hover_text("Toggle the CPU between being paused and running");
            if toggle.clicked() {
                self.step_state.toggle_pause();
                self.step_sender
                    .send(self.step_state.clone())
                    .unwrap_or_else(|err| {
                        tracing::error!("failed to send step state: {}", err);
                    });
            }
        });

        ui.separator();

        egui::ScrollArea::vertical()
            .auto_shrink([false, true])
            .stick_to_bottom(true)
            .show(ui, |ui| {
                for (index, state) in self.cpu_states.iter().enumerate() {
                    egui::Grid::new(index).show(ui, |ui| {
                        ui.horizontal(|ui| {
                            // TODO: sometimes one of these dont align?
                            let pc_label = ui
                                .label(&format!("{:04X}", state.program_counter))
                                .on_hover_text("Program counter");
                            let instr_label = ui
                                .selectable_label(false, state.formatted.clone())
                                .on_hover_text(format!("Flags: {}", state.status));

                            if instr_label.clicked() || pc_label.clicked() {
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

    fn cpu_status_ui(&self, ui: &mut egui::Ui, state: &CpuState) {
        ui.with_layout(egui::Layout::top_down_justified(egui::Align::TOP), |ui| {
            ui.heading(egui::RichText::new("CPU State").strong());
            ui.separator();

            self.registers_ui(ui, state);
            ui.separator();
            self.flags_ui(ui, state);
        });
    }

    fn selected_or_last_cpu_state(&self) -> Option<&CpuStateBox> {
        let selected_index = if self.selected_cpu_state_index.is_some() {
            self.selected_cpu_state_index.unwrap()
        } else {
            self.cpu_states.len().saturating_sub(1)
        };
        self.cpu_states.get(selected_index)
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

    fn memory_viewer_ui(
        ui: &mut egui::Ui,
        state: &CpuState,
        viewer: &mut egui_memory_editor::MemoryEditor,
    ) {
        ui.heading(egui::RichText::new("Memory viewer").strong());
        ui.separator();

        let mut mem = state.memory.clone();
        viewer.draw_editor_contents_read_only(ui, &mut mem, |mem, addr| {
            if addr >= mem.len() {
                tracing::warn!("memory viewer address out of bounds: {}", addr);
                return None;
            }
            Some(mem[addr])
        });
    }
}

impl eframe::App for Gui {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.update_cpu_state_cache();

        egui::SidePanel::left("disassembly")
            // Make sure the margins are consistent
            .frame(egui::Frame::central_panel(&egui::Style::default()))
            .show(ctx, |ui| self.disassembly_ui(ui));

        if let Some(state) = self.selected_or_last_cpu_state() {
            egui::CentralPanel::default().show(ctx, |ui| {
                self.cpu_status_ui(ui, state);
            });

            egui::SidePanel::right("memory_viewer")
                // Make sure the margins are consistent
                .frame(egui::Frame::central_panel(&egui::Style::default()))
                .show(ctx, |ui| {
                    Gui::memory_viewer_ui(ui, state, &mut self.memory_viewer.borrow_mut());
                });
        }

        // Calling this here will request another frame immediately after this one
        ctx.request_repaint();
    }
}
