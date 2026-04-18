#!/usr/bin/env python3
import sys
import re

# NUX Assembly to Native LLVM IR Cross-Compiler

def emit_header(f):
    f.write("; NUX Native LLVM IR Compiler\n")
    f.write("target datalayout = \"e-m:e-p270:32:32-p271:32:32-p272:64:64-i64:64-f80:128-n8:16:32:64-S128\"\n")
    f.write("target triple = \"x86_64-unknown-none-elf\"\n\n")
    
    # Global Stack
    f.write("@vm_stack = global [1024 x i64] zeroinitializer\n")
    f.write("@vm_stack_ptr = global i64 0\n\n")

def emit_push(f, val, inst_idx):
    f.write(f"  %sp_val_{inst_idx} = load i64, ptr @vm_stack_ptr\n")
    f.write(f"  %sp_ptr_{inst_idx} = getelementptr [1024 x i64], ptr @vm_stack, i64 0, i64 %sp_val_{inst_idx}\n")
    f.write(f"  store i64 {val}, ptr %sp_ptr_{inst_idx}\n")
    f.write(f"  %sp_next_{inst_idx} = add i64 %sp_val_{inst_idx}, 1\n")
    f.write(f"  store i64 %sp_next_{inst_idx}, ptr @vm_stack_ptr\n")

def emit_pop(f, reg_name):
    f.write(f"  %sp_curr_{reg_name} = load i64, ptr @vm_stack_ptr\n")
    f.write(f"  %sp_prev_{reg_name} = sub i64 %sp_curr_{reg_name}, 1\n")
    f.write(f"  store i64 %sp_prev_{reg_name}, ptr @vm_stack_ptr\n")
    f.write(f"  %sp_ptr_{reg_name} = getelementptr [1024 x i64], ptr @vm_stack, i64 0, i64 %sp_prev_{reg_name}\n")
    f.write(f"  %{reg_name} = load i64, ptr %sp_ptr_{reg_name}\n")

def emit_add(f, inst_idx):
    emit_pop(f, f"b_{inst_idx}")
    emit_pop(f, f"a_{inst_idx}")
    f.write(f"  %res_{inst_idx} = add i64 %a_{inst_idx}, %b_{inst_idx}\n")
    # Push back
    f.write(f"  %sp_val_{inst_idx} = load i64, ptr @vm_stack_ptr\n")
    f.write(f"  %sp_ptr_{inst_idx} = getelementptr [1024 x i64], ptr @vm_stack, i64 0, i64 %sp_val_{inst_idx}\n")
    f.write(f"  store i64 %res_{inst_idx}, ptr %sp_ptr_{inst_idx}\n")
    f.write(f"  %sp_next_{inst_idx} = add i64 %sp_val_{inst_idx}, 1\n")
    f.write(f"  store i64 %sp_next_{inst_idx}, ptr @vm_stack_ptr\n")

def emit_sys_write(f, inst_idx):
    # Syscall 1 (fd=1)
    # Print the character popped from stack
    emit_pop(f, f"char_{inst_idx}")
    # Write to a transient local array so we can pass address to syscall
    f.write(f"  %buf_{inst_idx} = alloca i8, align 1\n")
    f.write(f"  %tr_{inst_idx} = trunc i64 %char_{inst_idx} to i8\n")
    f.write(f"  store i8 %tr_{inst_idx}, ptr %buf_{inst_idx}\n")
    # Syscall: id=1, a1=1(stdout), a2=buf, a3=1(len)
    f.write(f"  %buf_i64_{inst_idx} = ptrtoint ptr %buf_{inst_idx} to i64\n")
    f.write(f"  call void asm sideeffect \"syscall\", \"{{rax}},{{rdi}},{{rsi}},{{rdx}},~{{rcx}},~{{r11}},~{{memory}}\"(i64 1, i64 1, i64 %buf_i64_{inst_idx}, i64 1)\n")

def parse_and_compile(input_file, output_file):
    with open(input_file, 'r') as fin:
        lines = fin.readlines()

    with open(output_file, 'w') as fout:
        emit_header(fout)
        fout.write("define i64 @_start() {\n")
        fout.write("entry:\n")

        inst_idx = 0
        for line in lines:
            line = line.strip()
            if not line or line.startswith('#') or line.startswith('//'):
                continue
            
            # Simple label parsing
            if line.endswith(':'):
                fout.write(f"  br label %{line[:-1]}\n")
                fout.write(f"{line[:-1]}:\n")
                continue

            parts = line.split()
            op = parts[0].upper()

            if op == 'PUSH':
                emit_push(fout, parts[1], inst_idx)
            elif op == 'POP':
                emit_pop(fout, f"ignore_{inst_idx}")
            elif op == 'ADD':
                emit_add(fout, inst_idx)
            elif op == 'PRINT_CHAR':
                emit_sys_write(fout, inst_idx)
            elif op == 'EXIT':
                emit_pop(fout, f"exit_code_{inst_idx}")
                fout.write(f"  call void asm sideeffect \"syscall\", \"{{rax}},{{rdi}},~{{rcx}},~{{r11}},~{{memory}}\"(i64 60, i64 %exit_code_{inst_idx})\n")
                # LLVM requires block terminator
                fout.write(f"  unreachable\n")
            elif op == 'JMP':
                fout.write(f"  br label %{parts[1]}\n")
            else:
                print(f"Warning: Unsupported instruction {op}")

            inst_idx += 1

        # Default end
        fout.write("  ret i64 0\n")
        fout.write("}\n")

if __name__ == "__main__":
    if len(sys.argv) < 3:
        print("Usage: nux_llvm.py <input.nux> <output.ll>")
        sys.exit(1)
    parse_and_compile(sys.argv[1], sys.argv[2])
    print(f"Hybrid compilation to {sys.argv[2]} successful.")
