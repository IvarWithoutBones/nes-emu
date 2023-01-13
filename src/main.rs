mod bus;
mod cartridge;
mod controller;
mod cpu;
mod gui;
mod ppu;

use bus::Bus;
use clap::Parser;
use gui::Gui;
use std::sync::mpsc::channel;

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

    // CPU state communication if the GUI is enabled
    let mut step_sender = None;
    let mut step_receiver = None;

    let mut state_sender = None;
    let mut state_receiver = None;

    if !args.without_gui {
        let (state_sender_c, state_receiver_c) = channel();
        state_sender = Some(state_sender_c);
        state_receiver = Some(state_receiver_c);

        let (step_sender_c, step_receiver_c) = channel();
        step_sender = Some(step_sender_c);
        step_receiver = Some(step_receiver_c);
    }

    let (pixel_sender, pixel_receiver) = channel();
    let (button_sender, button_receiver) = channel();
    let bus: Bus = Bus::new(button_receiver, pixel_sender, &rom_data);

    // Boot up the CPU
    let cpu_handle = cpu::spawn_thread(bus, state_sender, step_receiver);

    // Start the GUI, if enabled
    if !args.without_gui {
        Gui::run(
            "NES emu",
            state_receiver.unwrap(),
            step_sender.unwrap(),
            button_sender,
            pixel_receiver,
        );
    }

    if cpu_handle.join().is_err() {
        tracing::error!("CPU thread panicked");
    };
}
