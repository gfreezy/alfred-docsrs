#!/bin/bash -e
pwd
ls
ls workflow
if [ ! -f workflow/docsrs ]; then
    cargo build --release
    cp target/release/docsrs workflow/
fi

./package.sh workflow .
rm workflow/docsrs
