use alloc::string::String;
use alloc::vec::Vec;
use crate::drivers::video;
use crate::fs::vfs::FileType;

// =========================================================================
//  NUXC — Ainux Native C Compiler  (Phase 4 implementation)
//
//  Pipeline:
//    source.c ─► Lexer ─► Parser ─► CodeGen ─► x86_64 Assembly ─► ELF
//
//  Supported C subset:
//    • int / void functions  (single main() supported)
//    • return <int literal>;
//    • puts("…");  / write(str);
//    • int <name> = <int_literal>;  (local vars via stack)
//    • if (<cond>) { … }  — using ==, !=, <, >
//    • while (<cond>) { … }
//    • Inline comments // …
// =========================================================================

// ── Tokens ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
enum Tok {
    Ident(String),
    IntLit(i64),
    StrLit(String),
    Punct(char),
    Kw(Kw),
    Op(String),
    Eof,
}

#[derive(Debug, Clone, PartialEq)]
enum Kw {
    Int, Void, Return, If, Else, While, Puts, Write, Printf,
}

fn lex(src: &str) -> Vec<Tok> {
    let mut toks = Vec::new();
    let bytes = src.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        let b = bytes[i];
        // Skip whitespace
        if b.is_ascii_whitespace() { i += 1; continue; }
        // Line comments
        if i + 1 < bytes.len() && b == b'/' && bytes[i+1] == b'/' {
            while i < bytes.len() && bytes[i] != b'\n' { i += 1; }
            continue;
        }
        // Block comments
        if i + 1 < bytes.len() && b == b'/' && bytes[i+1] == b'*' {
            i += 2;
            while i + 1 < bytes.len() && !(bytes[i] == b'*' && bytes[i+1] == b'/') { i += 1; }
            i += 2;
            continue;
        }
        // String literals
        if b == b'"' {
            i += 1;
            let mut s = String::new();
            while i < bytes.len() && bytes[i] != b'"' {
                if bytes[i] == b'\\' && i + 1 < bytes.len() {
                    match bytes[i+1] {
                        b'n' => { s.push('\n'); i += 2; },
                        b't' => { s.push('\t'); i += 2; },
                        b'"' => { s.push('"'); i += 2; },
                        b'\\' => { s.push('\\'); i += 2; },
                        _ => { s.push(bytes[i+1] as char); i += 2; },
                    }
                } else {
                    s.push(bytes[i] as char);
                    i += 1;
                }
            }
            if i < bytes.len() { i += 1; } // closing "
            toks.push(Tok::StrLit(s));
            continue;
        }
        // Integer literals
        if b.is_ascii_digit() || (b == b'-' && i + 1 < bytes.len() && bytes[i+1].is_ascii_digit()) {
            let neg = b == b'-';
            if neg { i += 1; }
            let start = i;
            while i < bytes.len() && bytes[i].is_ascii_digit() { i += 1; }
            let n: i64 = core::str::from_utf8(&bytes[start..i]).unwrap_or("0").parse().unwrap_or(0);
            toks.push(Tok::IntLit(if neg { -n } else { n }));
            continue;
        }
        // Identifiers and keywords
        if b.is_ascii_alphabetic() || b == b'_' {
            let start = i;
            while i < bytes.len() && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'_') { i += 1; }
            let word = core::str::from_utf8(&bytes[start..i]).unwrap_or("");
            let tok = match word {
                "int"    => Tok::Kw(Kw::Int),
                "void"   => Tok::Kw(Kw::Void),
                "return" => Tok::Kw(Kw::Return),
                "if"     => Tok::Kw(Kw::If),
                "else"   => Tok::Kw(Kw::Else),
                "while"  => Tok::Kw(Kw::While),
                "puts"   => Tok::Kw(Kw::Puts),
                "write"  => Tok::Kw(Kw::Write),
                "printf" => Tok::Kw(Kw::Printf),
                _        => Tok::Ident(String::from(word)),
            };
            toks.push(tok);
            continue;
        }
        // Two-char operators
        if i + 1 < bytes.len() {
            let two = &bytes[i..i+2];
            let op = match two {
                b"==" => Some("=="),
                b"!=" => Some("!="),
                b"<=" => Some("<="),
                b">=" => Some(">="),
                _ => None,
            };
            if let Some(o) = op {
                toks.push(Tok::Op(String::from(o)));
                i += 2;
                continue;
            }
        }
        // Single-char punctuation / operators
        toks.push(Tok::Punct(b as char));
        i += 1;
    }
    toks.push(Tok::Eof);
    toks
}

