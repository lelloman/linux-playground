#!/bin/bash

cd linux
make -j20
cd arch/x86_64/boot
cd ../../../../

cp .config-buildroot buildroot/.config
cd buildroot
make -j16
cd ..

./build_programs.sh

./run.sh
