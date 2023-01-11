#!/bin/sh

sed -i 's/#PermitEmptyPasswords no/PermitEmptyPasswords yes/g' ${TARGET_DIR}/etc/ssh/sshd_config
sed -i 's/#PermitRootLogin prohibit-password/PermitRootLogin yes/g' ${TARGET_DIR}/etc/ssh/sshd_config

sed -r -i 's/console::respawn:.*/::respawn:-\/bin\/sh/g' ${TARGET_DIR}/etc/inittab
