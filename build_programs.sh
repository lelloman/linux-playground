#!/bin/bash

# build programs
cd nettest
cargo build --release
cd ../

# copy programs
rm programs/*
cp nettest/target/release/nettestserver programs

rm programs.img
dd if=/dev/zero of=programs.img bs=1M count=1024
mkfs.ext2 programs.img
sudo mount -o loop programs.img /mnt
sudo cp -a programs/* /mnt
chmod +x /mnt/*
sudo umount /mnt