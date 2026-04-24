# Virtual File System (VFS) and Pipes

The VFS layer provides a unified interface for all storage and communication streams in Ainux.

## 1. Core Abstractions
The VFS is built on two primary traits defined in `src/fs/vfs.rs`:
*   **Inode**: Represents a file or directory on disk (stat, lookup, read_dir).
*   **FileHandle**: Represents an open instance of a file (read, write, seek, truncate).

## 2. Supported Filesystems
*   **RootFS (Initrd)**: A basic in-memory filesystem used during boot.
*   **Ext4**: Industrial-grade storage support for persistent data.
*   **Console**: A special filesystem mapping `/dev/console` to the video and keyboard drivers.

## 3. Anonymous Pipes
Pipes are implemented as synchronized circular buffers in `src/fs/pipe.rs`.

*   **Mechanism**: A `Pipe` structure holds a `VecDeque<u8>` protected by a spinlock.
*   **Handles**: `PipeReader` and `PipeWriter` implement the `FileHandle` trait, allowing them to be used with standard `read` and `write` syscalls.
*   **Usage**: The shell uses `sys_pipe` (ID 11) to connect command output to command input (e.g., `ls | grep`).

## 4. Path Resolution
Paths are resolved using a recursive `walk` algorithm that starts at the root (`/`) or the current working directory (`CWD`). It respects symbolic links (planned) and directory permissions.
