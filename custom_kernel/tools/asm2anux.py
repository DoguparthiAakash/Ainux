#!/usr/bin/env python3
import struct
import sys

# OpCodes Map
OPCODES = {
    'PUSH': 0x01,
    'POP': 0x02,
    'ADD': 0x10,
    'SUB': 0x11,
    'DRAW_RECT': 0x20,
    'DRAW_IMG': 0x21,
    'WAIT': 0x30,
    'EXIT': 0xFF
}

def compile_line(line):
    # Remove inline comments
    if ';' in line:
        line = line.split(';')[0]
    
    parts = line.split()
    if not parts: return b''
    
    op_name = parts[0].upper()
    if op_name not in OPCODES:
        print(f"Unknown OpCode: {op_name}")
        return b''
        
    op = OPCODES[op_name]
    bytecode = struct.pack('B', op)
    
    # Handle Arguments (currently only Little Endian i64)
    for arg in parts[1:]:
        if arg.startswith('0x'):
            val = int(arg, 16)
        else:
            val = int(arg)
        bytecode += struct.pack('<q', val) # 64-bit int
        
    return bytecode

def main():
    if len(sys.argv) < 3:
        print("Usage: asm2anux.py <input.asm> <output.anux>")
        return

    with open(sys.argv[1], 'r') as f:
        lines = f.readlines()

    program = b''
    # Header (Magic + Version + Padding)
    program += b'ANUX'
    program += struct.pack('<H', 1) # v1
    program += b'\x00' * 58 # Padding to 64 bytes

    for line in lines:
        line = line.strip()
        if not line or line.startswith(';'): continue
        program += compile_line(line)

    with open(sys.argv[2], 'wb') as f:
        f.write(program)
    
    print(f"Compiled {sys.argv[2]} ({len(program)} bytes).")

if __name__ == "__main__":
    main()
