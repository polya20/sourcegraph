#!/usr/bin/env bash

cd $(dirname "$0")

targets=(
    x86_64-pc-windows-gnu
    # x86_64-apple-darwin
    # x86_64-unknown-linux-gnu
    # aarch64-apple-darwin
    # aarch64-unknown-linux-gnu
)

# cargo build --release
# file ../../target/release/bfg

# cross build --target aarch64-unknown-linux-gnu

for target in "${targets[@]}"
do
    rustup target add $target
    cargo zigbuild --target $target --profile=release-without-debug
done
