pub mod step_state;

use super::{default_frame, header_label};
use crate::{
    bus::CPU_RAM_SIZE,
    cpu::{flags::CpuFlags, CpuState},
};
pub use step_state::StepState;

use eframe::egui;
use egui_memory_editor::MemoryEditor;
use std::{
    cell::RefCell,
    sync::mpsc::{Receiver, Sender},
};

pub struct CpuDebugger {
    span: tracing::Span,
    cpu_state_receiver: Receiver<Box<CpuState>>,
    cpu_states: Vec<Box<CpuState>>,
    selected_cpu_state_index: Option<usize>,

    step_sender: Sender<StepState>,
    step_state: StepState,

    memory_viewer: RefCell<egui_memory_editor::MemoryEditor>,
    jump_to_bottom_clicked: bool,
    highlight_text_colour: egui::Color32,
}

impl CpuDebugger {
    const MAX_CPU_STATES: usize = 500;
    const CPU_STATES_BUFFER: usize = 100;

    fn span() -> tracing::Span {
        tracing::span!(tracing::Level::INFO, "cpu_debugger")
    }

    pub fn new(
        cpu_state_receiver: Receiver<Box<CpuState>>,
        step_sender: Sender<StepState>,
    ) -> Self {
        let mut mem_viewer_options =
            egui_memory_editor::option_data::MemoryEditorOptions::default();
        mem_viewer_options.show_ascii = false;
        mem_viewer_options.is_options_collapsed = true;
        mem_viewer_options.column_count = 32;
        let highlight_text_colour = mem_viewer_options.highlight_text_colour;
        let memory_viewer = MemoryEditor::new()
            .with_options(mem_viewer_options)
            .with_address_range("CPU RAM", 0..CPU_RAM_SIZE);

        Self {
            span: Self::span(),
            cpu_state_receiver,
            cpu_states: Vec::new(),
            selected_cpu_state_index: None,

            step_sender,
            step_state: StepState::default(),

            memory_viewer: RefCell::new(memory_viewer),
            jump_to_bottom_clicked: false,
            highlight_text_colour,
        }
    }

    /// Returns a widget containing the CPU debugger, to be drawn with egui
    pub fn widget(&mut self) -> impl egui::Widget + '_ {
        self.update_cpu_state_cache();

