use alloc::sync::Arc;
use alloc::vec::Vec;
use alloc::string::String;
use crate::fs::vfs::{Inode, ArcInode, FileStat, FileType, VfsResult, VfsError, FileHandle};

/// UnionInode merges two filesystem layers: upper (RW) and lower (RO)
#[derive(Debug)]
pub struct UnionInode {
    pub upper: Option<ArcInode>,
    pub lower: ArcInode,
}

impl crate::object::KernelObject for UnionInode {
    fn name(&self) -> alloc::string::String { alloc::format!("union-{}", self.lower.id()) }
    fn id(&self) -> usize { self.lower.id() ^ 0xF0F0F0F0 }
    fn object_type(&self) -> &'static str { "File" }

    fn snapshot(&self) -> Result<crate::object::ObjectSnapshot, &'static str> {
        Ok(crate::object::ObjectSnapshot { data: alloc::vec::Vec::new(), related_handles: alloc::vec::Vec::new() })
    }
    fn restore(&self, _snapshot: crate::object::ObjectSnapshot) -> Result<(), &'static str> {
        Ok(())
    }
}

impl UnionInode {
    pub fn new(upper: Option<ArcInode>, lower: ArcInode) -> Self {
        Self { upper, lower }
    }
}

impl Inode for UnionInode {
    fn inode_num(&self) -> u32 {
        // Return lower inode number as base, or deterministic combination
        self.lower.inode_num()
    }

    fn stat(&self) -> VfsResult<FileStat> {
        // Upper layer takes precedence for stats (e.g. size changes)
        if let Some(upper) = &self.upper {
            upper.stat()
        } else {
            self.lower.stat()
        }
    }

    fn lookup(&self, name: &str) -> VfsResult<ArcInode> {
        // 1. Check upper (private) layer
        let upper_res = if let Some(upper) = &self.upper {
            upper.lookup(name)
        } else {
            Err(VfsError::NotFound)
        };

        // 2. Check lower (base) layer
        let lower_res = self.lower.lookup(name);

        match (upper_res, lower_res) {
            (Ok(u), Ok(l)) => {
                // If both are directories, return a merged UnionInode
                let u_stat = u.stat()?;
                let l_stat = l.stat()?;
                if u_stat.file_type == FileType::Directory && l_stat.file_type == FileType::Directory {
                    Ok(Arc::new(UnionInode::new(Some(u), l)))
                } else {
                    Ok(u) // Upper shadows lower
                }
            },
            (Ok(u), _) => Ok(u),
            (_, Ok(l)) => {
                // If it's a directory in lower, we might want to return it as a UnionInode 
                // in case future files are created in the upper layer for this path.
                let l_stat = l.stat()?;
                if l_stat.file_type == FileType::Directory {
                     Ok(Arc::new(UnionInode::new(None, l)))
                } else {
                     Ok(l)
                }
            },
            (Err(e), _) => Err(e),
        }
    }

    fn open(&self, mode: u32) -> VfsResult<Arc<dyn FileHandle>> {
        // TODO: Implement Copy-on-Write (CoW) if mode includes Write and file is only in lower
        if let Some(upper) = &self.upper {
            upper.open(mode)
        } else {
            self.lower.open(mode)
        }
    }

    fn create(&self, name: &str, file_type: FileType) -> VfsResult<ArcInode> {
        if let Some(upper) = &self.upper {
            upper.create(name, file_type)
        } else {
            Err(VfsError::PermissionDenied) // Lower is Read-Only
        }
    }

    fn read_dir(&self) -> VfsResult<Vec<String>> {
        let mut entries = Vec::new();
        
        // Merge entries from both layers
        if let Some(upper) = &self.upper {
            if let Ok(u_entries) = upper.read_dir() {
                entries.extend(u_entries);
            }
        }
        
        if let Ok(l_entries) = self.lower.read_dir() {
            for e in l_entries {
                if !entries.contains(&e) {
                    entries.push(e);
                }
            }
        }
        
        Ok(entries)
    }

    fn mkdir(&self, name: &str) -> VfsResult<ArcInode> {
        if let Some(upper) = &self.upper {
            upper.mkdir(name)
        } else {
            Err(VfsError::PermissionDenied)
        }
    }

    fn unlink(&self, name: &str) -> VfsResult<()> {
        if let Some(upper) = &self.upper {
            upper.unlink(name)
        } else {
            Err(VfsError::PermissionDenied)
        }
    }

    fn remove_dir(&self, name: &str) -> VfsResult<()> {
        if let Some(upper) = &self.upper {
            upper.remove_dir(name)
        } else {
            Err(VfsError::PermissionDenied)
        }
    }

    fn rename(&self, old_name: &str, new_parent: ArcInode, new_name: &str) -> VfsResult<()> {
        Err(VfsError::NotImplemented)
    }

    fn link(&self, name: &str, inode: ArcInode) -> VfsResult<()> {
        Err(VfsError::NotImplemented)
    }

    fn chmod(&self, mode: u16) -> VfsResult<()> {
        if let Some(upper) = &self.upper {
            upper.chmod(mode)
        } else {
            Err(VfsError::PermissionDenied)
        }
    }

    fn chown(&self, uid: u16, gid: u16) -> VfsResult<()> {
        if let Some(upper) = &self.upper {
            upper.chown(uid, gid)
        } else {
            Err(VfsError::PermissionDenied)
        }
    }

    fn parent(&self) -> VfsResult<ArcInode> {
        // Step 7.2: Return parent of the primary layer (upper shadows lower)
        if let Some(upper) = &self.upper {
            upper.parent()
        } else {
            self.lower.parent()
        }
    }
}