// ── Simple recursive descent parser + inline code generator ───────────────

struct Compiler {
    toks: Vec<Tok>,
    pos: usize,
    asm: String,
    data: String,
    str_count: usize,
    lbl_count: usize,
    /// local var map: (name, stack_offset_from_rbp)
    locals: Vec<(String, i64)>,
    stack_depth: i64,
}

impl Compiler {
    fn new(toks: Vec<Tok>) -> Self {
        Self {
            toks,
            pos: 0,
            asm: String::new(),
            data: String::new(),
            str_count: 0,
            lbl_count: 0,
            locals: Vec::new(),
            stack_depth: 0,
        }
    }

    fn peek(&self) -> &Tok { &self.toks[self.pos] }

    fn next(&mut self) -> &Tok {
        let t = &self.toks[self.pos];
        if self.pos + 1 < self.toks.len() { self.pos += 1; }
        t
    }

    fn expect_punct(&mut self, c: char) {
        match self.peek().clone() {
            Tok::Punct(p) if p == c => { self.next(); },
            got => {
                // Skip silently — best-effort parsing
                let _ = got;
            }
        }
    }

    fn new_label(&mut self) -> String {
        let n = self.lbl_count;
        self.lbl_count += 1;
        alloc::format!(".Lacc_{}", n)
    }

    fn emit(&mut self, line: &str) {
        self.asm.push_str(line);
        self.asm.push('\n');
    }

    fn add_string(&mut self, s: &str) -> (String, usize) {
        let lbl = alloc::format!(".Lstr{}", self.str_count);
        let len_lbl = alloc::format!(".Lstr{}_len", self.str_count);
        self.str_count += 1;
        // Escape the string for AT&T asm
        let mut escaped = String::new();
        let mut actual_len = 0;
        for c in s.chars() {
            match c {
                '\n' => { escaped.push_str("\\n"); actual_len += 1; },
                '"'  => { escaped.push_str("\\\""); actual_len += 1; },
                '\t' => { escaped.push_str("\\t"); actual_len += 1; },
                '\\'  => { escaped.push_str("\\\\"); actual_len += 1; },
                other => { escaped.push(other); actual_len += other.len_utf8(); },
            }
        }
        self.data.push_str(&alloc::format!("{}:\n    .ascii \"{}\"\n{} = . - {}\n",
            lbl, escaped, len_lbl, lbl));
        (lbl, actual_len)
    }

    // Push integer value in RAX onto logical stack
    fn alloc_local(&mut self, name: &str) -> i64 {
        self.stack_depth += 8;
        let offset = -self.stack_depth;
        self.locals.push((String::from(name), offset));
        offset
    }

    fn local_offset(&self, name: &str) -> Option<i64> {
        for (n, off) in self.locals.iter().rev() {
            if n == name { return Some(*off); }
        }
        None
    }

    // ── Expression: returns value in %rax ───────────────────────────────
    fn parse_expr(&mut self) {
        match self.peek().clone() {
            Tok::IntLit(n) => {
                self.next();
                self.emit(&alloc::format!("    movq ${}, %rax", n));
            }
            Tok::StrLit(s) => {
                let s_clone = s.clone();
                self.next();
                let (lbl, _) = self.add_string(&s_clone);
                self.emit(&alloc::format!("    leaq {}(%rip), %rax", lbl));
            }
            Tok::Ident(name) => {
                let name_clone = name.clone();
                self.next();
                if let Some(off) = self.local_offset(&name_clone) {
                    self.emit(&alloc::format!("    movq {}(%rbp), %rax", off));
                } else {
                    self.emit("    xor %rax, %rax");  // undefined var → 0
                }
            }
            Tok::Punct('(') => {
                self.next();
                self.parse_expr();
                self.expect_punct(')');
            }
            _ => {
                self.emit("    xor %rax, %rax");
            }
        }
    }

