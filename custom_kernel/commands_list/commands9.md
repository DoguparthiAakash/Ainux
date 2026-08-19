
| Command            | Purpose / Use Case                                 | Example                                    |
| ------------------ | -------------------------------------------------- | ------------------------------------------ |
| **cfdisk**   | Interactive disk partition manager                 | `sudo cfdisk /dev/sda`                   |
| **df**       | Shows filesystem disk space usage                  | `df -h`                                  |
| **dosfsck**  | Checks and repairs FAT filesystems (USB, SD cards) | `sudo dosfsck /dev/sdb1`                 |
| **dump**     | Backup filesystem data (low-level backup)          | `sudo dump -0u -f backup.dump /dev/sda1` |
| **dumpe2fs** | Shows ext2/ext3/ext4 filesystem metadata           | `sudo dumpe2fs /dev/sda1`                |
| **fdisk**    | Partition table editor                             | `sudo fdisk /dev/sda`                    |
| **mount**    | Mount filesystem/device                            | `sudo mount /dev/sdb1 /mnt`              |
| **restore**  | Restore data from`dump`backup                    | `restore -f backup.dump`                 |
| **sync**     | Flush buffered data to disk immediately            | `sync`                                   |
