#!/bin/sh

swapon /dev/vdb
swapon /dev/vdc
mount -t debugfs none /sys/kernel/debug/
