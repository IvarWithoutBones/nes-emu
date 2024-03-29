use super::{default_frame, header_label};
use crate::{
    cpu::{CpuRam, CpuState},
    glue::StepState,
    util::CircularBuffer,
};
use eframe::egui;
use egui_memory_editor::MemoryEditor;
use std::{
    cell::RefCell,
    sync::mpsc::{Receiver, Sender},
};

pub struct CpuDebugger {
    span: tracing::Span,

    cpu_state_receiver: Receiver<CpuState>,
    cpu_states: CircularBuffer<CpuState, 500>,
    selected_cpu_state_index: Option<usize>,

    step_sender: Sender<StepState>,
    step_state: StepState,

    memory_viewer: RefCell<egui_memory_editor::MemoryEditor>,
    jump_to_bottom_clicked: bool,
    highlight_text_colour: egui::Color32,
}

impl CpuDebugger {
    fn span() -> tracing::Span {
        tracing::span!(tracing::Level::INFO, "cpu_debugger")
    }

    pub fn new(cpu_state_receiver: Receiver<CpuState>, step_sender: Sender<StepState>) -> Self {
        let mut mem_viewer_options =
            egui_memory_editor::option_data::MemoryEditorOptions::default();

        mem_viewer_options.show_ascii = false;
        mem_viewer_options.is_options_collapsed = true;
        mem_viewer_options.column_count = 24;

        let highlight_text_colour = mem_viewer_options.highlight_text_colour;
        let memory_viewer = MemoryEditor::new()
            .with_options(mem_viewer_options)
            .with_address_range("CPU RAM", 0..CpuRam::SIZE);

        Self {
            span: Self::span(),
            cpu_state_receiver,
            cpu_states: CircularBuffer::new(),
            selected_cpu_state_index: None,

            step_sender,
            step_state: StepState::default(),

            memory_viewer: RefCell::new(memory_viewer),
            jump_to_bottom_clicked: false,
            highlight_text_colour,
        }
    }

    pub fn clear_states(&mut self) {
        self.cpu_states.clear();
        self.selected_cpu_state_index = None;
    }

    fn send_step_state(&mut self) {
        self.step_sender.send(self.step_state.clone()).unwrap();
    }

    pub fn pause(&mut self) {
        self.step_state.paused = true;
        self.send_step_state();
    }

    pub fn unpause(&mut self) {
        self.step_state.paused = false;
        self.send_step_state();
    }

    pub fn toggle_pause(&mut self) {
        self.step_state.paused = !self.step_state.paused;
        self.send_step_state();
    }

    pub fn step(&mut self) {
        self.step_state.paused = true;
        self.step_state.step = true;
        self.send_step_state();
    }

    fn selected_or_last_cpu_state(&self) -> Option<&CpuState> {
        if let Some(index) = self.selected_cpu_state_index {
            self.cpu_states[index].as_ref()
        } else {
            self.cpu_states.last()
        }
    }

    pub fn update_buffer(&mut self) {
        while let Ok(state) = self.cpu_state_receiver.try_recv() {
            self.cpu_states.push(state);
        }
    }

    /// Returns a widget containing the CPU debugger, to be drawn with egui
    pub fn widget(&mut self) -> impl egui::Widget + '_ {
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
                            || (self.selected_cpu_state_index.is_none()
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
                    ui.label(format!("${num:02X}"));
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
                    let header_text = format!("Flags (${:02X})", u8::from(state.status));
                    ui.label(egui::RichText::new(header_text).strong());
                    ui.end_row();

                    label(ui, "Negative", state.status.negative());
                    label(ui, "Overflow", state.status.overflow());
                    label(
                        ui,
                        "Interrupts disabled",
                        state.status.interrupts_disabled(),
                    );
                    label(ui, "Zero", state.status.zero());
                    label(ui, "Carry", state.status.carry());
                });

            egui::Grid::new("flags_grid_two")
                .striped(true)
                .show(ui, |ui| {
                    ui.end_row(); // Spacing because the previous grid contains the header label

                    label(ui, "Decimal", state.status.decimal());
                    label(ui, "Break", state.status.break_1());
                    label(ui, "Break2", state.status.break_2());
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
        let mut mem = &state.memory;
        viewer.draw_editor_contents_read_only(ui, &mut mem, |mem, addr| {
            if addr >= mem.len() {
                tracing::warn!("memory viewer address out of bounds: {}", addr);
                return None;
            }
            Some(mem[addr.try_into().unwrap()])
        });
    }
}
