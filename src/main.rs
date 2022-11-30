mod cartridge;
mod cpu;

fn main() {
    let rom = cartridge::Cartridge::from_path("roms/snake.nes").unwrap();
    let mut cpu = cpu::CPU::new();
    cpu.load_program(rom.program_rom);
    cpu.run();
}
