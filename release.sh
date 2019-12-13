#!/bin/bash -ex
pwd
ls -l
ls -l workflow
if [ ! -f "workflow/docsrs" ]; then
    cargo build --release
    cp target/release/docsrs workflow/
fi

chmod +x workflow/docsrs

./package.sh workflow .
rm workflow/docsrs
