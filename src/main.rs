mod bus;
mod cartridge;
mod controller;
mod cpu;
mod gui;
mod ppu;
mod util;

use bus::Bus;
use clap::Parser;
use gui::Gui;
use std::{str::FromStr, sync::mpsc::channel};
use tracing_subscriber::{
    filter::{self, LevelFilter},
    fmt,
    prelude::*,
    reload::{self, Handle},
    Registry,
};

#[derive(Parser)]
#[command(author = "IvarWithoutBones", about = "A NES emulator written in Rust.")]
struct Args {
    #[arg(short, long)]
    rom: Option<String>,

    #[arg(short, long)]
    without_gui: bool,

    // https://docs.rs/tracing-subscriber/0.3.16/tracing_subscriber/filter/struct.EnvFilter.html#example-syntax
    // Without the GUI framework cluttering logs: '--log-level tracing,eframe=info'
    #[arg(short, long)]
    log_level: Option<String>,
}

#[cfg(not(target_arch = "wasm32"))]
fn tracing_init(log_level: Option<String>) -> Handle<LevelFilter, Registry> {
    let log_level = log_level.unwrap_or_else(|| "info".to_string());
    let filter = filter::LevelFilter::from_str(&log_level).unwrap();
    let (filter, reload_handle) = reload::Layer::new(filter);
    tracing_subscriber::registry()
        .with(filter)
        // Do not display 'nes-emu' and the time in every log message
        .with(fmt::layer().without_time().with_target(false))
        .init();
    reload_handle
}

#[cfg(target_arch = "wasm32")]
fn tracing_init(_log_level: Option<String>) -> Handle<LevelFilter, Registry> {
    // TODO: The reload handle is a no-op on wasm, so we can't change the log level
    let filter = filter::LevelFilter::from_str("info").unwrap();
    let (filter, reload_handle) = reload::Layer::new(filter);
    console_error_panic_hook::set_once();
    tracing_wasm::set_as_global_default();
    reload_handle
}

fn main() {
    let args = Args::parse();
    let log_reload_handle = tracing_init(args.log_level);

    // CPU state communication
    let (rom_sender, rom_receiver) = channel();

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
    let bus: Bus = Bus::new(button_receiver, pixel_sender, rom_receiver);

    // Boot up the CPU
    #[cfg(not(target_arch = "wasm32"))]
    let cpu_handle = cpu::spawn_thread(bus, state_sender, step_receiver);

    if let Some(rom) = args.rom {
        rom_sender.send(rom.into()).unwrap_or_else(|err| {
            tracing::error!("failed to send ROM path: {}", err);
        });
    }

    // Start the GUI, if enabled
    if !args.without_gui {
        Gui::run(
            "NES emu",
            log_reload_handle,
            state_receiver.unwrap(),
            step_sender.unwrap(),
            rom_sender,
            button_sender,
            pixel_receiver,
            #[cfg(target_arch = "wasm32")]
            bus,
        );
    }

    #[cfg(not(target_arch = "wasm32"))]
    if cpu_handle.join().is_err() {
        tracing::error!("CPU thread panicked");
    };
}
