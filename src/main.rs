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
    let log_level = args.log_level.unwrap_or_else(|| "info".to_string());
    if tracing::subscriber::set_global_default(
        tracing_subscriber::fmt()
            .with_env_filter(log_level)
            .with_target(false) // Dont display 'nes_emu' for every span
            .without_time()
            .finish(),
    )
    .is_err()
    {
        panic!("failed to set global tracing subscriber");
    }

    // Load data for cartridge
    let rom_data = std::fs::read(&args.rom).unwrap_or_else(|err| {
        tracing::error!("failed to read file \"{}\": \"{}\"", args.rom, err);
        std::process::exit(1);
    });

    // Instantiate the bus
    let bus: Bus = Bus::new(&rom_data);

    // Spawn the CPU thread
    let (instruction_sender, instruction_receiver) = channel();
    let cpu_handle = thread::spawn(move || {
        let mut cpu = CPU::new(bus);
        cpu.reset();

        loop {
            if let Some(instr_state) = cpu.step() {
                let ptr = Box::new(instr_state);
                if instruction_sender.send(ptr).is_err() {
                    // GUI has died, so the CPU should too.
                    break;
                };
            } else {
                // Some sort of error occured, should communicate to the GUI in the future.
                break;
            }
            // thread::sleep(std::time::Duration::from_millis(1000));
        }
    });

    if !args.without_gui {
        Gui::run("NES emu", instruction_receiver);
    }

    if cpu_handle.join().is_err() {
        tracing::error!("CPU thread panicked");
    };
}
