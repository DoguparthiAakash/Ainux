use alloc::string::String;
use alloc::vec::Vec;
use crate::drivers::video;
use crate::fs::vfs::FileHandle;

pub fn cmd_nuxa(args: &[&str]) {
    if args.len() < 4 || args[2] != "-o" {
        video::put_str("Usage: nuxa <source.s> -o <target.alo>\n");
        return;
    }
    
    let source_file = args[1];
    let target_file = args[3];
    
    // Load Source
    let mut source_code = String::new();
    if let Ok(inode) = crate::shell::find_inode(source_file) {
        if let Ok(handle) = inode.open(0) {
             let mut data = alloc::vec![0u8; 16384];
             if let Ok(bytes) = handle.read(&mut data, 0) {
                 source_code = String::from(core::str::from_utf8(&data[..bytes]).unwrap_or(""));
             }
        }
    } else {
        video::put_str("nuxa: Source not found.\n");
        return;
    }

    if source_code.is_empty() {
        video::put_str("nuxa: Source file is empty.\n");
        return;
    }

    video::put_str(&alloc::format!("Forwarding {} to Host LLVM Backend...\n", source_file));
    crate::apps::nuxc::compile_via_host(&source_code, target_file);
}
