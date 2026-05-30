#!/bin/bash

# Linux only
# This is just a quick sanity check for CI. For the simple parsers we're fuzzing, crashes tend to be found _very_ quickly

cargo afl build --manifest-path fuzz/Cargo.toml --features afl

AFL_NO_UI=1 AFL_QUIET=1 timeout 30s cargo afl fuzz -i fuzz/timestamp/in -o target/fuzz_timestamp target/debug/fuzz_timestamp
