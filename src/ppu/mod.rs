mod object_attribute;
mod palette;
pub mod registers;
pub mod renderer;

use super::bus::{Clock, CycleCount};
use crate::cartridge::{Cartridge, Mirroring};
use object_attribute::{Object, ObjectAttributeMemory};
use registers::Register;
use renderer::{PixelBuffer, Renderer};
use std::{ops::RangeInclusive, sync::mpsc::Sender};

const VIDEO_RAM_SIZE: usize = 0x800;
pub type VideoRam = [u8; VIDEO_RAM_SIZE];

type ScanlineCount = u16;

/// https://www.nesdev.org/wiki/PPU
pub struct Ppu {
    span: tracing::Span,
    pub renderer: Renderer,
    mirroring: Option<Mirroring>,

    data_buffer: u8,
    vram: VideoRam,
    pub oam: ObjectAttributeMemory,

    control: registers::Control,
    mask: registers::Mask,
    pub status: registers::Status,
    scroll: registers::Scroll,
    address: registers::Address,

    cycles: CycleCount,
    scanline: ScanlineCount,
    trigger_nmi: bool,
}

impl Ppu {
    const PATTERN_TABLE_RANGE: RangeInclusive<u16> = 0..=0x1FFF;
    const NAMETABLE_RANGE: RangeInclusive<u16> = 0x2000..=0x3EFF;
    const PALETTE_RAM_RANGE: RangeInclusive<u16> = 0x3F00..=0x3FFF;

    pub fn new(pixel_sender: Sender<Box<PixelBuffer>>) -> Self {
        Self {
            span: tracing::span!(tracing::Level::INFO, "ppu"),
            renderer: Renderer::new(pixel_sender),
            mirroring: None,

            data_buffer: 0,
            vram: [0; VIDEO_RAM_SIZE],
            oam: ObjectAttributeMemory::default(),

            control: registers::Control::default(),
            mask: registers::Mask::default(),
            status: registers::Status::default(),
            scroll: registers::Scroll::default(),
            address: registers::Address::default(),

            cycles: 0,
            scanline: 0,
            trigger_nmi: false,
        }
    }

    pub fn load_cartridge(&mut self, cartridge: &Cartridge) {
        self.mirroring = Some(cartridge.header.mirroring);
        self.renderer.pattern_table = Some(cartridge.character_rom.clone());
    }

    /// https://www.nesdev.org/wiki/Mirroring#Nametable_Mirroring
    fn mirror_nametable(&self, addr: u16) -> u16 {
        // TODO: no idea how this works
        let mirrored_vram = addr & 0b10111111111111; // mirror down 0x3000-0x3eff to 0x2000 - 0x2eff
        let vram_index = mirrored_vram - 0x2000; // to vram vector
        let name_table = vram_index / 0x400; // to the name table index
        match (&self.mirroring.unwrap(), name_table) {
            (Mirroring::Vertical, 2) | (Mirroring::Vertical, 3) => vram_index - 0x800,
            (Mirroring::Horizontal, 2) => vram_index - 0x400,
            (Mirroring::Horizontal, 1) => vram_index - 0x400,
            (Mirroring::Horizontal, 3) => vram_index - 0x800,
            _ => vram_index,
        }
    }

    fn update_data_buffer(&mut self, value: u8) -> u8 {
        let result = self.data_buffer;
        self.data_buffer = value;
        result
    }

    #[tracing::instrument(skip(self), parent = &self.span)]
    pub fn render(&mut self) {
        if self.mask.show_background() {
            self.renderer.draw_background(
                self.control.background_bank(),
                self.scroll.x,
                self.scroll.y,
                self.control.nametable_start(),
                &self.mirroring.unwrap(),
                &self.vram,
            );
        }

        if self.mask.show_sprites() {
            self.renderer
                .draw_sprites(self.control.sprite_bank(), &self.oam);
        }

        self.renderer.update();
        tracing::info!("rendering frame");
    }

    #[tracing::instrument(skip(self), parent = &self.span)]
    fn increment_vram_address(&mut self) {
        let incr = self.control.vram_address_increment();
        self.address.increment(incr);
        tracing::trace!(
            "address register incremented to ${:02X}",
            self.address.value
        )
    }

    #[tracing::instrument(skip(self), parent = &self.span)]
    fn read_status(&mut self) -> u8 {
        let result = self.status.read();
        tracing::trace!("returning status register: {}", self.status);
        self.status.reset_vblank();
        self.address.reset_latch();
        self.scroll.reset_latch();
        result
    }

    fn write_control(&mut self, data: u8) {
        let nmi_before = self.control.nmi_at_vblank();
        self.control = registers::Control::from(data);
        if !nmi_before && self.status.in_vblank() && self.control.nmi_at_vblank() {
            self.trigger_nmi = true;
        }
    }

    fn write_mask(&mut self, data: u8) {
        self.mask = registers::Mask::from(data);
    }

