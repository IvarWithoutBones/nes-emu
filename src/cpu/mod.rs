mod addressing_mode;
pub mod flags;
mod instructions;

use self::flags::CpuFlags;
use crate::{
    bus::{Bus, Clock, CycleCount, Device, Memory},
    util,
};
pub use addressing_mode::AddressingMode;
use std::{
    fmt,
    ops::{Index, IndexMut},
};

/// Writable memory for the CPU
#[derive(Clone)]
pub struct CpuRam {
    pub data: [u8; Self::SIZE],
}

impl CpuRam {
    pub const SIZE: usize = 0x800;

    const fn mirror(address: u16) -> usize {
        // Addressing is 11 bits, so we need to mask the top 5 off
        (address & 0b0000_0111_1111_1111) as usize
    }

    pub const fn len(&self) -> usize {
        Self::SIZE
    }
}

impl Device for CpuRam {
    fn contains(&self, address: u16) -> bool {
        (0..=0x1FFF).contains(&address)
    }
}

impl Default for CpuRam {
    fn default() -> Self {
        Self {
            data: [0; Self::SIZE],
        }
    }
}

impl Index<u16> for CpuRam {
    type Output = u8;

    fn index(&self, index: u16) -> &Self::Output {
        &self.data[Self::mirror(index)]
    }
}

impl IndexMut<u16> for CpuRam {
    fn index_mut(&mut self, index: u16) -> &mut Self::Output {
        &mut self.data[Self::mirror(index)]
    }
}

/// See https://www.nesdev.org/wiki/CPU
pub struct Cpu {
    span: tracing::Span,
    pub accumulator: u8,
    pub register_x: u8,
    pub register_y: u8,
    pub program_counter: u16,
    pub stack_pointer: u8,
    pub flags: CpuFlags,
    pub bus: Bus,
}

impl Cpu {
    const STACK_OFFSET: u16 = 0x0100;
    const STACK_RESET: u8 = 0xFD;

    const NMI_VECTOR: u16 = 0xFFFA;
    const RESET_VECTOR: u16 = 0xFFFC;
    pub const BREAK_VECTOR: u16 = 0xFFFE;

    pub fn new(bus: Bus) -> Cpu {
        Cpu {
            span: tracing::span!(tracing::Level::INFO, "cpu"),
            stack_pointer: Cpu::STACK_RESET,
            flags: CpuFlags::default(),
            program_counter: 0,
            accumulator: 0,
            register_x: 0,
            register_y: 0,
            bus,
        }
    }

    #[tracing::instrument(skip(self), parent = &self.span)]
    pub fn reset(&mut self) {
        tracing::info!("resetting");
        self.bus.reset();
        self.flags = CpuFlags::new();
        self.stack_pointer = Cpu::STACK_RESET;
        self.accumulator = 0;
        self.register_x = 0;
        self.register_y = 0;
        self.program_counter = self.read_word(Cpu::RESET_VECTOR);
        tracing::info!("initialising, PC={:04X}", self.program_counter);
    }

    /// Check if two values are contained on a different page in memory
    pub const fn is_on_different_page(a: u16, b: u16) -> bool {
        (a & 0xFF00) != (b & 0xFF00)
    }

    pub fn push_byte(&mut self, data: u8) {
        self.write_byte(Cpu::STACK_OFFSET + self.stack_pointer as u16, data);
        self.stack_pointer = self.stack_pointer.wrapping_sub(1);
        tracing::trace!("pushed ${:02X} to the stack", data);
    }

    pub fn pop_byte(&mut self) -> u8 {
        self.stack_pointer = self.stack_pointer.wrapping_add(1);
        let data = self.read_byte(Cpu::STACK_OFFSET + self.stack_pointer as u16);
        tracing::trace!("popped ${:02X} off the stack", data);
        data
    }

    pub fn push_word(&mut self, data: u16) {
        for byte in u16::to_be_bytes(data) {
            self.push_byte(byte);
        }
    }

    pub fn pop_word(&mut self) -> u16 {
        u16::from_le_bytes([self.pop_byte(), self.pop_byte()])
    }

    pub fn update_zero_and_negative_flags(&mut self, value: u8) {
        self.flags.set_negative(util::nth_bit(value, 7));
        self.flags.set_zero(value == 0);
    }

    #[tracing::instrument(skip(self), parent = &self.span)]
    pub fn non_maskable_interrupt(&mut self) {
        tracing::info!("NMI triggered");
        let mut flags = self.flags;
        flags.set_break_1(false);
        flags.set_break_2(true);

        self.push_word(self.program_counter);
        self.push_byte(flags.into());

        self.flags.set_interrupts_disabled(true);
        self.program_counter = self.read_word(Self::NMI_VECTOR);
        self.tick(2);
    }

    #[tracing::instrument(skip(self), parent = &self.span)]
    pub fn step(&mut self) -> Option<CpuState> {
        if self.bus.ppu.poll_nmi() {
            self.non_maskable_interrupt();
        }

        let opcode = self.read_byte(self.program_counter);
        let (instr, mode, cycles) = instructions::decode(opcode).unwrap_or_else(|| {
            panic!(
                "invalid opcode ${:02X} at PC ${:04X}",
                opcode, self.program_counter
            )
        });

        let state = CpuState {
            instruction: instr.format(self, mode),
            accumulator: self.accumulator,
            register_x: self.register_x,
            register_y: self.register_y,
            program_counter: self.program_counter,
            stack_pointer: self.stack_pointer,
            status: self.flags,
            memory: self.bus.cpu_ram.clone(),
        };

        tracing::debug!("{}  {}", self, state.instruction);

        (instr.function)(self, mode);
        if !instr.changes_program_counter {
            // Some instructions (e.g. JMP) set the program counter themselves
            self.program_counter = self.program_counter.wrapping_add(mode.len());
        }

        #[cfg(test)]
        if instr.name == "BRK" {
            return None;
        }

        self.tick(*cycles);
        Some(state)
    }
}

impl Memory for Cpu {
    fn read_byte(&mut self, address: u16) -> u8 {
        self.bus.read_byte(address)
    }

    fn write_byte(&mut self, address: u16, data: u8) {
        self.bus.write_byte(address, data)
    }
}

impl Clock for Cpu {
    fn tick_impl(&mut self, cycles: CycleCount) {
        self.bus.tick(cycles);
    }
}

impl fmt::Display for Cpu {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{:04X}  A:{:02X} X:{:02X} Y:{:02X} P:{:02X} SP:{:02X} C:{} {}",
            self.program_counter,
            self.accumulator,
            self.register_x,
            self.register_y,
            u8::from(self.flags),
            self.stack_pointer,
            self.bus.cycles,
            self.flags
        )
    }
}

/// Passed to the GUI for the debugger. Could maybe be used for savestates in the future?
pub struct CpuState {
    pub instruction: String,
    pub accumulator: u8,
    pub register_x: u8,
    pub register_y: u8,
    pub program_counter: u16,
    pub stack_pointer: u8,
    pub status: CpuFlags,
    pub memory: CpuRam,
}
