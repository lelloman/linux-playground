#!/bin/bash

git clone git@github.com:lelloman/linux.git
cd linux
git checkout playground-5.19
cd ../
cp .config-linux linux/.config

git clone git://git.buildroot.net/buildroot
cp .config-buildroot buildroot/.config
