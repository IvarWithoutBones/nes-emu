mod bus;
mod cartridge;
mod cpu;
mod gui;

use bus::Bus;
use cartridge::Cartridge;
use clap::Parser;
use cpu::CPU;
use gui::Gui;
use std::{sync::mpsc::channel, thread};

#[derive(Parser)]
#[command(author = "IvarWithoutBones", about = "A NES emulator written in Rust.")]
struct Args {
    #[arg(short, long)]
    quiet: bool,

    #[arg(short, long)]
    rom: String,
}

fn main() {
    let (sender, receiver) = channel();
    let args = Args::parse();

    let cart = Cartridge::from_path(&args.rom).unwrap_or_else(|e| {
        eprintln!("error: {}", e);
        std::process::exit(1);
    });

    let bus: Bus = Bus::new(cart, args.quiet);
    let mut cpu = CPU::new(bus);
    cpu.reset();

    thread::spawn(move || {
        cpu.run_with(sender);
    });

    Gui::run("NES emu", receiver);
}
