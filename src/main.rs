mod bus;
mod cartridge;
mod cpu;
mod gui;

use bus::Bus;
use clap::Parser;
use cpu::CPU;
use gui::Gui;
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
    let step_receiver = if args.without_gui {
        None
    } else {
        Some(step_receiver)
    };

    // Actually spawn the CPU thread
    let cpu_handle = thread::spawn(move || {
        let mut cpu = CPU::new(bus);
        cpu.reset();

        loop {
            if step_receiver.is_some() {
                // TODO: This is horribly ineffecient.
                if step_receiver.as_ref().unwrap().try_recv().is_err() {
                    thread::sleep(std::time::Duration::from_millis(10));
                    continue;
                }
            }

            if let Some(instr_state) = cpu.step() {
                let ptr = Box::new(instr_state);
                if cpu_state_sender.send(ptr).is_err() {
                    // GUI has died, so the CPU should too.
                    break;
                };
            } else {
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
