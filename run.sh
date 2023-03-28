#!/bin/bash


sudo qemu-system-x86_64 \
  -kernel linux/arch/x86_64/boot/bzImage \
  -nographic \
  -drive format=raw,file=buildroot/output/images/rootfs.ext2,if=virtio \
  -append "root=/dev/vda console=ttyS0 nokaslr cgroup_no_v1=all systemd.unified_cgroup_hierachy=1" \
  -drive format=raw,file=swap.img,if=virtio \
  -display none \
  -m 1G \
  -enable-kvm \
  -smp 4 \
  -device e1000,netdev=eth0 \
  -netdev user,id=eth0,hostfwd=tcp::5555-:22,hostfwd=udp::6666-:6667,hostfwd=tcp::3333-:4444 \
  -cpu host \
  -virtfs local,path=shared,mount_tag=host0,security_model=passthrough,id=host0 \
