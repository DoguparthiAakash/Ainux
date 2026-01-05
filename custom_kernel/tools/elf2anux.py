
import struct
import sys
from elftools.elf.elffile import ELFFile
from elftools.elf.constants import SH_FLAGS

# struct AnuxHeader {
#     magic: [u8; 4] = "ANUX"
#     version: u32 = 1
#     entry_point: u64
#     code_size: u64
#     data_size: u64
#     stack_size: u64
#     flags: u64
#     checksum: u64
#     reserved: [u8; 16]
# }
# Total: 64 bytes

def convert(elf_path, anux_path):
    with open(elf_path, 'rb') as f:
        elf = ELFFile(f)
        
        # 1. Find Text and Data
        text_section = elf.get_section_by_name('.text')
        data_section = elf.get_section_by_name('.data')
        rodata_section = elf.get_section_by_name('.rodata')
        
        if not text_section:
            print("Error: No .text section found")
            return

        entry_point = elf.header['e_entry']
        code_data = text_section.data()
        
        # Combine data and rodata
        const_data = b''
        if rodata_section:
            const_data += rodata_section.data()
        if data_section:
            const_data += data_section.data()
            
        # Align everything to 4096 bytes for simplicity in v1
        # In a real format we'd map segments properly. 
        # For this "Tiny VM" / Simple Loader:
        # We just concat CODE + DATA.
        
        # Header
        magic = b'ANUX'
        version = 1
        code_len = len(code_data)
        data_len = len(const_data)
        stack_size = 16 * 1024 # 16KB default
        flags = 0
        checksum = 0 # TODO
        reserved = b'\x00' * 16
        
        header = struct.pack('<4sIQMQQQQ16s', 
            magic, version, entry_point, 
            code_len, data_len, stack_size, 
            flags, checksum, reserved
        )
        
        with open(anux_path, 'wb') as out:
            out.write(header)
            out.write(code_data)
            out.write(const_data)
            
        print(f"Converted {elf_path} -> {anux_path}")
        print(f"Entry: {hex(entry_point)}")
        print(f"Code: {code_len} bytes")
        print(f"Data: {data_len} bytes")

if __name__ == '__main__':
    if len(sys.argv) != 3:
        print("Usage: elf2anux.py <input.elf> <output.anux>")
        sys.exit(1)
    convert(sys.argv[1], sys.argv[2])
