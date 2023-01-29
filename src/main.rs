mod bus;
mod cartridge;
mod controller;
mod cpu;
mod glue;
mod gui;
mod ppu;
mod util;

use {
    clap::Parser,
    glue::EmulatorUi,
    gui::Gui,
    std::str::FromStr,
    tracing_subscriber::{
        filter::{self, LevelFilter},
        fmt,
        prelude::*,
        reload::{self, Handle},
        Registry,
    },
};

#[derive(Parser)]
#[command(author = "IvarWithoutBones", about = "A NES emulator written in Rust.")]
struct Args {
    #[arg(short, long)]
    rom: Option<String>,

    #[arg(short, long)]
    without_gui: bool,

    // TODO: EnvFilter is currently broken
    // https://docs.rs/tracing-subscriber/0.3.16/tracing_subscriber/filter/struct.EnvFilter.html#example-syntax
    // Without the GUI framework cluttering logs: '--log-level tracing,eframe=info'
    #[arg(short, long)]
    log_level: Option<String>,
}

fn tracing_init(
    log_level: Option<String>,
) -> Result<Handle<LevelFilter, Registry>, filter::LevelParseError> {
    let log_level = log_level.unwrap_or_else(|| "info".to_string());
    let filter = filter::LevelFilter::from_str(&log_level)?;
    let (filter, reload_handle) = reload::Layer::new(filter);
    tracing_subscriber::registry()
        .with(filter)
        // Do not display 'nes-emu' and the time in every log message
        .with(fmt::layer().without_time().with_target(false))
        .init();
    Ok(reload_handle)
}

impl EmulatorUi for Gui {
    fn start_ui(ui: glue::UiCommunication) {
        Gui::run(
            "NES emu",
            ui.log_reload_handle,
            ui.cpu_state_receiver.unwrap(),
            ui.step_sender.unwrap(),
            ui.button_sender,
            ui.pixel_receiver,
            ui.rom_sender,
        );
    }
}

fn main() {
    let args = Args::parse();
    let log_reload_handle = tracing_init(args.log_level).unwrap_or_else(|err| {
        eprintln!("invalid log level: {err}");
        std::process::exit(1);
    });

    let (cpu, ui) = glue::init(!args.without_gui, log_reload_handle);
    let cpu_handle = cpu.spawn();

    if let Some(rom) = args.rom {
        ui.rom_sender.send(rom.into()).unwrap();
    }

    if !args.without_gui {
        Gui::start_ui(ui);
    }

    if cpu_handle.join().is_err() {
        tracing::error!("CPU thread panicked");
    };
}