    // ── Condition: evaluates to ZF/CF set (je/jne/jl/jg usable) ────────
    fn parse_condition(&mut self) -> String {
        self.parse_expr();
        self.emit("    movq %rax, %rcx");
        let op_tok = self.peek().clone();
        let jmp_if_false = match &op_tok {
            Tok::Op(op) => {
                let j = match op.as_str() {
                    "==" => "jne",
                    "!=" => "je",
                    "<"  => "jge",
                    ">"  => "jle",
                    "<=" => "jg",
                    ">=" => "jl",
                    _    => "jne",
                };
                self.next();
                self.parse_expr();
                self.emit("    cmpq %rax, %rcx");
                j
            }
            Tok::Punct('<') => {
                self.next();
                self.parse_expr();
                self.emit("    cmpq %rax, %rcx");
                "jge"
            }
            Tok::Punct('>') => {
                self.next();
                self.parse_expr();
                self.emit("    cmpq %rax, %rcx");
                "jle"
            }
            _ => {
                // Treat non-zero as true
                self.emit("    testq %rax, %rax");
                "je"
            }
        };
        String::from(jmp_if_false)
    }

    // ── Statement ────────────────────────────────────────────────────────
    fn parse_stmt(&mut self) {
        match self.peek().clone() {
            Tok::Kw(Kw::Return) => {
                self.next();
                if *self.peek() != Tok::Punct(';') {
                    self.parse_expr();
                } else {
                    self.emit("    xor %rax, %rax");
                }
                self.expect_punct(';');
                self.emit("    movq %rax, %rdi");
                self.emit("    movq $60, %rax");
                self.emit("    syscall");
            }
            Tok::Kw(Kw::Puts) | Tok::Kw(Kw::Write) | Tok::Kw(Kw::Printf) => {
                self.next();
                self.expect_punct('(');
                match self.peek().clone() {
                    Tok::StrLit(s) => {
                        let s_clone = s.clone();
                        self.next();
                        let (lbl, actual_len) = self.add_string(&s_clone);
                        self.emit("    movq $1, %rax");        // sys_write
                        self.emit("    movq $1, %rdi");        // stdout
                        self.emit(&alloc::format!("    leaq {}(%rip), %rsi", lbl));
                        self.emit(&alloc::format!("    movq ${}, %rdx", actual_len));
                        self.emit("    syscall");
                    }
                    Tok::Ident(name) => {
                        let name_clone = name.clone();
                        self.next();
                        if let Some(off) = self.local_offset(&name_clone) {
                            self.emit(&alloc::format!("    movq {}(%rbp), %rsi", off));
                        } else {
                            self.emit("    xor %rsi, %rsi");
                        }
                        self.emit("    movq $1, %rax");
                        self.emit("    movq $1, %rdi");
                        self.emit("    movq $128, %rdx");  // guess length
                        self.emit("    syscall");
                    }
                    _ => {}
                }
                self.expect_punct(')');
                self.expect_punct(';');
            }
            Tok::Kw(Kw::Int) => {
                self.next();
                if let Tok::Ident(name) = self.peek().clone() {
                    let name_clone = name.clone();
                    self.next();
                    let off = self.alloc_local(&name_clone);
                    if *self.peek() == Tok::Punct('=') {
                        self.next();
                        self.parse_expr();
                        self.emit(&alloc::format!("    movq %rax, {}(%rbp)", off));
                    }
                    self.expect_punct(';');
                }
            }
            Tok::Ident(name) => {
                let name_clone = name.clone();
                self.next();
                if *self.peek() == Tok::Punct('=') {
                    self.next();
                    self.parse_expr();
                    if let Some(off) = self.local_offset(&name_clone) {
                        self.emit(&alloc::format!("    movq %rax, {}(%rbp)", off));
                    }
                    self.expect_punct(';');
                } else {
                    // function call or expression statement — skip to ;
                    while *self.peek() != Tok::Punct(';') && *self.peek() != Tok::Eof {
                        self.next();
                    }
                    self.expect_punct(';');
                }
            }
            Tok::Kw(Kw::If) => {
                self.next();
                self.expect_punct('(');
                let jmp_false = self.parse_condition();
                self.expect_punct(')');
                let else_lbl = self.new_label();
                let end_lbl = self.new_label();
                self.emit(&alloc::format!("    {} {}", jmp_false, else_lbl));
                self.expect_punct('{');
                self.parse_block();
                if *self.peek() == Tok::Kw(Kw::Else) {
                    self.next();
                    self.emit(&alloc::format!("    jmp {}", end_lbl));
                    self.emit(&alloc::format!("{}:", else_lbl));
                    self.expect_punct('{');
                    self.parse_block();
                    self.emit(&alloc::format!("{}:", end_lbl));
                } else {
                    self.emit(&alloc::format!("{}:", else_lbl));
                }
            }
            Tok::Kw(Kw::While) => {
                self.next();
                let loop_lbl = self.new_label();
                let end_lbl = self.new_label();
                self.emit(&alloc::format!("{}:", loop_lbl));
                self.expect_punct('(');
                let jmp_false = self.parse_condition();
                self.expect_punct(')');
                self.emit(&alloc::format!("    {} {}", jmp_false, end_lbl));
                self.expect_punct('{');
                self.parse_block();
                self.emit(&alloc::format!("    jmp {}", loop_lbl));
                self.emit(&alloc::format!("{}:", end_lbl));
            }
            Tok::Punct('{') => {
                self.next();
                self.parse_block();
            }
            Tok::Punct(';') => { self.next(); }
            Tok::Eof => {}
            _ => {
                // skip unknown token
                self.next();
            }
        }
    }

