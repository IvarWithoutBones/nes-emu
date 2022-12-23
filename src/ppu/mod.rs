mod object_attribute;
pub(crate) mod registers;

use super::bus::{Clock, CycleCount};
use crate::cartridge::Mirroring;
use object_attribute::ObjectAttributeMemory;
use registers::Register;
use std::ops::RangeInclusive;

type ScanlineCount = u16;

/// https://www.nesdev.org/wiki/PPU
pub struct Ppu {
    span: tracing::Span,
    mirroring: Mirroring,
    character_rom: Vec<u8>,

    data_buffer: u8,
    palette_table: [u8; Self::PALETTE_TABLE_SIZE],
    vram: [u8; Self::VRAM_SIZE],
    pub oam: ObjectAttributeMemory,

    control: registers::Control,
    mask: registers::Mask,
    status: registers::Status,
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

    const PALETTE_TABLE_SIZE: usize = 32;
    const VRAM_SIZE: usize = 0x800;

    pub fn new(character_rom: Vec<u8>, mirroring: Mirroring) -> Self {
        Self {
            span: tracing::span!(tracing::Level::INFO, "ppu"),
            mirroring,
            character_rom,

            data_buffer: 0,
            palette_table: [0; Self::PALETTE_TABLE_SIZE],
            vram: [0; Self::VRAM_SIZE],
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

    /// https://www.nesdev.org/wiki/Mirroring#Nametable_Mirroring
    const fn mirror_nametable(&self, addr: u16) -> u16 {
        // TODO: no idea how this works
        let mirrored_vram = addr & 0b10111111111111; // mirror down 0x3000-0x3eff to 0x2000 - 0x2eff
        let vram_index = mirrored_vram - 0x2000; // to vram vector
        let name_table = vram_index / 0x400; // to the name table index
        match (&self.mirroring, name_table) {
            (Mirroring::Vertical, 2) | (Mirroring::Vertical, 3) => vram_index - 0x800,
            (Mirroring::Horizontal, 2) => vram_index - 0x400,
            (Mirroring::Horizontal, 1) => vram_index - 0x400,
            (Mirroring::Horizontal, 3) => vram_index - 0x800,
            _ => vram_index,
        }
    }

    const fn mirror_palette_table(&self, addr: u16) -> usize {
        (addr % Self::PALETTE_TABLE_SIZE as u16) as usize
    }

    fn update_data_buffer(&mut self, value: u8) -> u8 {
        let result = self.data_buffer;
        self.data_buffer = value;
        result
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
        self.control.update(data);
        if !nmi_before && self.status.in_vblank() && self.control.nmi_at_vblank() {
            self.trigger_nmi = true;
        }
    }

    /// Helper for reading from PPUDATA
    #[tracing::instrument(skip(self), parent = &self.span)]
    fn read_data(&mut self) -> u8 {
        let addr = self.address.value;
        self.increment_vram_address();

        if Self::PATTERN_TABLE_RANGE.contains(&addr) {
            let result = self.character_rom[addr as usize];
            tracing::info!("pattern table read at ${:04X}: ${:02X}", addr, result);
            return self.update_data_buffer(result);
        } else if Self::NAMETABLE_RANGE.contains(&addr) {
            let result = self.vram[self.mirror_nametable(addr) as usize];
            tracing::info!("nametable read at ${:04X}: ${:02X}", addr, result);
            return self.update_data_buffer(result);
        } else if Self::PALETTE_RAM_RANGE.contains(&addr) {
            // TODO: This should set the data buffer to the nametable "below" the pattern table
            let result = self.palette_table[self.mirror_palette_table(addr)];
            tracing::info!("palette RAM read at ${:04X}: ${:02X}", addr, result);
            return result;
        } else {
            tracing::error!("invalid data read at ${:04X}", addr);
            panic!()
        }
    }

    /// Helper for writing with PPUDATA
    #[tracing::instrument(skip(self, data), parent = &self.span)]
    fn write_data(&mut self, data: u8) {
        let addr = self.address.value;

        if Self::PATTERN_TABLE_RANGE.contains(&addr) {
            tracing::error!(
                "attempting to write to read-only character ROM at ${:04X}: ${:02X}",
                addr,
                data
            );
        } else if Self::NAMETABLE_RANGE.contains(&addr) {
            let vram_index = self.mirror_nametable(addr) as usize;
            tracing::info!("nametable write at ${:04X}: ${:02X}", vram_index, data);
            self.vram[vram_index] = data;
        } else if Self::PALETTE_RAM_RANGE.contains(&addr) {
            let palette_index = self.mirror_palette_table(addr);
            tracing::info!("palette RAM write at ${:04X}: ${:02X}", palette_index, data);
            self.palette_table[palette_index] = data;
        } else {
            tracing::info!("invalid data write at ${:04X}: ${:02X}", addr, data);
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
            Register::Mask => self.mask.update(data),
            Register::ObjectAttributeAddress => self.oam.write_address(data),
            Register::ObjectAttributeData => self.oam.write_data(data),
            Register::Scroll => self.scroll.update(data),
            Register::Address => self.address.update(data),
            Register::Data => self.write_data(data),
            Register::ObjectAttributeDirectMemoryAccess => {
                tracing::error!(
                    "invalid addressing for register {}, write of ${:02X}",
                    register,
                    data
                );
                panic!()
            }
            _ => {
                tracing::error!("unimplemented register {} write of ${:02X}", register, data);
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
}

impl Clock for Ppu {
    const MULTIPLIER: usize = 3;

    #[tracing::instrument(skip(self, cycles), parent = &self.span)]
    fn tick_internal(&mut self, cycles: CycleCount) {
        const CYCLES_PER_SCANLINE: CycleCount = 341;
        const SCANLINES_PER_FRAME: ScanlineCount = 261;
        const VBLANK_SCANLINE: ScanlineCount = 241;

        self.cycles += cycles;
        if self.cycles >= CYCLES_PER_SCANLINE {
            self.set_cycles(self.cycles - CYCLES_PER_SCANLINE);
            self.scanline += 1;

            if self.scanline == VBLANK_SCANLINE {
                self.status.set_vblank();
                tracing::info!("setting vblank: {}", self.status);
                if self.control.nmi_at_vblank() {
                    self.trigger_nmi = true;
                }
            }

            if self.scanline > SCANLINES_PER_FRAME {
                // Drawn everything, starting all over again
                self.scanline = 0;
                self.trigger_nmi = false;
                self.status.reset_vblank();
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
