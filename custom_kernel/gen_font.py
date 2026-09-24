import urllib.request
import re
import sys

url = "https://raw.githubusercontent.com/torvalds/linux/master/lib/fonts/font_8x16.c"
content = urllib.request.urlopen(url).read().decode('utf-8')

hex_vals = re.findall(r"(0x[0-9a-fA-F]{2})", content)

# skip { 0, 0, FONTDATAMAX, 0 } which is {0x0, 0x0, 0x1000, 0x0} so it might be encoded differently. Wait! In the downloaded file it was:
# { 0, 0, FONTDATAMAX, 0 }, {
# Let's just find all 0x... and take the last 4096 values!
if len(hex_vals) < 4096:
    print(f"Not enough hex values found. Found {len(hex_vals)}")
    sys.exit(1)

hex_vals = hex_vals[-4096:]

with open("c_src/font.c", "w") as f:
    f.write("#include \"font.h\"\n\n")
    f.write("const unsigned char font_8x16[256][16] = {\n")
    for i in range(256):
        f.write("    {")
        f.write(", ".join(hex_vals[i*16:(i+1)*16]))
        f.write("},\n")
    f.write("};\n")

print("Generated font.c")
