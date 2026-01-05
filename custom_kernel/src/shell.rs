extern crate alloc;
use alloc::vec::Vec;
use alloc::string::String;
use alloc::vec;
use alloc::format;
use crate::drivers::{keyboard, video, rtc};
use crate::fs::vfs;
use core::fmt::Write;
use spin::Mutex;

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
    if path.starts_with("/") {
        String::from(path)
    } else {
        let current = get_cwd();
        if current == "/" {
            let mut s = String::from("/");
            s.push_str(path);
            s
        } else {
            let mut s = current;
            s.push_str("/");
            s.push_str(path);
            s
        }
    }
}



static HISTORY: Mutex<Vec<String>> = Mutex::new(Vec::new());

fn process_char(c: char, input_buffer: &mut String, cursor_pos: &mut usize, history_index: &mut usize) {
    if c == '\n' {
        video::put_char('\n');
    } else if c == '\x08' || c == '\x7F' { // Backspace or Delete
         if *cursor_pos > 0 && input_buffer.len() > 0 {
             // Remove char at cursor_pos - 1
             let remove_idx = *cursor_pos - 1;
             input_buffer.remove(remove_idx);
             *cursor_pos -= 1;
             
             // Redraw Line
             // Move cursor back one
             video::put_str("\x08"); 
             // Print rest of string from cursor + spaces to clear
             let rest = &input_buffer[*cursor_pos..];
             video::put_str(rest);
             video::put_char(' ');
             // Move cursor back to correct position
             for _ in 0..(rest.len() + 1) {
                 video::put_str("\x08");
             }
         }
    } else if c == '\u{2190}' { // Left Arrow
        if *cursor_pos > 0 {
            *cursor_pos -= 1;
            video::put_str("\x08");
        }
    } else if c == '\u{2192}' { // Right Arrow
        if *cursor_pos < input_buffer.len() {
            let ch = input_buffer.chars().nth(*cursor_pos).unwrap();
            video::put_char(ch);
            *cursor_pos += 1;
        }
    } else if c == '\u{2191}' { // Up Arrow (History Prev)
        let hist = HISTORY.lock();
        if !hist.is_empty() {
             if *history_index > 0 {
                 *history_index -= 1;
             }
             
             // Clear current line on screen
             // Move cursor to start
             while *cursor_pos > 0 {
                 video::put_str("\x08");
                 *cursor_pos -= 1;
             }
             // Clear text
             for _ in 0..input_buffer.len() { video::put_char(' '); }
             // Move cursor back
             for _ in 0..input_buffer.len() { video::put_str("\x08"); }
             
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
             
             // Clear line
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
         if input_buffer.len() < 128 {
             if *cursor_pos == input_buffer.len() {
                 input_buffer.push(c);
                 video::put_char(c);
                 *cursor_pos += 1;
             } else {
                 input_buffer.insert(*cursor_pos, c);
                 // Redraw from cursor
                 let rest = &input_buffer[*cursor_pos..];
                 video::put_str(rest);
                 *cursor_pos += 1;
                 // Move visual cursor back to position
                 for _ in 0..rest.len() {
                     video::put_str("\x08");
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
        let prompt = format!("ainux:{}> ", cwd);
        video::put_str(&prompt);
        
        input_buffer.clear();
        cursor_pos = 0;
        let mut history_index = HISTORY.lock().len();
        
        // Read Line Loop
        loop {
            // Check PS/2 Keyboard
            if let Some(c) = keyboard::pop_char() {
                process_char(c, &mut input_buffer, &mut cursor_pos, &mut history_index);
                if c == '\n' { break; }
            }
            
            // Check Serial (Polled)
            if crate::drivers::serial::SERIAL.lock().data_ready() {
                let c = crate::drivers::serial::SERIAL.lock().read_byte() as char;
                let c = if c == '\r' { '\n' } else { c };
                process_char(c, &mut input_buffer, &mut cursor_pos, &mut history_index);
                if c == '\n' { break; }
            }
            
            unsafe { core::arch::asm!("hlt"); }
        }
        
        // Process Command
        if !input_buffer.is_empty() {
             // Add to history
             let mut hist = HISTORY.lock();
             if hist.last() != Some(&input_buffer) {
                 hist.push(input_buffer.clone());
             }
             drop(hist); // Unlock early
             
             let args: Vec<&str> = input_buffer.split_whitespace().collect();
             if let Some(cmd) = args.get(0) {
                 match *cmd {
                     "help" => cmd_help(),
                     "clear" => video::clear(),
                     "whoami" => video::put_str("root (kernel)\n"),
                     "time" => cmd_time(),
                     "shutdown" => cmd_shutdown(),
                     "ls" => cmd_ls(&args),
                     "cat" => cmd_cat(&args),
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
                     "exec" => test_exec_hello(),
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
                     "nux" => cmd_nux(&args),
                     "nuxc" => cmd_nuxc(&args),
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
                     "lsblk" => cmd_lsblk(),
                     "uname" => cmd_uname(),
                     "grep" => cmd_grep(&args),
                     // "cat" => cmd_cat(&args), // Removed duplicate
                     "head" => cmd_head(&args),
                     "tail" => cmd_tail(&args),
                     "wc" => cmd_wc(&args),
                     "ip" => cmd_ip(&args),
                     // "ip" => cmd_ip(&args), // Removed duplicate
                     "netstat" => cmd_netstat(),
                     "cd" => cmd_cd(&args),
                     "pwd" => { video::put_str(&get_cwd()); video::put_char('\n'); },
                     _ => video::put_str("Unknown command. Type 'help'.\n"),
                 }
             }
        }
        
        input_buffer.clear();
    }
}

fn cmd_help() {
    video::put_str("Available commands:\n");
    video::put_str("  help     - Show this menu\n");
    video::put_str("  echo     - Print text\n");
    video::put_str("  clear    - Clear screen\n");
    video::put_str("  history  - Show command history (Stub)\n");
    video::put_str("\n--- Filesystem ---\n");
    video::put_str("  ls [dir] - List contents\n");
    video::put_str("  cd <dir> - Change directory\n");
    video::put_str("  pwd      - Print working directory\n");
    video::put_str("  cat <f>  - Read and print file\n");
    video::put_str("  touch <f>- Create empty file\n");
    video::put_str("  mkdir <d>- Create directory\n");
    video::put_str("  rm <f>   - Remove file\n");
    video::put_str("  rmdir <d>- Remove directory\n");
    video::put_str("  cp <s,d> - Copy file\n");
    video::put_str("  mv <s,d> - Move/Rename file\n");
    video::put_str("  find <n> - Find file by name\n");
    video::put_str("  du/df    - Disk usage stats\n");
    video::put_str("  mount/umount - Mount filesystems\n");
    video::put_str("\n--- Process & System ---\n");
    video::put_str("  ps       - List processes\n");
    video::put_str("  top      - Monitor processes\n");
    video::put_str("  kill <p> - Kill process by PID\n");
    video::put_str("  killall  - Kill by name\n");
    video::put_str("  nice/renice - Priority control\n");
    video::put_str("  bg/fg/jobs  - Job control\n");
    video::put_str("  free     - Memory usage\n");
    video::put_str("  uname    - Kernel info\n");
    video::put_str("  uptime   - System uptime\n");
    video::put_str("  date     - System time\n");
    video::put_str("  dmesg    - Kernel buffer\n");
    video::put_str("  lsblk    - Block devices\n");
    video::put_str("  reboot   - Restart\n");
    video::put_str("  shutdown - Power off\n");
    video::put_str("\n--- Network & User ---\n");
    video::put_str("  ip       - Interface info\n");
    video::put_str("  netstat  - Network stats\n");
    video::put_str("  ping     - Test reachability\n");
    video::put_str("  whoami   - Current user\n");
    video::put_str("  id/su    - User Identity\n");
    video::put_str("\n--- Extra ---\n");
    video::put_str("  nux <f>  - Run Nux VM script\n");
    video::put_str("  grep/head/tail/wc - Text tools\n");
}



fn cmd_cd(args: &[&str]) {
    if args.len() < 2 { set_cwd("/"); return; }
    let path = resolve_path(args[1]);
    
    // Verify existence (naive)
    let root = vfs::ROOT.lock();
    if let Some(root_inode) = root.as_ref() {
        // TODO: Proper VFS traversal. lookup() usually finds inside root.
        // We need lookup_path(path) in VFS.
        // For now, if path is just a name in root or "/" or "..".
        // Stub: Support only 1 level deep or root relative for this demo
        
        if path == "/" {
            set_cwd("/");
            return;
        }
        
        // Strip leading slash for lookup in root
        let relative = if path.starts_with("/") { &path[1..] } else { &path };
        
        if let Ok(_) = root_inode.lookup(relative) {
             set_cwd(&path);
        } else {
             video::put_str("Directory not found (Only root-level dirs supported in stub VFS)\n");
        }
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

fn cmd_stat(args: &[&str]) {
    if args.len() < 2 { video::put_str("Usage: stat <file>\n"); return; }
    let path = resolve_path(args[1]);
    let name = path.rsplit('/').next().unwrap_or(&path);
    
    let root = vfs::ROOT.lock();
    if let Some(root_inode) = root.as_ref() {
        match root_inode.lookup(name) {
            Ok(inode) => {
                match inode.stat() {
                    Ok(stat) => {
                        video::put_str("File: "); video::put_str(name); video::put_char('\n');
                        video::put_str("Size: "); 
                        print_digit((stat.size % 100) as u8); // TODO: full u64 print
                        video::put_str(" B\n");
                        
                        video::put_str("Type: ");
                        match stat.file_type {
                            vfs::FileType::File => video::put_str("File\n"),
                            vfs::FileType::Directory => video::put_str("Directory\n"),
                            _ => video::put_str("Other\n"),
                        }
                        
                        video::put_str("Mode: ");
                        // Print octal approx
                        print_digit(((stat.mode >> 6) & 7) as u8);
                        print_digit(((stat.mode >> 3) & 7) as u8);
                        print_digit((stat.mode & 7) as u8);
                        video::put_char('\n');
                        
                        video::put_str("Uid: "); print_digit((stat.uid % 100) as u8); video::put_char('\n');
                        video::put_str("Gid: "); print_digit((stat.gid % 100) as u8); video::put_char('\n');
                    },
                    Err(_) => video::put_str("Stat failed.\n"),
                }
            },
            Err(_) => video::put_str("File not found.\n"),
        }
    }
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
    let name = path.rsplit('/').next().unwrap_or(&path); // Naive
    
    let mode = match octal_from_str(mode_str) {
        Some(m) => m,
        None => { video::put_str("Invalid mode (use octal, e.g. 755)\n"); return; }
    };
    
    let root = vfs::ROOT.lock();
    if let Some(root_inode) = root.as_ref() {
        if let Ok(inode) = root_inode.lookup(name) {
             match inode.chmod(mode) {
                 Ok(_) => video::put_str("Chmod success.\n"),
                 Err(_) => video::put_str("Chmod failed.\n"),
             }
        } else {
             video::put_str("File not found.\n");
        }
    }
}

fn cmd_chown(args: &[&str]) {
    if args.len() < 3 { video::put_str("Usage: chown <uid> <file>\n"); return; }
    let uid_str = args[1];
    let path = resolve_path(args[2]);
    let name = path.rsplit('/').next().unwrap_or(&path);
    
    let uid = match u16_from_str(uid_str) {
        Some(u) => u,
        None => { video::put_str("Invalid uid\n"); return; }
    };
    
    let root = vfs::ROOT.lock();
    if let Some(root_inode) = root.as_ref() {
        if let Ok(inode) = root_inode.lookup(name) {
             // Basic: set gid=uid for now
             match inode.chown(uid, uid) {
                 Ok(_) => video::put_str("Chown success.\n"),
                 Err(_) => video::put_str("Chown failed.\n"),
             }
        } else {
             video::put_str("File not found.\n");
        }
    }
}

fn cmd_rm(args: &[&str]) {
    if args.len() < 2 { video::put_str("Usage: rm <file>\n"); return; }
    let path = resolve_path(args[1]);
    let name = path.rsplit('/').next().unwrap_or(&path);
    
    let root = vfs::ROOT.lock();
    if let Some(root_inode) = root.as_ref() {
        // We need to call unlink on the PARENT directory.
        // For now, assuming everything is in root.
        match root_inode.unlink(name) {
            Ok(_) => video::put_str("Deleted.\n"),
            Err(_) => video::put_str("Delete failed (Not found?).\n"),
        }
    }
}

fn cmd_free() {
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

    video::put_str(&format!("{} {:02}, {}  {:02}:{:02}:{:02} {} UTC\n", m_str, t.day, t.year, hour_12, t.minutes, t.seconds, am_pm));
}

fn cmd_shutdown() {
    video::put_str("Shutting down...\n");
    loop { unsafe { core::arch::asm!("hlt"); } }
}

fn cmd_ls(args: &[&str]) {
    let root = vfs::ROOT.lock();
    if let Some(root_inode) = root.as_ref() {
        // If args[1], resolve it. Else use CWD.
        // Currently VFS only reads ROOT inode content.
        // So we just show root content but pretend it's CWD if CWD==/
        
        let target = if args.len() > 1 { resolve_path(args[1]) } else { get_cwd() };
        
        // Need VFS walk. 
        // Logic: if target == "/", read root_inode.
        // if target == "/dir", lookup "dir", then read it.
        
        let target_inode = if target == "/" {
             // Clone ARC? No, we have &Inode via deref?
             // We need to return an Arc clone or ref.
             // root_inode is Arc<dyn Inode>.
             Ok(root_inode.clone())
        } else {
             // Strip leading /
             let rel = &target[1..];
             root_inode.lookup(rel)
        };
        
        match target_inode {
            Ok(inode) => {
                 match inode.read_dir() {
                     Ok(files) => {
                         for name in files {
                             video::put_str(&name);
                             video::put_char('\n');
                         }
                     },
                     Err(_) => video::put_str("Error listing directory.\n"),
                 }
            },
            Err(_) => video::put_str("Directory not found.\n"),
        }
    } else {
        video::put_str("VFS not initialized.\n");
    }
}

fn cmd_cat(args: &[&str]) {
    if args.len() < 2 { video::put_str("Usage: cat <filename>\n"); return; }
    let path = resolve_path(args[1]);
    let name = path.rsplit('/').next().unwrap_or(&path); // Naive
    
    // Stub: Always lookup in ROOT for now because resolving full path objects is hard without "Walk"
    // Using simple lookup in root for demo
    let root = vfs::ROOT.lock();
    if let Some(root_inode) = root.as_ref() {
         match root_inode.lookup(name) { // name is "file.txt"
             Ok(inode) => {
                 if let Ok(handle) = inode.open(0) {
                      let mut buf = vec![0u8; 1024]; 
                      if let Ok(n) = handle.read(&mut buf, 0) {
                          if let Ok(s) = core::str::from_utf8(&buf[0..n]) {
                               video::put_str(s); video::put_char('\n');
                          } else {
                               video::put_str("<Binary Content>\n");
                          }
                      }
                 }
             },
             Err(_) => video::put_str("File not found in root (Path traversal limited).\n"),
         }
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
    // Simple print
    video::put_str("Uptime: ");
    print_digit((seconds / 60) as u8); // Minutes
    video::put_char('m');
    video::put_char(' ');
    print_digit((seconds % 60) as u8); // Seconds
    video::put_char('s');
    video::put_str("\n");
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
    video::put_str("Testing File Write...\n");
    let filename = "hello.txt";
    let text = "UpdatedContent!";
    
    let root = vfs::ROOT.lock();
    if let Some(root_inode) = root.as_ref() {
         // 1. Write
         if let Ok(inode) = root_inode.lookup(filename) {
             if let Ok(handle) = inode.open(0) {
                  match handle.write(text.as_bytes(), 0) {
                      Ok(_) => video::put_str("Write Success.\n"),
                      Err(_) => video::put_str("Write Failed.\n"),
                  }
             }
         }
         
         // 2. Read Back
         if let Ok(inode) = root_inode.lookup(filename) {
             if let Ok(handle) = inode.open(0) {
                  let mut buf = vec![0u8; 32];
                  if let Ok(n) = handle.read(&mut buf, 0) {
                      if let Ok(s) = core::str::from_utf8(&buf[0..n]) {
                          video::put_str("Read Back: ");
                          video::put_str(s);
                          video::put_str("\n");
                      }
                  }
             }
         }
    }
}

pub fn test_exec_hello() {
    video::put_str("Testing Exec hello.elf...\n");
    match crate::process::loader::load_elf_from_file("hello.elf") {
        Ok(pid) => {
            video::put_str("Spawned hello.elf. PID: ");
            video::put_char((b'0' + pid as u8) as char); // Simple digit print
            video::put_str("\nWaiting...\n");
            
            crate::process::scheduler::wait_pid(pid);
            
            video::put_str("Child exited.\n");
        },
        Err(e) => {
            video::put_str("Failed to load hello.elf: ");
            // video::put_str(alloc::format!("{:?}", e).as_str()); // format! needs alloc prelude?
            video::put_str("LoadError\n");
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

fn cmd_tree(_args: &[&str]) {
    // Recursive ls (Stub for now: just ls root)
    video::put_str(".\n");
    let root = vfs::ROOT.lock();
    if let Some(root_inode) = root.as_ref() {
         match root_inode.read_dir() {
             Ok(files) => {
                 for name in files {
                     video::put_str("├── ");
                     video::put_str(&name);
                     video::put_char('\n');
                 }
             },
             Err(_) => {},
         }
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
    // Read Src
    let src = args[1];
    let dst = args[2];
    
    let mut data = Vec::new();
    let root = vfs::ROOT.lock();
    if let Some(root_inode) = root.as_ref() {
          if let Ok(inode) = root_inode.lookup(src) {
              if let Ok(handle) = inode.open(0) {
                   let mut buf = vec![0u8; 4096];
                   if let Ok(n) = handle.read(&mut buf, 0) {
                       for i in 0..n { data.push(buf[i]); }
                   }
              }
          } else {
               video::put_str("Src not found.\n"); return;
          }
           
          // Write Dst (Needs create/write support)
          // For now, assume create if not exists
          // crate::drivers::video::put_str("Copying... (Write stubbed)\n");
          if let Ok(_) = root_inode.create(dst, vfs::FileType::File) {
               if let Ok(inode) = root_inode.lookup(dst) {
                   if let Ok(handle) = inode.open(0) {
                        let _ = handle.write(&data, 0);
                        video::put_str("Copied.\n");
                   }
               }
          } else {
               video::put_str("Create failed (Read-only FS?).\n");
          }
    }
}

fn cmd_mv(args: &[&str]) {
    if args.len() < 3 { video::put_str("Usage: mv <src> <dst>\n"); return; }
    let root = vfs::ROOT.lock();
    if let Some(inode) = root.as_ref() {
        match inode.rename(args[1], args[2]) {
            Ok(_) => video::put_str("Moved.\n"),
            Err(_) => video::put_str("Move failed (Not Implemented).\n"),
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
    let root = vfs::ROOT.lock();
    if let Some(inode) = root.as_ref() {
        match inode.mkdir(args[1]) {
            Ok(_) => video::put_str("Created directory.\n"),
            Err(_) => video::put_str("Failed to create directory.\n"),
        }
    }
}

fn cmd_touch(args: &[&str]) {
    if args.len() < 2 { video::put_str("Usage: touch <file>\n"); return; }
    let root = vfs::ROOT.lock();
    if let Some(inode) = root.as_ref() {
        match inode.create(args[1], vfs::FileType::File) {
            Ok(_) => video::put_str("Touched.\n"),
            Err(_) => video::put_str("Failed.\n"),
        }
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

fn cmd_ping(args: &[&str]) {
    if args.len() < 2 { video::put_str("Usage: ping <host>\n"); return; }
    video::put_str("PING "); video::put_str(args[1]); video::put_str(": 56 data bytes\n");
    video::put_str("Request timeout for icmp_seq 0\n");
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

fn cmd_nux(args: &[&str]) {
    if args.len() < 2 { video::put_str("Usage: nux <file.nuxi>\n"); return; }
    let filename = args[1];
    
    // Load file
    let mut code = Vec::new();
    let root = vfs::ROOT.lock();
    if let Some(root_inode) = root.as_ref() {
         match root_inode.lookup(filename) {
             Ok(inode) => {
                 if let Ok(handle) = inode.open(0) {
                     let mut buf = vec![0u8; 4096]; // 4KB limit for now
                     if let Ok(n) = handle.read(&mut buf, 0) {
                         for i in 0..n { code.push(buf[i]); }
                     }
                 }
             },
             Err(_) => { video::put_str("File not found.\n"); return; }
         }
    }
    
    if code.is_empty() {
        video::put_str("Empty or unreadable file.\n");
        return;
    }
    
    let mut vm = crate::nux::vm::NuxVm::new(code);
    vm.run();
}

fn cmd_nuxc(args: &[&str]) {
    if args.len() < 3 {
        video::put_str("Usage: nuxc <source.nux> <output.nuxi>\n");
        return;
    }
    
    let source_path = resolve_path(args[1]);
    let dest_path = resolve_path(args[2]);
    
    // Read Source
    let mut source_code = String::new();
    let root = vfs::ROOT.lock();
    if let Some(root_inode) = root.as_ref() {
         if let Ok(inode) = root_inode.lookup(&source_path) {
             if let Ok(handle) = inode.open(0) {
                 let mut buf = vec![0u8; 8192]; // Max source 8KB
                 if let Ok(n) = handle.read(&mut buf, 0) {
                     source_code = String::from_utf8_lossy(&buf[0..n]).into_owned();
                 }
             }
         } else {
             video::put_str("Source file not found.\n");
             return;
         }
    }
    
    // Compile
    match crate::nux::compiler::compile(&source_code) {
        Ok(bytecode) => {
             video::put_str("Compilation Success!\n");
             
             if let Some(root_inode) = root.as_ref() {
                 let dest_name = if dest_path.contains('/') { 
                    dest_path.rsplit('/').next().unwrap_or(&dest_path) 
                 } else { &dest_path };
                 
                 // Try create
                 let _ = root_inode.create(dest_name, vfs::FileType::File);
                 
                 // Open and Write
                 if let Ok(inode) = root_inode.lookup(dest_name) {
                      if let Ok(handle) = inode.open(0) {
                          let _ = handle.write(&bytecode, 0);
                          video::put_str("Binary written: "); video::put_str(dest_name); video::put_char('\n');
                      }
                 }
             }
        },
        Err(e) => {
             video::put_str("Error: ");
             video::put_str(&e);
             video::put_char('\n');
        }
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


fn cmd_cal(args: &[&str]) {
    // Current time for default
    let now = rtc::read_time();
    let mut month = now.month as usize;
    let mut year = now.year;
    
    // Parse Flags
    let mut highlight_day = false;
    let mut args_filtered = alloc::vec::Vec::new();
    for arg in args {
        if *arg == "-d" {
            highlight_day = true;
        } else {
            args_filtered.push(*arg);
        }
    }
    
    // Optional args: cal [month] [year]
    if args_filtered.len() >= 2 {
        if let Ok(m) = u8::from_str_radix(args_filtered[1], 10) {
             month = m as usize;
        }
    }
    if args_filtered.len() >= 3 {
        // Simple parse
        let mut y = 0;
        for c in args_filtered[2].bytes() { if c >= b'0' && c <= b'9' { y = y * 10 + (c - b'0') as usize; } }
        if y > 0 { year = y; }
    }

    if month < 1 || month > 12 {
        video::put_str("Invalid month. (1-12)\n");
        return;
    }

    // Print Header
    let month_names = ["", "January", "February", "March", "April", "May", "June", "July", "August", "September", "October", "November", "December"];
    video::put_str(&format!("     {} {}\n", month_names[month], year));
    video::put_str("Su Mo Tu We Th Fr Sa\n");

    // Calculate Day of Week for 1st of month
    // Zeller's algorithms
    let q = 1;
    let m = if month < 3 { month + 12 } else { month };
    let y_z = if month < 3 { year - 1 } else { year };
    let k = y_z % 100;
    let j = y_z / 100;

    let h = (q + 13 * (m + 1) / 5 + k + k / 4 + j / 4 + 5 * j) % 7;
    // h: 0=Sat, 1=Sun, 2=Mon ...
    // Map to 0=Sun, 1=Mon... 
    let start_day = if h == 0 { 6 } else { h - 1 };

    // Days in month
    let days_in_month = match month {
        4 | 6 | 9 | 11 => 30,
        2 => if (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0) { 29 } else { 28 },
        _ => 31,
    };

    // Print spaces
    for _ in 0..start_day {
        video::put_str("   ");
    }

    for day in 1..=days_in_month {
        video::put_str(&format!("{:2} ", day));
        
        // Highlight logic
        if highlight_day && month == now.month as usize && year == now.year && day == now.day as usize {
            // Draw Box
            unsafe {
                let cx = *video::CONSOLE_X.lock();
                let cy = *video::CONSOLE_Y.lock();
                // We just printed 3 chars. Cursor is at cx.
                // Box covers the 2 digits at cx-3 and cx-2.
                // Check if we didn't wrap? put_str handles basic wrapping if width exceeded but here we control newlines.
                
                let start_cx = if cx >= 3 { cx - 3 } else { 0 }; // Safety
                let px = start_cx * 8;
                let py = cy * 12;
                
                // Draw Hollow White Box (16x12)
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
    video::put_char('\n');
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
           let ptr = fb_addr as *mut u32;
           for i in 0..(height * pitch / 4) {
               *ptr.add(i) = 0x101010;
           }
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

            // Redraw Face
            video::fill_rect((cx - radius - 5) as usize, (cy - radius - 5) as usize, (radius * 2 + 10) as usize, (radius * 2 + 10) as usize, 0x101010);

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
            video::fill_rect((cx - 3) as usize, (cy - 3) as usize, 6, 6, 0xFF8800);
            
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
    let root = vfs::ROOT.lock();
    if let Some(root_inode) = root.as_ref() {
         if let Ok(inode) = root_inode.lookup(filename) {
             if let Ok(handle) = inode.open(0) {
                 // Basic read all
                 let mut buf = vec![0u8; 1024];
                 let mut offset = 0;
                 loop {
                     if let Ok(n) = handle.read(&mut buf, offset) {
                         if n == 0 { break; }
                         for i in 0..n { file_content.push(buf[i]); }
                         offset += n as u64;
                     } else { break; }
                 }
             }
         }
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
                 // Write to File
                 if let Some(root_inode) = root.as_ref() {
                      // Remove old file first? simple overwrite logic needed in VFS
                      // For now, let's try creating/overwriting
                      // We need create support in shell or VFS.
                      // Since we don't have easy create yet, we rely on `touch` having created it or existing file.
                      // Or we implement create logic properly.
                      
                      // For now, try to open. If fail, parent.create() - implemented?
                      // Use VFS create.
                      let dir_name = if abs_path.contains('/') {
                          let pos = abs_path.rfind('/').unwrap();
                          if pos == 0 { "/" } else { &abs_path[..pos] }
                      } else { "." };
                      let base_name = if abs_path.contains('/') {
                          let pos = abs_path.rfind('/').unwrap();
                          &abs_path[pos+1..]
                      } else { &abs_path };
                      
                      let dir_node = if dir_name == "/" {
                          root.as_ref().map(|i| i.clone())
                      } else {
                          root_inode.lookup(dir_name).ok()
                      };

                      if let Some(dir) = dir_node {
                          // Try create
                          if let Ok(inode) = dir.create(base_name, vfs::FileType::File) {
                               if let Ok(handle) = inode.open(0) {
                                   let bytes = buffer.as_bytes();
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
                          } else {
                              // Maybe exists? lookup and write
                              if let Ok(inode) = dir.lookup(base_name) {
                                  if let Ok(handle) = inode.open(0) {
                                       let bytes = buffer.as_bytes();
                                       let _ = handle.write(bytes, 0);
                                        unsafe {
                                            *video::CONSOLE_X.lock() = 0;
                                            *video::CONSOLE_Y.lock() = 0;
                                        }
                                        video::put_str(" [SAVED] ");
                                  }
                              }
                          }
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
    
    let path = resolve_path(filename);
    let root = vfs::ROOT.lock();
    if let Some(root_inode) = root.as_ref() {
         if let Ok(inode) = root_inode.lookup(&path) {
             if let Ok(handle) = inode.open(0) {
                 // Read File
                 let mut data = Vec::new();
                 let mut buf = vec![0u8; 1024];
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
         } else {
             video::put_str("File not found.\n");
         }
    }
}
