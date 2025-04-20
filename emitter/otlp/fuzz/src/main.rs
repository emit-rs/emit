#![allow(dead_code)]

#[path = "../../src/error.rs"]
mod error;

#[path = "../../src/baggage.rs"]
mod baggage;

use self::error::*;

use std::str;

fn main() {
    afl::fuzz!(|data: &[u8]| {
        let Ok(s) = str::from_utf8(data) else {
            return;
        };

        // Just ensure we don't panic
        let _ = baggage::parse(s);
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::fs;

    #[test]
    fn valid() {
        for case in fs::read_dir("./in").unwrap() {
            let content = fs::read_to_string(case.unwrap().path()).unwrap();

            baggage::parse(&content).unwrap();
        }
    }
}