    fn parse_block(&mut self) {
        while *self.peek() != Tok::Punct('}') && *self.peek() != Tok::Eof {
            self.parse_stmt();
        }
        self.expect_punct('}');
    }

    fn parse_func(&mut self) {
        // Skip return type
        match self.peek() {
            Tok::Kw(Kw::Int) | Tok::Kw(Kw::Void) => { self.next(); }
            _ => {}
        }
        // Function name
        let fn_name = if let Tok::Ident(n) = self.peek().clone() {
            let n = n.clone();
            self.next();
            n
        } else {
            self.next();
            String::from("unknown")
        };

        // Args
        self.expect_punct('(');
        while *self.peek() != Tok::Punct(')') && *self.peek() != Tok::Eof {
            self.next();
        }
        self.expect_punct(')');

        // Emit function prologue
        self.emit(&alloc::format!(".global {}", fn_name));
        self.emit(&alloc::format!("{}:", fn_name));
        self.emit("    pushq %rbp");
        self.emit("    movq %rsp, %rbp");
        // Reserve 128 bytes for locals (simplified)
        self.emit("    subq $128, %rsp");

        self.expect_punct('{');
        self.parse_block();

        // Default exit if no explicit return
        self.emit("    movq $60, %rax");
        self.emit("    xor %rdi, %rdi");
        self.emit("    syscall");
        self.emit("    leave");
        self.emit("    ret");
        self.emit("");
    }

    fn compile(&mut self) -> String {
        // Parse all top-level declarations / functions
        while *self.peek() != Tok::Eof {
            match self.peek() {
                Tok::Kw(Kw::Int) | Tok::Kw(Kw::Void) => self.parse_func(),
                _ => { self.next(); }
            }
        }

        let mut out = String::new();
        // .data section first
        if !self.data.is_empty() {
            out.push_str(".section .data\n");
            out.push_str(&self.data);
            out.push('\n');
        }
        // .text section
        out.push_str(".section .text\n");
        out.push_str(".global _start\n");
        out.push_str("_start:\n");
        out.push_str("    call main\n");
        out.push_str("    movq $60, %rax\n");
        out.push_str("    xor %rdi, %rdi\n");
        out.push_str("    syscall\n\n");
        out.push_str(&self.asm);
        out
    }
}

// ── ELF writer: produces a minimal ELF64 from flat x86_64 machine code ───

fn write_elf64(code: &[u8], target: &str) -> bool {
    write_elf64_pub(code, target)
}

