#!/usr/bin/env python3
import sys
import struct
import re

# OpCodes Map (Must match VM)
OPCODES = {
    'PUSH': 0x01,
    'POP': 0x02,
    'ADD': 0x10,
    'SUB': 0x11,
    'DRAW_RECT': 0x20,
    'DRAW_IMG': 0x21,
    'PEEK': 0x40,
    'POKE': 0x41,
    'PEEK8': 0x42,  # Peek Byte
    'POKE8': 0x43,  # Poke Byte
    'JMP': 0x60,    # Unconditional Jump
    'JE': 0x61,     # Jump if Equal (Pop A, Pop B)
    'CALL': 0x70,   # Call Function
    'RET': 0x71,    # Return
    'KERNEL_OP': 0x80, # Kernel Operation (Pop ID)
    'SLEEP': 0x30,
    'EXIT': 0xFF
}

class NuxCompiler:
    def __init__(self):
        self.bytecode = bytearray()
        self.labels = {} # "LabelName" -> ByteOffset
        self.patches = [] # (Offset, LabelName) tuple

    def compile(self, source):
        # Pre-process: Handle Imports
        lines = []
        for line in source.split('\n'):
            line = line.strip()
            if line.startswith('#include'):
                # Simple include (files in same dir or lib/)
                filename = line.split()[1].strip('"')
                # Auto-append .nuxi if missing? User should provide full name.
                try:
                    with open(filename, 'r') as f:
                        lines.extend(f.read().split('\n'))
                except:
                     # Try lib/
                     try:
                        with open(f"lib/{filename}", 'r') as f:
                             lines.extend(f.read().split('\n'))
                     except:
                        print(f"Warning: Could not include {filename}")
            else:
                lines.append(line)

        # Pass 1: Code Gen
        for line in lines:
            line = line.strip()
            if not line or line.startswith('//'): continue
            
            # Label Definition: "label_name:"
            if line.endswith(':'):
                label_name = line[:-1]
                self.labels[label_name] = len(self.bytecode)
                continue
                
            if line.startswith('fn '):
                # Function syntax: "fn name() {"
                # Treat as label "name"
                name = line.split()[1].split('(')[0]
                self.labels[name] = len(self.bytecode)
                continue
            
            if line == '}':
                # End of function => RET
                self.emit_op('RET')
                continue

            if 'draw_rect' in line:
                args = re.findall(r'0x[0-9A-Fa-f]+|\d+', line)
                if len(args) == 5:
                    c, h, w, y, x = args
                    self.emit_push(int(x))
                    self.emit_push(int(y))
                    self.emit_push(int(w))
                    self.emit_push(int(h))
                    self.emit_push(int(c, 16) if '0x' in c else int(c))
                    self.emit_op('DRAW_RECT')
                    
            elif line.startswith('*'):
                # POKE
                parts = line.split('=')
                if len(parts) == 2:
                    addr_s = parts[0].strip().replace('*', '')
                    val_s = parts[1].strip().replace(';', '')
                    self.emit_push(int(addr_s, 16) if '0x' in addr_s else int(addr_s))
                    self.emit_push(int(val_s, 16) if '0x' in val_s else int(val_s))
                    self.emit_op('POKE')

            elif line.startswith('kernel_op'):
                # kernel_op(1);
                arg = line.split('(')[1].split(')')[0]
                self.emit_push(int(arg))
                self.emit_op('KERNEL_OP')

            elif line.startswith('call '):
                # call <label>;
                target = line.split()[1].strip(';')
                self.emit_op('CALL')
                self.patches.append((len(self.bytecode), target))
                self.bytecode += b'\x00\x00\x00\x00\x00\x00\x00\x00' # Placeholder

            elif line.startswith('jmp '):
                target = line.split()[1].strip(';')
                self.emit_op('JMP')
                self.patches.append((len(self.bytecode), target))
                self.bytecode += b'\x00\x00\x00\x00\x00\x00\x00\x00'

            elif line.startswith('exit'):
                self.emit_op('EXIT')
                
        # Pass 2: Patch Labels
        for offset, label in self.patches:
            if label not in self.labels:
                print(f"Error: Undefined label '{label}'")
                continue
                
            addr = self.labels[label]
            # Write 64-bit address at offset
            self.bytecode[offset:offset+8] = struct.pack('<q', addr)
        
        print(f"DEBUG: Labels: {self.labels}")

    def emit_push(self, val):
        self.bytecode += struct.pack('B', OPCODES['PUSH'])
        self.bytecode += struct.pack('<q', val)

    def emit_op(self, op_name):
        self.bytecode += struct.pack('B', OPCODES[op_name])

    def get_binary(self):
        # Header
        header = b'ANUX'
        header += struct.pack('<H', 1)
        header += b'\x00' * 58
        return header + self.bytecode

def main():
    if len(sys.argv) < 3:
        print("Usage: nuxc.py <input.nux> <output.anux>")
        return

    with open(sys.argv[1], 'r') as f:
        source = f.read()

    compiler = NuxCompiler()
    compiler.compile(source)
    
    with open(sys.argv[2], 'wb') as f:
        f.write(compiler.get_binary())
        
    print(f"Compiled {sys.argv[2]}")

if __name__ == "__main__":
    main()
