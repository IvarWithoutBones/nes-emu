use crate::cartridge::Cartridge;

pub const PROGRAM_ROM_START: u16 = 0x8000;
const PROGRAM_ROM_END: u16 = 0xFFFF;

const CPU_RAM_SIZE: usize = 2048;
const CPU_RAM_START: u16 = 0x0000;
const CPU_RAM_MIRROR_END: u16 = 0x1FFF;

const PPU_REGISTERS: u16 = 0x2000;
const PPU_REGISTERS_MIRROR_END: u16 = 0x3FFF;

pub trait Clock {
    fn tick_internal(&mut self, cycles: u64);
    fn get_cycles(&self) -> u64;
    fn set_cycles(&mut self, cycles: u64);

    const MULTIPLIER: u64 = 1;
    fn tick(&mut self, cycles: u64) {
        self.tick_internal(cycles * Self::MULTIPLIER);
    }
}

pub trait Memory {
    fn read_byte(&self, address: u16) -> u8;
    fn write_byte(&mut self, address: u16, data: u8);

    fn read_word(&self, address: u16) -> u16 {
        u16::from_le_bytes([self.read_byte(address), self.read_byte(address + 1)])
    }

    fn write_word(&mut self, address: u16, data: u16) {
        for (i, val) in u16::to_le_bytes(data).iter().enumerate() {
            self.write_byte((address as usize + i).try_into().unwrap(), *val);
        }
    }
}

pub struct Bus {
    pub cartridge: Cartridge,
    pub cpu_ram: [u8; CPU_RAM_SIZE],
    pub cycles: u64,
    // This is convenient as the bus is the middle man between other components
    pub quiet: bool,
}

impl Bus {
    pub fn new(cartridge: Cartridge, quiet: bool) -> Self {
        Bus {
            cpu_ram: [0; CPU_RAM_SIZE],
            cycles: 0,
            cartridge,
            quiet,
        }
    }

    fn read_program_rom(&self, address: u16) -> u8 {
        let mut addr = address - PROGRAM_ROM_START;
        if self.cartridge.program_rom.len() == 0x4000 && addr >= 0x4000 {
            // Mirror, if required
            addr %= 0x4000;
        }
        self.cartridge.program_rom[addr as usize]
    }

    const fn to_cpu_ram_address(address: u16) -> usize {
        // Addressing is 11 bits, so we need to mask the top 5 off
        (address & 0b0000_0111_1111_1111) as usize
    }
}

impl Memory for Bus {
    fn read_byte(&self, address: u16) -> u8 {
        match address {
            CPU_RAM_START..=CPU_RAM_MIRROR_END => self.cpu_ram[Self::to_cpu_ram_address(address)],
            PROGRAM_ROM_START..=PROGRAM_ROM_END => self.read_program_rom(address),

            PPU_REGISTERS..=PPU_REGISTERS_MIRROR_END => {
                println!(
                    "warning: unimplemented PPU register read at ${:04X}",
                    address
                );
                0
            }

            _ => {
                println!("warning: unimplemented read at ${:04X}", address);
                0
            }
        }
    }

    fn write_byte(&mut self, address: u16, data: u8) {
        match address {
            CPU_RAM_START..=CPU_RAM_MIRROR_END => {
                self.cpu_ram[Self::to_cpu_ram_address(address)] = data;
            }

            PROGRAM_ROM_START..=PROGRAM_ROM_END => println!(
                "warning: attempted to write to program ROM at ${:04X}: ${:02X}",
                address, data
            ),

            PPU_REGISTERS..=PPU_REGISTERS_MIRROR_END => {
                println!(
                    "warning: unimplemented PPU register write at ${:04X}: ${:02X}",
                    address, data
                );
            }

            _ => println!(
                "waring: unimplemented write at ${:04X}: ${:02X}",
                address, data
            ),
        }
    }
}

impl Clock for Bus {
    fn tick_internal(&mut self, cycles: u64) {
        self.cycles += cycles;
    }

    fn set_cycles(&mut self, cycles: u64) {
        self.cycles = cycles;
    }

    fn get_cycles(&self) -> u64 {
        self.cycles
    }
}
