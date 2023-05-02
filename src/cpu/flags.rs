use tartan_bitfield::bitfield;

bitfield! {
    /// https://www.nesdev.org/wiki/Status_flags
    pub struct CpuFlags(u8) {
        [0] pub carry,
        [1] pub zero,
        [2] pub interrupts_disabled,
        [3] pub decimal, // No effect
        [4] pub break_1,
        [5] pub break_2, // No effect
        [6] pub overflow,
        [7] pub negative,
    }
}

impl CpuFlags {
    pub fn new() -> Self {
        Self::default()
            .with_interrupts_disabled(true)
            .with_break_1(true)
            .with_break_2(true)
    }

    fn format(&self, flag: bool, c: char) -> char {
        if flag {
            c
        } else {
            '-'
        }
    }
}

impl std::fmt::Display for CpuFlags {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut string = String::with_capacity(8);
        string.push(self.format(self.carry(), 'C'));
        string.push(self.format(self.zero(), 'Z'));
        string.push(self.format(self.interrupts_disabled(), 'I'));
        string.push(self.format(self.decimal(), 'D'));
        string.push(self.format(self.break_1(), 'B'));
        string.push(self.format(self.break_2(), 'B'));
        string.push(self.format(self.overflow(), 'O'));
        string.push(self.format(self.negative(), 'N'));
        write!(f, "{string}")
    }
}