        move |ui: &mut egui::Ui| {
            ui.horizontal(|ui| {
                egui::SidePanel::left("disassembly")
                    .frame(default_frame())
                    .resizable(false)
                    .show(ui.ctx(), |ui| self.instructions_ui(ui));

                if let Some(state) = self.selected_or_last_cpu_state() {
                    egui::CentralPanel::default().show(ui.ctx(), |ui| {
                        egui::TopBottomPanel::top("cpu_status")
                            .frame(default_frame())
                            .show_inside(ui, |ui| self.cpu_status_ui(ui, state));
                        egui::CentralPanel::default().show_inside(ui, |ui| {
                            Self::memory_viewer_ui(ui, state, &mut self.memory_viewer.borrow_mut());
                        });
                    });
                }
            })
            .response
        }
    }

    fn selected_or_last_cpu_state(&self) -> Option<&Box<CpuState>> {
        let selected_index = if self.selected_cpu_state_index.is_some() {
            self.selected_cpu_state_index.unwrap()
        } else {
            self.cpu_states.len().saturating_sub(1)
        };
        self.cpu_states.get(selected_index)
    }

    fn update_cpu_state_cache(&mut self) {
        // TODO: Cache the actual strings we need to render, computing them every frame is expensive.
        while let Ok(state) = self.cpu_state_receiver.try_recv() {
            self.cpu_states.push(state);
            // Trim the cache if it gets too big, so we don't run out of memory.
            if self.cpu_states.len() > (Self::MAX_CPU_STATES + Self::CPU_STATES_BUFFER) {
                self.cpu_states.drain(0..Self::CPU_STATES_BUFFER);
            }
        }
    }

    #[tracing::instrument(skip(self, ui), parent = &self.span)]
    fn instructions_ui(&mut self, ui: &mut egui::Ui) {
        ui.spacing_mut().item_spacing = egui::Vec2::new(7.5, 7.5);
        header_label(ui, "Instructions");

        ui.vertical_centered_justified(|ui| {
            let jump_to_bottom = ui
                .button("Jump to bottom")
                .on_hover_text("Jump to the latest instruction");
            if jump_to_bottom.clicked() {
                self.jump_to_bottom_clicked = true;
                self.selected_cpu_state_index = None;
            } else {
                self.jump_to_bottom_clicked = false;
            }
        });

        ui.horizontal_top(|ui| {
            let step = ui
                .button("Step")
                .on_hover_text("Step the CPU one instruction");
            if step.clicked() {
                self.selected_cpu_state_index = None;
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
                self.selected_cpu_state_index = None;
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
                        // Highlight the selected instruction
                        let pc_text = if self.selected_cpu_state_index == Some(index)
                            || (self.selected_cpu_state_index == None
                                && index == self.cpu_states.len() - 1)
                        {
                            egui::RichText::new(format!("{:04X}", state.program_counter))
                                .color(self.highlight_text_colour)
                                .strong()
                        } else {
                            egui::RichText::new(format!("{:04X}", state.program_counter))
                        };
                        let pc_label = ui.label(pc_text).on_hover_text("Program counter");

                        let instr_label = ui
                            .selectable_label(false, state.instruction.clone())
                            .on_hover_text(format!("Flags: {}", state.status));

                        if instr_label.clicked() || pc_label.clicked() {
                            self.selected_cpu_state_index = Some(index);
                        }
                    });
                }

                if self.jump_to_bottom_clicked {
                    ui.scroll_to_cursor(Some(egui::Align::BOTTOM));
                }
            });
    }

    fn registers_ui(&self, ui: &mut egui::Ui, state: &CpuState) {
        ui.vertical(|ui| {
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
                label(ui, "A register", state.accumulator);
                label(ui, "X register", state.register_x);
                label(ui, "Y register", state.register_y);
            });
        });
    }

    fn flags_ui(&self, ui: &mut egui::Ui, state: &CpuState) {
        let label = |ui: &mut egui::Ui, text: &str, enabled: bool| {
            ui.label(text);
            ui.label(if enabled { "true" } else { "false" });
            ui.end_row();
        };

        ui.horizontal_top(|ui| {
            egui::Grid::new("flags_grid_one")
                .striped(true)
                .show(ui, |ui| {
                    let header_text = format!("Flags (${:02X})", state.status);
                    ui.label(egui::RichText::new(header_text).strong());
                    ui.end_row();

                    label(ui, "Negative", state.status.contains(CpuFlags::Negative));
                    label(ui, "Overflow", state.status.contains(CpuFlags::Overflow));
                    label(
                        ui,
                        "Interrupts disabled",
                        state.status.contains(CpuFlags::InterruptsDisabled),
                    );
                    label(ui, "Zero", state.status.contains(CpuFlags::Zero));
                    label(ui, "Carry", state.status.contains(CpuFlags::Carry));
                });
            egui::Grid::new("flags_grid_two")
                .striped(true)
                .show(ui, |ui| {
                    ui.end_row(); // Spacing because the previous grid contains the header label

                    label(ui, "Decimal", state.status.contains(CpuFlags::Decimal));
                    label(ui, "Break", state.status.contains(CpuFlags::Break));
                    label(ui, "Break2", state.status.contains(CpuFlags::Break2));
                });
        });
    }

    fn cpu_status_ui(&self, ui: &mut egui::Ui, state: &CpuState) {
        header_label(ui, "CPU Status");

        // TODO: might be useful to display stack contents here as well
        ui.horizontal_top(|ui| {
            self.registers_ui(ui, state);
            ui.separator();
            self.flags_ui(ui, state);
        });
    }

    #[tracing::instrument(skip(ui, state, viewer), parent = &Self::span())]
    fn memory_viewer_ui(
        ui: &mut egui::Ui,
        state: &CpuState,
        viewer: &mut egui_memory_editor::MemoryEditor,
    ) {
        header_label(ui, "Memory Viewer");
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
