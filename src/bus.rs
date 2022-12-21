use crate::cartridge::Cartridge;
use crate::ppu::Ppu;

pub const CPU_RAM_SIZE: usize = 2048;

pub const PROGRAM_ROM_START: u16 = 0x8000;
const PROGRAM_ROM_END: u16 = 0xFFFF;

pub trait Clock {
    const MULTIPLIER: u64 = 1;

    fn tick_internal(&mut self, cycles: u64);
    fn get_cycles(&self) -> u64;
    fn set_cycles(&mut self, cycles: u64);

    fn tick(&mut self, cycles: u64) {
        self.tick_internal(cycles * Self::MULTIPLIER);
    }

    fn tick_once_if(&mut self, condition: bool) {
        if condition {
            self.tick(1);
        }
    }
}

pub trait Memory {
    fn read_byte(&mut self, address: u16) -> u8;
    fn write_byte(&mut self, address: u16, data: u8);

    fn read_word(&mut self, address: u16) -> u16 {
        u16::from_le_bytes([self.read_byte(address), self.read_byte(address + 1)])
    }

    fn write_word(&mut self, address: u16, data: u16) {
        for (i, val) in u16::to_le_bytes(data).iter().enumerate() {
            self.write_byte((address as usize + i).try_into().unwrap(), *val);
        }
    }
}

pub struct Bus {
    span: tracing::Span,
    pub cartridge: Cartridge,
    pub cpu_ram: [u8; CPU_RAM_SIZE],
    pub cycles: u64,
    pub ppu: Ppu,
}

impl Bus {
    const CPU_RAM_START: u16 = 0x0000;
    const CPU_RAM_MIRROR_END: u16 = 0x1FFF;

    fn from_cart(cart: Cartridge) -> Bus {
        let span = tracing::span!(tracing::Level::INFO, "bus");
        tracing::info!("succesfully initialized");
        Bus {
            span,
            cartridge: cart.clone(),
            ppu: Ppu::new(cart.character_rom, cart.header.mirroring),
            cpu_ram: [0; CPU_RAM_SIZE],
            cycles: 0,
        }
    }

    pub fn new(rom_data: &Vec<u8>) -> Self {
        // Should the CPU be initialized here as well?
        let cartridge = Cartridge::from_bytes(rom_data).unwrap_or_else(|err| {
            tracing::error!("failed to load cartridge: \"{}\"", err);
            std::process::exit(1);
        });

        Self::from_cart(cartridge)
    }

    /// Generate a dummy bus, used for tests
    #[allow(dead_code)]
    pub fn new_dummy(data: Vec<u8>) -> Self {
        let cartridge = Cartridge::new_dummy(data).unwrap_or_else(|err| {
            tracing::error!("failed to load cartridge: \"{}\"", err);
            std::process::exit(1);
        });

        Self::from_cart(cartridge)
    }

    fn read_program_rom(&self, address: u16) -> u8 {
        let mut addr = address - PROGRAM_ROM_START;
        if self.cartridge.program_rom.len() == 0x4000 && addr >= 0x4000 {
            // Mirror, if required
            addr %= 0x4000;
        }
        tracing::trace!("reading program ROM at ${:04X}", addr);
        self.cartridge.program_rom[addr as usize]
    }

    const fn to_cpu_ram_address(address: u16) -> usize {
        // Addressing is 11 bits, so we need to mask the top 5 off
        (address & 0b0000_0111_1111_1111) as usize
    }

    fn read_cpu_ram(&self, address: u16) -> u8 {
        let addr = Self::to_cpu_ram_address(address);
        tracing::trace!("reading CPU RAM at ${:04X}", addr);
        self.cpu_ram[addr]
    }

    fn write_cpu_ram(&mut self, address: u16, data: u8) {
        let addr = Self::to_cpu_ram_address(address);
        tracing::trace!("writing to CPU RAM at ${:04X}: ${:02X}", addr, data);
        self.cpu_ram[addr] = data;
    }
}

impl Memory for Bus {
    #[tracing::instrument(skip(self, address), parent = &self.span, level = tracing::Level::TRACE)]
    fn read_byte(&mut self, address: u16) -> u8 {
        match address {
            Self::CPU_RAM_START..=Self::CPU_RAM_MIRROR_END => self.read_cpu_ram(address),
            PROGRAM_ROM_START..=PROGRAM_ROM_END => self.read_program_rom(address),

            _ => {
                if let Some(mutability) = crate::ppu::registers::get_mutability(address) {
                    if mutability.readable() {
                        tracing::trace!("PPU register read at ${:04X}", address);
                        return self.ppu.read_data();
                    } else {
                        tracing::error!("reading write-only PPU register ${:04X}", address);
                        panic!()
                    }
                }

                tracing::warn!("unimplemented read at ${:04X}", address);
                0
            }
        }
    }

    #[tracing::instrument(skip(self, address, data), parent = &self.span)]
    fn write_byte(&mut self, address: u16, data: u8) {
        match address {
            Self::CPU_RAM_START..=Self::CPU_RAM_MIRROR_END => self.write_cpu_ram(address, data),

            PROGRAM_ROM_START..=PROGRAM_ROM_END => tracing::warn!(
                "attempted to write to program ROM at ${:04X}: ${:02X}",
                address,
                data
            ),

            _ => {
                if let Some(mutability) = crate::ppu::registers::get_mutability(address) {
                    if mutability.writable() {
                        tracing::warn!(
                            "unimplemented PPU register write at ${:04X}: ${:02X}",
                            address,
                            data
                        );
                        return; // TODO: right call
                    } else {
                        tracing::error!("writing read-only PPU register ${:04X}", address);
                        panic!()
                    }
                }

                tracing::warn!("unimplemented write at ${:04X}: ${:02X}", address, data);
            }
        }
    }
}

// TODO: tracing?
impl Clock for Bus {
    fn tick_internal(&mut self, cycles: u64) {
        self.cycles += cycles;
    }

    fn get_cycles(&self) -> u64 {
        self.cycles
    }

    fn set_cycles(&mut self, cycles: u64) {
        self.cycles = cycles;
    }
}
