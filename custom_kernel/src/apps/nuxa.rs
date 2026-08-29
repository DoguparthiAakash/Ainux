use alloc::string::String;
use crate::drivers::video;
use crate::fs::vfs::FileType;

pub fn cmd_nuxa(args: &[&str]) {
    if args.len() < 2 {
        video::put_str("Usage: nuxa <source.s> [-o output.elf]\n");
        video::put_str("Ainux Native Assembler — AT&T syntax x86_64\n");
        return;
    }

    let source_file = args[1];
    let output_file = if args.len() >= 4 && args[2] == "-o" {
        args[3]
    } else {
        "/bin/a.elf"
    };

    // Read source
    let mut source = String::new();
    match crate::shell::find_inode(source_file) {
        Ok(inode) => {
            if let Ok(handle) = inode.open(0) {
                let mut buf = alloc::vec![0u8; 32768];
                if let Ok(n) = handle.read(&mut buf, 0) {
                    source = String::from(core::str::from_utf8(&buf[..n]).unwrap_or(""));
                }
            }
        }
        Err(_) => {
            video::put_str(&alloc::format!("nuxa: Cannot open '{}'\n", source_file));
            return;
        }
    }

    if source.is_empty() {
        video::put_str("nuxa: Source file is empty.\n");
        return;
    }

    video::put_str(&alloc::format!("[nuxa] Assembling {} ...\n", source_file));

    // Reuse nuxc's assembler
    let machine_code = crate::apps::nuxc::assemble_asm_pub(&source);
    video::put_str(&alloc::format!("[nuxa] {} bytes of machine code generated\n", machine_code.len()));

    if machine_code.is_empty() {
        video::put_str("[nuxa] Error: No code produced. Check your assembly syntax.\n");
        return;
    }

    if crate::apps::nuxc::write_elf64_pub(&machine_code, output_file) {
        video::put_str(&alloc::format!("[nuxa] Done! ELF written to '{}'\n", output_file));
    } else {
        video::put_str("[nuxa] Error: Could not write output ELF.\n");
    }
}
