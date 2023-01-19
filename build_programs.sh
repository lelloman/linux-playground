#!/bin/bash

# build programs
cd nettest
cargo build --release
cd ../

cd kmallocer
cargo build --release
cd ../

cd mfiller
cargo build --release
cd ../

cd kmallocer-mod
make
cd ../

# setup shared/bin
rm -rf shared/bin
mkdir shared/bin

# copy programs
cp nettest/target/release/nettestserver shared/bin
cp kmallocer/target/release/kmallocer shared/bin
cp mfiller/target/release/mfiller shared/bin
cp kmallocer-mod/kmallocer.ko shared
