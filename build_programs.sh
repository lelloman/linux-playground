#!/bin/bash

# build programs
cd mfiller
cargo build --release
cd ../

cd kmallocer-mod
make clean all
cd ../

cd eboostctl
cargo build --release
cd ../

cd kmallocerctl
cargo build --release
cd ../

# setup shared/bin
rm -rf shared/bin
mkdir shared/bin

# copy programs
cp eboostctl/target/release/eboostctl shared/bin
cp mfiller/target/release/mfiller shared/bin
cp kmallocerctl/target/release/kmallocerctl shared/bin
cp kmallocer-mod/kmallocer.ko shared
