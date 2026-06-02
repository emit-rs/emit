fn main() {
    emit_fuzz::main(de);
}

pub fn de(input: &[u8]) {
    let Ok(input) = std::str::from_utf8(input) else {
        return;
    };

    let Ok(set) = emit::span::SpanLinkSet::try_from_str(input) else {
        return;
    };

    assert_eq!(set, emit::span::SpanLinkSet::try_from_str(&set.to_string()).unwrap());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn initial() {
        emit_fuzz::initial_cases("span_link_set", de);
    }

    #[test]
    fn repro() {
        emit_fuzz::repro_cases("span_link_set", de);
    }
}
