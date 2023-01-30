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
    button_receiver: Receiver<controller::Buttons>,
    pixel_sender: ppu::PixelSender,
    cpu_state_sender: Option<Sender<cpu::CpuState>>,
    step_receiver: Option<Receiver<StepState>>,
    reboot_receiver: Option<Receiver<()>>,

    // TODO: switch to byte array receiver
    rom_receiver: Receiver<PathBuf>,
    unload_rom_receiver: Receiver<()>,
}

impl CpuCommunication {
    pub fn spawn(self) -> std::thread::JoinHandle<()> {
        std::thread::spawn(move || {
            let bus = bus::Bus::new(self.button_receiver, self.pixel_sender, self.rom_receiver);
            let mut cpu = cpu::Cpu::new(bus);
            let mut step_state = StepState::default();
            let mut inserted_cartridge = false;

            cpu.reset();

            loop {
                if self.unload_rom_receiver.try_recv().is_ok() {
                    inserted_cartridge = false;
                    cpu.bus.unload_cartridge();
                }

                if !cpu.bus.has_cartridge() {
                    std::thread::sleep(std::time::Duration::from_millis(100));
                    continue;
                } else if !inserted_cartridge {
                    inserted_cartridge = true;
                    cpu.reset();
                }

                if let Some(reboot_receiver) = self.reboot_receiver.as_ref() {
                    if reboot_receiver.try_recv().is_ok() {
                        cpu.reset();
                    }
                }

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
    pub cpu_state_receiver: Option<Receiver<cpu::CpuState>>,
    pub log_reload_handle: LogReloadHandle,

    pub step_sender: Option<Sender<StepState>>,
    pub reboot_sender: Option<Sender<()>>,

    pub rom_sender: Sender<PathBuf>,
    pub unload_rom_sender: Sender<()>,
}

pub trait EmulatorUi {
    fn start_ui(ui_comm: UiCommunication);
}

pub fn init(
    with_gui: bool,
    log_reload_handle: LogReloadHandle,
) -> (CpuCommunication, UiCommunication) {
    let (rom_sender, rom_receiver) = channel();
    let (unload_rom_sender, unload_rom_receiver) = channel();
    let (pixel_sender, pixel_receiver) = channel();
    let (button_sender, button_receiver) = channel();

    let (step_sender, step_receiver) = if with_gui {
        let (step_sender, step_receiver) = channel();
        (Some(step_sender), Some(step_receiver))
    } else {
        (None, None)
    };

    let (reboot_sender, reboot_receiver) = if with_gui {
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

    let cpu_comm = CpuCommunication {
        rom_receiver,
        unload_rom_receiver,
        button_receiver,
        pixel_sender,
        cpu_state_sender,
        step_receiver,
        reboot_receiver,
    };

    let ui_comm = UiCommunication {
        rom_sender,
        unload_rom_sender,
        button_sender,
        pixel_receiver,
        cpu_state_receiver,
        step_sender,
        reboot_sender,
        log_reload_handle,
    };

    (cpu_comm, ui_comm)
}
