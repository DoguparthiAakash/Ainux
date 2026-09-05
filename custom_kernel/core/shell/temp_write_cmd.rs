
fn cmd_write(args: &[&str]) {
    if args.len() < 3 {
        video::put_str("Usage: write <file> <content>\n");
        return;
    }
    
    let path = resolve_path(args[1]);
    let content = args[2..].join(" "); // Join rest of args
    
    let root = vfs::ROOT.lock();
    if let Some(root_inode) = root.as_ref() {
        let relative = if path.starts_with("/") { &path[1..] } else { &path };
        
        // Lookup file
        if let Ok(inode) = root_inode.lookup(relative) {
             // Open file
             if let Ok(handle) = inode.open(0) {
                 // Write content
                 match handle.write(content.as_bytes(), 0) {
                     Ok(n) => {
                         video::put_str("Written ");
                         print_digit(n as u8); // Approximate, buggy for > 255
                         video::put_str(" bytes.\n");
                     },
                     Err(_) => video::put_str("Write Failed.\n"),
                 }
             } else {
                 video::put_str("Open Failed.\n");
             }
        } else {
            video::put_str("File not found. Use 'touch' first.\n");
        }
    }
}