pub fn write_elf64_pub(code: &[u8], target: &str) -> bool {
    let load_addr: u64 = 0x400000;
    let text_offset: u64 = 0x78;  // after ELF header + 2 PHDRs

    let file_size = text_offset + code.len() as u64;

    let mut buf: Vec<u8> = Vec::with_capacity(file_size as usize);

    // ELF Header (64 bytes)
    buf.extend_from_slice(b"\x7fELF");
    buf.push(2); // 64-bit
    buf.push(1); // little-endian
    buf.push(1); // ELF version 1
    buf.push(0); // System V ABI
    buf.extend_from_slice(&[0u8; 8]); // padding
    buf.extend_from_slice(&2u16.to_le_bytes()); // ET_EXEC
    buf.extend_from_slice(&0x3Eu16.to_le_bytes()); // x86-64
    buf.extend_from_slice(&1u32.to_le_bytes()); // version
    buf.extend_from_slice(&(load_addr + text_offset).to_le_bytes()); // entry
    buf.extend_from_slice(&64u64.to_le_bytes()); // phoff
    buf.extend_from_slice(&0u64.to_le_bytes()); // shoff
    buf.extend_from_slice(&0u32.to_le_bytes()); // flags
    buf.extend_from_slice(&64u16.to_le_bytes()); // ehsize
    buf.extend_from_slice(&56u16.to_le_bytes()); // phentsize
    buf.extend_from_slice(&1u16.to_le_bytes()); // phnum
    buf.extend_from_slice(&64u16.to_le_bytes()); // shentsize
    buf.extend_from_slice(&0u16.to_le_bytes()); // shnum
    buf.extend_from_slice(&0u16.to_le_bytes()); // shstrndx

    // Program header (56 bytes)
    buf.extend_from_slice(&1u32.to_le_bytes()); // PT_LOAD
    buf.extend_from_slice(&5u32.to_le_bytes()); // PF_R | PF_X
    buf.extend_from_slice(&0u64.to_le_bytes()); // offset
    buf.extend_from_slice(&load_addr.to_le_bytes()); // vaddr
    buf.extend_from_slice(&load_addr.to_le_bytes()); // paddr
    buf.extend_from_slice(&file_size.to_le_bytes()); // filesz
    buf.extend_from_slice(&file_size.to_le_bytes()); // memsz
    buf.extend_from_slice(&0x200000u64.to_le_bytes()); // align 2MB

    // Pad to text_offset
    while buf.len() < text_offset as usize {
        buf.push(0);
    }

    // Code
    buf.extend_from_slice(code);

    // Write to filesystem — try given target first, then fallback to CWD
    let paths_to_try = [
        alloc::string::String::from(target),
        // If target has a /, just use the basename in root
        {
            let basename = target.rsplit('/').next().unwrap_or(target);
            alloc::string::String::from(basename)
        },
    ];

    for try_path in paths_to_try.iter() {
        match crate::shell::find_parent_and_name(try_path) {
            Ok((parent, name)) => {
                // Create or open file
                let inode = match parent.lookup(&name) {
                    Ok(n) => n,
                    Err(_) => match parent.create(&name, FileType::File) {
                        Ok(n) => n,
                        Err(e) => {
                            video::put_str(&alloc::format!("[nuxc] Cannot create '{}': {:?}\n", try_path, e));
                            continue;
                        }
                    },
                };
                match inode.open(0) {
                    Ok(h) => {
                        let _ = h.truncate();
                        match h.write(&buf, 0) {
                            Ok(n) => {
                                if try_path != target {
                                    video::put_str(&alloc::format!("[nuxc] Note: wrote to '{}' instead of '{}'\n", try_path, target));
                                }
                                video::put_str(&alloc::format!("[nuxc] ELF written: {} bytes to '{}'\n", n, try_path));
                                return true;
                            }
                            Err(e) => {
                                video::put_str(&alloc::format!("[nuxc] Write failed for '{}': {:?}\n", try_path, e));
                                continue;
                            }
                        }
                    }
                    Err(_) => {
                        video::put_str(&alloc::format!("[nuxc] Cannot open '{}' for writing\n", try_path));
                        continue;
                    }
                }
            }
            Err(e) => {
                video::put_str(&alloc::format!("[nuxc] Path '{}' not found: {:?}\n", try_path, e));
                continue;
            }
        }
    }
    false
}

// ── x86_64 in-kernel assembler (AT&T syntax, single-pass, labels) ─────────
// This is a minimal subset assembler that handles the output of our codegen.

fn encode_imm64(val: i64) -> [u8; 8] {
    val.to_le_bytes()
}

fn assemble_asm(asm: &str) -> Vec<u8> {
    assemble_asm_pub(asm)
}

