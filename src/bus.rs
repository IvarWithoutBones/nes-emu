use crate::{
    cartridge::{Cartridge, MapperInstance},
    controller,
    cpu::CpuRam,
    ppu::{self, renderer::PixelBuffer, Ppu},
};
use std::{
    cell::RefCell,
    path::PathBuf,
    rc::Rc,
    sync::mpsc::{channel, Receiver, Sender},
    time,
};

pub type CycleCount = usize;

pub trait Clock {
    const MULTIPLIER: usize = 1;

    fn tick_impl(&mut self, cycles: CycleCount);

    fn tick(&mut self, cycles: CycleCount) {
        self.tick_impl(cycles * Self::MULTIPLIER);
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
            self.write_byte(address + i as u16, *val);
        }
    }
}

pub trait Device {
    fn contains(&self, address: u16) -> bool;
}

pub struct Bus {
    span: tracing::Span,
    pub mapper: Option<MapperInstance>,
    pub cpu_ram: CpuRam,
    pub cycles: CycleCount,
    pub ppu: Ppu,
    pub controller: controller::Controller,
    rom_receiver: Receiver<PathBuf>,
    time_since_last_frame: time::Instant,
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
            mapper: None,
            ppu: Ppu::new(pixel_sender),
            cpu_ram: CpuRam::default(),
            cycles: 0,
            controller: controller::Controller::new(button_receiver),
            time_since_last_frame: time::Instant::now(),
        }
    }

    pub fn load_cartridge(&mut self, cartridge: Cartridge) {
        let cart = Rc::new(RefCell::new(cartridge.into()));
        self.mapper = Some(cart.clone());
        self.ppu.load_mapper(cart);
    }

    pub fn has_mapper(&mut self) -> bool {
        if let Ok(path) = self.rom_receiver.try_recv() {
            let cartridge = Cartridge::from(path);
            self.load_cartridge(cartridge);
        }
        self.mapper.is_some()
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
    fn read_byte(&mut self, address: u16) -> u8 {
        if self.controller.contains(address) {
            self.controller.read()
        } else if self.cpu_ram.contains(address) {
            self.cpu_ram[address]
        } else if self
            .mapper
            .as_ref()
            .map_or(false, |c| c.borrow().contains(address))
        {
            self.mapper.as_mut().unwrap().borrow_mut().read_cpu(address)
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
            }
        } else {
            tracing::warn!("unimplemented read at ${:04X}", address);
            0
        }
    }

    #[tracing::instrument(skip(self, address, data), parent = &self.span)]
    fn write_byte(&mut self, address: u16, data: u8) {
        if self.controller.contains(address) {
            self.controller.write(data);
        } else if self.cpu_ram.contains(address) {
            self.cpu_ram[address] = data;
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
        } else if self
            .mapper
            .as_ref()
            .map_or(false, |c| c.borrow().contains(address))
        {
            self.mapper
                .as_ref()
                .unwrap()
                .borrow_mut()
                .write_cpu(address, data);
        } else {
            tracing::warn!("unimplemented write at ${:04X}: ${:02X}", address, data);
        }
    }
}

impl Clock for Bus {
    fn tick_impl(&mut self, cycles: CycleCount) {
        self.controller.update();
        self.cycles += cycles;

        let vblank_before = self.ppu.status.in_vblank();
        self.ppu.tick(cycles);
        let vblank_after = self.ppu.status.in_vblank();

        if !vblank_before && vblank_after {
            self.ppu.render();

            // TODO: how accurate is this?
            if self.time_since_last_frame.elapsed() < time::Duration::from_millis(16) {
                std::thread::sleep(
                    time::Duration::from_millis(16) - self.time_since_last_frame.elapsed(),
                );
            }
            self.time_since_last_frame = time::Instant::now();
        }
    }
}
