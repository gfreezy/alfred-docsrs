#!/usr/bin/env bash
set -ex

if [ ! -f "$DIR/workflow/docsrs" ]; then
    cargo build --release
    cp target/release/docsrs workflow/
fi

chmod +x workflow/docsrs

./package.sh workflow .
rm workflow/docsrs
