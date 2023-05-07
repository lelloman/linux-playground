#!/bin/bash


sudo qemu-system-x86_64 \
  -kernel linux/arch/x86_64/boot/bzImage \
  -nographic \
  -drive format=raw,file=buildroot/output/images/rootfs.ext2,if=virtio \
  -append "root=/dev/vda console=ttyS0 nokaslr cgroup_no_v1=all systemd.unified_cgroup_hierachy=1" \
  -drive format=raw,file=swap1.img,if=virtio \
  -display none \
  -m 200M \
  -enable-kvm \
  -smp 4 \
  -device e1000,netdev=eth0 \
  -netdev user,id=eth0,hostfwd=tcp::5555-:22,hostfwd=udp::6666-:6667,hostfwd=tcp::3333-:4444 \
  -cpu host \
  -virtfs local,path=shared,mount_tag=host0,security_model=passthrough,id=host0 \

# mstress -j1 --hold-time-ms 100 --max-pool-percent-flip-seconds 1 --stride 4096 --bytes 120000000

#-append "root=/dev/vda console=ttyS0 nokaslr cgroup_no_v1=all systemd.unified_cgroup_hierachy=1 netconsole=4444@10.0.2.15/eth0,6665@10.0.2.2/58:11:22:2a:2e:fd"
#  -drive format=raw,file=swap1.img,if=virtio \
