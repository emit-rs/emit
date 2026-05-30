fn main() {
    emit_fuzz::main(de);
}

pub fn de(input: &[u8]) {
    // Just make sure we don't panic
    let Ok(input) = std::str::from_utf8(input) else {
        return;
    };

    let _ = emit::Timestamp::try_from_str(input);
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
