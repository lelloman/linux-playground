#!/bin/bash

git clone https://github.com/torvalds/linux
cd linux
git checkout v5.19
cd ../
cp .config-linux linux/.config

git clone git://git.buildroot.net/buildroot
cp .config-buildroot buildroot/.config
