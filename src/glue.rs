// A module that glues together the GUI and the emulator, providing functions to instantiate both.
// The idea is to make this generic in the future, so that other GUI frameworks can be used.
// It would also be nice to make the GUI emulator-agnostic, but that requires more work.

use {
    crate::{bus, controller, cpu, ppu, LogReloadHandle},
    std::{
        path::PathBuf,
        sync::mpsc::{channel, Receiver, Sender},
    },
};

/// State of execution. This is used to step per-instruction and to pause the CPU.
#[derive(Clone)]
pub struct StepState {
    pub paused: bool,
    pub step: bool,
}

impl Default for StepState {
    fn default() -> Self {
        Self {
            paused: false,
            step: true,
        }
    }
}

impl StepState {
    pub fn step(&mut self) {
        self.step = true;
        self.paused = true;
    }

    pub fn toggle_pause(&mut self) {
        self.paused = !self.paused;
    }
}

pub struct CpuCommunication {
    // TODO: switch to byte array receiver
    rom_receiver: Receiver<PathBuf>,
    button_receiver: Receiver<controller::Buttons>,
    pixel_sender: ppu::PixelSender,
    cpu_state_sender: Option<Sender<cpu::CpuState>>,
    step_receiver: Option<Receiver<StepState>>,
}

impl CpuCommunication {
    pub fn spawn(self) -> std::thread::JoinHandle<()> {
        std::thread::spawn(move || {
            let bus = bus::Bus::new(self.button_receiver, self.pixel_sender, self.rom_receiver);
            let mut cpu = cpu::Cpu::new(bus);
            let mut step_state = StepState::default();

            while !cpu.bus.has_mapper() {
                std::thread::sleep(std::time::Duration::from_millis(100));
            }

            cpu.reset();

            loop {
                if let Some(step_receiver) = self.step_receiver.as_ref() {
                    if let Ok(new_step_state) = step_receiver.try_recv() {
                        step_state = new_step_state;
                    }

                    if step_state.paused {
                        if step_state.step {
                            step_state.step = false;
                        } else {
                            std::thread::sleep(std::time::Duration::from_millis(10));
                            continue;
                        }
                    }
                }

                if let Some(instr_state) = cpu.step() {
                    if let Some(ref cpu_state_sender) = self.cpu_state_sender {
                        if cpu_state_sender.send(instr_state).is_err() {
                            tracing::error!("failed to send CPU state, exiting cpu thread");
                            // GUI has died, so the CPU should too.
                            break;
                        };
                    }
                } else {
                    tracing::error!("error while stepping the CPU, exiting cpu thread");
                    // Some sort of error occured, should communicate to the GUI in the future.
                    break;
                }
            }
        })
    }
}

pub struct UiCommunication {
    pub button_sender: Sender<controller::Buttons>,
    pub pixel_receiver: ppu::PixelReceiver,
    pub rom_sender: Sender<PathBuf>,
    pub cpu_state_receiver: Option<Receiver<cpu::CpuState>>,
    pub step_sender: Option<Sender<StepState>>,
    pub log_reload_handle: LogReloadHandle,
}

pub trait EmulatorUi {
    fn start_ui(ui_comm: UiCommunication);
}

pub fn init(
    with_gui: bool,
    log_reload_handle: LogReloadHandle,
) -> (CpuCommunication, UiCommunication) {
    let (rom_sender, rom_receiver) = channel();

    let (step_sender, step_receiver) = if with_gui {
        let (step_sender, step_receiver) = channel();
        (Some(step_sender), Some(step_receiver))
    } else {
        (None, None)
    };

    let (cpu_state_sender, cpu_state_receiver) = if with_gui {
        let (cpu_state_sender, cpu_state_receiver) = channel();
        (Some(cpu_state_sender), Some(cpu_state_receiver))
    } else {
        (None, None)
    };

    let (pixel_sender, pixel_receiver) = channel();
    let (button_sender, button_receiver) = channel();

    let cpu_comm = CpuCommunication {
        rom_receiver,
        button_receiver,
        pixel_sender,
        cpu_state_sender,
        step_receiver,
    };

    let ui_comm = UiCommunication {
        log_reload_handle,
        button_sender,
        pixel_receiver,
        rom_sender,
        cpu_state_receiver,
        step_sender,
    };

    (cpu_comm, ui_comm)
}
