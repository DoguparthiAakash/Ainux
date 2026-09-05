use std::env;
use std::path::PathBuf;
use std::process::Command;

fn main() {
    let _out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    let out_dir = env::var("OUT_DIR").unwrap();
    let target = env::var("TARGET").unwrap_or_default();

    // Only compile C files for bare-metal kernel target
    if !target.contains("x86_64-unknown-none") && !target.contains("x86_64-elf") {
        return;
    }

    // On Windows: delegate to WSL's gcc cross-compiler
    // On Linux/WSL: use cc::Build directly
    let is_windows = cfg!(target_os = "windows");

    if is_windows {
        build_via_wsl(&out_dir);
    } else {
        build_via_cc(&out_dir);
    }

    println!("cargo:rerun-if-changed=c_src");
}

fn c_source_files() -> Vec<String> {
    let mut files = vec![
        "c_src/gfx.c".to_string(),
        "c_src/font.c".to_string(),
        "c_src/text.c".to_string(),
        "c_src/ps2.c".to_string(),
        "c_src/log.c".to_string(),
        "c_src/libc/ctype.c".to_string(),
        "c_src/libc/stdio.c".to_string(),
        "c_src/libc/stdlib.c".to_string(),
        "c_src/libc/string.c".to_string(),
    ];

    let io_files = [
        "initrd.c", "vfs.c", "axfs.c", "elf.c", "ext2.c", "fat32.c", "gpt.c", "mbr.c", "pipe.c",
    ];
    for f in io_files.iter() {
        files.push(format!("c_src/ex/io/{}", f));
    }

    let driver_files = [
        "ata.c", "keyboard.c", "mouse.c", "pci.c", "pci_msi.c", "rtc.c", "timer.c",
    ];
    for f in driver_files.iter() {
        files.push(format!("c_src/drivers/{}", f));
    }

    files.push("c_src/drivers/usb/xhci.c".to_string());

    let net_files = [
        "arp.c", "dns.c", "ethernet.c", "icmp.c", "ip.c", "netdev.c", "sim_wifi.c", "tcp.c", "udp.c",
    ];
    for f in net_files.iter() {
        files.push(format!("c_src/net/{}", f));
    }

    files.push("c_src/crypto/aes.c".to_string());
    
    // BSD Multi-Kernel C-Integration testing
    // Temporarily disabled due to missing untracked bsd_compat headers
    // files.push("c_src/bsd_external/openbsd/lib/libc/string/explicit_bzero.c".to_string());
    
    // FreeBSD VFS integration
    // files.push("c_src/bsd_external/freebsd/sys/kern/vfs_init.c".to_string());
    
    // NetBSD Networking Source
    // files.push("external/netbsd-src/sys/net/if.c".to_string());
    
    files
}

fn kernel_cflags() -> Vec<&'static str> {
    vec![
        "-ffreestanding",
        "-fno-stack-protector",
        "-fno-pic",
        "-fno-pie",
        "-mno-red-zone",
        "-mno-sse",
        "-mno-sse2",
        "-mcmodel=kernel",
        "-Wno-unused-variable",
        "-Wno-unused-but-set-variable",
        "-Wno-unused-function",
        "-Wno-unused-parameter",
        "-Wno-address-of-packed-member",
        "-nostdlib",
        "-O2",
        "-DDEF_WEAK(x)=", // Ignore OpenBSD weak symbol macros
        "-D__KERNEL_RCSID(x,y)=", // Ignore NetBSD RCS ID macros
        "-D__NetBSD__", // Fake NetBSD for networking code
    ]
}

