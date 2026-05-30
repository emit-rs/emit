use core::fmt;

pub(super) struct Buffer<const N: usize> {
    value: [u8; N],
    idx: usize,
}

impl<const N: usize> Buffer<N> {
    pub(super) fn new() -> Self {
        Buffer {
            value: [0; N],
            idx: 0,
        }
    }

    #[cfg(any(feature = "sval", feature = "serde"))]
    pub(super) fn reset(&mut self) {
        self.idx = 0;
    }

    #[cfg(any(feature = "sval", feature = "serde"))]
    pub(super) fn as_bytes(&self) -> &[u8] {
        &self.value[..self.idx]
    }

    pub(super) fn push_str(&mut self, s: &str) -> bool {
        let s = s.as_bytes();
        let next_idx = self.idx + s.len();

        if next_idx <= self.value.len() {
            self.value[self.idx..next_idx].copy_from_slice(s);
            self.idx = next_idx;

            true
        } else {
            false
        }
    }

    pub(super) fn buffer(&mut self, value: impl fmt::Display) -> Result<&[u8], fmt::Error> {
        use fmt::Write as _;

        self.idx = 0;

        write!(self, "{}", value)?;

        Ok(&self.value[..self.idx])
    }
}

impl<const N: usize> fmt::Write for Buffer<N> {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        if self.push_str(s) {
            Ok(())
        } else {
            Err(fmt::Error)
        }
    }
}
