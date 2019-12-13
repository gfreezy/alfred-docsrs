#!/bin/bash -e
if [ ! -f workflow/docsrs ]; then
    cargo build --release
    cp target/release/docsrs workflow/
fi

./package.sh workflow .
rm workflow/docsrs
