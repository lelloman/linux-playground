#!/bin/bash

# build programs
cd nettest
cargo build --release
cd ../

cd kmallocer
cargo build --release
cd ../

# copy programs
rm -rf programs
mkdir programs
mkdir fs_overlay/programs
cp nettest/target/release/nettestserver programs
cp kmallocer/target/release/kmallocer programs
cp kmallocer/cmallocer programs

sudo rm programs.img
dd if=/dev/zero of=programs.img bs=1M count=1024
mkfs.ext2 programs.img
sudo mount -o loop programs.img /mnt
sudo cp -a programs/* /mnt
sudo chmod +x /mnt/*
sudo umount /mnt