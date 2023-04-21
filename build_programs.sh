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

cd mallocator
cargo build --release
cd ../

cd mstress
cargo build --release
cd ../

# setup shared/bin
rm -rf shared/bin
mkdir shared/bin

# copy programs
cp mstress/target/release/mstress shared/bin
cp eboostctl/target/release/eboostctl shared/bin
cp mfiller/target/release/mfiller shared/bin
cp mallocator/target/release/mallocator shared/bin
cp kmallocer/kmallocerctl/target/release/kmallocerctl shared/bin
cp psipoll/target/release/psipoll shared/bin
cp psipoll/cpsipoll shared/bin
cp config_cgroup2.sh shared/bin
cp init.sh shared/bin
