#!/bin/bash

dd if=/dev/zero of=swap.img bs=1M count=200
mkswap swap.img
tune2fs -c0 -i0 swap.img

# run on host:
# mkswap /dev/vdb && swapon /dev/vdb