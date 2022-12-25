mod bus;
mod cartridge;
mod cpu;
mod gui;
mod ppu;

use bus::Bus;
use clap::Parser;
use cpu::Cpu;
use gui::{cpu_debugger::StepState, Gui};
use std::{sync::mpsc::channel, thread};
use tracing;
use tracing_subscriber;

#[derive(Parser)]
#[command(author = "IvarWithoutBones", about = "A NES emulator written in Rust.")]
struct Args {
    #[arg(short, long)]
    rom: String,

    #[arg(short, long)]
    without_gui: bool,

    // https://docs.rs/tracing-subscriber/0.3.16/tracing_subscriber/filter/struct.EnvFilter.html#example-syntax
    // Without the GUI framework cluttering logs: '--log-level tracing,eframe=info'
    #[arg(short, long)]
    log_level: Option<String>,
}

fn main() {
    let args = Args::parse();

    // Set up the logger
    let log_level = args.log_level.unwrap_or_else(|| "debug".to_string());
    let subscriber = tracing_subscriber::fmt()
        .with_env_filter(log_level)
        .with_target(false) // Dont display 'nes_emu' for every span
        .without_time()
        .finish();
    tracing::subscriber::set_global_default(subscriber).expect("Failed to set default subscriber");

    // Load data for cartridge
    let rom_data = std::fs::read(&args.rom).unwrap_or_else(|err| {
        tracing::error!("failed to read file \"{}\": \"{}\"", args.rom, err);
        std::process::exit(1);
    });

    // Instantiate the bus
    let bus: Bus = Bus::new(&rom_data);

    // CPU state communication
    let (cpu_state_sender, cpu_state_receiver) = channel();
    let (step_sender, step_receiver) = channel();

    let cpu_state_sender = if args.without_gui {
        None
    } else {
        Some(cpu_state_sender)
    };

    let step_receiver = if args.without_gui {
        None
    } else {
        Some(step_receiver)
    };

    // Actually spawn the CPU thread
    let cpu_handle = thread::spawn(move || {
        let mut step_state = StepState::default();
        let mut cpu = Cpu::new(bus);
        cpu.reset();

        loop {
            if let Some(step_receiver) = step_receiver.as_ref() {
                if let Ok(new_step_state) = step_receiver.try_recv() {
                    step_state = new_step_state;
                }

                if step_state.paused {
                    if step_state.step {
                        step_state.step = false;
                    } else {
                        thread::sleep(std::time::Duration::from_millis(10));
                        continue;
                    }
                }
            }

            if let Some(instr_state) = cpu.step() {
                if let Some(ref cpu_state_sender) = cpu_state_sender {
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
    });

    if !args.without_gui {
        Gui::run("NES emu", cpu_state_receiver, step_sender);
    }

    if cpu_handle.join().is_err() {
        tracing::error!("CPU thread panicked");
    };
}
