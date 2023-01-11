#!/bin/bash

cd linux
make -j16
cd arch/x86_64/boot
cd ../../../../
cd buildroot
make -j16
cd ..
./run.sh
