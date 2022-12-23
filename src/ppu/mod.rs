pub(crate) mod registers;

use crate::cartridge::Mirroring;
use registers::Register;
use std::ops::RangeInclusive;

/// https://www.nesdev.org/wiki/PPU
pub struct Ppu {
    span: tracing::Span,
    mirroring: Mirroring,
    character_rom: Vec<u8>,

    nmi_occured: bool,
    nmi_output: bool,

    palette_table: [u8; Self::PALETTE_TABLE_SIZE],
    object_attribute_table: [u8; Self::OBJECT_ATTRIBUTE_TABLE_SIZE],
    vram: [u8; Self::VRAM_SIZE],

    control: registers::Control,
    mask: registers::Mask,
    status: registers::Status,
    object_attribute_address: registers::ObjectAttributeAddress,
    scroll: registers::Scroll,
    address: registers::Address,
    data: registers::Data,
}

impl Ppu {
    const PATTERN_TABLE_RANGE: RangeInclusive<u16> = 0..=0x1FFF;
    const NAMETABLE_RANGE: RangeInclusive<u16> = 0x2000..=0x3EFF;
    const PALETTE_RAM_RANGE: RangeInclusive<u16> = 0x3F00..=0x3FFF;

    const PALETTE_TABLE_SIZE: usize = 32;
    pub const OBJECT_ATTRIBUTE_TABLE_SIZE: usize = 0x100;
    const VRAM_SIZE: usize = 0x800;

    pub fn new(character_rom: Vec<u8>, mirroring: Mirroring) -> Self {
        Self {
            span: tracing::span!(tracing::Level::INFO, "ppu"),
            mirroring,
            character_rom,

            nmi_occured: false,
            nmi_output: false,

            palette_table: [0; Self::PALETTE_TABLE_SIZE],
            object_attribute_table: [0; Self::OBJECT_ATTRIBUTE_TABLE_SIZE],
            vram: [0; Self::VRAM_SIZE],

            control: registers::Control::default(),
            mask: registers::Mask::default(),
            status: registers::Status::default(),
            object_attribute_address: registers::ObjectAttributeAddress::default(),
            scroll: registers::Scroll::default(),
            address: registers::Address::default(),
            data: registers::Data::default(),
        }
    }

