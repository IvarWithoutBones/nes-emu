use tartan_bitfield::bitfield;

bitfield! {
    /*
        7  bit  0
        ---- ----
        VPHB SINN
        |||| ||||
        |||| ||++- Base nametable address
        |||| ||    (0 = $2000; 1 = $2400; 2 = $2800; 3 = $2C00)
        |||| |+--- VRAM address increment per CPU read/write of PPUDATA
        |||| |     (0: add 1, going across; 1: add 32, going down)
        |||| +---- Sprite pattern table address for 8x8 sprites
        ||||       (0: $0000; 1: $1000; ignored in 8x16 mode)
        |||+------ Background pattern table address (0: $0000; 1: $1000)
        ||+------- Sprite size (0: 8x8 pixels; 1: 8x16 pixels – see PPU OAM#Byte 1)
        |+-------- PPU parent/child select
        |          (0: read backdrop from EXT pins; 1: output color on EXT pins)
        +--------- Generate an NMI at the start of the vertical blanking interval (0: off; 1: on)
    */
    /// https://www.nesdev.org/wiki/PPU_registers#PPUCTRL
    pub struct Control(u8) {
        [0..=1] pub nametable_address: u8,
        [2] pub vram_address_increment_bit,
        [3] pub sprite_pattern_table_bank,
        [4] pub background_pattern_table_bank,
        [5] pub sprite_size,
        [6] pub parent_child_select,
        [7] pub non_maskable_interrupt_at_vblank,
    }
}

impl Control {
    pub fn vram_address_increment(&self) -> u8 {
        if self.vram_address_increment_bit() {
            32
        } else {
            1
        }
    }

    pub fn background_bank(&self) -> usize {
        if self.background_pattern_table_bank() {
            0x1000
        } else {
            0
        }
    }

    pub fn sprite_bank(&self) -> Option<usize> {
        if self.sprite_size() {
            // 8x16 sprite, bank will be ignored
            None
        } else if self.sprite_pattern_table_bank() {
            Some(0x1000)
        } else {
            Some(0)
        }
    }
}
