#!/bin/bash

# build programs
cd mfiller
cargo build --release
cd ../

cd kmallocer/mod
make clean all
cd ../kmallocerctl
cargo build --release
cd ../../

cd eboostctl
cargo build --release
cd ../

cd psipoll
cargo build --release
gcc main.c -o cpsipoll
cd ../

# setup shared/bin
rm -rf shared/bin
mkdir shared/bin

# copy programs
cp eboostctl/target/release/eboostctl shared/bin
cp mfiller/target/release/mfiller shared/bin
cp kmallocer/kmallocerctl/target/release/kmallocerctl shared/bin
cp psipoll/target/release/psipoll shared/bin
cp psipoll/cpsipoll shared/bin
cp config_cgroup2.sh shared/bin
