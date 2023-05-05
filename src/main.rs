mod bus;
mod cartridge;
mod cheat;
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
    tracing_subscriber::{
        filter::{LevelFilter, ParseError},
        fmt,
        prelude::*,
        reload::{self, Handle},
        EnvFilter, Registry,
    },
};

pub type LogReloadHandle = Handle<EnvFilter, Registry>;

fn tracing_init(log_level: Option<String>) -> Result<LogReloadHandle, ParseError> {
    let filter = EnvFilter::builder()
        // Disabling regex is recommanded when parsing from untrusted sources
        .with_regex(false)
        .with_default_directive(LevelFilter::ERROR.into())
        .parse(log_level.unwrap_or_default())?;
    let (filter, reload_handle) = reload::Layer::new(filter);

    tracing_subscriber::registry()
        .with(filter)
        // Do not display 'nes-emu' and the time in every log message
        .with(fmt::layer().without_time().with_target(false))
        .init();
    Ok(reload_handle)
}

#[derive(Parser)]
#[command(author = "IvarWithoutBones", about = "A NES emulator written in Rust.")]
struct Args {
    #[arg(short, long)]
    rom: Option<String>,

    #[arg(short, long)]
    without_gui: bool,

    // https://docs.rs/tracing-subscriber/0.3.16/tracing_subscriber/filter/struct.EnvFilter.html#example-syntax
    #[arg(short, long)]
    log_level: Option<String>,
}

impl EmulatorUi for Gui {
    fn start_ui(ui: glue::UiCommunication) {
        Gui::run(
            "NES emu",
            ui.log_reload_handle,
            ui.cpu_state_receiver.unwrap(),
            ui.button_sender,
            ui.pixel_receiver,
            ui.cheat_sender.unwrap(),
            (ui.step_sender.unwrap(), ui.reboot_sender.unwrap()),
            (ui.rom_sender, ui.unload_rom_sender),
        );
    }
}

fn main() {
    let args = Args::parse();
    let log_reload_handle = tracing_init(args.log_level.clone()).unwrap_or_else(|err| {
        eprintln!(
            "failed to set log level '{}': {err}",
            args.log_level.unwrap()
        );
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
