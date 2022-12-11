pub const fn nth_bit(value: u8, n: u8) -> bool {
    value & (1 << n) != 0
}
