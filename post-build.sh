#!/bin/sh

sed -i 's/#PermitEmptyPasswords no/PermitEmptyPasswords yes/g' ${TARGET_DIR}/etc/ssh/sshd_config
sed -i 's/#PermitRootLogin prohibit-password/PermitRootLogin yes/g' ${TARGET_DIR}/etc/ssh/sshd_config

sed -r -i 's/console::respawn:.*/::respawn:-\/bin\/sh/g' ${TARGET_DIR}/etc/inittab
echo "/dev/sda  /programs   ext2    defaults   0   0" >> ${TARGET_DIR}/etc/fstab
echo "export PATH=\$PATH:/programs" >> ${TARGET_DIR}/etc/profile