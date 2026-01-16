#!/bin/bash
IMAGE=disk_btrfs.img
SIZE=64M

echo "Creating $IMAGE ($SIZE)..."
truncate -s $SIZE $IMAGE

echo "Formatting as Btrfs..."
mkfs.btrfs -f --label "AINUX_BTRFS" $IMAGE

echo "Populating..."
mkdir -p mnt
sudo mount -o loop $IMAGE mnt
sudo sh -c "echo 'Hello Btrfs World!' > mnt/hello.txt"
sudo sh -c "mkdir mnt/system"
sudo sh -c "echo 'System Config' > mnt/system/config.ini"
sudo umount mnt
rmdir mnt

echo "Done. Created $IMAGE"
