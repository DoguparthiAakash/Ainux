import os
from PIL import Image

TARGET_SIZE = 32

cursors_to_convert = {
    'CURSOR_PTR': 'left_ptr.png',
    'CURSOR_RESIZE_H': 'sb_h_double_arrow.png',
    'CURSOR_RESIZE_V': 'sb_v_double_arrow.png',
    'CURSOR_WAIT': 'wait-01.png',
    'CURSOR_TEXT': 'xterm.png',
}

out_file = "drivers/cursors.rs"
base_dir = "bitmaps/bitmaps/XCursor-Pro-Dark"

with open(out_file, "w") as f:
    f.write("// Auto-generated from PNG cursors\n")
    f.write("pub const CURSOR_SIZE: usize = 32;\n\n")

    for name, filename in cursors_to_convert.items():
        path = os.path.join(base_dir, filename)
        if not os.path.exists(path):
            print(f"File not found: {path}")
            continue
        
        img = Image.open(path).convert("RGBA")
        img = img.resize((TARGET_SIZE, TARGET_SIZE), Image.Resampling.BILINEAR)
        
        f.write(f"pub const {name}: [u32; CURSOR_SIZE * CURSOR_SIZE] = [\n")
        
        pixels = img.load()
        for y in range(TARGET_SIZE):
            f.write("    ")
            for x in range(TARGET_SIZE):
                r, g, b, a = pixels[x, y]
                # Convert to ARGB format expected by our framebuffer
                # Actually, our framebuffer is XRGB (or ARGB) where X is ignored but we can use A for blending later
                # Format: 0xAARRGGBB
                val = (a << 24) | (r << 16) | (g << 8) | b
                f.write(f"0x{val:08X}, ")
            f.write("\n")
            
        f.write("];\n\n")

print(f"Generated {out_file}")
