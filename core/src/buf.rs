use core::fmt;

/**
An internal utility for buffering `Display` into `&str`.
*/
pub(crate) struct Buffer<const N: usize> {
    value: [u8; N],
    idx: usize,
}

impl<const N: usize> Buffer<N> {
    pub(crate) fn new() -> Self {
        Buffer {
            value: [0; N],
            idx: 0,
        }
    }

    pub(crate) fn buffer(&mut self, value: impl fmt::Display) -> Option<&[u8]> {
        use fmt::Write as _;

        self.idx = 0;

        write!(self, "{}", value).ok()?;

        Some(&self.value[..self.idx])
    }
}

impl<const N: usize> fmt::Write for Buffer<N> {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        let s = s.as_bytes();
        let next_idx = self.idx + s.len();

        if next_idx <= self.value.len() {
            self.value[self.idx..next_idx].copy_from_slice(s);
            self.idx = next_idx;

            Ok(())
        } else {
            Err(fmt::Error)
        }
    }
}