    /// Helper for reading from PPUDATA
    #[tracing::instrument(skip(self), parent = &self.span)]
    fn read_data(&mut self) -> u8 {
        let addr = self.address.value;
        self.increment_vram_address();

        if Self::PATTERN_TABLE_RANGE.contains(&addr) {
            let result = self.renderer.pattern_table.as_ref().unwrap()[addr as usize];
            tracing::debug!("pattern table read at ${:04X}: ${:02X}", addr, result);
            self.update_data_buffer(result)
        } else if Self::NAMETABLE_RANGE.contains(&addr) {
            let result = self.vram[self.mirror_nametable(addr) as usize];
            tracing::debug!("nametable read at ${:04X}: ${:02X}", addr, result);
            self.update_data_buffer(result)
        } else if Self::PALETTE_RAM_RANGE.contains(&addr) {
            // TODO: This should set the data buffer to the nametable "below" the pattern table
            let result = self.renderer.palette[addr.into()];
            tracing::debug!("palette RAM read at ${:04X}: ${:02X}", addr, result);
            result
        } else {
            tracing::error!("invalid data read at ${:04X}", addr);
            panic!()
        }
    }

    /// Helper for writing with PPUDATA
    #[tracing::instrument(skip(self, data), parent = &self.span)]
    fn write_data(&mut self, data: u8) {
        let addr = self.address.value;

        if Self::NAMETABLE_RANGE.contains(&addr) {
            let vram_index = self.mirror_nametable(addr) as usize;
            self.vram[vram_index] = data;
            tracing::debug!("nametable write at ${:04X}: ${:02X}", vram_index, data);
        } else if Self::PALETTE_RAM_RANGE.contains(&addr) {
            self.renderer.palette[addr.into()] = data;
            tracing::debug!("palette RAM write of ${:02X}", data);
        } else if Self::PATTERN_TABLE_RANGE.contains(&addr) {
            tracing::error!(
                "attempting to write to read-only character ROM at ${:04X}: ${:02X}",
                addr,
                data
            );
            panic!()
        } else {
            tracing::error!("invalid data write at ${:04X}: ${:02X}", addr, data);
            panic!()
        }

        self.increment_vram_address();
    }

    #[tracing::instrument(skip(self, register), parent = &self.span)]
    pub fn read_register(&mut self, register: &Register) -> u8 {
        let result = match register {
            Register::Status => self.read_status(),
            Register::ObjectAttributeData => self.oam.read_data(),
            Register::Data => self.read_data(),
            _ => {
                tracing::error!("unimplemented register {} read", register);
                panic!()
            }
        };
        tracing::trace!("register {} read: ${:02X}", register, result);
        result
    }

    #[tracing::instrument(skip(self, register, data), parent = &self.span)]
    pub fn write_register(&mut self, register: &Register, data: u8) {
        match register {
            Register::Control => self.write_control(data),
            Register::Mask => self.write_mask(data),
            Register::ObjectAttributeAddress => self.oam.write_address(data),
            Register::ObjectAttributeData => self.oam.write_data(data),
            Register::Scroll => self.scroll.update(data),
            Register::Address => self.address.update(data),
            Register::Data => self.write_data(data),
            _ => {
                tracing::error!("invalid register {} write of ${:02X}", register, data);
                panic!()
            }
        }
        tracing::trace!("register {} write: ${:02X}", register, data);
    }

    pub fn poll_nmi(&mut self) -> bool {
        let result = self.trigger_nmi;
        if result {
            self.trigger_nmi = false;
        }
        result
    }

    fn is_sprite_zero_hit(&self, cycle: usize) -> bool {
        // TODO: This should check if a non-opaque BG pixel overlaps with a non-opaque sprite zero pixel,
        // instead of triggering on any sprite zero pixel.
        let obj = Object::from(&self.oam.memory[0..4]);
        (obj.y == self.scanline as usize) && obj.x <= cycle && self.mask.show_sprites()
    }
}

impl Clock for Ppu {
    const MULTIPLIER: usize = 3;

    #[tracing::instrument(skip(self, cycles), parent = &self.span)]
    fn tick_impl(&mut self, cycles: CycleCount) {
        const CYCLES_PER_SCANLINE: CycleCount = 341;
        const SCANLINES_PER_FRAME: ScanlineCount = 261;
        const VBLANK_SCANLINE: ScanlineCount = 241;

        self.cycles += cycles;
        if self.cycles >= CYCLES_PER_SCANLINE {
            if self.is_sprite_zero_hit(self.cycles) {
                self.status.set_sprite_zero(true);
            }

            self.cycles -= CYCLES_PER_SCANLINE;
            self.scanline += 1;

            if self.scanline == VBLANK_SCANLINE {
                self.status.set_vblank();
                if self.control.nmi_at_vblank() {
                    self.trigger_nmi = true;
                }
                self.status.set_sprite_zero(false);
                tracing::debug!("entering vblank, status: {}", self.status);
            }

            if self.scanline > SCANLINES_PER_FRAME {
                // Drawn every pixel, starting over
                self.scanline = 0;
                self.trigger_nmi = false;
                self.status.reset_vblank();
                self.status.set_sprite_zero(false);
                tracing::debug!("finished computing frame");
            }
        }
    }
}
