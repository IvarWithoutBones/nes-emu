use bitflags::bitflags;

bitflags! {
    /*
        See https://www.nesdev.org/wiki/Status_flags

        7  bit  0
        ---- ----
        NVss DIZC
        |||| ||||
        |||| |||+- Carry
        |||| ||+-- Zero
        |||| |+--- Interrupt Disable
        |||| +---- Decimal
        ||++------ No CPU effect, see: the B flag
        |+-------- Overflow
        +--------- Negative
    */
    #[derive(Debug, Clone, PartialEq)]
    pub struct CpuFlags: u8 {
        const Carry              = 0b0000_0001;
        const Zero               = 0b0000_0010;
        const InterruptsDisabled = 0b0000_0100;
        const Decimal            = 0b0000_1000; // No effect
        const Break              = 0b0001_0000;
        const Break2             = 0b0010_0000; // No effect
        const Overflow           = 0b0100_0000;
        const Negative           = 0b1000_0000;
    }
}

impl Default for CpuFlags {
    fn default() -> CpuFlags {
        Self::InterruptsDisabled | Self::Break | Self::Break2
        // Self::IRQ | Self::BREAK2 // Hack to diff against nestest log, above is correct
    }
}

impl CpuFlags {
    const fn format(&self, flag: CpuFlags, display: char) -> char {
        if self.contains(flag) {
            display
        } else {
            '-'
        }
    }
}

impl std::fmt::Display for CpuFlags {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut string = String::with_capacity(8);
        string.push(self.format(CpuFlags::Negative, 'N'));
        string.push(self.format(CpuFlags::Overflow, 'O'));
        string.push(self.format(CpuFlags::Break2, 'B'));
        string.push(self.format(CpuFlags::Break, 'B'));
        string.push(self.format(CpuFlags::Decimal, 'D'));
        string.push(self.format(CpuFlags::InterruptsDisabled, 'I'));
        string.push(self.format(CpuFlags::Zero, 'Z'));
        string.push(self.format(CpuFlags::Carry, 'C'));
        write!(f, "{}", string)
    }
}
