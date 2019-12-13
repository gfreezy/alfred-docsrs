#!/usr/bin/env bash
set -ex
bash --version
DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
echo $DIR
cd $DIR
ls -l workflow/docsrs
test -f workflow/docsrs && echo exist

[ ! -f "$DIR/workflow/docsrs" ] && echo not exist

if [ ! -f "$DIR/workflow/docsrs" ]; then
    cargo build --release
    cp target/release/docsrs workflow/
fi

chmod +x workflow/docsrs

./package.sh workflow .
rm workflow/docsrs
