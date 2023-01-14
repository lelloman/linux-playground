#!/bin/bash


sudo qemu-system-x86_64 \
  -kernel linux/arch/x86_64/boot/bzImage \
  -nographic \
  -drive format=raw,file=buildroot/output/images/rootfs.ext2,if=virtio \
  -append "root=/dev/vda console=ttyS0 nokaslr netconsole=+4444@10.0.2.15/eth0,6665@10.0.2.2/58:11:22:2a:2e:fd" \
  -display none \
  -m 300M \
  -enable-kvm \
  -cpu host \
  -smp $(nproc) \
  -device e1000,netdev=eth0 \
  -netdev user,id=eth0,hostfwd=tcp::5555-:22,hostfwd=udp::6666-:6667
