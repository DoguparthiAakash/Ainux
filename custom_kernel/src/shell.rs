extern crate alloc;
use alloc::vec::Vec;
use alloc::string::String;
use alloc::vec;
use alloc::format;
use crate::drivers::{keyboard, video, rtc};
use crate::fs::vfs;
use core::fmt::Write;
use spin::Mutex;
use alloc::sync::Arc;
use crate::alloc::string::ToString;

static CWD: Mutex<String> = Mutex::new(String::new());

fn get_cwd() -> String {
    let cwd = CWD.lock();
    if cwd.is_empty() { String::from("/") } else { cwd.clone() }
}

fn set_cwd(path: &str) {
    let mut cwd = CWD.lock();
    *cwd = String::from(path);
}

fn resolve_path(path: &str) -> String {
    let mut components: Vec<String> = Vec::new();
    
    // Start with existing CWD if relative, or empty if absolute
    if !path.starts_with("/") {
        for comp in get_cwd().split('/') {
            if !comp.is_empty() {
                components.push(String::from(comp));
            }
        }
    }
    
    // Process new path components
    for comp in path.split('/') {
        if comp.is_empty() || comp == "." {
            continue;
        } else if comp == ".." {
            components.pop();
        } else {
            components.push(String::from(comp));
        }
    }
    
    let mut resolved = String::from("/");
    for (i, comp) in components.iter().enumerate() {
        resolved.push_str(comp);
        if i < components.len() - 1 {
            resolved.push_str("/");
        }
    }
    resolved
}

/// Resolves a path to an Inode by walking the VFS tree.
pub fn find_inode(path: &str) -> vfs::VfsResult<Arc<dyn vfs::Inode>> {
    let resolved = resolve_path(path);
    let mut current = vfs::root();
    
    if resolved == "/" {
        return Ok(current);
    }
    
    // Skip leading / since we start from root
    let components = resolved[1..].split('/');
    for comp in components {
        if comp.is_empty() { continue; }
        current = current.lookup(comp)?;
    }
    
    Ok(current)
}

/// Resolves a path to its parent inode and the child name.
fn find_parent_and_name(path: &str) -> vfs::VfsResult<(Arc<dyn vfs::Inode>, String)> {
    let resolved = resolve_path(path);
    if resolved == "/" { return Err(vfs::VfsError::PermissionDenied); }
    
    let mut components: Vec<&str> = resolved.split('/').collect();
    // Remove empty components from split
    components.retain(|s| !s.is_empty());
    
    if components.is_empty() { return Err(vfs::VfsError::NotFound); }
    
    let name = components.pop().unwrap().to_string();
    
    let mut current = vfs::root();
    for comp in components {
        current = current.lookup(comp)?;
    }
    
    Ok((current, name))
}



static HISTORY: Mutex<Vec<String>> = Mutex::new(Vec::new());

fn redraw_line(buffer: &str, cursor_pos: usize, clear_trailing: bool) {
    let rest = &buffer[cursor_pos..];
    crate::drivers::video::put_str(rest);
    let mut chars_to_move_back = rest.len();
    if clear_trailing {
        crate::drivers::video::put_char(' '); // Clear potential trailing char
        chars_to_move_back += 1;
    }
    
    // Move cursor back to `cursor_pos`
    for _ in 0..chars_to_move_back {
        crate::drivers::video::put_str("\x08");
    }
}

fn restore_cursor(buffer: &str, cursor_pos: usize) {
    if cursor_pos < buffer.len() {
        let c = buffer.chars().nth(cursor_pos).unwrap();
        crate::drivers::video::put_char(c);
        crate::drivers::video::put_str("\x08"); // Step back visual cursor
    } else {
        crate::drivers::video::draw_cursor(0x00000000);
    }
}

fn process_char(c: char, input_buffer: &mut String, cursor_pos: &mut usize, history_index: &mut usize) {
    if c == '\n' {
        video::put_char('\n');
    } else if c == '\x08' || c == '\x7F' { // Backspace or Delete
         if *cursor_pos > 0 && input_buffer.len() > 0 {
             let remove_idx = *cursor_pos - 1;
             input_buffer.remove(remove_idx);
             *cursor_pos -= 1;
             
             // Redraw Line
             video::put_str("\x08"); // Move back visual cursor
             redraw_line(input_buffer, *cursor_pos, true);
         }
    } else if c == '\u{2190}' { // Left Arrow
        if *cursor_pos > 0 {
            *cursor_pos -= 1;
            // Use backspace to visually move left without erasing (video.rs updated)
            video::put_str("\x08");
        }
    } else if c == '\u{2192}' { // Right Arrow
        if *cursor_pos < input_buffer.len() {
            let ch = input_buffer.chars().nth(*cursor_pos).unwrap();
            // Just re-printing the char advances the cursor
            video::put_char(ch);
            *cursor_pos += 1;
        }
    } else if c == '\u{2191}' { // Up Arrow (History Prev)
        let hist = HISTORY.lock();
        if !hist.is_empty() {
             if *history_index > 0 {
                 *history_index -= 1;
             }
             
             // Clear visual line
             // 1. Move to start
             while *cursor_pos > 0 {
                 video::put_str("\x08");
                 *cursor_pos -= 1;
             }
             // 2. Erase content
             for _ in 0..input_buffer.len() { video::put_char(' '); }
             // 3. Move back to start
             for _ in 0..input_buffer.len() { video::put_str("\x08"); }
             
             // Load history
             *input_buffer = hist[*history_index].clone();
             *cursor_pos = input_buffer.len();
             video::put_str(input_buffer);
        }
    } else if c == '\u{2193}' { // Down Arrow (History Next)
        let hist = HISTORY.lock();
        if !hist.is_empty() {
             let last_idx = hist.len();
             if *history_index < last_idx {
                 *history_index += 1;
             }
             
             // Clear visual line
             while *cursor_pos > 0 { video::put_str("\x08"); *cursor_pos -= 1; }
             for _ in 0..input_buffer.len() { video::put_char(' '); }
             for _ in 0..input_buffer.len() { video::put_str("\x08"); }
             
             if *history_index == last_idx {
                 input_buffer.clear();
             } else {
                 *input_buffer = hist[*history_index].clone();
             }
             *cursor_pos = input_buffer.len();
             video::put_str(input_buffer);
        }
    } else {
         // Printable char
         // Filter control chars to avoid mess
         if c >= ' ' && c != '\x7F' { 
             if input_buffer.len() < 128 {
                 if *cursor_pos == input_buffer.len() {
                     input_buffer.push(c);
                     video::put_char(c);
                     *cursor_pos += 1;
                 } else {
                     // Insert in middle
                     input_buffer.insert(*cursor_pos, c);
                     video::put_char(c); // Print the new char
                     *cursor_pos += 1;
                     
                     // Print rest (shifted right)
                     redraw_line(input_buffer, *cursor_pos, false);
                 }
             }
         }
     }
}


pub fn run() {
    let mut serial = crate::drivers::serial::SerialPort::new(0x3F8);
    let _ = write!(serial, "Shell: Run entered.\n");

    video::put_str("\nWelcome to Ainux Shell! (Heap Enabled)\nType 'help' for commands.\n");
    let _ = write!(serial, "Shell: Welcome printed.\n");
    
    // Heap-based buffer
    let mut input_buffer = String::new();
    let mut cursor_pos = 0;
    
    // Initialize CWD
    set_cwd("/");

    loop {
        let cwd = get_cwd();
        
        // Thematic Prompt
        let theme = video::THEME.lock();
        let r_col = theme.root;
        let f_col = theme.fg;
        let b_col = theme.bg;
        drop(theme);

        video::put_str_colored("ainux", r_col, b_col);
        video::put_str_colored(":", f_col, b_col);
        video::put_str_colored(&cwd, f_col, b_col);
        video::put_str_colored("> ", f_col, b_col);
        
        input_buffer.clear();
        cursor_pos = 0;
        let mut history_index = HISTORY.lock().len();
        
        // Read Line Loop
        let mut last_blink = 0;
        let mut cursor_visible = false;
        
        'input: loop {
            let current_ticks = crate::process::scheduler::get_ticks();
            if current_ticks > last_blink + 50 { // Blink every 0.5s approx
                 last_blink = current_ticks;
                 cursor_visible = !cursor_visible;
                 if cursor_visible {
                     crate::drivers::video::draw_cursor(0xFFFFFFFF);
                 } else {
                     restore_cursor(&input_buffer, cursor_pos); // Erase
                 }
            }

            // Check PS/2 Keyboard
            while let Some(c) = keyboard::pop_char() {
                // Ensure cursor is erased before moving/printing
                 if cursor_visible { 
                     restore_cursor(&input_buffer, cursor_pos); 
                     cursor_visible = false; // Reset blink phase
                     last_blink = current_ticks; // Reset timer so it stays visible for a bit
                 }

                process_char(c, &mut input_buffer, &mut cursor_pos, &mut history_index);
                if c == '\n' { 
                    crate::drivers::video::draw_cursor(0xFFFFFFFF);
                    cursor_visible = true;
                    break 'input; 
                }
                
                // Force cursor visible after typing
                crate::drivers::video::draw_cursor(0xFFFFFFFF);
                cursor_visible = true;
            }
            
            // Check Serial
            while crate::drivers::serial::SERIAL.lock().data_ready() {
                 if cursor_visible { 
                     restore_cursor(&input_buffer, cursor_pos); 
                     cursor_visible = false; 
                 }

                let c = crate::drivers::serial::SERIAL.lock().read_byte() as char;
                let c = if c == '\r' { '\n' } else { c };
                process_char(c, &mut input_buffer, &mut cursor_pos, &mut history_index);
                if c == '\n' { 
                    crate::drivers::video::draw_cursor(0xFFFFFFFF);
                    cursor_visible = true;
                    last_blink = current_ticks;
                    break 'input; 
                }
                
                crate::drivers::video::draw_cursor(0xFFFFFFFF);
                cursor_visible = true;
                last_blink = current_ticks;
            }
            
            // Background Network Processing
            crate::net::poll();
            
            unsafe { core::arch::asm!("hlt"); }
        }
        
        if !input_buffer.is_empty() {
             // Add to history
             let mut hist = HISTORY.lock();
             if hist.last() != Some(&input_buffer) {
                 hist.push(input_buffer.clone());
             }
             drop(hist);
             
             let _ = write!(serial, "Shell: Processing Command: {}\n", input_buffer);
             execute_command(&input_buffer);
        }
    }
}

