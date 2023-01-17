#!/bin/bash

cd linux
make -j22
cd ../

cp .config-buildroot buildroot/.config
cd buildroot
make -j22
cd ..

./build_programs.sh

./run.sh