fn build_via_wsl(out_dir: &str) {
    // Convert Windows path to WSL path
    // e.g. E:\Ainux\... -> /mnt/e/Ainux/...
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    let wsl_manifest = windows_to_wsl_path(&manifest_dir);
    let wsl_out = windows_to_wsl_path(out_dir);

    let sources = c_source_files();
    let flags = kernel_cflags();

    // Check which compilers are available in WSL (prefer cross-compiler)
    let gcc = find_wsl_gcc();

    let mut object_files: Vec<String> = Vec::new();

    let manifest_path = std::path::Path::new(&manifest_dir);

    // BSD Shims
    let bsd_compat_dir = windows_to_wsl_path(manifest_path.join("c_src/bsd_compat/include").to_str().unwrap());
    
    // FreeBSD VFS Source
    let freebsd_sys_dir = windows_to_wsl_path(manifest_path.join("c_src/bsd_external/freebsd/sys").to_str().unwrap());
    let c_dir = manifest_path.join("c_src");
    let c_dir_wsl = windows_to_wsl_path(c_dir.to_str().unwrap());

    // NetBSD Networking Source
    let netbsd_sys_dir = windows_to_wsl_path(manifest_path.join("external/netbsd-src/sys").to_str().unwrap());
    let netbsd_if_c = windows_to_wsl_path(manifest_path.join("external/netbsd-src/sys/net/if.c").to_str().unwrap());

    for src in &sources {
        let src_path = format!("{}/{}", wsl_manifest, src);
        // Object file name: flatten path separator
        let obj_name = src.replace("/", "_").replace(".c", ".o");
        let obj_path = format!("{}/{}", wsl_out, obj_name);
        object_files.push(obj_path.clone());

        let mut args: Vec<String> = vec![gcc.clone()];
        args.extend(flags.iter().map(|s| s.to_string()));
        args.extend([
            format!("-I{}", c_dir_wsl),
            format!("-I{}/libc", c_dir_wsl),
            format!("-I{}/include", wsl_manifest),
            format!("-I{}", bsd_compat_dir),
            format!("-I{}/external/netbsd-src/sys", wsl_manifest),
        ]);
        
        if src.contains("freebsd") {
            args.push(format!("-I{}", freebsd_sys_dir));
        }

        args.extend([
            "-c".to_string(),
            src_path,
            "-o".to_string(),
            obj_path,
        ]);

        let mut bash_cmd = String::new();
        for arg in &args {
            bash_cmd.push_str(&format!("'{}' ", arg));
        }
        let status = Command::new("wsl")
            .args(["-e", "bash", "-c", &bash_cmd])
            .status()
            .unwrap_or_else(|_| panic!("Failed to run wsl gcc for {}", src));

        if !status.success() {
            // Some legacy C files may have errors — warn but don't hard fail
            println!("cargo:warning=C file {} failed to compile (non-fatal)", src);
        }
    }

    // Compile ASM files via NASM in WSL
    let asm_sources = ["arch/x86_64/asm/boot.asm", "arch/x86_64/asm/utils.asm"];
    for src in &asm_sources {
        let src_path = format!("{}/{}", wsl_manifest, src);
        let obj_name = src.replace("/", "_").replace(".asm", ".o");
        let obj_path = format!("{}/{}", wsl_out, obj_name);
        object_files.push(obj_path.clone());

        let status = Command::new("wsl")
            .args(["nasm", "-f", "elf64", &src_path, "-o", &obj_path])
            .status()
            .unwrap_or_else(|_| panic!("Failed to run nasm for {}", src));

        if !status.success() {
            println!("cargo:warning=ASM file {} failed to compile", src);
        }
    }

    // Link all object files into a static library
    let existing_objs: Vec<String> = object_files
        .iter()
        .filter(|o| std::path::Path::new(&wsl_unix_to_windows(o)).exists())
        .cloned()
        .collect();

    if !existing_objs.is_empty() {
        let mut ar_args = vec!["ar".to_string(), "rcs".to_string(), format!("{}/liblegacy_kernel.a", wsl_out)];
        ar_args.extend(existing_objs.clone());
        let status = Command::new("wsl")
            .args(&ar_args)
            .status()
            .expect("Failed to run wsl ar");

        if status.success() {
            println!("cargo:rustc-link-search={}", out_dir);
            println!("cargo:rustc-link-lib=static=legacy_kernel");
        } else {
            println!("cargo:warning=Failed to archive legacy C objects — kernel will use pure Rust");
        }
    } else {
        println!("cargo:warning=No C objects compiled — kernel will use pure Rust drivers");
    }
}

fn build_via_cc(out_dir: &str) {
    let _ = out_dir;
    let mut build = cc::Build::new();
    let sources = c_source_files();
    for src in &sources {
        build.file(src);
    }
    let flags = kernel_cflags();
    for flag in &flags {
        build.flag(flag);
    }
    build.include("c_src")
        .include("c_src/libc")
        .include("include")
        .include("c_src/ex")
        .include("c_src/net")
        .include("c_src/crypto")
        .include("c_src/drivers")
        .include("c_src/drivers/usb")
        .include("c_src/bsd_compat/include")
        // Note: For cc::Build we ideally split builds, but for now we include both.
        // If there are conflicts, we should split cc::Build into two libraries.
        .include("c_src/bsd_external/freebsd/sys")
        .include("external/netbsd-src/sys")
        .compile("legacy_kernel");
}

fn windows_to_wsl_path(windows_path: &str) -> String {
    // E:\Ainux\foo -> /mnt/e/Ainux/foo
    let p = windows_path.replace('\\', "/");
    if p.len() >= 2 && p.chars().nth(1) == Some(':') {
        let drive = p.chars().next().unwrap().to_lowercase().to_string();
        format!("/mnt/{}{}", drive, &p[2..])
    } else {
        p
    }
}

fn wsl_unix_to_windows(wsl_path: &str) -> String {
    // /mnt/e/foo -> e:\foo (best-effort for existence check)
    if wsl_path.starts_with("/mnt/") {
        let rest = &wsl_path[5..];
        let drive = &rest[..1];
        let path = &rest[1..];
        format!("{}:{}", drive.to_uppercase(), path.replace('/', "\\"))
    } else {
        wsl_path.replace('/', "\\")
    }
}

fn find_wsl_gcc() -> String {
    // Prefer x86_64-elf-gcc, fall back to x86_64-linux-gnu-gcc, then gcc
    let candidates = [
        "x86_64-elf-gcc",
        "x86_64-linux-gnu-gcc",
        "gcc",
    ];

    for candidate in candidates.iter() {
        let status = Command::new("wsl")
            .args(["-e", "bash", "-c", &format!("which {}", candidate)])
            .output();
        if let Ok(out) = status {
            if out.status.success() {
                return candidate.to_string();
            }
        }
    }

    // Default fallback
    "x86_64-linux-gnu-gcc".to_string()
}