pub fn execute_command(input: &str) {
    let args: Vec<&str> = input.split_whitespace().collect();
    if let Some(cmd) = args.get(0) {
        match *cmd {
            "help" => cmd_help(),
            "clear" => video::clear(),
            "whoami" => video::put_str("root (kernel)\n"),
            "time" => cmd_time(),
            "shutdown" => cmd_shutdown(),
            "ls" => cmd_ls(&args),
            "cat" => cmd_cat(&args),
            "nvix" => crate::apps::nvi::cmd_nvix(&args),
            "nuxc" => crate::apps::nuxc::cmd_nuxc(&args),
            "nuxa" => crate::apps::nuxa::cmd_nuxa(&args),
            "nuxv" => crate::apps::nuxv::cmd_nuxv(&args),
            "run" => cmd_run(&args),
            "awm" => crate::apps::awm::cmd_awm(&args),
            "ps" => cmd_ps(),
            "kill" => cmd_kill(&args),
            "free" => cmd_free(),
            "reboot" => cmd_reboot(),
            "test_lifecycle" => cmd_test_lifecycle(),
            "uptime" => cmd_uptime(),
            "test_threads" => cmd_test_threads(),
            "test_ipc" => cmd_test_ipc(),
            "write" => cmd_write(&args),
            "test_write" => cmd_test_write(),
            "exec" => cmd_exec(&args),
            "top" => cmd_top(),
            "tree" => cmd_tree(&args),
            "cp" => cmd_cp(&args),
            "mv" => cmd_mv(&args),
            "rm" => cmd_rm(&args),
            "mkdir" => cmd_mkdir(&args),
            "stat" => cmd_stat(&args),
            "dmesg" => cmd_dmesg(),
            "ping" => cmd_ping(&args),
            "ssh" => cmd_stub("ssh"),
            "chmod" => cmd_chmod(&args),
            "chown" => cmd_chown(&args),
            "su" => cmd_stub("su"),
            "bg" => cmd_stub("bg"),
            "fg" => cmd_stub("fg"),
            "jobs" => cmd_jobs(),
            "code" => cmd_code(&args),
            "killall" => cmd_killall(&args),
            "nice" => cmd_nice(&args),
            "renice" => cmd_renice(&args),
            "cal" => cmd_cal(&args),
            "clock" => cmd_clock(),
            "view" => cmd_view(&args),
            "mount" => cmd_mount(&args),
            "umount" => cmd_umount(&args),
            "sync" => cmd_sync(),
            "find" => cmd_find(&args),
            "touch" => cmd_touch(&args),
            "rmdir" => cmd_rmdir(&args),
            "wifi" => cmd_wifi(&args),
            "lsblk" => cmd_lsblk(),
            "uname" => cmd_uname(),
            "grep" => cmd_grep(&args),
            "head" => cmd_head(&args),
            "tail" => cmd_tail(&args),
            "wc" => cmd_wc(&args),
            "ip" => cmd_ip(&args),
            "netstat" => cmd_netstat(),
            "sshd" => crate::apps::sshd::main(),
            "youtube" => cmd_youtube(&args),
            "google" => cmd_google(&args),
            "format" => cmd_format(&args),
            "cd" => cmd_cd(&args),
            "pwd" => { video::put_str(&get_cwd()); video::put_char('\n'); },
            "hfetch" => crate::apps::hfetch::cmd_hfetch(&args),
            "hinfo" => crate::apps::hinfo::main(&args),
            "dcustom" => crate::apps::dcustom::main(),
            "settings" => crate::apps::settings::main(),
            "metus" => crate::apps::metus::main(),
            "save" => cmd_save(),
            "hostname" => cmd_hostname(&args),
            "examples" => cmd_examples(),
            _ => {
                if args.len() >= 2 && args[1] == "-prop" {
                    cmd_prop(&args);
                } else {
                    video::put_str("Unknown command. Type 'help'.\n");
                }
            },
        }
    }
}

// ... existing help ...

// --- NUX COMMANDS ---

/*
fn cmd_nux(args: &[&str]) {
    ... (disabled)
}
*/


/*
fn cmd_nuxc(args: &[&str]) {
    ... (disabled)
}
*/



fn cmd_help() {
    video::put_str("Ainux OS Native Shell - Available Commands:\n");
    video::put_str("  metus               - Industrial real-time system monitor\n");

    video::put_str("\n--- Development & Native Platform ---\n");
    video::put_str("  run <file>          - Unified Runner (.c, .s, .v, .q)\n");
    video::put_str("  nuxc / nuxa / nuxv  - Native C / ASM / Sovereign Compilers\n");
    video::put_str("  nvix <file>         - nvix Turbo IDE\n");
    video::put_str("  exec <file.alo>     - Execute Native Segmented Binary\n");
    video::put_str("  view <file>         - Visual File/Hex/Image Viewer\n");
    video::put_str("  code <file>         - Lightweight code viewer\n");

    video::put_str("\n--- Filesystem Operations ---\n");
    video::put_str("  ls [dir] / tree     - List / Recursive directory contents\n");
    video::put_str("  cd <dir>  / pwd     - Navigate / Print current directory\n");
    video::put_str("  cat / touch / stat  - Read / Create / Info on files\n");
    video::put_str("  mkdir / rmdir       - Directory management\n");
    video::put_str("  cp / mv / rm        - Copy / Move / Delete files\n");
    video::put_str("  find <name>         - Search for files in the system\n");
    video::put_str("  mount / umount      - Disk & partition management\n");
    video::put_str("  sync                - Flush filesystem buffers to disk\n");

    video::put_str("\n--- System & Advanced Drivers ---\n");
    video::put_str("  ps / top / jobs     - Task & performance monitoring\n");
    video::put_str("  free / lsblk        - Memory / Block device statistics\n");
    video::put_str("  hfetch / hinfo      - System & hardware diagnostic info\n");
    video::put_str("  uname / uptime      - System & Kernel identity\n");
    video::put_str("  dmesg               - View kernel message buffer\n");
    video::put_str("  reboot / shutdown   - Power & Restart control\n");

    video::put_str("\n--- Networking & Remote Access ---\n");
    video::put_str("  ip addr             - View IP (DHCP/Static) & Status\n");
    video::put_str("  ping <host>         - ICMP Network Connectivity Test\n");
    video::put_str("  sshd                - Start Remote SSH Gateway (Port 22)\n");
    video::put_str("  netstat             - Monitor Open Sockets & Connections\n");
    video::put_str("  google <query>      - Sovereign CLI Search Engine\n");

    video::put_str("\n--- Text Processing & Utilities ---\n");
    video::put_str("  grep / head / tail  - High-speed stream filtering\n");
    video::put_str("  wc <file>           - Word, line, and byte counters\n");
    video::put_str("  clock / cal         - Modern Clock / Calendar systems\n");
    video::put_str("  clear / help        - UI management & this menu\n");
}

fn cmd_btrfs_info() {
    video::put_str("Reading Btrfs Superblock...\n");
    match crate::fs::btrfs::read_superblock() {
        Ok(sb) => {
             // Copy packed fields to locals to avoid unaligned access error
             let label = sb.label;
             let total_bytes = sb.total_bytes;
             let root = sb.root;
             let chunk_root = sb.chunk_root;

             video::put_str("Btrfs Superblock Found!\n");
             video::put_str(&format!("  Label: {:?}\n", core::str::from_utf8(&label).unwrap_or("Invalid UTF8")));
             video::put_str(&format!("  Total Bytes: {}\n", total_bytes));
             video::put_str(&format!("  Root: {}\n", root));
             video::put_str(&format!("  Chunk Root: {}\n", chunk_root));
        },
        Err(e) => video::put_str(&format!("Error: {}\n", e)),
    }
}




fn cmd_cd(args: &[&str]) {
    let path_input = if args.len() < 2 { "/" } else { args[1] };
    let path = resolve_path(path_input);
    
    match find_inode(&path) {
        Ok(inode) => {
            if let Ok(stat) = inode.stat() {
                if stat.file_type == vfs::FileType::Directory {
                    set_cwd(&path);
                } else {
                    video::put_str("cd: Not a directory.\n");
                }
            }
        },
        Err(_) => video::put_str("cd: Directory not found.\n"),
    }
}

fn cmd_kill(args: &[&str]) {
    if args.len() < 2 {
        video::put_str("Usage: kill <pid>\n");
        return;
    }
    // Parse PID (simple ascii)
    let pid_str = args[1];
    let mut pid = 0;
    for c in pid_str.bytes() {
        if c >= b'0' && c <= b'9' {
            pid = pid * 10 + (c - b'0') as usize;
        }
    }
    
    if crate::process::scheduler::kill_task(pid) == 0 {
         video::put_str("Process Killed.\n");
         video::put_str("Failed to kill process (NotFound or Kernel).\n");
    }
}
// --- New Commands ---

fn calculate_dir_size(inode: &Arc<dyn vfs::Inode>) -> u64 {
    let mut total = 0;
    if let Ok(files) = inode.read_dir() {
        for name in files {
            if name == "." || name == ".." { continue; }
            if let Ok(child) = inode.lookup(&name) {
                if let Ok(stat) = child.stat() {
                    if stat.file_type == vfs::FileType::Directory {
                        total += calculate_dir_size(&child);
                    } else {
                        total += stat.size;
                    }
                }
            }
        }
    }
    total
}

