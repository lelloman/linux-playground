#!/bin/bash


sudo qemu-system-x86_64 \
  -kernel linux/arch/x86_64/boot/bzImage \
  -nographic \
  -drive format=raw,file=buildroot/output/images/rootfs.ext2,if=virtio \
  -drive format=raw,file=programs.img \
  -append "root=/dev/vda console=ttyS0 nokaslr" \
  -display none \
  -m 300M \
  -enable-kvm \
  -cpu host \
  -smp 2 \
  -device e1000,netdev=eth0 \
  -netdev user,id=eth0,hostfwd=tcp::5555-:22,hostfwd=udp::6666-:6667,hostfwd=tcp::3333-:4444 \


  #-append "root=/dev/vda console=ttyS0 nokaslr netconsole=+4444@10.0.2.15/eth0,6665@10.0.2.2/58:11:22:2a:2e:fd" \