    /// https://www.nesdev.org/wiki/Mirroring#Nametable_Mirroring
    fn mirror_nametable_addr(&self, addr: u16) -> u16 {
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

    fn mirror_palette_table(&self, addr: u16) -> usize {
        (addr % Self::PALETTE_TABLE_SIZE as u16) as usize
    }

    #[tracing::instrument(skip(self), parent = &self.span)]
    fn increment_vram_address(&mut self) {
        self.address
            .increment(self.control.vram_address_increment());
        tracing::trace!("address register increment: ${:02X}", self.address.value)
    }

    /// Helper for reading from OAMDATA
    #[tracing::instrument(skip(self), parent = &self.span)]
    fn read_object_attribute(&mut self) -> u8 {
        // TODO: do we increment OAMADDR when not in vertical or forced blanking?
        // TODO: do we index with the OAMDATA value in any scenario?
        let index = self.object_attribute_address.value as usize;
        let result = self.object_attribute_table[index];
        tracing::trace!("object attribute read at ${:02X}: ${:02X}", index, result);
        result
    }

    /// Helper for writing to OAMDATA
    #[tracing::instrument(skip(self), parent = &self.span)]
    fn write_object_attribute(&mut self, data: u8) {
        // TODO: do we index with the OAMDATA value in any scenario?
        let index = self.object_attribute_address.value as usize;
        tracing::trace!("object attribute write at ${:02X}: ${:02X}", index, data);
        self.object_attribute_table[index] = data;
        self.object_attribute_address.increment();
    }

    /// DMA to copy a slice of bytes from CPU RAM to OAM
    pub fn object_attribute_direct_memory_access(&mut self, data: &[u8]) {
        let starting_point = self.object_attribute_address.value as usize;
        for (mut index, byte) in data.iter().enumerate() {
            index = index.wrapping_add(starting_point);
            self.object_attribute_table[index] = *byte;
        }
    }

    fn read_status(&mut self) -> u8 {
        self.nmi_occured = false;
        self.address.reset_latch();
        self.status.read()
    }

    fn write_control(&mut self, data: u8) {
        self.control.update(data);
        self.nmi_output = self
            .control
            .contains(registers::Control::NonMaskableInterruptAtVBlank);
    }

    /// Helper for reading from PPUDATA
    #[tracing::instrument(skip(self), parent = &self.span)]
    fn read_data(&mut self) -> u8 {
        let addr = self.address.value;
        self.increment_vram_address();

        if Self::PATTERN_TABLE_RANGE.contains(&addr) {
            let result = self.character_rom[addr as usize];
            tracing::trace!("pattern table read at ${:04X}: ${:02X}", addr, result);
            return self.data.update_buffer(result);
        } else if Self::NAMETABLE_RANGE.contains(&addr) {
            let result = self.vram[self.mirror_nametable_addr(addr) as usize];
            tracing::trace!("nametable read at ${:04X}: ${:02X}", addr, result);
            return self.data.update_buffer(result);
        } else if Self::PALETTE_RAM_RANGE.contains(&addr) {
            // TODO: This should set the data buffer to the nametable "below" the pattern table
            let result = self.palette_table[self.mirror_palette_table(addr)];
            tracing::trace!("palette RAM read at ${:04X}: ${:02X}", addr, result);
            return result;
        } else {
            tracing::error!("invalid data read at ${:04X}", addr);
            panic!()
        }
    }

    /// Helper for writing with PPUDATA
    #[tracing::instrument(skip(self), parent = &self.span)]
    fn write_data(&mut self, data: u8) {
        let addr = self.address.value;

        if Self::PATTERN_TABLE_RANGE.contains(&addr) {
            tracing::error!(
                "attempting to write to read-only character ROM at ${:04X}: ${:02X}",
                addr,
                data
            );
        } else if Self::NAMETABLE_RANGE.contains(&addr) {
            let vram_index = self.mirror_nametable_addr(addr) as usize;
            tracing::trace!("nametable write at ${:04X}: ${:02X}", vram_index, data);
            self.vram[vram_index] = data;
        } else if Self::PALETTE_RAM_RANGE.contains(&addr) {
            let palette_index = self.mirror_palette_table(addr);
            tracing::trace!("palette RAM write at ${:04X}: ${:02X}", palette_index, data);
            self.palette_table[palette_index] = data;
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
            Register::ObjectAttributeData => self.read_object_attribute(),
            Register::Data => self.read_data(),
            _ => {
                tracing::error!("unimplemented register {:?} read", register);
                panic!()
            }
        };
        tracing::trace!("register {:?} read: ${:02X}", register, result);
        result
    }

    #[tracing::instrument(skip(self, register, data), parent = &self.span)]
    pub fn write_register(&mut self, register: &Register, data: u8) {
        match register {
            Register::Control => self.write_control(data),
            Register::Mask => self.mask.update(data),
            Register::ObjectAttributeAddress => self.object_attribute_address.update(data),
            Register::ObjectAttributeData => self.write_object_attribute(data),
            Register::Scroll => self.scroll.update(data),
            Register::Address => self.address.update(data),
            Register::Data => self.write_data(data),
            Register::ObjectAttributeDirectMemoryAccess => {
                tracing::error!(
                    "invalid addressing of register {:?} write (${:02X})",
                    register,
                    data
                );
                panic!()
            }
            _ => {
                tracing::error!(
                    "unimplemented register {:?} write of ${:02X}",
                    register,
                    data
                );
                panic!()
            }
        }
        tracing::trace!("register {:?} write: ${:02X}", register, data);
    }

    pub fn poll_nmi(&self) -> bool {
        self.nmi_occured && self.nmi_output
    }
}