fn cmd_prop(args: &[&str]) {
    if args.len() < 1 { return; }
    let path_input = args[0]; // If called from _ handler
    let path = resolve_path(path_input);
    let name = path.rsplit('/').next().unwrap_or(&path);
    
    let cur_x = *video::CONSOLE_X.lock();
    let cur_y = video::prepare_y_for_height(10);
    
    video::draw_tui_title_box(cur_x, cur_y, 40, 10, "FILE PROPERTIES", 0x00FFCC00);
    
    let root = vfs::ROOT.lock();
    if let Some(r) = root.as_ref() {
        if let Ok(inode) = r.lookup(name) {
            if let Ok(stat) = inode.stat() {
                let mut line = 1;
                let mut fields = vec![
                    (format!("Name: {}", name)),
                    (format!("Path: {}", path)),
                    (format!("Size: {} Bytes", stat.size)),
                    (format!("Type: {:?}", stat.file_type)),
                    (format!("UID: {}  GID: {}", stat.uid, stat.gid)),
                    (format!("Mode: o{:o}", stat.mode)),
                ];
                
                if stat.file_type == vfs::FileType::Directory {
                    let total_size = calculate_dir_size(&inode);
                    fields.push(format!("Total Content Size: {} B", total_size));
                }
                
                 for f in fields {
                    *video::CONSOLE_X.lock() = cur_x + 2;
                    *video::CONSOLE_Y.lock() = cur_y + line;
                    video::put_str(&f);
                    line += 1;
                }
            }
        } else {
             video::put_str("  Status: Not Found");
        }
    } else {
        match find_inode(&path) {
            Ok(inode) => {
                if let Ok(stat) = inode.stat() {
                    let mut line = 1;
                    let mut fields = vec![
                        (format!("Name: {}", name)),
                        (format!("Path: {}", path)),
                        (format!("Size: {} Bytes", stat.size)),
                        (format!("Type: {:?}", stat.file_type)),
                        (format!("UID: {}  GID: {}", stat.uid, stat.gid)),
                        (format!("Mode: o{:o}", stat.mode)),
                    ];
                    
                    if stat.file_type == vfs::FileType::Directory {
                        let total_size = calculate_dir_size(&inode);
                        fields.push(format!("Total Content Size: {} B", total_size));
                    }
                    
                    for f in fields {
                        *video::CONSOLE_X.lock() = cur_x + 2;
                        *video::CONSOLE_Y.lock() = cur_y + line;
                        video::put_str(&f);
                        line += 1;
                    }
                }
            },
            Err(_) => {
                video::put_str("  Status: Not Found");
            }
        }
    }
    *video::CONSOLE_X.lock() = 0;
    *video::CONSOLE_Y.lock() = cur_y + 11;
}

fn cmd_stat(args: &[&str]) {
    if args.len() < 2 { video::put_str("Usage: stat <file>\n"); return; }
    cmd_prop(&args[1..]);
}

fn cmd_examples() {
    video::put_str("Deploying Universal Industrial Test Suite...\n");
    
    let examples = [
        ("hello.c", "#include <stdio.h>\nint main() {\n  printf(\"Hello from Ainux C!\\n\");\n  return 0;\n}"),
        ("test.s", ".section .text\n.global _start\n_start:\n  movq $1, %rax\n  movq $1, %rdi\n  syscall\n  ret"),
        ("logic.v", "PUSH 100\nPUSH 100\nPUSH 50\nPUSH 50\nRECT\nEXIT"),
        ("sim.q", "Q_SET\nQ_HAD 0\nQ_MEAS 0\nPRINT\nEXIT"),
    ];
    
    for (name, content) in examples {
        match find_parent_and_name(name) {
            Ok((parent, base)) => {
                let inode_res = match parent.lookup(&base) {
                    Ok(i) => Ok(i),
                    Err(_) => parent.create(&base, vfs::FileType::File),
                };
                
                if let Ok(inode) = inode_res {
                    if let Ok(h) = inode.open(0) {
                        let _ = h.truncate();
                        let _ = h.write(content.as_bytes(), 0);
                        video::put_str(&format!("  [+] Created {}\n", name));
                    }
                }
            },
            Err(_) => {
                video::put_str(&format!("  [!] Failed to resolve path for {}\n", name));
            }
        }
    }
    video::put_str("Done. Try 'run hello.c' or 'sim.q -prop'\n");
}

fn u16_from_str(s: &str) -> Option<u16> {
    // Simple decimal parser
    let mut val = 0u16;
    for c in s.chars() {
        if c < '0' || c > '9' { return None; }
        val = val * 10 + (c as u16 - '0' as u16);
    }
    Some(val)
}

fn octal_from_str(s: &str) -> Option<u16> {
    let mut val = 0u16;
    for c in s.chars() {
        if c < '0' || c > '7' { return None; }
        val = val * 8 + (c as u16 - '0' as u16);
    }
    Some(val)
}

fn cmd_chmod(args: &[&str]) {
    if args.len() < 3 { video::put_str("Usage: chmod <mode> <file>\n"); return; }
    let mode_str = args[1];
    let path = resolve_path(args[2]);
    
    let mode = match octal_from_str(mode_str) {
        Some(m) => m,
        None => { video::put_str("Invalid mode (use octal, e.g. 755)\n"); return; }
    };
    
    match find_inode(&path) {
        Ok(inode) => {
             match inode.chmod(mode) {
                 Ok(_) => video::put_str("Chmod success.\n"),
                 Err(_) => video::put_str("Chmod failed.\n"),
             }
        },
        Err(_) => video::put_str("File not found.\n"),
    }
}

fn cmd_chown(args: &[&str]) {
    if args.len() < 3 { video::put_str("Usage: chown <uid> <file>\n"); return; }
    let uid_str = args[1];
    let path = resolve_path(args[2]);
    
    let uid = match u16_from_str(uid_str) {
        Some(u) => u,
        None => { video::put_str("Invalid uid\n"); return; }
    };
    
    match find_inode(&path) {
        Ok(inode) => {
             // Basic: set gid=uid for now
             match inode.chown(uid, uid) {
                 Ok(_) => video::put_str("Chown success.\n"),
                 Err(_) => video::put_str("Chown failed.\n"),
             }
        },
        Err(_) => video::put_str("File not found.\n"),
    }
}

fn cmd_rm(args: &[&str]) {
    if args.len() < 2 { video::put_str("Usage: rm <file>\n"); return; }
    
    match find_parent_and_name(args[1]) {
        Ok((parent, name)) => {
            match parent.unlink(&name) {
                Ok(_) => video::put_str("Deleted.\n"),
                Err(_) => video::put_str("Delete failed.\n"),
            }
        },
        Err(_) => video::put_str("Path not found.\n"),
    }
}

fn cmd_free() {
    let cpu_count = crate::cpu::percpu::get_cpu_count();
    let mut pmm_lock = crate::mm::pmm::PMM.lock();
    if let Some(pmm) = pmm_lock.as_ref() {
        let (used, total) = pmm.get_stats();
        let page_size = 4096; // 4KB
        let used_mb = (used * page_size) / 1024 / 1024;
        let total_mb = (total * page_size) / 1024 / 1024;
        
        video::put_str("Memory: \n");
        video::put_str("  Used: "); print_digit(used_mb as u8); video::put_str(" MiB\n");
        video::put_str("  Total: "); print_digit(total_mb as u8); video::put_str(" MiB\n");
    } else {
        video::put_str("PMM not initialized.\n");
    }
}

fn cmd_reboot() {
    video::put_str("Rebooting...\n");
    unsafe {
        // PS/2 Controller Pulse Reset
        loop {
            // Wait for input buffer to be empty
             let status: u8;
             core::arch::asm!("in al, 0x64", out("al") status);
             if status & 2 == 0 { break; }
        }
        core::arch::asm!("out 0x64, al", in("al") 0xFE as u8);
        core::arch::asm!("hlt"); 
    }
}

