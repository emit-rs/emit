/*!
Compile-time log filtering.
*/

use std::env;

pub fn matches_build_filter() -> bool {
    match (env::var("EMIT_FILTER"), env::var("CARGO_CRATE_NAME")) {
        (Ok(filter), Ok(this_crate)) => matches(&filter, &this_crate),
        _ => true,
    }
}

fn matches(filter: &str, this_crate: &str) -> bool {
    // Just a simple `this_crate` in `crate_1, crate_2, .. , crate_n` filter
    filter.is_empty()
        || filter
            .split(',')
            .any(|include| include.trim() == this_crate)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_empty() {
        assert!(matches("", "a"));
    }

    #[test]
    fn matches_true() {
        assert!(matches("a", "a"));
        assert!(matches("c, a, b", "a"));
    }

    #[test]
    fn matches_false() {
        assert!(!matches("b", "a"));
        assert!(!matches("c, b", "a"));
    }
}