pub fn assemble_asm_pub(asm: &str) -> Vec<u8> {
    let mut code: Vec<u8> = Vec::new();
    // Label table: (name, offset)
    let mut labels: Vec<(String, u64)> = Vec::new();
    // Fixups: (offset_in_code, label_name, is_32bit_rel)
    let mut fixups: Vec<(usize, String, bool)> = Vec::new();

    for raw_line in asm.lines() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') || line.starts_with(".section") {
            continue;
        }

        // Label definition
        if line.ends_with(':') && !line.starts_with('.') {
            let lbl = &line[..line.len()-1];
            labels.push((String::from(lbl), code.len() as u64));
            continue;
        }
        if line.starts_with(".L") && line.ends_with(':') {
            let lbl = &line[..line.len()-1];
            labels.push((String::from(lbl), code.len() as u64));
            continue;
        }

        // .global / .ascii etc — skip
        if line.starts_with('.') {
            // Handle .ascii for data embedding
            if line.starts_with(".ascii ") {
                let s = &line[7..];
                let s = s.trim_matches('"');
                // Very simple: push chars
                for b in s.bytes() {
                    code.push(b);
                }
            }
            continue;
        }

        // Parse instruction
        let (mnem, rest) = if let Some(p) = line.find(' ') {
            (&line[..p], line[p..].trim())
        } else {
            (line, "")
        };

        match mnem {
            "nop" => code.push(0x90),
            "ret" => code.push(0xC3),
            "leave" => code.push(0xC9),
            "syscall" => { code.push(0x0F); code.push(0x05); }
            "pushq" => {
                let op = parse_operand(rest);
                match op {
                    Operand::Reg(r) => code.push(0x50 + reg_idx(r)),
                    Operand::Imm(n) => {
                        if n >= -128 && n <= 127 {
                            code.push(0x6A);
                            code.push(n as i8 as u8);
                        } else {
                            code.push(0x68);
                            code.extend_from_slice(&(n as i32).to_le_bytes());
                        }
                    }
                    _ => {}
                }
            }
            "popq" => {
                let op = parse_operand(rest);
                if let Operand::Reg(r) = op {
                    code.push(0x58 + reg_idx(r));
                }
            }
            "movq" => {
                let (src, dst) = split_two(rest);
                let src_op = parse_operand(src);
                let dst_op = parse_operand(dst);
                match (src_op, dst_op) {
                    (Operand::Imm(n), Operand::Reg(r)) => {
                        // REX.W + MOV r64, imm64 : 0x48 0xB8+r imm64
                        code.push(0x48);
                        code.push(0xB8 + reg_idx(r));
                        code.extend_from_slice(&encode_imm64(n));
                    }
                    (Operand::Reg(s), Operand::Reg(d)) => {
                        // REX.W + MOV r/m64, r64 : 0x48 0x89 ModRM
                        code.push(0x48); code.push(0x89);
                        code.push(0xC0 | (reg_idx(s) << 3) | reg_idx(d));
                    }
                    (Operand::Mem(reg, disp), Operand::Reg(d)) => {
                        // MOV r64, [rbp+disp]
                        code.push(0x48); code.push(0x8B);
                        if disp >= -128 && disp <= 127 {
                            code.push(0x40 | (reg_idx(d) << 3) | reg_idx(reg));
                            code.push(disp as i8 as u8);
                        } else {
                            code.push(0x80 | (reg_idx(d) << 3) | reg_idx(reg));
                            code.extend_from_slice(&(disp as i32).to_le_bytes());
                        }
                    }
                    (Operand::Reg(s), Operand::Mem(reg, disp)) => {
                        // MOV [rbp+disp], r64
                        code.push(0x48); code.push(0x89);
                        if disp >= -128 && disp <= 127 {
                            code.push(0x40 | (reg_idx(s) << 3) | reg_idx(reg));
                            code.push(disp as i8 as u8);
                        } else {
                            code.push(0x80 | (reg_idx(s) << 3) | reg_idx(reg));
                            code.extend_from_slice(&(disp as i32).to_le_bytes());
                        }
                    }
                    _ => {}
                }
            }
            "xor" => {
                let (src, dst) = split_two(rest);
                if let (Operand::Reg(s), Operand::Reg(d)) = (parse_operand(src), parse_operand(dst)) {
                    code.push(0x48); code.push(0x31);
                    code.push(0xC0 | (reg_idx(s) << 3) | reg_idx(d));
                }
            }
            "cmpq" => {
                let (src, dst) = split_two(rest);
                let s_op = parse_operand(src);
                let d_op = parse_operand(dst);
                match (s_op, d_op) {
                    (Operand::Reg(s), Operand::Reg(d)) => {
                        code.push(0x48); code.push(0x39);
                        code.push(0xC0 | (reg_idx(s) << 3) | reg_idx(d));
                    }
                    _ => {}
                }
            }
            "testq" => {
                let (s_str, d_str) = split_two(rest);
                if let (Operand::Reg(s), Operand::Reg(d)) = (parse_operand(s_str), parse_operand(d_str)) {
                    code.push(0x48); code.push(0x85);
                    code.push(0xC0 | (reg_idx(s) << 3) | reg_idx(d));
                }
            }
            "subq" => {
                let (src, dst) = split_two(rest);
                match (parse_operand(src), parse_operand(dst)) {
                    (Operand::Imm(n), Operand::Reg(d)) => {
                        if n >= 0 && n <= 127 {
                            code.push(0x48); code.push(0x83); code.push(0xE8 | reg_idx(d)); code.push(n as u8);
                        } else {
                            code.push(0x48); code.push(0x81); code.push(0xE8 | reg_idx(d));
                            code.extend_from_slice(&(n as i32).to_le_bytes());
                        }
                    }
                    _ => {}
                }
            }
            "addq" => {
                let (src, dst) = split_two(rest);
                match (parse_operand(src), parse_operand(dst)) {
                    (Operand::Imm(n), Operand::Reg(d)) => {
                        if n >= 0 && n <= 127 {
                            code.push(0x48); code.push(0x83); code.push(0xC0 | reg_idx(d)); code.push(n as u8);
                        } else {
                            code.push(0x48); code.push(0x81); code.push(0xC0 | reg_idx(d));
                            code.extend_from_slice(&(n as i32).to_le_bytes());
                        }
                    }
                    _ => {}
                }
            }
            "leaq" => {
                // leaq label(%rip), %reg — RIP-relative LEA
                let (src, dst) = split_two(rest);
                if let Operand::Reg(d) = parse_operand(dst) {
                    // Extract label from "label(%rip)"
                    if let Some(p) = src.find('(') {
                        let lbl_name = &src[..p];
                        // REX.W + LEA r64, [RIP+rel32]  : 0x48 0x8D ModRM(00_reg_101) rel32
                        code.push(0x48); code.push(0x8D);
                        code.push((reg_idx(d) << 3) | 5);
                        // placeholder rel32 — fixup later
                        let fixup_pos = code.len();
                        code.extend_from_slice(&0i32.to_le_bytes());
                        fixups.push((fixup_pos, String::from(lbl_name), true));
                    }
                }
            }
            "call" => {
                let lbl_name = rest.trim();
                code.push(0xE8);
                let fixup_pos = code.len();
                code.extend_from_slice(&0i32.to_le_bytes());
                fixups.push((fixup_pos, String::from(lbl_name), true));
            }
            "jmp" => {
                let lbl_name = rest.trim();
                code.push(0xE9);
                let fixup_pos = code.len();
                code.extend_from_slice(&0i32.to_le_bytes());
                fixups.push((fixup_pos, String::from(lbl_name), true));
            }
            "je" | "jz" => encode_cond_jmp(&mut code, &mut fixups, rest, 0x84),
            "jne" | "jnz" => encode_cond_jmp(&mut code, &mut fixups, rest, 0x85),
            "jl" => encode_cond_jmp(&mut code, &mut fixups, rest, 0x8C),
            "jg" => encode_cond_jmp(&mut code, &mut fixups, rest, 0x8F),
            "jle" => encode_cond_jmp(&mut code, &mut fixups, rest, 0x8E),
            "jge" => encode_cond_jmp(&mut code, &mut fixups, rest, 0x8D),
            _ => {}
        }
    }

    // Apply fixups
    let base: u64 = 0x400000 + 0x78; // load addr + text offset
    for (pos, lbl, _) in &fixups {
        let target_offset = labels.iter().find(|(n, _)| n == lbl).map(|(_, o)| *o).unwrap_or(0);
        // RIP-relative: target = base + target_offset; fixup_site = base + pos + 4
        let rip_at_end = base + (*pos as u64) + 4;
        let target_abs = base + target_offset;
        let rel = target_abs.wrapping_sub(rip_at_end) as i32;
        code[*pos..*pos+4].copy_from_slice(&rel.to_le_bytes());
    }

    code
}

