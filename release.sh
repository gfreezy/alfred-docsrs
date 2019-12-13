#!/bin/bash -e
ls -l workflow/docsrs

[ ! -f "workflow/docsrs" ] && echo not exist

if [ ! -f "workflow/docsrs" ]; then
    cargo build --release
    cp target/release/docsrs workflow/
fi

chmod +x workflow/docsrs

./package.sh workflow .
rm workflow/docsrs
