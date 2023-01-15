#!/bin/bash

rm programs.img
dd if=/dev/zero of=programs.img bs=1M count=1024
mkfs.ext2 programs.img
sudo mount -o loop programs.img /mnt
sudo cp -a programs/* /mnt
chmod +x /mnt/*
sudo umount /mnt