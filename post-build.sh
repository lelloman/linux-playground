#!/bin/sh

sed -i 's/#PermitEmptyPasswords no/PermitEmptyPasswords yes/g' ${TARGET_DIR}/etc/ssh/sshd_config
sed -i 's/#PermitRootLogin prohibit-password/PermitRootLogin yes/g' ${TARGET_DIR}/etc/ssh/sshd_config

sed -r -i 's/console::respawn:.*/::respawn:-\/bin\/sh/g' ${TARGET_DIR}/etc/inittab
echo "host0   /shared    9p      trans=virtio,version=9p2000.L   0 0" >> ${TARGET_DIR}/etc/fstab
echo "export PATH=\$PATH:/shared/bin" >> ${TARGET_DIR}/etc/profile