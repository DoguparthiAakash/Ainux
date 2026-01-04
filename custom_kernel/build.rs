use std::env;
use std::path::PathBuf;

fn main() {
    // Get the target directory
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    
    // Compile C files
    cc::Build::new()
        .file("src/c/gfx.c")
        .file("src/c/font.c")
        .file("src/c/text.c")
        .file("src/c/ps2.c")
        .flag("-ffreestanding")
        .flag("-fno-stack-protector")
        .flag("-fno-pic")
        .flag("-mno-red-zone")
        .flag("-mno-sse")
        .flag("-mno-sse2")
        .flag("-mcmodel=kernel")
        .flag("-nostdlib")
        .flag("-O2")
        .include("src/c")
        .compile("legacy_gfx");
    
    println!("cargo:rerun-if-changed=src/c/gfx.c");
    println!("cargo:rerun-if-changed=src/c/font.c");
    println!("cargo:rerun-if-changed=src/c/gfx.h");
    println!("cargo:rerun-if-changed=src/c/font.h");
}
