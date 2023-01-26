use bitflags::BitFlags;

/// Format a flag in a BitFlags enabled struct.
pub trait FormatBitFlags {
    fn format(&self, flag: Self, display: char) -> char;
}

impl<T: BitFlags> FormatBitFlags for T {
    /// Format a flag to the given character if present, or a dash if not.
    fn format(&self, flag: Self, display: char) -> char {
        if self.contains(flag) {
            display
        } else {
            '-'
        }
    }
}

/// Get the status of bit N in the given value.
pub const fn nth_bit(value: u8, n: u8) -> bool {
    value & (1 << n) != 0
}

/// Combine two booleans into a 2-bit value.
pub const fn combine_bools(high: bool, low: bool) -> u8 {
    ((1 & high as u8) << 1) | (1 & low as u8)
}

/// Expand an array of T into an array of Option<T>, filling the unknown elements with None.
pub const fn expand_array<T, const IN_LEN: usize, const OUT_LEN: usize>(
    input: &[T; IN_LEN],
) -> [Option<T>; OUT_LEN]
where
    T: Copy,
{
    let mut expanded = [None; OUT_LEN];
    let mut i = 0;
    while i < input.len() {
        expanded[i] = Some(input[i]);
        i += 1;
    }
    expanded
}
