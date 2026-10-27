#!/usr/bin/env sh
set -eu
cd "$(dirname "$0")/.."
cargo fmt --check
cargo test --release --locked
cargo clippy --release --all-targets --locked -- -D warnings
cargo build --release --locked
printf '\nBinary: %s/target/release/geditor\n' "$(pwd)"