fn encode_cond_jmp(code: &mut Vec<u8>, fixups: &mut Vec<(usize, String, bool)>, rest: &str, opcode2: u8) {
    let lbl = rest.trim();
    code.push(0x0F);
    code.push(opcode2);
    let pos = code.len();
    code.extend_from_slice(&0i32.to_le_bytes());
    fixups.push((pos, String::from(lbl), true));
}

#[derive(Debug)]
enum Operand<'a> {
    Reg(&'a str),
    Imm(i64),
    Mem(&'a str, i64),  // (base_reg, displacement)
    Lbl(&'a str),
}

fn reg_idx(name: &str) -> u8 {
    match name {
        "rax" | "%rax" => 0,
        "rcx" | "%rcx" => 1,
        "rdx" | "%rdx" => 2,
        "rbx" | "%rbx" => 3,
        "rsp" | "%rsp" => 4,
        "rbp" | "%rbp" => 5,
        "rsi" | "%rsi" => 6,
        "rdi" | "%rdi" => 7,
        _ => 0,
    }
}

fn parse_operand(s: &str) -> Operand {
    let s = s.trim();
    // Register
    if s.starts_with('%') {
        return Operand::Reg(&s[1..]);
    }
    // Immediate: $N
    if s.starts_with('$') {
        let n_str = s[1..].trim();
        let n: i64 = n_str.parse().unwrap_or(0);
        return Operand::Imm(n);
    }
    // Memory: disp(%reg) or (%reg)
    if s.contains('(') {
        let p = s.find('(').unwrap_or(0);
        let disp_str = &s[..p];
        let reg_str = &s[p+1..s.len().saturating_sub(1)];
        let disp: i64 = if disp_str.is_empty() { 0 } else { disp_str.parse().unwrap_or(0) };
        let reg = if reg_str.starts_with('%') { &reg_str[1..] } else { reg_str };
        return Operand::Mem(reg, disp);
    }
    // Label
    Operand::Lbl(s)
}

fn split_two(s: &str) -> (&str, &str) {
    // Split on first comma, trimming spaces
    if let Some(p) = s.find(',') {
        (s[..p].trim(), s[p+1..].trim())
    } else {
        (s, "")
    }
}

// ── Public API ────────────────────────────────────────────────────────────

pub fn cmd_nuxc(args: &[&str]) {
    if args.len() < 2 {
        video::put_str("Usage: nuxc <source.c> [-o output.elf]\n");
        video::put_str("       gcc|cc|clang <source.c> [-o output.elf]\n");
        video::put_str("Ainux Native C Compiler (acc) — Supports: int/void fns, return,\n");
        video::put_str("  puts(\"..\"), if/else, while, int locals, ==, !=, <, >, <=, >=\n");
        return;
    }

    let source_file = args[1];
    let output_file = if args.len() >= 4 && args[2] == "-o" {
        args[3]
    } else {
        "a.elf"   // write to CWD by default (safer than /bin/a.elf)
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
            video::put_str(&alloc::format!("nuxc: Cannot open '{}'\n", source_file));
            return;
        }
    }

    if source.is_empty() {
        video::put_str("nuxc: Source file is empty.\n");
        return;
    }

    video::put_str(&alloc::format!("[nuxc] Compiling {} ...\n", source_file));

    // 1. Lex
    let toks = lex(&source);
    video::put_str(&alloc::format!("[nuxc] Lexed {} tokens\n", toks.len()));

    // 2. Parse + Codegen → AT&T Assembly
    let mut comp = Compiler::new(toks);
    let asm_text = comp.compile();
    video::put_str(&alloc::format!("[nuxc] Generated {} bytes of assembly\n", asm_text.len()));

    // Optionally save .s file for inspection
    let asm_path = alloc::format!("{}.s", source_file);
    if let Ok((parent, name)) = crate::shell::find_parent_and_name(&asm_path) {
        let _ = match parent.lookup(&name) {
            Ok(n) => Ok(n),
            Err(_) => parent.create(&name, FileType::File),
        }.map(|inode| {
            if let Ok(h) = inode.open(0) {
                let _ = h.truncate();
                let _ = h.write(asm_text.as_bytes(), 0);
            }
        });
        video::put_str(&alloc::format!("[nuxc] Assembly saved to {}\n", asm_path));
    }

    // 3. Assemble → machine code
    let machine_code = assemble_asm(&asm_text);
    video::put_str(&alloc::format!("[nuxc] Assembled {} bytes of machine code\n", machine_code.len()));

    if machine_code.is_empty() {
        video::put_str("[nuxc] Error: No code generated. Check your source.\n");
        return;
    }

    // 4. Write ELF64
    if write_elf64(&machine_code, output_file) {
        video::put_str(&alloc::format!("[nuxc] Done! Binary written to '{}'\n", output_file));
        video::put_str(&alloc::format!("       Run with: exec {}\n", output_file));
    } else {
        video::put_str("[nuxc] Error: Could not write output ELF.\n");
    }
}

pub fn compile_via_host(_source_code: &str, _target_file: &str) {
    video::put_str("nuxc: Host compilation disabled. Use 'nuxc' for native compilation.\n");
}
