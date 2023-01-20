#!/bin/bash

# build programs
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
cp mfiller/target/release/mfiller shared/bin
cp kmallocer-mod/kmallocer.ko shared
