use core::fmt;

/**
Trim ASCII whitespace from the start of `s`.
*/
#[cfg(feature = "alloc")]
pub(crate) fn trim_start(s: &[u8]) -> &[u8] {
    let start = s
        .iter()
        .position(|&b| !b.is_ascii_whitespace())
        .unwrap_or(s.len());
    &s[start..]
}

/**
Trim ASCII whitespace from the end of `s`.
*/
#[cfg(feature = "alloc")]
pub(crate) fn trim_end(s: &[u8]) -> &[u8] {
    let end = s
        .iter()
        .rposition(|&b| !b.is_ascii_whitespace())
        .map(|i| i + 1)
        .unwrap_or(0);
    &s[..end]
}

/**
Trim ASCII whitespace from `s`.
*/
#[cfg(feature = "alloc")]
pub(crate) fn trim(s: &[u8]) -> &[u8] {
    trim_end(trim_start(s))
}

/**
Find the first occurrence of any `(needle, skip)` in `haystack`.
*/
#[cfg(feature = "alloc")]
pub(crate) fn find(haystack: &[u8], needle: &[(u8, u8)]) -> Option<(usize, usize)> {
    needle
        .iter()
        .filter_map(|(n, cs)| {
            haystack
                .iter()
                .position(|&b| b == *n)
                .map(|c| (c, *cs as usize))
        })
        .next()
}

/**
An internal utility for buffering `Display` into `&str`.
*/
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

    #[cfg(all(feature = "alloc", any(feature = "sval", feature = "serde")))]
    pub(super) fn reset(&mut self) {
        self.idx = 0;
    }

    #[cfg(all(feature = "alloc", any(feature = "sval", feature = "serde")))]
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
