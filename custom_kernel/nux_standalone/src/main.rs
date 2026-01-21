mod lexer;
mod compiler;
mod assembler;

use std::env;
use std::fs;
use std::process;

fn main() {
    let args: Vec<String> = env::args().collect();
    
    if args.len() < 2 {
        eprintln!("Usage: nux <file.nux>");
        eprintln!("       nux compile <file.nux> <output.nuxi>");
        process::exit(1);
    }
    
    let mode = if args.len() >= 3 && args[1] == "compile" {
        "compile"
    } else {
        "run"
    };
    
    let input_file = if mode == "compile" { &args[2] } else { &args[1] };
    
    // Read source file
    let source = match fs::read_to_string(input_file) {
        Ok(content) => content,
        Err(e) => {
            eprintln!("Error reading file '{}': {}", input_file, e);
            process::exit(1);
        }
    };
    
    println!("Compiling {}...", input_file);
    
    // Compile high-level Nux to assembly
    let asm = match compiler::compile_to_asm_source(&source) {
        Ok(assembly) => assembly,
        Err(errors) => {
            eprintln!("Compilation errors:");
            for err in errors {
                eprintln!("  {}", err);
            }
            process::exit(1);
        }
    };
    
    if mode == "compile" {
        // Assemble to bytecode
        let bytecode = match assembler::compile(&asm) {
            Ok(bc) => bc,
            Err(e) => {
                eprintln!("Assembly error: {}", e);
                process::exit(1);
            }
        };
        
        let output_file = &args[3];
        match fs::write(output_file, bytecode) {
            Ok(_) => println!("Compiled to {}", output_file),
            Err(e) => {
                eprintln!("Error writing output: {}", e);
                process::exit(1);
            }
        }
    } else {
        println!("Compilation successful!");
        println!("\nGenerated Assembly:");
        println!("{}", asm);
    }
}
