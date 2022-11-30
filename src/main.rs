mod cpu;
mod rom;

fn main() {
    let rom = rom::Rom::from_file("roms/donkey-kong.nes");

    let mut cpu = cpu::CPU::new();
    cpu.load_program(rom.program_rom);

    cpu.run();
}
