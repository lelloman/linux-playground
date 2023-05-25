#!/bin/sh

mount -t cgroup2 cgroup2 /cgroup2

echo "+memory" > /cgroup2/cgroup.subtree_control

mkdir /cgroup2/foo
echo 10000000 > /cgroup2/foo/memory.max