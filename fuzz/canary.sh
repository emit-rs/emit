#!/bin/bash
set -e

# Linux only
# This is just a quick sanity check for CI. For the simple parsers we're fuzzing, crashes tend to be found _very_ quickly

cargo test --manifest-path fuzz/Cargo.toml
cargo afl build --manifest-path fuzz/Cargo.toml --features afl

# Add new fuzz cases here
AFL_NO_UI=1 timeout 17s cargo afl fuzz -i fuzz/timestamp/in -o fuzz/target/timestamp fuzz/target/debug/timestamp > /dev/null 2>&1 &

echo "waiting for fuzz run..."
sleep 20s

cargo test --manifest-path fuzz/Cargo.toml --features force
