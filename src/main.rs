mod cpu;

fn main() {
    let mut cpu = cpu::CPU::new();
    cpu.load_program(vec![0xA9, 0x42, 0x00]);
    cpu.run();
}
