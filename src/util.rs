use std::ops::{Index, IndexMut};

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

/// A fixed size circular buffer.
pub struct CircularBuffer<T, const N: usize> {
    data: [Option<T>; N],
    current: usize,
}

impl<T, const N: usize> CircularBuffer<T, N> {
    const DEFAULT: Option<T> = None;

    pub const fn new() -> Self {
        Self {
            data: [Self::DEFAULT; N],
            current: 0,
        }
    }

    pub const fn len(&self) -> usize {
        N
    }

    /// Push a value into the buffer, overwriting the oldest value if the buffer is full.
    pub fn push(&mut self, value: T) {
        self.data[self.current] = Some(value);
        self.current = self.current.wrapping_add(1) % N;
    }

    /// Get the last value pushed into the buffer.
    pub fn last(&self) -> Option<&T> {
        self.data[self.current.saturating_sub(1)].as_ref()
    }

    pub fn clear(&mut self) {
        self.data = [Self::DEFAULT; N];
        self.current = 0;
    }

    pub fn iter(&self) -> impl Iterator<Item = &T> {
        let start = self.current;
        let end = self.current.wrapping_add(N);
        (start..end).filter_map(move |i| self.data[i % N].as_ref())
    }
}

impl<T, const N: usize> Index<usize> for CircularBuffer<T, N> {
    type Output = Option<T>;

    fn index(&self, index: usize) -> &Self::Output {
        &self.data[self.current.wrapping_add(index) % N]
    }
}

impl<T, const N: usize> IndexMut<usize> for CircularBuffer<T, N> {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.data[self.current.wrapping_add(index) % N]
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_nth_bit() {
        let value = 0b1010_1010;
        assert!(!nth_bit(value, 0));
        assert!(nth_bit(value, 1));
        assert!(!nth_bit(value, 2));
        assert!(nth_bit(value, 3));
        assert!(!nth_bit(value, 4));
        assert!(nth_bit(value, 5));
        assert!(!nth_bit(value, 6));
        assert!(nth_bit(value, 7));
    }
}
