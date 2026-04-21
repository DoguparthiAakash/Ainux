use std::env;
use std::path::PathBuf;

fn main() {
    // Get the target directory
    let _out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    
    // Compile C files
    let mut build = cc::Build::new();
    
    // Legacy Core
    build.file("src/c/gfx.c")
         .file("src/c/font.c")
         .file("src/c/text.c")
         .file("src/c/ps2.c")
         .file("src/c/log.c");

    // LibC
    build.file("src/c/libc/ctype.c")
         .file("src/c/libc/stdio.c")
         .file("src/c/libc/stdlib.c")
         .file("src/c/libc/string.c");

    // Legacy IO / VFS
    let io_files = vec![
        "initrd.c", "vfs.c", "axfs.c", "elf.c", "ext2.c", "fat32.c", "gpt.c", "mbr.c", "pipe.c"
    ];
    for f in io_files {
        build.file(format!("src/c/ex/io/{}", f));
    }

    // Legacy Drivers
    let driver_files = vec![
        "ata.c", "keyboard.c", "mouse.c", "pci.c", "pci_msi.c", "rtc.c", "timer.c"
    ];
    for f in driver_files {
        build.file(format!("src/c/drivers/{}", f));
    }

    // USB (inside drivers/usb)
    build.file("src/c/drivers/usb/xhci.c");

    // Network Stack
    let net_files = vec![
        "arp.c", "dns.c", "ethernet.c", "icmp.c", "ip.c", "netdev.c", "sim_wifi.c", "tcp.c", "udp.c"
    ];
    for f in net_files {
        build.file(format!("src/c/net/{}", f));
    }

    // Crypto
    build.file("src/c/crypto/aes.c");

    build.flag("-ffreestanding")
        .flag("-fno-stack-protector")
        .flag("-fno-pic")
        .flag("-mno-red-zone")
        .flag("-mno-sse")
        .flag("-mno-sse2");
    
    // Only apply -mcmodel=kernel if using GCC or Clang (not MSVC)
    if build.get_compiler().is_like_gnu() || build.get_compiler().is_like_clang() {
        build.flag("-mcmodel=kernel");
    }

    build.flag("-Wno-unused-variable")
        .flag("-Wno-unused-but-set-variable")
        .flag("-Wno-unused-function")
        .flag("-Wno-unused-parameter")
        .flag("-Wno-address-of-packed-member")
        .flag("-nostdlib")
        .flag("-O2") // Optimization
        .include("src/c")
        .include("src/c/libc")
        .include("src/c/include")
        .include("src/c/ex")
        .include("src/c/net")
        .include("src/c/crypto")
        .include("src/c/drivers") // Drivers root for drivers/ata.h
        .include("src/c/drivers/usb")
        // .warnings(false) // Maybe suppress warnings for legacy code?
        .compile("legacy_kernel");
    
    // Re-run triggers
    println!("cargo:rerun-if-changed=src/c");
}
