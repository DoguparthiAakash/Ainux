mod compiler;
mod vm;
mod lexer;
mod high_level;
mod editor;
mod transpiler;

use std::env;
use std::fs;
use std::io::Write;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        print_usage();
        return;
    }

    let command = &args[1];
    match command.as_str() {
        "build" | "compile" => {
            if args.len() < 3 {
                println!("Usage: nux build <source.nux> [output.nuxi]");
                return;
            }
            let source_path = &args[2];
            let output_path = if args.len() > 3 { &args[3] } else { "out.nuxi" };

            let source = match fs::read_to_string(source_path) {
                Ok(s) => s,
                Err(e) => {
                    println!("Error reading source file: {}", e);
                    return;
                }
            };

            // Try High Level First (heuristic: if contains "print(")
            // Or just try compile_high_level, if error, try asm.
            use lexer::Span;
            use high_level::CompileError;

            let result: Result<Vec<u8>, Vec<CompileError>> = if source.contains("print(") || source.contains(";") {
                 // println!("Compiling as High-Level Nux...");
                 high_level::compile_high_level(&source)
            } else {
                 // println!("Compiling as NuxASM...");
                 compiler::compile(&source).map_err(|e| vec![CompileError { message: e, span: Span { line: 0, col: 0 } }])
            };

            match result {
                Ok(bytes) => {
                    match fs::File::create(output_path) {
                        Ok(mut f) => {
                            if let Err(e) = f.write_all(&bytes) {
                                println!("Error writing output file: {}", e);
                            } else {
                                println!("Successfully compiled to {}", output_path);
                            }
                        },
                        Err(e) => println!("Error creating output file: {}", e),
                    }
                },
                Err(errors) => {
                    println!("❌ Compilation Failed with {} errors:", errors.len());
                    for e in errors {
                        println!("Error: {}", e.message);
                    }
                }
            }
        },
        "run" => {
            if args.len() < 3 {
                println!("Usage: nux run <file.nux or file.nuxi>");
                return;
            }
            let path = &args[2];
            
            // Case 1: Running a compiled binary directly (.nuxi)
            if path.ends_with(".nuxi") {
                match fs::read(path) {
                    Ok(bytes) => {
                        let mut machine = vm::NuxVm::new(bytes);
                        machine.run();
                    },
                    Err(e) => println!("Error reading binary file: {}", e),
                }
                return;
            }

            // Case 2: Running Source Code (.nux) -> Compile & Run with Versioning
            let source_content = match fs::read_to_string(path) {
                Ok(s) => s,
                Err(e) => {
                    println!("Error reading source file: {}", e);
                    return;
                }
            };

            // 1. Compile (Check for Errors)
            println!("Compiling {}...", path);
            use lexer::Span;
            use high_level::CompileError;

            let result: Result<Vec<u8>, Vec<CompileError>> = if source_content.contains("print(") || source_content.contains(";") {
                 high_level::compile_high_level(&source_content)
            } else {
                 compiler::compile(&source_content).map_err(|e| vec![CompileError { message: e, span: Span { line: 0, col: 0 } }])
            };

            let compiled_bytes = match result {
                Ok(b) => b,
                 Err(errors) => {
                    println!("❌ Compilation Failed with {} errors:", errors.len());
                    
                    let lines: Vec<&str> = source_content.lines().collect();

                    for e in errors {
                        println!("Error: {}", e.message);
                        
                        let mut line_num = e.span.line;
                        let mut col_num = e.span.col;
                        
                         // Handle EOF or Out of Bounds
                        if line_num > lines.len() {
                            line_num = lines.len();
                            if line_num > 0 { col_num = lines[line_num - 1].len() + 1; }
                        }
                        
                        if line_num > 0 {
                            let line_str = lines[line_num - 1];
                            println!("  --> {}:{}:{}", path, line_num, col_num);
                            println!("   |");
                            println!("{:3}| {}", line_num, line_str);
                            
                            // Print pointer
                            print!("   | ");
                            for _ in 0..(col_num.saturating_sub(1)) { print!(" "); }
                            println!("^");
                            println!("   |");
                        }
                        println!(""); // Spacer
                    }
                    return; 
                }
            };

            // 2. Prepare Directory structure
            let path_obj = std::path::Path::new(path);
            let stem = path_obj.file_stem().unwrap().to_str().unwrap();
            let dir_name = format!("{}_nux", stem);
            
            if let Err(e) = fs::create_dir_all(&dir_name) {
                println!("Error creating directory {}: {}", dir_name, e);
                return;
            }

            let binary_name = format!("{}.nuxi", stem);
            let binary_path = std::path::Path::new(&dir_name).join(&binary_name);

            // 3. Versioning (Backup old .nuxi if exists)
            if binary_path.exists() {
                let backup_name = format!("{}.v1", binary_name);
                let backup_path = std::path::Path::new(&dir_name).join(backup_name);
                
                if let Err(e) = fs::rename(&binary_path, &backup_path) {
                    println!("Warning: Failed to backup old binary: {}", e);
                } else {
                    println!("Saved previous version to {}", backup_path.display());
                }
            }

            // 4. Write New Binary
            if let Err(e) = fs::write(&binary_path, &compiled_bytes) {
                println!("Error writing new binary: {}", e);
                return;
            }
            println!("✅ Successfully compiled to {}", binary_path.display());

            // 5. Run
            println!("Running...");
            let mut machine = vm::NuxVm::new(compiled_bytes);
            machine.run();
        },
        "edit" => {
             if args.len() < 3 {
                println!("Usage: nux edit <file>");
                return;
             }
             editor::run(&args[2]);
        },
        "translate" => {
             if args.len() < 3 {
                println!("Usage: nux translate <source.nux> [--target c|asm] [--profile std|embedded]");
                return;
             }
             let source_path = &args[2];
             
             // Parse Flags
             let mut target_str = "c";
             let mut profile_str = "std";
             
             let mut i = 3;
             while i < args.len() {
                 match args[i].as_str() {
                     "--target" => {
                         if i+1 < args.len() { target_str = &args[i+1]; i+=1; }
                     },
                     "--profile" => {
                         if i+1 < args.len() { profile_str = &args[i+1]; i+=1; }
                     },
                     _ => {}
                 }
                 i += 1;
             }
             
             let profile = match profile_str {
                 "embedded" => transpiler::TranspileProfile::Embedded,
                 _ => transpiler::TranspileProfile::Standard,
             };
             
             let source = match fs::read_to_string(source_path) {
                Ok(s) => s,
                Err(e) => { println!("Error reading source file: {}", e); return; }
             };
             
             println!("Compiling to ASM...");
             let asm_source = match high_level::compile_to_asm_source(&source) {
                Ok(s) => s,
                Err(errors) => { 
                    println!("Compilation Failed with {} errors:", errors.len()); 
                    for e in errors { println!("Error: {}", e.message); }
                    return; 
                }
             };
             
             match target_str {
                 "c" => {
                     let c_code = transpiler::transpile_to_c(&asm_source, &profile);
                     let out_name = format!("{}.c", source_path);
                     if let Err(e) = fs::write(&out_name, c_code) {
                         println!("Error writing output: {}", e);
                     } else {
                         println!("✅ Generated C source: {}", out_name);
                     }
                 },
                 "asm" => {
                     let out_name = format!("{}.nux.asm", source_path);
                     if let Err(e) = fs::write(&out_name, asm_source) {
                         println!("Error writing output: {}", e);
                     } else {
                         println!("✅ Generated NuxASM source: {}", out_name);
                     }
                 },
                 _ => println!("Unknown target: {}", target_str),
             }
        },
        "build-native" => {
            if args.len() < 3 {
                println!("Usage: nux build-native <source.nux> [output_binary] [--profile std|embedded]");
                return;
            }
            let source_path = &args[2];
            let output_path = if args.len() > 3 && !args[3].starts_with("-") { &args[3] } else { "a.out" };

             let mut profile = transpiler::TranspileProfile::Standard;
             for arg in args.iter() {
                 if arg == "--profile" { /* handled implicitly by next arg check? simple parser needed */ }
                 if arg == "embedded" { profile = transpiler::TranspileProfile::Embedded; } // Hacky argument check
             }
             // robust arg check
             if args.contains(&"--profile".to_string()) {
                 let idx = args.iter().position(|r| r == "--profile").unwrap();
                 if idx + 1 < args.len() && args[idx+1] == "embedded" {
                     profile = transpiler::TranspileProfile::Embedded;
                 }
             }

            let source = match fs::read_to_string(source_path) {
                Ok(s) => s,
                Err(e) => { println!("Error reading source file: {}", e); return; }
            };
            
            println!("Transpiling to ASM...");
            let asm_source = match high_level::compile_to_asm_source(&source) {
                Ok(s) => s,
                Err(errors) => { 
                    println!("High-Level Compilation Failed with {} errors:", errors.len()); 
                    for e in errors {
                        println!("Error: {}", e.message);
                    }
                    return; 
                }
            };
            
            println!("Transpiling to C and Compiling...");
            let config = transpiler::TranspilerConfig {
                target: transpiler::TranspileTarget::C,
                profile: profile,
            };
            
            match transpiler::transpile_and_compile(&asm_source, output_path, &config) {
                Ok(_) => println!("✅ Build Process Completed."),
                Err(e) => println!("❌ Build Failed: {}", e),
            }
        },
        "version" => {
            println!("Nux SDK v0.3.0-portable (JIT Enabled)");
        },
        _ => {
            println!("Unknown command: {}", command);
            print_usage();
        }
    }
}

fn print_usage() {
    println!("Nux Language Portable SDK");
    println!("Usage:");
    println!("  nux build <source.nux> [output.nuxi]  - Compile (Auto-detects HighLevel/ASM)");
    println!("  nux run   <binary.nuxi>               - Run a Nux binary");
    println!("  nux edit  <file>                      - Open IDE/Editor");
    println!("  nux version                           - Show version");
}
