use crate::cartridge::Cartridge;
use crate::controller;
use crate::ppu::renderer::PixelBuffer;
use crate::ppu::{self, *};
use std::ops::RangeInclusive;
use std::path::PathBuf;
use std::sync::mpsc::{channel, Receiver, Sender};

pub const CPU_RAM_SIZE: usize = 2048;
const CPU_RAM_RANGE: RangeInclusive<u16> = 0..=0x1FFF;
pub const PROGRAM_ROM_RANGE: RangeInclusive<u16> = 0x8000..=0xFFFF;
// TODO: Second controller
const CONTROLLER_RANGE: RangeInclusive<u16> = 0x4016..=0x4016;

pub type CycleCount = usize;

pub trait Clock {
    const MULTIPLIER: usize = 1;

    fn tick_internal(&mut self, cycles: CycleCount);
    fn get_cycles(&self) -> CycleCount;
    fn set_cycles(&mut self, cycles: CycleCount);

    fn tick(&mut self, cycles: CycleCount) {
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
    pub cartridge: Option<Cartridge>,
    pub cpu_ram: [u8; CPU_RAM_SIZE],
    pub cycles: CycleCount,
    pub ppu: Ppu,
    pub controller: controller::Controller,
    rom_receiver: Receiver<PathBuf>,
}

impl Bus {
    pub fn new(
        button_receiver: Receiver<controller::Buttons>,
        pixel_sender: Sender<Box<PixelBuffer>>,
        rom_receiver: Receiver<PathBuf>,
    ) -> Bus {
        let span = tracing::span!(tracing::Level::INFO, "bus");
        tracing::info!("succesfully initialized");
        Bus {
            span,
            rom_receiver,
            cartridge: None,
            ppu: Ppu::new(pixel_sender),
            cpu_ram: [0; CPU_RAM_SIZE],
            cycles: 0,
            controller: controller::Controller::new(button_receiver),
        }
    }

    pub fn load_cartridge(&mut self, cartridge: Cartridge) {
        self.ppu.load_cartridge(&cartridge);
        self.cartridge = Some(cartridge);
    }

    pub fn has_cartridge(&mut self) -> bool {
        if let Ok(path) = self.rom_receiver.try_recv() {
            let cartridge = Cartridge::from(path);
            self.load_cartridge(cartridge);
        }
        self.cartridge.is_some()
    }

    const fn to_cpu_ram_address(address: u16) -> u16 {
        // Addressing is 11 bits, so we need to mask the top 5 off
        address & 0b0000_0111_1111_1111
    }

    /// Generate a dummy bus, used for tests
    #[allow(dead_code)]
    pub fn new_dummy(data: Vec<u8>) -> Self {
        let cartridge = Cartridge::new_dummy(data).unwrap_or_else(|err| {
            tracing::error!("failed to load cartridge: \"{}\"", err);
            std::process::exit(1);
        });
        let mut bus = Self::new(channel().1, channel().0, channel().1);
        bus.load_cartridge(cartridge);
        bus
    }
}

impl Memory for Bus {
    #[tracing::instrument(skip(self, address), parent = &self.span)]
    fn read_byte(&mut self, mut address: u16) -> u8 {
        if CONTROLLER_RANGE.contains(&address) {
            self.controller.read()
        } else if PROGRAM_ROM_RANGE.contains(&address) {
            address -= PROGRAM_ROM_RANGE.start();
            let cart = self.cartridge.as_ref().unwrap();
            if cart.header.program_rom_pages == 1 {
                address %= 0x4000;
            }
            let result = cart.program_rom[address as usize];
            tracing::trace!("program ROM read at ${:04X}: ${:02X}", address, result);
            result
        } else if CPU_RAM_RANGE.contains(&address) {
            address = Self::to_cpu_ram_address(address);
            let result = self.cpu_ram[address as usize];
            tracing::trace!("CPU RAM read at ${:04X}: ${:02X}", address, result);
            result
        } else if let Some((register, mutability)) = ppu::registers::get_register(address) {
            if mutability.readable() {
                tracing::trace!("PPU register {} read at ${:04X}", register, address);
                self.ppu.read_register(register)
            } else {
                tracing::error!(
                    "reading write-only PPU register {} at ${:04X}",
                    register,
                    address
                );
                0
                // panic!()
            }
        } else {
            tracing::error!("unimplemented read at ${:04X}", address);
            0
            // panic!()
        }
    }

    #[tracing::instrument(skip(self, address, data), parent = &self.span)]
    fn write_byte(&mut self, mut address: u16, data: u8) {
        if CONTROLLER_RANGE.contains(&address) {
            self.controller.write(data);
        } else if CPU_RAM_RANGE.contains(&address) {
            address = Self::to_cpu_ram_address(address);
            tracing::trace!("writing to CPU RAM at ${:04X}: ${:02X}", address, data);
            self.cpu_ram[address as usize] = data;
        } else if let Some((register, mutability)) = ppu::registers::get_register(address) {
            if mutability.writable() {
                tracing::trace!(
                    "PPU register {} write at ${:04X}: ${:02X}",
                    register,
                    address,
                    data
                );

                // TODO: this isn't the prettiest, but we need special behavior from the bus for DMA
                if register == &ppu::registers::Register::ObjectAttributeDirectMemoryAccess {
                    let range = self.ppu.oam.dma(data);
                    for i in range {
                        let byte = self.read_byte(i.try_into().unwrap());
                        self.ppu.oam.write_data(byte);
                    }
                } else {
                    self.ppu.write_register(register, data);
                }
            } else {
                tracing::error!(
                    "writing read-only PPU register {} at ${:04X}",
                    register,
                    address
                );
                panic!()
            }
        } else if PROGRAM_ROM_RANGE.contains(&address) {
            tracing::error!(
                "writing read-only program ROM at ${:04X}: ${:02X}",
                address,
                data
            );
            // panic!()
        } else {
            tracing::error!("unimplemented write at ${:04X}: ${:02X}", address, data);
            // panic!()
        }
    }
}

impl Clock for Bus {
    fn tick_internal(&mut self, cycles: CycleCount) {
        self.controller.update();
        self.cycles += cycles;

        let vblank_before = self.ppu.status.in_vblank();
        self.ppu.tick(cycles);
        let vblank_after = self.ppu.status.in_vblank();

        if !vblank_before && vblank_after {
            self.ppu.render();

            // This is a hack to ensure we dont send too many frames to the renderer at once, locking up the GUI.
            // TODO: Should be removed when proper timing is implemented.
            if cfg!(not(debug_assertions)) && cfg!(not(target_arch = "wasm32")) {
                std::thread::sleep(std::time::Duration::from_millis(10));
            }
        }
    }

    fn get_cycles(&self) -> CycleCount {
        self.cycles
    }

    fn set_cycles(&mut self, cycles: CycleCount) {
        self.cycles = cycles;
    }
}
