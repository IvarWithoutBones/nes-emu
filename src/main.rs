mod bus;
mod cartridge;
mod cpu;

use bus::Bus;
use cartridge::Cartridge;
use clap::Parser;
use cpu::{assembler, CPU};

#[derive(Parser)]
#[command(author = "IvarWithoutBones", about = "A NES emulator written in Rust.")]
struct Args {
    #[arg(short, long)]
    quiet: bool,

    #[arg(short, long)]
    rom: String,
}

#[allow(unreachable_code)]
fn main() {
    assembler::run();

    return;

    let args = Args::parse();

    let cart = Cartridge::from_path(&args.rom);
    if cart.is_err() {
        println!("Failed to load ROM: {}", args.rom);
        return;
    }
    let cart = cart.unwrap();

    let bus: Bus = Bus::new(cart, args.quiet);
    let mut cpu = CPU::new(bus);

    cpu.reset();
    cpu.run();
}
