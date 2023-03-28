#!/bin/sh

swapon /dev/vdb
mount -t debugfs none /sys/kernel/debug/
