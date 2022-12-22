pub(crate) mod registers;

use crate::cartridge::Mirroring;
use registers::Register;

/// https://www.nesdev.org/wiki/PPU
pub struct Ppu {
    span: tracing::Span,
    mirroring: Mirroring,
    character_rom: Vec<u8>,

    palette_table: [u8; Self::PALETTE_TABLE_SIZE],
    object_attribute_table: [u8; Self::OBJECT_ATTRIBUTE_TABLE_SIZE],
    vram: [u8; Self::VRAM_SIZE],
    data_buffer: u8,

    control: registers::Control,
    mask: registers::Mask,
    status: registers::Status,
    object_attribute_address: registers::ObjectAttributeAddress,
    object_attribute_data: registers::ObjectAttributeData,
    scroll: registers::Scroll,
    address: registers::Address,
    object_attribute_direct_memory_access: registers::ObjectAttributeDirectMemoryAccess,
}

impl Ppu {
    const PALETTE_TABLE_SIZE: usize = 0x20;
    const OBJECT_ATTRIBUTE_TABLE_SIZE: usize = 0x100;
    const VRAM_SIZE: usize = 0x800;

    const PATTERN_TABLE_START: u16 = 0;
    const PATTERN_TABLE_END: u16 = 0x1FFF;

    const NAMETABLE_START: u16 = 0x2000;
    const NAMETABLE_MIRRORS_END: u16 = 0x3EFF;

    const PALETTE_RAM_START: u16 = 0x3F00;
    const PALETTE_RAM_MIRRORS_END: u16 = 0x3FFF;

    pub fn new(character_rom: Vec<u8>, mirroring: Mirroring) -> Self {
        Self {
            span: tracing::span!(tracing::Level::INFO, "ppu"),
            mirroring,
            character_rom,

            palette_table: [0; Self::PALETTE_TABLE_SIZE],
            object_attribute_table: [0; Self::OBJECT_ATTRIBUTE_TABLE_SIZE],
            vram: [0; Self::VRAM_SIZE],
            data_buffer: 0,

            control: registers::Control::default(),
            mask: registers::Mask::default(),
            status: registers::Status::default(),
            object_attribute_address: registers::ObjectAttributeAddress::default(),
            object_attribute_data: registers::ObjectAttributeData::default(),
            scroll: registers::Scroll::default(),
            address: registers::Address::default(),
            object_attribute_direct_memory_access:
                registers::ObjectAttributeDirectMemoryAccess::default(),
        }
    }

    fn update_buffer(&mut self, value: u8) -> u8 {
        let result = self.data_buffer;
        self.data_buffer = value;
        result
    }

    fn increment_vram_address(&mut self) {
        self.address
            .increment(self.control.vram_address_increment());
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

    #[tracing::instrument(skip(self), parent = &self.span)]
    pub fn read_data(&mut self) -> u8 {
        let addr = self.address.value;
        self.increment_vram_address();

        match addr {
            Self::PATTERN_TABLE_START..=Self::PATTERN_TABLE_END => {
                tracing::trace!(addr, "pattern table read");
                self.update_buffer(self.character_rom[addr as usize])
            }

            Self::NAMETABLE_START..=Self::NAMETABLE_MIRRORS_END => {
                tracing::trace!(addr, "nametable read");
                self.update_buffer(self.vram[self.mirror_nametable_addr(addr) as usize])
            }

            Self::PALETTE_RAM_START..=Self::PALETTE_RAM_MIRRORS_END => {
                tracing::trace!(addr, "palette ram read");
                self.palette_table[(addr - Self::PALETTE_RAM_START) as usize]
            }

            _ => {
                tracing::error!(addr, "invalid data read");
                panic!()
            }
        }
    }

    #[tracing::instrument(skip(self, register), parent = &self.span)]
    pub fn read_register(&mut self, register: &Register) -> u8 {
        let result = match register {
            Register::Status => self.status.read(),
            Register::ObjectAttributeData => self.object_attribute_data.read(),
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
        tracing::trace!("register write: ${:02X}", data);
        match register {
            Register::Control => self.control.update(data),
            Register::Mask => self.mask.update(data),
            Register::ObjectAttributeAddress => self.object_attribute_address.update(data),
            Register::ObjectAttributeData => self.object_attribute_data.update(data),
            Register::Scroll => self.scroll.update(data),
            Register::Address => self.address.update(data),
            // Register::Data
            Register::ObjectAttributeDirectMemoryAccess => {
                self.object_attribute_direct_memory_access.update(data)
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
    }
}
