#!/bin/sh -x
# preparation
#

git clone https://github.com/o-jill/ruversi.git
cd ruversi
# git checkout master
cargo run --release --features=mate1,avx,withtt -- --help

