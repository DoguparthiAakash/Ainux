import struct
import sys

def main():
    filepath = "cursor/Twilight-cursors/cursors/default"
    outpath = "kernel/cursor_icon.h"
    
    with open(filepath, "rb") as f:
        # Header
        magic = f.read(4)
        if magic != b'Xcur':
            print("Not an Xcursor file")
            return
        
        f.read(4) # Header size
        f.read(4) # Version
        ntoc = struct.unpack('<I', f.read(4))[0]
        
        print(f"TOC Entries: {ntoc}")
        
        best_size = 0
        best_pos = 0
        
        # Read TOC
        for i in range(ntoc):
            chunk_type = struct.unpack('<I', f.read(4))[0]
            subtype = struct.unpack('<I', f.read(4))[0] # Size usually
            pos = struct.unpack('<I', f.read(4))[0]
            
            if chunk_type == 0xfffd0002: # Image
                size = subtype
                print(f"Found Image: Size {size} at {pos}")
                
                # Prefer 32x32, or 24x24
                if size == 32:
                    best_size = size
                    best_pos = pos
                    break
                if best_size == 0 or (abs(size - 32) < abs(best_size - 32)):
                    best_size = size
                    best_pos = pos
        
        if best_pos == 0:
            print("No image found!")
            return
            
        print(f"Selected Size: {best_size} at {best_pos}")
        f.seek(best_pos)
        
        # Image Header
        hsize = struct.unpack('<I', f.read(4))[0]
        type_ = struct.unpack('<I', f.read(4))[0]
        subtype = struct.unpack('<I', f.read(4))[0]
        version = struct.unpack('<I', f.read(4))[0]
        width = struct.unpack('<I', f.read(4))[0]
        height = struct.unpack('<I', f.read(4))[0]
        xhot = struct.unpack('<I', f.read(4))[0]
        yhot = struct.unpack('<I', f.read(4))[0]
        delay = struct.unpack('<I', f.read(4))[0]
        
        print(f"W={width} H={height} Hot=({xhot},{yhot})")
        
        # Pixels
        # Xcursor stores them as ARGB (0xAARRGGBB)
        # Verify: Are they pre-multiplied? Usually yes.
        # We can just copy them.
        
        pixels = []
        for i in range(width * height):
            pixel = struct.unpack('<I', f.read(4))[0]
            pixels.append(pixel)
            
        # Write Header
        with open(outpath, "w") as out:
            out.write("#ifndef CURSOR_ICON_H\n")
            out.write("#define CURSOR_ICON_H\n\n")
            out.write("#include <stdint.h>\n\n")
            out.write(f"#define CURSOR_WIDTH {width}\n")
            out.write(f"#define CURSOR_HEIGHT {height}\n")
            out.write(f"#define CURSOR_X_HOT {xhot}\n")
            out.write(f"#define CURSOR_Y_HOT {yhot}\n\n")
            out.write("static const uint32_t cursor_icon[] = {\n")
            
            for i, p in enumerate(pixels):
                if i % 8 == 0:
                    out.write("    ")
                out.write(f"0x{p:08X}, ")
                if i % 8 == 7:
                    out.write("\n")
            
            out.write("\n};\n\n")
            out.write("#endif\n")

if __name__ == "__main__":
    main()
