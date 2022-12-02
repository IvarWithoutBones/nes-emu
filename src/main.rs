mod bus;
mod cartridge;
mod cpu;
mod instructions;

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
    let mut cpu = CPU::new(Cartridge::from_path(&args.rom).unwrap(), args.quiet);
    cpu.reset();
    cpu.run();
}
