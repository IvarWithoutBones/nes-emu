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
    let (instruction_sender, instruction_receiver) = channel();
    let args = Args::parse();

    let cart = Cartridge::from_path(&args.rom).unwrap_or_else(|e| {
        eprintln!("error: {}", e);
        std::process::exit(1);
    });

    let bus: Bus = Bus::new(cart, args.quiet);

    thread::spawn(move || {
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
        }
    });

    Gui::run("NES emu", instruction_receiver);
}
