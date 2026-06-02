fn main() {
    emit_fuzz::main(de);
}

pub fn de(input: &[u8]) {
    let Ok(input) = std::str::from_utf8(input) else {
        return;
    };

    let Ok(ts) = emit::Timestamp::try_from_str(input) else {
        return;
    };

    assert_eq!(ts, emit::Timestamp::try_from_str(&ts.to_string()).unwrap());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn initial() {
        emit_fuzz::initial_cases("timestamp", de);
    }

    #[test]
    fn repro() {
        emit_fuzz::repro_cases("timestamp", de);
    }
}
