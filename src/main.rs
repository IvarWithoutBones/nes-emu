mod bus;
mod cartridge;
mod cpu;

use bus::Bus;
use cartridge::Cartridge;
use clap::Parser;
use cpu::CPU;

#[derive(Parser)]
#[command(author = "IvarWithoutBones", about = "A NES emulator written in Rust.")]
struct Args {
    #[arg(short, long)]
    quiet: bool,

    #[arg(short, long)]
    rom: String,
}

fn main() {
    let args = Args::parse();

    let cart = Cartridge::from_path(&args.rom).unwrap_or_else(|e| {
        eprintln!("error: {}", e);
        std::process::exit(1);
    });

    let bus: Bus = Bus::new(cart, args.quiet);
    let mut cpu = CPU::new(bus);

    cpu.reset();
    cpu.run();
}