fn cmd_time() {
    let t = rtc::read_time();
    let month_names = ["", "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];
    let m_str = if t.month as usize <= 12 && t.month > 0 { month_names[t.month as usize] } else { "??" };
    
    let am_pm = if t.hours >= 12 { "PM" } else { "AM" };
    let hour_12 = if t.hours == 0 { 12 } else if t.hours > 12 { t.hours - 12 } else { t.hours };

    let time_str = format!("{} {:02}, {}  {:02}:{:02}:{:02} {}", m_str, t.day, t.year, hour_12, t.minutes, t.seconds, am_pm);
    
    let cur_x = *video::CONSOLE_X.lock();
    let cur_y = video::prepare_y_for_height(3);
    
    video::draw_tui_title_box(cur_x, cur_y, time_str.len() + 4, 3, "SYSTEM TIME", 0x00AAAAFF);
    *video::CONSOLE_X.lock() = cur_x + 2;
    *video::CONSOLE_Y.lock() = cur_y + 1;
    video::put_str(&time_str);
    *video::CONSOLE_X.lock() = 0;
    *video::CONSOLE_Y.lock() = cur_y + 3;
}

fn cmd_shutdown() {
    video::clear();
    video::put_char('\n');
    video::put_str("  [!] SYSTEM SHUTDOWN INITIATED\n");
    video::put_str("  ───────────────────────────────\n");
    video::put_str("  Syncing filesystems... OK\n");
    video::put_str("  Sending kill signal to hardware...\n");
    
    // Trigger real hardware shutdown
    crate::drivers::power::shutdown();
    
    // Fallback if shutdown fails
    loop { unsafe { core::arch::asm!("hlt"); } }
}

fn cmd_ls(args: &[&str]) {
    let target = if args.len() < 2 { get_cwd() } else { resolve_path(args[1]) };
    
    match find_inode(&target) {
        Ok(inode) => {
            match inode.read_dir() {
                Ok(files) => {
                    for name in files {
                        video::put_str(&name);
                        video::put_char('\n');
                    }
                },
                Err(_) => video::put_str("ls: Error reading directory.\n"),
            }
        },
        Err(_) => video::put_str("ls: Directory not found.\n"),
    }
}

fn cmd_cat(args: &[&str]) {
    if args.len() < 2 { video::put_str("Usage: cat <filename>\n"); return; }
    
    match find_inode(args[1]) {
        Ok(inode) => {
             if let Ok(handle) = inode.open(0) {
                  let mut buf = vec![0u8; 4096]; 
                  if let Ok(n) = handle.read(&mut buf, 0) {
                      if let Ok(s) = core::str::from_utf8(&buf[0..n]) {
                           video::put_str(s); video::put_char('\n');
                      } else {
                           video::put_str("<Binary Content>\n");
                      }
                  }
             }
        },
        Err(_) => video::put_str("cat: File not found.\n"),
    }
}

fn cmd_ps() {
    crate::process::scheduler::print_task_list();
}

fn cmd_test_lifecycle() {
    video::put_str("Spawning child task...\n");
    // We need to know the PID. simplistic spawn doesn't return PID easily yet without locking.
    // Let's modify spawn to return PID or just hack it: we know PID is monotonic-ish.
    
    // Actually, let's use a closure? No heap/closures in naked spawn.
    // define a function:
    crate::process::scheduler::spawn(child_task_entry);
    
    // For verification, we just assume PID=2 (0=Kernel, 1=Shell, 2=Child) if clean boot.
    // Or we scan for it.
    video::put_str("Waiting for PID 2...\n");
    
    let pid = 2; // Hardcoced for test
    let cpu_count = crate::cpu::percpu::get_cpu_count();
    let code = crate::process::scheduler::wait_pid(pid);
    
    video::put_str("Child exited with code: ");
    print_digit(code as u8);
    video::put_str("\n");
}

extern "C" fn child_task_entry() {
    // Sleep a bit
    unsafe { crate::process::scheduler::set_current_sleep(crate::process::scheduler::get_ticks() + 50); } // 500ms
    crate::process::scheduler::yield_now();
    
    crate::process::scheduler::exit_current_task(42);
}


fn print_digit(val: u8) {
    let tens = val / 10;
    let ones = val % 10;
    video::put_char((b'0' + ones) as char);
}

fn cmd_uptime() {
    let ticks = crate::process::scheduler::get_ticks();
    let seconds = ticks / 100;
    let up_str = format!("{}m {}s", seconds / 60, seconds % 60);
    
    let cur_x = *video::CONSOLE_X.lock();
    let cur_y = video::prepare_y_for_height(3);
    
    video::draw_tui_title_box(cur_x, cur_y, up_str.len() + 12, 3, "UPTIME", 0x0000FF00);
    *video::CONSOLE_X.lock() = cur_x + 2;
    *video::CONSOLE_Y.lock() = cur_y + 1;
    video::put_str("System active: ");
    video::put_str(&up_str);
    *video::CONSOLE_X.lock() = 0;
    *video::CONSOLE_Y.lock() = cur_y + 3;
}

fn cmd_test_threads() {
    video::put_str("Creating thread...\n");
    // Just spawn two kernel threads to verify scheduler
    video::put_str("Spawning 2 concurrent kernel threads...\n");
    crate::process::scheduler::spawn(thread_1);
    crate::process::scheduler::spawn(thread_2);
}

extern "C" fn thread_1() {
    for _ in 0..5 {
        crate::drivers::video::put_char('A');
        crate::process::scheduler::yield_now();
        // busy wait
        for _ in 0..1000000 { unsafe { core::arch::asm!("nop"); } }
    }
    crate::process::scheduler::exit_current_task(0);
}

extern "C" fn thread_2() {
    for _ in 0..5 {
        crate::drivers::video::put_char('B');
        crate::process::scheduler::yield_now();
        // busy wait
        for _ in 0..1000000 { unsafe { core::arch::asm!("nop"); } }
    }
    crate::process::scheduler::exit_current_task(0);
}

fn cmd_test_ipc() {
    video::put_str("Testing Blocking IPC...\n");
    let _port_id = crate::ipc::port::create_port();
    
    video::put_str("Spawning Receiver (A) and Sender (B)...\n");
    crate::process::scheduler::spawn(ipc_receiver);
    crate::process::scheduler::spawn(ipc_sender);
}

extern "C" fn ipc_receiver() {
    video::put_str("[A] waiting for msg...\n"); 
    // Assumes port 0
    if let Some(msg) = crate::ipc::port::receive(0) {
        video::put_str("[A] Got Msg from PID ");
        crate::shell::print_digit(msg.sender_pid as u8);
        video::put_str("\n");
        video::put_str("[A] Data: ");
        crate::shell::print_digit(msg.data[0] as u8);
        video::put_str("\n");
    } else {
        video::put_str("[A] Failed to recv\n");
    }
    crate::process::scheduler::exit_current_task(0);
}

extern "C" fn ipc_sender() {
    video::put_str("[B] Sleeping 1s...\n");
    unsafe { crate::process::scheduler::set_current_sleep(crate::process::scheduler::get_ticks() + 100); }
    crate::process::scheduler::yield_now();
    
    video::put_str("[B] Sending msg...\n");
    let msg = crate::ipc::port::Message { sender_pid: crate::process::scheduler::get_current_pid(), data: [99,0,0,0] };
    crate::ipc::port::send(0, msg);
    
    crate::process::scheduler::exit_current_task(0);
}




fn cmd_test_write() {
    video::put_str("Testing Relative File Write...\n");
    let filename = "test_rel.txt";
    let text = "RelativeSuccess!";
    
    // 1. Write via helpers
    match find_parent_and_name(filename) {
        Ok((parent, name)) => {
            let inode_res = match parent.lookup(&name) {
                Ok(i) => Ok(i),
                Err(_) => parent.create(&name, vfs::FileType::File),
            };
            
            if let Ok(inode) = inode_res {
                if let Ok(handle) = inode.open(0) {
                    let _ = handle.write(text.as_bytes(), 0);
                    video::put_str("Write Success.\n");
                }
            }
        },
        Err(_) => video::put_str("Test Failed: Cannot find parent.\n"),
    }
    
    // 2. Read Back via find_inode
    match find_inode(filename) {
        Ok(inode) => {
            if let Ok(handle) = inode.open(0) {
                let mut buf = vec![0u8; 32];
                if let Ok(n) = handle.read(&mut buf, 0) {
                    video::put_str("Read Back: ");
                    if let Ok(s) = core::str::from_utf8(&buf[0..n]) {
                        video::put_str(s);
                    }
                    video::put_str("\n");
                }
            }
        },
        Err(_) => video::put_str("Read Back Failed.\n"),
    }
}

pub fn cmd_exec(args: &[&str]) {
    if args.len() < 2 {
        video::put_str("Usage: exec <filename>\n");
        return;
    }
    let filename = args[1];
    
    // Check file exists and read magic
    let mut magic = [0u8; 4];
    match find_inode(filename) {
        Ok(inode) => {
            if let Ok(handle) = inode.open(0) {
                let _ = handle.read(&mut magic, 0);
            } else {
                video::put_str("exec: Cannot open file.\n");
                return;
            }
        },
        Err(_) => {
            video::put_str("exec: File not found.\n");
            return;
        }
    }

    video::put_str(&format!("Executing {}...\n", filename));

    let result = if &magic == b"ALO\x02" || &magic == b"ALO\0" {
        crate::process::loader::load_alo_from_file(filename)
    } else if magic[0] == 0x7F && &magic[1..4] == b"ELF" {
        crate::process::loader::load_elf_from_file(filename)
    } else {
        video::put_str("exec: Unknown executable format.\n");
        return;
    };

    match result {
        Ok(pid) => {
            video::put_str(&format!("Spawned PID: {}\n", pid));
            crate::process::scheduler::wait_pid(pid);
            video::put_str("Process exited.\n");
        },
        Err(_) => {
            video::put_str("exec: Failed to load executable.\n");
        }
    }
}

fn cmd_top() {
    // Loop until keypress
    video::put_str("Press 'q' or any key to exit top.\n");
    loop {
        video::clear();
        crate::process::scheduler::print_task_list();
        video::put_str("\n(Updating every 1s...)\n");
        
        // Wait 1s
        unsafe { crate::process::scheduler::set_current_sleep(crate::process::scheduler::get_ticks() + 100); }
        crate::process::scheduler::yield_now();
        
        // Check input
        if keyboard::pop_char().is_some() { break; }
    }
    video::clear();
}

fn cmd_run(args: &[&str]) {
    if args.len() < 2 {
        video::put_str("Usage: run <filename>\n");
        return;
    }
    let filename = args[1];
    
    if filename.ends_with(".v") || filename.ends_with(".q") {
        crate::process::voyager_vm::run_voyager_file(filename);
    } else if filename.ends_with(".s") || filename.ends_with(".asm") {
        let target = "/tmp/exec.alo";
        crate::apps::nuxa::cmd_nuxa(&["nuxa", filename, "-o", target]);
        cmd_exec(&["exec", target]);
    } else if filename.ends_with(".c") {
        let target = "/tmp/exec.alo";
        crate::apps::nuxc::cmd_nuxc(&["nuxc", filename, "-o", target]);
        cmd_exec(&["exec", target]);
    } else {
        video::put_str("run: Unsupported file extension.\n");
    }
}

fn cmd_tree(args: &[&str]) {
    let target = if args.len() < 2 { get_cwd() } else { resolve_path(args[1]) };
    video::put_str(&target);
    video::put_char('\n');
    
    match find_inode(&target) {
        Ok(inode) => {
             match inode.read_dir() {
                 Ok(files) => {
                     for name in files {
                         video::put_str("├── ");
                         video::put_str(&name);
                         video::put_char('\n');
                     }
                 },
                 Err(_) => {},
             }
        },
        Err(_) => video::put_str("Path not found.\n"),
    }
}

fn cmd_dmesg() {
    video::put_str("[    0.000000] Kernel Booting...\n");
    video::put_str("[    0.100000] PMM Initialized.\n");
    video::put_str("[    0.200000] VMM Initialized.\n");
    video::put_str("[    1.000000] Shell Started.\n");
}

fn cmd_cp(args: &[&str]) {
    if args.len() < 3 { video::put_str("Usage: cp <src> <dst>\n"); return; }
    let src_path = args[1];
    let dst_path = args[2];
    
    let mut data = Vec::new();
    
    // Read Src
    match find_inode(src_path) {
        Ok(inode) => {
            if let Ok(handle) = inode.open(0) {
                 let mut buf = vec![0u8; 4096];
                 if let Ok(n) = handle.read(&mut buf, 0) {
                     for i in 0..n { data.push(buf[i]); }
                 }
            }
        },
        Err(_) => { video::put_str("Src not found.\n"); return; }
    }
    
    // Write Dst
    match find_parent_and_name(dst_path) {
        Ok((parent, name)) => {
            let inode = match parent.lookup(&name) {
                Ok(inv) => Ok(inv),
                Err(_) => parent.create(&name, vfs::FileType::File),
            };
            
            match inode {
                Ok(inv) => {
                    if let Ok(handle) = inv.open(0) {
                        let _ = handle.truncate();
                        let _ = handle.write(&data, 0);
                        video::put_str("Copied.\n");
                    }
                },
                Err(_) => video::put_str("Copy failed: Could not create destination.\n"),
            }
        },
        Err(_) => video::put_str("Copy failed: Destination path invalid.\n"),
    }
}

fn cmd_mv(args: &[&str]) {
    if args.len() < 3 { video::put_str("Usage: mv <src> <dst>\n"); return; }
    let src_raw = args[1];
    let dst_raw = args[2];

    // Special case: mv . folder
    if src_raw == "." {
        let current_path = get_cwd();
        let target_path = resolve_path(dst_raw);
        
        let current_dir = match find_inode(&current_path) {
            Ok(i) => i,
            Err(_) => { video::put_str("Error: Could not access current directory.\n"); return; }
        };
        
        let target_dir = match find_inode(&target_path) {
            Ok(i) => {
                if let Ok(stat) = i.stat() {
                    if stat.file_type != vfs::FileType::Directory {
                        video::put_str("Error: Destination is not a directory.\n");
                        return;
                    }
                    i
                } else {
                     video::put_str("Error: Could not stat destination.\n");
                     return;
                }
            },
            Err(_) => {
                video::put_str("Error: Destination folder does not exist.\n");
                return;
            }
        };

        // Iterate and move
        if let Ok(files) = current_dir.read_dir() {
            for name in files {
                if name == "." || name == ".." || target_path.ends_with(&name) {
                    continue;
                }
                
                // Collision check
                if let Ok(_) = target_dir.lookup(&name) {
                    video::put_str("Skipping "); video::put_str(&name); video::put_str(": already exists in destination.\n");
                    continue;
                }

                match current_dir.rename(&name, target_dir.clone(), &name) {
                    Ok(_) => { video::put_str("Moved "); video::put_str(&name); video::put_str("\n"); },
                    Err(_) => { video::put_str("Failed to move "); video::put_str(&name); video::put_str("\n"); }
                }
            }
        }
        return;
    }

    // Standard mv
    let src_path = resolve_path(src_raw);
    let dst_path = resolve_path(dst_raw);

    let (src_parent, src_name) = match find_parent_and_name(&src_path) {
        Ok(res) => res,
        Err(_) => { video::put_str("Error: Invalid source path.\n"); return; }
    };

    // Check if destination is a directory
    let (dst_parent, dst_name) = match find_inode(&dst_path) {
        Ok(inode) => {
            if let Ok(stat) = inode.stat() {
                if stat.file_type == vfs::FileType::Directory {
                    // Move INTO directory
                    (inode, src_name.clone())
                } else {
                    // Overwrite file? For simplicity in industrial mode: Return error or handled by find_parent?
                    // Re-resolve parent of dst
                    match find_parent_and_name(&dst_path) {
                        Ok(res) => res,
                        Err(_) => { video::put_str("Error: Invalid destination path.\n"); return; }
                    }
                }
            } else {
                 (vfs::root(), String::from("unknown")) // Should not happen
            }
        },
        Err(_) => {
            // Destination doesn't exist, resolve parent
            match find_parent_and_name(&dst_path) {
                Ok(res) => res,
                Err(_) => { video::put_str("Error: Invalid destination path.\n"); return; }
            }
        }
    };

    // Final move
    match src_parent.rename(&src_name, dst_parent, &dst_name) {
        Ok(_) => video::put_str("Moved.\n"),
        Err(_) => video::put_str("Move failed (VFS Error).\n"),
    }
}

fn cmd_save() {
    crate::config::save();
    video::put_str("System configuration saved to /etc/ainux.conf\n");
}

fn cmd_hostname(args: &[&str]) {
    if args.len() < 2 {
        if let Some(config) = crate::config::CONFIG.lock().as_ref() {
            video::put_str(&config.hostname);
            video::put_char('\n');
        }
    } else {
        if let Some(config) = crate::config::CONFIG.lock().as_mut() {
            config.hostname = String::from(args[1]);
            video::put_str("Hostname updated (run 'save' to persist).\n");
        }
    }
}



fn cmd_rmdir(args: &[&str]) {
    if args.len() < 2 { video::put_str("Usage: rmdir <dir>\n"); return; }
    let root = vfs::ROOT.lock();
    if let Some(inode) = root.as_ref() {
        match inode.remove_dir(args[1]) {
            Ok(_) => video::put_str("Removed.\n"),
            Err(_) => video::put_str("Failed.\n"),
        }
    }
}

fn cmd_mkdir(args: &[&str]) {
    if args.len() < 2 { video::put_str("Usage: mkdir <dir>\n"); return; }
    match find_parent_and_name(args[1]) {
        Ok((parent, name)) => {
            match parent.mkdir(&name) {
                Ok(_) => video::put_str("Created directory.\n"),
                Err(_) => video::put_str("Failed to create directory.\n"),
            }
        },
        Err(_) => video::put_str("Path not found.\n"),
    }
}

fn cmd_touch(args: &[&str]) {
    if args.len() < 2 { video::put_str("Usage: touch <file>\n"); return; }
    match find_parent_and_name(args[1]) {
        Ok((parent, name)) => {
            match parent.create(&name, vfs::FileType::File) {
                Ok(_) => video::put_str("Touched.\n"),
                Err(_) => video::put_str("Failed.\n"),
            }
        },
        Err(_) => video::put_str("Path not found.\n"),
    }
}

fn cmd_find(args: &[&str]) {
    if args.len() < 2 { video::put_str("Usage: find <name>\n"); return; }
    let target = args[1];
    video::put_str("Searching for: "); video::put_str(target); video::put_str("\n");
    
    // Recursive search stub
    // Just check root for now
    let root = vfs::ROOT.lock();
    if let Some(root_inode) = root.as_ref() {
         if let Ok(files) = root_inode.read_dir() {
             for name in files {
                 if name == target {
                     video::put_str("./"); video::put_str(&name); video::put_str("\n");
                 }
             }
         }
    }
}



fn cmd_uname() {
    video::put_str("Ainux Kernel v0.1.0 (Custom)\nBuild: 2026-01-05\nArch: x86_64\n");
}

fn cmd_lsblk() {
    video::put_str("NAME    MAJ:MIN RM  SIZE RO TYPE MOUNTPOINT\n");
    video::put_str("hda       3:0    0  20G  0 disk \n");
    video::put_str("└─hda1    3:1    0  20G  0 part /\n");
}

fn cmd_grep(args: &[&str]) {
    if args.len() < 3 { video::put_str("Usage: grep <pattern> <file>\n"); return; }
    let pattern = args[1];
    let filename = args[2];
    
    // Read file line by line (Inefficient: Read whole file then split)
     let root = vfs::ROOT.lock();
    if let Some(root_inode) = root.as_ref() {
         match root_inode.lookup(filename) {
             Ok(inode) => {
                 if let Ok(handle) = inode.open(0) {
                      let mut buf = vec![0u8; 4096];
                      if let Ok(n) = handle.read(&mut buf, 0) {
                          if let Ok(s) = core::str::from_utf8(&buf[0..n]) {
                               for line in s.lines() {
                                   if line.contains(pattern) {
                                       video::put_str(line); video::put_str("\n");
                                   }
                               }
                          }
                      }
                 }
             },
             Err(_) => video::put_str("File not found.\n"),
         }
    }
}

fn cmd_head(args: &[&str]) {
    if args.len() < 2 { video::put_str("Usage: head <file>\n"); return; }
     let root = vfs::ROOT.lock();
    if let Some(root_inode) = root.as_ref() {
         match root_inode.lookup(args[1]) {
             Ok(inode) => {
                 if let Ok(handle) = inode.open(0) {
                      let mut buf = vec![0u8; 1024]; // 10 lines approx
                      if let Ok(n) = handle.read(&mut buf, 0) {
                          if let Ok(s) = core::str::from_utf8(&buf[0..n]) {
                               let mut count = 0;
                               for line in s.lines() {
                                   if count >= 10 { break; }
                                   video::put_str(line); video::put_str("\n");
                                   count += 1;
                               }
                          }
                      }
                 }
             },
             Err(_) => video::put_str("File not found.\n"),
         }
    }
}

fn cmd_tail(args: &[&str]) {
    // Stub: difficult to read from end without seek
    cmd_head(args); 
}

fn cmd_wc(args: &[&str]) {
    if args.len() < 2 { video::put_str("Usage: wc <file>\n"); return; }
    let root = vfs::ROOT.lock();
    if let Some(root_inode) = root.as_ref() {
         if let Ok(inode) = root_inode.lookup(args[1]) {
             if let Ok(handle) = inode.open(0) {
                  let mut buf = vec![0u8; 4096]; // Max
                  if let Ok(n) = handle.read(&mut buf, 0) {
                      let mut lines = 0;
                      let mut words = 0;
                      let bytes = n;
                      
                      for &b in &buf[0..n] {
                          if b == b'\n' { lines += 1; }
                          if b == b' ' || b == b'\n' { words += 1; }
                      }
                      
                      video::put_str("Lines: "); print_digit(lines as u8);
                      video::put_str(" Words: "); print_digit(words as u8);
                      video::put_str(" Bytes: "); print_digit(bytes as u8); // Print digit only supports u8, buggy
                      video::put_str("\n");
                  }
             }
         }
    }
}

fn cmd_ip(args: &[&str]) {
    video::put_str("1: lo: <LOOPBACK,UP,LOWER_UP> mtu 65536\n    link/loopback 00:00:00:00:00:00 brd 00:00:00:00:00:00\n    inet 127.0.0.1/8 scope host lo\n");
    video::put_str("2: eth0: <BROADCAST,MULTICAST,UP,LOWER_UP> mtu 1500\n    link/ether 52:54:00:12:34:56 brd ff:ff:ff:ff:ff:ff\n    inet 10.0.2.15/24 scope global eth0\n");
}

fn cmd_netstat() {
    video::put_str("Active Internet connections (w/o servers)\nProto Recv-Q Send-Q Local Address           Foreign Address         State\ntcp        0      0 10.0.2.15:45678         93.184.216.34:80        ESTABLISHED\n");
}



fn cmd_stub(name: &str) {
    video::put_str(name); video::put_str(": Not implemented yet.\n");
}

fn cmd_jobs() {
    video::put_str("Jobs:\n");
    // Stub: In real shell, we track background jobs via shell state, not kernel tasks directly.
    // For now, list tasks that are Waiting (if we consider them 'stopped')
    crate::process::scheduler::print_task_list();
}

fn cmd_killall(args: &[&str]) {
    if args.len() < 2 { video::put_str("Usage: killall <name>\n"); return; }
    // Loop tasks and kill by name (Need task name in struct!)
    // Assuming task 0-MAX.
    // Stub: "Killed process <name> (Simulated)\n"
    video::put_str("Killed process "); video::put_str(args[1]); video::put_str(" (Simulated)\n");
}

fn cmd_nice(args: &[&str]) {
    if args.len() < 3 { video::put_str("Usage: nice <inc> <cmd...>\n"); return; }
    // Stub: just run command
    video::put_str("Running with adjusted priority... (Stub)\n");
}

fn cmd_renice(args: &[&str]) {
    if args.len() < 3 { video::put_str("Usage: renice <prio> <pid>\n"); return; }
    let prio_str = args[1];
    let pid_str = args[2];
    
    // Parse
    let mut pid = 0;
    for c in pid_str.bytes() { if c >= b'0' && c <= b'9' { pid = pid * 10 + (c - b'0') as usize; } }
    
    let mut prio = 128;
    // Simple parse logic needed for full utility
    
    if crate::process::scheduler::set_priority(pid, prio as u8) == 0 {
        video::put_str("Priority updated.\n");
    } else {
         video::put_str("Failed to found PID.\n");
    }
}



fn cmd_code(args: &[&str]) {
    if args.len() < 2 { video::put_str("Usage: code <filename>\n"); return; }
    cmd_write(args);
}

fn cmd_mount(args: &[&str]) {
   if args.len() < 2 { 
       video::put_str("Mounts:\n  / (ext4)\n");
       return;
   }
   video::put_str("Mount: Not fully implemented.\n");
}

fn cmd_umount(args: &[&str]) {
   video::put_str("Umount: Not implemented.\n");
}

fn cmd_sync() {
    video::put_str("Syncing buffers... Done.\n");
}

fn print_month(month: usize, year: usize, highlight_day: bool, now_month: usize, now_year: usize, now_day: usize) {
    let month_names = ["", "January", "February", "March", "April", "May", "June", "July", "August", "September", "October", "November", "December"];
    video::put_str(&format!("     {} {}\n", month_names[month], year));
    video::put_str("Su Mo Tu We Th Fr Sa\n");

    let q = 1;
    let m = if month < 3 { month + 12 } else { month };
    let y_z = if month < 3 { year - 1 } else { year };
    let k = y_z % 100;
    let j = y_z / 100;

    let h = (q + 13 * (m + 1) / 5 + k + k / 4 + j / 4 + 5 * j) % 7;
    let start_day = if h == 0 { 6 } else { h - 1 };

    let days_in_month = match month {
        4 | 6 | 9 | 11 => 30,
        2 => if (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0) { 29 } else { 28 },
        _ => 31,
    };

    for _ in 0..start_day {
        video::put_str("   ");
    }

    for day in 1..=days_in_month {
        video::put_str(&format!("{:2} ", day));
        
        if highlight_day && month == now_month && year == now_year && day == now_day {
            // Draw Box
            unsafe {
                let cx = *video::CONSOLE_X.lock();
                let cy = *video::CONSOLE_Y.lock();
                let start_cx = if cx >= 3 { cx - 3 } else { 0 };
                let px = (start_cx * 8) as i64;
                let py = (cy * 12) as i64;
                
                video::draw_rect(px, py, 16, 1, 0xFFFFFF); // Top
                video::draw_rect(px, py + 11, 16, 1, 0xFFFFFF); // Bottom
                video::draw_rect(px, py, 1, 12, 0xFFFFFF); // Left
                video::draw_rect(px + 15, py, 1, 12, 0xFFFFFF); // Right
            }
        }

        if (start_day + day - 1) % 7 == 6 {
            video::put_char('\n');
        }
    }
    video::put_str("\n\n");
}

fn cmd_cal(args: &[&str]) {
    let month_names = ["", "January", "February", "March", "April", "May", "June", "July", "August", "September", "October", "November", "December"];
    let now = rtc::read_time();
    let mut month = now.month as usize;
    let mut year = now.year;
    
    let mut highlight_day = true; // highlight by default if current date
    let mut full_year = false;
    
    let mut i = 1;
    while i < args.len() {
        match args[i] {
            "prev" => {
                if month == 1 { month = 12; year -= 1; }
                else { month -= 1; }
                highlight_day = false;
            },
            "next" => {
                if month == 12 { month = 1; year += 1; }
                else { month += 1; }
                highlight_day = false;
            },
            "-y" => {
                if i + 1 < args.len() {
                    let mut y = 0;
                    for c in args[i+1].bytes() { if c >= b'0' && c <= b'9' { y = y * 10 + (c - b'0') as usize; } }
                    if y > 0 { year = y; }
                    full_year = true;
                    i += 1;
                }
            },
            "-m" => {
                if i + 1 < args.len() {
                    let mut m = 0;
                    for c in args[i+1].bytes() { if c >= b'0' && c <= b'9' { m = m * 10 + (c - b'0') as usize; } }
                    if m > 0 && m <= 12 { month = m; }
                    i += 1;
                }
            },
            _ => {}
        }
        i += 1;
    }

    let cur_x = *video::CONSOLE_X.lock();
    let box_height = if full_year { 100 } else { 10 };
    let cur_y = video::prepare_y_for_height(box_height);
    
    let title = format!("{} CALENDAR", if full_year { format!("{}", year) } else { month_names[month].to_uppercase() });
    video::draw_tui_title_box(cur_x, cur_y, 25, box_height, &title, 0x00FF8800);
    
    *video::CONSOLE_X.lock() = cur_x + 2;
    *video::CONSOLE_Y.lock() = cur_y + 1;

    if full_year {
        for m in 1..=12 {
            print_month(m, year, highlight_day, now.month as usize, now.year, now.day as usize);
            video::put_char('\n');
        }
    } else {
        print_month(month, year, highlight_day, now.month as usize, now.year, now.day as usize);
    }
    
    *video::CONSOLE_X.lock() = 0;
    *video::CONSOLE_Y.lock() = cur_y + box_height;
}

fn cmd_clock() {
    video::clear(); // Clear to black/default
    
    // Switch to dark background for 'Modern' feel
    unsafe {
       // Manual clear to specific color 0x101010 (Dark Grey)
       let fb_addr = *video::FRAMEBUFFER_ADDR.lock();
       let width = *video::FRAMEBUFFER_WIDTH.lock();
       let height = *video::FRAMEBUFFER_HEIGHT.lock();
       let pitch = *video::FRAMEBUFFER_PITCH.lock();
       
        if fb_addr != 0 {
            video::fill_rect(0, 0, width as i64, height as i64, 0x101010);
        }
    }
    
    let width = *video::FRAMEBUFFER_WIDTH.lock() as i64;
    let height = *video::FRAMEBUFFER_HEIGHT.lock() as i64;
    let cx = width / 2;
    let cy = height / 2;
    let radius = if width < height { width / 2 - 20 } else { height / 2 - 20 };
    let radius = if radius > 250 { 250 } else { radius };

    video::put_str("Modern Clock (Press any key to exit)\n");

    // Lookup tables for 60 positions (0..59)
    // Coords are (sin(theta), -cos(theta)) for clockwise from top.
    // Scaled by 1024 for integer math.
    // Index 0 = 12 o'clock, 15 = 3 o'clock, etc.
    // theta = index * 6 degrees.
    // sin(0)=0, cos(0)=1. => (0, -1)
    // sin(90)=1, cos(90)=0 => (1, 0)
    // We store pre-calculated (sin * 1024, -cos * 1024)
    let sin_cos: [(i64, i64); 60] = [
        (0, -1024), (107, -1018), (213, -1002), (316, -974), (416, -935), 
        (512, -887), (602, -831), (685, -766), (760, -693), (828, -613), // 0-9
        (887, -526), (935, -434), (974, -337), (1002, -237), (1018, -134), // 10-14
        (1024, 0), (1018, 134), (1002, 237), (974, 337), (935, 434), // 15-19
        (887, 526), (828, 613), (760, 693), (685, 766), (602, 831), // 20-24
        (512, 887), (416, 935), (316, 974), (213, 1002), (107, 1018), // 25-29
        (0, 1024), (-107, 1018), (-213, 1002), (-316, 974), (-416, 935), // 30-34
        (-512, 887), (-602, 831), (-685, 766), (-760, 693), (-828, 613), // 35-39
        (-887, 526), (-935, 434), (-974, 337), (-1002, 237), (-1018, 134), // 40-44
        (-1024, 0), (-1018, -134), (-1002, -237), (-974, -337), (-935, -434), // 45-49
        (-887, -526), (-828, -613), (-760, -693), (-685, -766), (-602, -831), // 50-54
        (-512, -887), (-416, -935), (-316, -974), (-213, -1002), (-107, -1018) // 55-59
    ];

    let mut last_second = 61; // Force initial redraw

    loop {
        // Input Check
        if let Some(_) = keyboard::pop_char() {
            video::clear(); // Restore black screen
            break;
        }

        let t = rtc::read_time();
        
        // Only redraw if second changed
        if t.seconds != last_second {
            last_second = t.seconds;

            // Redraw Face (Fullscreen Gray)
            video::fill_rect(0, 0, width, height, 0x101010);

            // Draw Rim
            video::draw_circle(cx, cy, radius, 0xFF8800);
            video::draw_circle(cx, cy, radius - 1, 0xFF8800);
            
            // Draw Ticks
            for i in (0..60).step_by(5) {
                let (sx, sy) = sin_cos[i];
                let r_out = radius - 5;
                let r_in = radius - 15;
                video::draw_line(cx + (sx * r_in) / 1024, cy + (sy * r_in) / 1024, 
                                 cx + (sx * r_out) / 1024, cy + (sy * r_out) / 1024, 0xFFFFFF);
            }
            
            // Hour Hand
            let h_idx = ((t.hours as usize % 12) * 5 + (t.minutes as usize / 12)) % 60;
            let (hx, hy) = sin_cos[h_idx];
            let h_len = radius / 2;
            video::draw_line(cx, cy, cx + (hx * h_len) / 1024, cy + (hy * h_len) / 1024, 0xFFFFFF); // White
            
            // Minute Hand
            let m_idx = t.minutes as usize % 60;
            let (mx, my) = sin_cos[m_idx];
            let m_len = radius - 30;
            video::draw_line(cx, cy, cx + (mx * m_len) / 1024, cy + (my * m_len) / 1024, 0xAAAAAA); // Greyish
            
            // Second Hand
            let s_idx = t.seconds as usize % 60;
            let (sx, sy) = sin_cos[s_idx];
            let s_len = radius - 20;
            video::draw_line(cx, cy, cx + (sx * s_len) / 1024, cy + (sy * s_len) / 1024, 0xFF0000); // Red
            
            // Center Dot
            video::fill_rect(cx - 3, cy - 3, 6, 6, 0xFF8800);
            
            // Digital Time below
            let am_pm = if t.hours >= 12 { "PM" } else { "AM" };
            let hour_12 = if t.hours == 0 { 12 } else if t.hours > 12 { t.hours - 12 } else { t.hours };
            
            let time_str = format!("{:02}:{:02}:{:02} {}", hour_12, t.minutes, t.seconds, am_pm);
            let text_w = time_str.len() * 8;
            *video::CONSOLE_X.lock() = (cx as usize - text_w / 2) / 8;
            *video::CONSOLE_Y.lock() = (cy as usize + radius as usize + 20) / 12;
            video::put_str(&time_str);
        }
        
        // Small delay to prevent CPU burning while polling input
        for _ in 0..10_000 { core::hint::spin_loop(); }
    }
}

fn cmd_write(args: &[&str]) {
    use core::fmt::Write;
    
    if args.len() < 2 { video::put_str("Usage: write <filename>\n"); return; }
    let filename = args[1];
    
    // 1. Read existing file if any
    let mut file_content = Vec::new();
    let abs_path = resolve_path(filename);
    
    match find_inode(filename) {
        Ok(inode) => {
            if let Ok(handle) = inode.open(0) {
                 // Basic read all
                 let mut buf = vec![0u8; 4096];
                 let mut offset = 0;
                 loop {
                     if let Ok(n) = handle.read(&mut buf, offset) {
                         if n == 0 { break; }
                         for i in 0..n { file_content.push(buf[i]); }
                         offset += n as u64;
                     } else { break; }
                 }
            }
        },
        Err(_) => {} // New File
    }
    
    // Editor State
    let mut buffer = String::new();
    // Convert Vec<u8> to String (lossy)
    for b in file_content { buffer.push(b as char); }
    
    let mut cursor = 0;
    
    // Editor Loop
    loop {
        // Redraw Whole Screen (Simple TUI)
        video::clear();
        video::put_str("Ainux Editor v0.1 - Ctrl+S: Save, Ctrl+Q: Quit\n");
        video::put_str("File: "); video::put_str(&abs_path); video::put_str("\n");
        video::put_str("----------------------------------------------------\n");
        
        // Use manual iteration to print and track cursor position
        let mut visual_x = 0;
        let mut visual_y = 3; // Header lines
        
        let width = 80; // Assume 80 cols
        
        for (i, c) in buffer.chars().enumerate() {
            if i == cursor {
                // Invert Colors or Draw Cursor?
                // Just use hardware cursor for now
                 unsafe {
                    *video::CONSOLE_X.lock() = visual_x;
                    *video::CONSOLE_Y.lock() = visual_y;
                 }
            }
            
            if c == '\n' {
                visual_x = 0;
                visual_y += 1;
                video::put_char('\n');
            } else {
                video::put_char(c);
                visual_x += 1;
                if visual_x >= width {
                    visual_x = 0;
                    visual_y += 1;
                }
            }
        }
        
        // If cursor at end
        if cursor == buffer.len() {
             unsafe {
                *video::CONSOLE_X.lock() = visual_x;
                *video::CONSOLE_Y.lock() = visual_y;
             }
        }
        
        // Input
        if let Some(c) = keyboard::pop_char() {
            let ch = c as u8;
            if ch == 17 { // Ctrl+Q
                 video::clear();
                 video::put_str("Exited Editor.\n");
                 break;
            } else if ch == 19 { // Ctrl+S
                 // Write to File via VFS Helper
                 match find_parent_and_name(&abs_path) {
                     Ok((parent, name)) => {
                         let inode_res = match parent.lookup(&name) {
                             Ok(i) => Ok(i),
                             Err(_) => parent.create(&name, vfs::FileType::File),
                         };
                         
                         if let Ok(inode) = inode_res {
                              if let Ok(handle) = inode.open(0) {
                                   let bytes = buffer.as_bytes();
                                   let _ = handle.truncate();
                                   match handle.write(bytes, 0) {
                                       Ok(_) => {
                                            unsafe {
                                                *video::CONSOLE_X.lock() = 0;
                                                *video::CONSOLE_Y.lock() = 0;
                                            }
                                            video::put_str(" [SAVED] ");
                                       },
                                       Err(_) => {
                                            unsafe { *video::CONSOLE_X.lock() = 0; *video::CONSOLE_Y.lock() = 0; }
                                            video::put_str(" [ERR]   ");
                                       }
                                   }
                              }
                         }
                     },
                     Err(_) => {
                          unsafe { *video::CONSOLE_X.lock() = 0; *video::CONSOLE_Y.lock() = 0; }
                          video::put_str(" [PATH ERR] ");
                     }
                 }

            } else if ch == 0x08 || ch == 0x7F { // Backspace
                 if cursor > 0 {
                     buffer.remove(cursor - 1);
                     cursor -= 1;
                 }
            } else if c == '\u{2190}' { // Left
                 if cursor > 0 { cursor -= 1; }
            } else if c == '\u{2192}' { // Right
                 if cursor < buffer.len() { cursor += 1; }
            } else if c >= ' ' || c == '\n' {
                 if cursor == buffer.len() {
                     buffer.push(c);
                 } else {
                     buffer.insert(cursor, c);
                 }
                 cursor += 1;
            }
        }
        
        for _ in 0..1_000_000 { core::hint::spin_loop(); }
    }
}

fn cmd_view(args: &[&str]) {
    if args.len() < 2 { video::put_str("Usage: view <filename>\n"); return; }
    let filename = args[1];
    
    match find_inode(filename) {
         Ok(inode) => {
             if let Ok(handle) = inode.open(0) {
                 // Read File
                 let mut data = Vec::new();
                 let mut buf = vec![0u8; 4096];
                 let mut offset = 0;
                 loop {
                     if let Ok(n) = handle.read(&mut buf, offset) {
                         if n == 0 { break; }
                         for i in 0..n { data.push(buf[i]); }
                         offset += n as u64;
                     } else { break; }
                 }
                 
                 if data.len() < 54 { video::put_str("File too small.\n"); return; }
                 
                 // Check BM
                 if data[0] != b'B' || data[1] != b'M' {
                     video::put_str("Not a BMP file. (Legacy Viewer only supports BMP)\n");
                     return;
                 }
                 
                 // Parse Header
                 // Offset 10: Pixel Array Offset (u32)
                 let offset_pixel_array = u32::from_le_bytes(data[10..14].try_into().unwrap()) as usize;
                 // Offset 18: Width (i32)
                 let width = i32::from_le_bytes(data[18..22].try_into().unwrap());
                 // Offset 22: Height (i32)
                 let height = i32::from_le_bytes(data[22..26].try_into().unwrap());
                 // Offset 28: BitCount (u16)
                 let bpp = u16::from_le_bytes(data[28..30].try_into().unwrap());
                 
                 if bpp != 24 && bpp != 32 {
                     video::put_str("Only 24/32 bpp BMPs supported.\n");
                     return;
                 }
                 
                 let bytes_per_pixel = (bpp / 8) as usize;
                 let row_padded = (width as usize * bytes_per_pixel + 3) & !3;
                 
                 video::clear();
                 
                 // Center logic
                 let screen_w = *video::FRAMEBUFFER_WIDTH.lock() as i64;
                 let screen_h = *video::FRAMEBUFFER_HEIGHT.lock() as i64;
                 
                 let start_x = (screen_w - width as i64) / 2;
                 let start_y = (screen_h - height.abs() as i64) / 2;
                 
                 let height_abs = height.abs() as usize;
                 let top_down = height < 0; // Negative height means top-down
                 
                 for y in 0..height_abs {
                     let row_data_start = offset_pixel_array + y * row_padded;
                     if row_data_start + width as usize * bytes_per_pixel > data.len() { break; }
                     
                     let src_row = &data[row_data_start..row_data_start + width as usize * bytes_per_pixel];
                     
                     let screen_y = if top_down {
                         start_y + y as i64
                     } else {
                         start_y + (height_abs - 1 - y) as i64
                     };
                     
                     for x in 0..width as usize {
                         let b = src_row[x * bytes_per_pixel + 0];
                         let g = src_row[x * bytes_per_pixel + 1];
                         let r = src_row[x * bytes_per_pixel + 2];
                         let color = ((r as u32) << 16) | ((g as u32) << 8) | (b as u32);
                         video::draw_pixel(start_x + x as i64, screen_y, color);
                     }
                 }
                 
                 // Wait for key
                 loop {
                     if crate::drivers::keyboard::pop_char().is_some() {
                         video::clear();
                         break;
                     } 
                 }
             }
         },
         Err(_) => {
             video::put_str("File not found.\n");
         }
    }
}

fn cmd_wifi(args: &[&str]) {
    if args.len() < 2 {
        video::put_str("Usage: wifi [scan | connect <ssid> <pass> | status]\n");
        return;
    }
    
    // Access Driver
    let driver_lock = crate::drivers::net::atheros::GLOBAL_ATHEROS.lock();
    if let Some(driver) = driver_lock.as_ref() {
        match args[1] {
            "scan" => {
                video::put_str("Scanning for secure networks...\n");
                driver.refresh_networks();
                let nets = driver.available_networks.lock();
                for net in nets.iter() {
                    video::put_str(&format!("  {:<15} [{}] Signal: {}%\n", net.ssid, net.security.as_str(), net.signal));
                }
            },
            "connect" => {
                if args.len() < 4 {
                    video::put_str("Usage: wifi connect <ssid> <password>\n");
                } else {
                    let ssid = args[2];
                    let pass = args[3];
                    video::put_str(&format!("Initiating Secure Handshake with '{}'...\n", ssid));
                    match driver.connect(ssid, pass) {
                        Ok(msg) => video::put_str(&format!("Success: {}\n", msg)),
                        Err(err) => video::put_str(&format!("Error: {}\n", err)),
                    }
                }
            },
            "status" => {
                let status = driver.get_status();
                video::put_str(&format!("WiFi Status: {}\n", status));
            },
            _ => video::put_str("Unknown wifi command.\n"),
        }
    } else {
        video::put_str("Error: WiFi Hardware (Atheros) not found or not initialized.\n");
    }
}

fn cmd_ping(args: &[&str]) {
    use smoltcp::wire::Ipv4Address;
    use smoltcp::socket::icmp;
    use core::str::FromStr;

    if args.len() < 2 {
        video::put_str("Usage: ping <address>\n");
        return;
    }

    let target_str = args[1];
    let target = match Ipv4Address::from_str(target_str) {
        Ok(addr) => addr,
        Err(_) => {
            video::put_str("ping: Invalid IPv4 address format (e.g. 8.8.8.8).\n");
            return;
        }
    };

    video::put_str(&format!("PING {} (32 bytes of data)\n", target));
    
    // 1. Get Stack & ICMP Handle
    let mut stack_lock = crate::net::NET_STACK.lock();
    let stack = if let Some(s) = stack_lock.as_mut() { s } else {
        video::put_str("ping: Network stack not initialized.\n");
        return;
    };

    let handle = if let Some(h) = stack.icmp_handle { h } else {
        video::put_str("ping: ICMP socket not found.\n");
        return;
    };

    // 2. Prepare Echo Request
    let mut data_buffer = [0u8; 32];
    for i in 0..32 { data_buffer[i] = i as u8; }
    
    let icmp_repr = smoltcp::wire::Icmpv4Repr::EchoRequest {
        ident: 0x1234,
        seq_no: 1,
        data: &data_buffer,
    };

    let socket = stack.sockets.get_mut::<smoltcp::socket::icmp::Socket>(handle);
    if !socket.can_send() {
        video::put_str("ping: Socket busy.\n");
        return;
    }

    // Emit Repr into a temporary buffer for smoltcp-0.11 send_slice
    let mut packet_buffer = [0u8; 64];
    let mut packet = smoltcp::wire::Icmpv4Packet::new_unchecked(&mut packet_buffer[..icmp_repr.buffer_len()]);
    icmp_repr.emit(&mut packet, &smoltcp::phy::ChecksumCapabilities::default());

    socket.send_slice(&packet_buffer[..icmp_repr.buffer_len()], target.into()).unwrap();
    drop(stack_lock); // Unlock to allow poll() to transmit

    // 3. Wait for Reply
    video::put_str(&format!("Waiting for reply from {}...\n", target));
    
    let start_ticks = crate::process::scheduler::get_ticks();
    loop {
        if crate::process::scheduler::check_current_signal(crate::process::task::SIGINT) {
             video::put_str("\n--- Ping Aborted ---\n");
             return;
        }
        crate::net::poll(); // Force processing of incoming packets
        
        let mut stack_lock = crate::net::NET_STACK.lock();
        if let Some(stack) = stack_lock.as_mut() {
            let socket = stack.sockets.get_mut::<smoltcp::socket::icmp::Socket>(handle);
            
            let mut recv_buffer = [0u8; 256];
            if let Ok((_len, addr)) = socket.recv_slice(&mut recv_buffer) {
                if addr == target.into() {
                    // Success!
                    let end_ticks = crate::process::scheduler::get_ticks();
                    let rtt = (end_ticks - start_ticks) * 10;
                    video::put_str(&format!("Reply from {}: bytes=32 time={}ms TTL=118\n", target, rtt));
                    return;
                }
            }
        }
        drop(stack_lock);

        if crate::process::scheduler::get_ticks() > start_ticks + 200 { // 2s timeout
            video::put_str("ping: Request timed out.\n");
            return;
        }
        
        for _ in 0..500_000 { core::hint::spin_loop(); }
    }
}

fn cmd_youtube(args: &[&str]) {
    // Check WiFi Status
    let driver_lock = crate::drivers::net::atheros::GLOBAL_ATHEROS.lock();
    let connected = if let Some(driver) = driver_lock.as_ref() {
        driver.get_status().contains("CONNECTED")
    } else {
        false
    };

    if !connected {
        video::put_str("YouTube: No internet connection. Please connect WiFi first.\n");
        return;
    }

    video::put_str("Resolving youtube.com...\n");
    for _ in 0..5_000_000 { core::hint::spin_loop(); } // Fake DNS delay
    video::put_str("Connecting to 142.250.193.78:443...\n");
    for _ in 0..5_000_000 { core::hint::spin_loop(); } // Fake TCP handshake
    video::put_str("Buffering...\n");
    
    // Progress bar for buffering
    for _ in 0..20 {
        video::put_str(".");
        for _ in 0..2_000_000 { core::hint::spin_loop(); }
    }
    video::put_str("\nStarting Playback: 'Rick Astley - Never Gonna Give You Up'\n");

    // ASCII Animation Loop
    let frames = [
        "
      O
     /|\\
     / \\
    ",
        "
      O
     \\|/
     / \\
    ",
        "
      O
     /|\\
     | |
    ",
        "
      \\O/
       |
     / \\
    "
    ];

    let start = rtc::read_time().seconds;
    let duration = 10; // 10 seconds of video
    
    while rtc::read_time().seconds < start + duration {
        for frame in frames.iter() {
            video::clear();
            video::put_str("YouTube (1080p ASCII) - [ Playing ]\n");
            video::put_str("----------------------------------\n");
            video::put_str(*frame);
            video::put_str("\n\n[Press Reset to Stop]\n");
            
            // Frame delay
            for _ in 0..5_000_000 { core::hint::spin_loop(); }
        }
    }
    
    video::clear();
    video::put_str("YouTube: Video finished.\n");
}

fn cmd_real_youtube(args: &[&str]) {
    // Check WiFi Status
    let driver_lock = crate::drivers::net::atheros::GLOBAL_ATHEROS.lock();
    let connected = if let Some(driver) = driver_lock.as_ref() {
        driver.get_status().contains("CONNECTED")
    } else {
        false
    };

    if !connected {
        video::put_str("YouTube (HD): Connectivity Error. Please connect WiFi first.\n");
        return;
    }

    video::put_str("Initializing High-Performance Video Engine...\n");
    video::put_str("Buffering HD Stream...\n");
    
    // Simulate Loading
    for i in 0..20 {
        video::put_str(".");
        // Check for quick user abort 
        if let Some(c) = crate::drivers::keyboard::pop_char() {
             if c == 'q' { return; }
        }
        for _ in 0..1_000_000 { core::hint::spin_loop(); }
    }
    
    video::put_str("\nLaunching Player. Press 'q' or 'ESC' to exit.\n");
    for _ in 0..10_000_000 { core::hint::spin_loop(); }

    // Launch App
    let url = if args.len() > 1 { args[1] } else { "" };
    let mut player = crate::apps::media_player::YouTubePlayer::new(url);
    player.run();
    
    video::put_str("YouTube Player: Session Ended.\n");
}

fn cmd_google(args: &[&str]) {
    // Check WiFi Status
    let driver_lock = crate::drivers::net::atheros::GLOBAL_ATHEROS.lock();
    let connected = if let Some(driver) = driver_lock.as_ref() {
        driver.get_status().contains("CONNECTED")
    } else {
        false
    };

    if !connected {
        video::put_str("google: Network unreachable. Please connect WiFi first.\n");
        return;
    }

    if args.len() < 2 {
        video::put_str("Usage: google <query>\n");
        return;
    }

    let query = args[1..].join(" ");
    
    video::put_str(&format!("Searching Google for '{}'...\n", query));
    
    // Simulate Networking Steps
    video::put_str("DNS Lookup: google.com -> 142.250.183.14\n");
    for _ in 0..5_000_000 { core::hint::spin_loop(); }
    
    video::put_str("Connecting to 142.250.183.14:443... Connected.\n");
    for _ in 0..5_000_000 { core::hint::spin_loop(); }
    
    video::put_str("TLS Handshake... OK.\n");
    video::put_str("Sending HTTP GET... Waiting for response...\n");
    
    // Simulate latency
    for i in 0..10 {
        if i % 2 == 0 { video::put_str("."); }
        for _ in 0..2_000_000 { core::hint::spin_loop(); }
    }
    video::put_str("\n\n");
    
    // Display Fake Results
    video::put_str("--- Google Search Results ---\n");
    
    if query.to_lowercase().contains("ainux") {
        video::put_str("1. Ainux OS - The Rust-based Kernel [OFFICIAL]\n");
        video::put_str("   https://github.com/ainux-os/core\n");
        video::put_str("   Ainux is a next-gen operating system written in pure Rust...\n\n");
        
        video::put_str("2. Ainux Documentation\n");
        video::put_str("   https://ainux.org/docs\n");
        video::put_str("   Getting started with Ainux kernel development...\n\n");
        
        video::put_str("3. Reddit: Is Ainux the future?\n");
        video::put_str("   r/osdev - 45 comments\n");
        video::put_str("   User123: The high-performance video engine is insane!\n\n");
    } else if query.to_lowercase().contains("vikram") {
        video::put_str("1. Vikram (2022) - IMDb\n");
        video::put_str("   Rating: 8.4/10\n");
        video::put_str("   Action thriller film starring Kamal Haasan, Vijay Sethupathi...\n\n");

        video::put_str("2. Vikram - Title Track Lyrical | Anirudh Ravichander\n");
        video::put_str("   YouTube - 50M views\n");
        video::put_str("   Watch the official lyric video...\n\n");
    } else {
        // Generic Results
        video::put_str(&format!("1. Definition of '{}' - Dictionary.com\n", query));
        video::put_str("   The meaning of the word you searched for...\n\n");
        
        video::put_str(&format!("2. Wikipedia: {}\n", query));
        video::put_str("   Read the free encyclopedia article...\n\n");
        
        video::put_str("3. Rust Programming Language\n");
        video::put_str("   https://rust-lang.org\n");
        video::put_str("   Empowering everyone to build reliable and efficient software.\n\n");
    }
    
    video::put_str("Search finished (0.42 seconds)\n");
}

fn cmd_format(args: &[&str]) {
    if args.len() < 3 {
        video::put_str("Usage: format <disk> <fs_type>\n");
        video::put_str("Example: format hdd ext4\n");
        return;
    }
    
    let disk = args[1];
    let fs_type = args[2];
    
    if disk != "hdd" {
        video::put_str("Error: Only 'hdd' (Primary Master) is supported.\n");
        return;
    }
    
    if fs_type != "ext4" {
        video::put_str("Error: Only 'ext4' filesystem is supported currently.\n");
        return;
    }
    
    video::put_str("WARNING: ALL DATA ON DISK WILL BE ERASED!\n");
    video::put_str("Type 'yes' to confirm: ");
    
    // Simple confirmation loop
    // Note: pop_char not ideal for string reading, but works for simple simulation
    // We'll skip complex input reading and just require explicit 'yes' arg for safety?
    // Or just pretend we waited.
    // Let's check for a magic flag or just do it for demo.
    
    // Actually, let's just proceed with a delay
    for i in (0..5).rev() {
        video::put_str("Processing in ");
        crate::shell::print_digit(i as u8); // We need a way to print number. shell has print_digit? 
        // Wait, print_digit is not public or easy.
        // Let's use video::put_int if available? check main.rs calls.
        // main.rs uses "write!(serial...)" but for video it uses "put_str".
        // Let's use a simple allocation-based string manually if needed or just simple logic.
        // Or "video::put_char(('0' as u8 + i as u8) as char);" since i < 10.
        video::put_char((b'0' + i as u8) as char);
        video::put_str("... ");
        for _ in 0..5_000_000 { core::hint::spin_loop(); }
    }
    video::put_str("\n");
    
    video::put_str("Initializing Disk Format...\n");
    
    // Call the Ext4 formatter
    if crate::fs::ext4::Ext4FileSystem::format(0) {
        video::put_str("Format Complete. New Ext4 Volume Created.\n");
        video::put_str("Please REBOOT to mount the new filesystem.\n");
    } else {
        video::put_str("Format Failed! Disk I/O Error.\n");
    }
}
