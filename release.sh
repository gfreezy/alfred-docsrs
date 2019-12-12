#!/bin/bash -e
cargo build --release
cp target/release/docsrs workflow/
./package.sh workflow .
