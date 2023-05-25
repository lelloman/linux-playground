#!/bin/sh

swapon /dev/vdb
swapon /dev/vdc
mount -t debugfs none /sys/kernel/debug/
echo 5 > /proc/sys/vm/nr_hugepages