mod bus;
mod cartridge;
mod cpu;
mod instructions;

use cartridge::Cartridge;
use cpu::CPU;
use std::env;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        panic!("No ROM specified");
    }

    let mut cpu = CPU::new(Cartridge::from_path(&args[1]).unwrap());
    cpu.reset();
    cpu.run();
}
