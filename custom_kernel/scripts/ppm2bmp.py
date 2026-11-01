import sys
import struct

def ppm_to_bmp(ppm_path, bmp_path):
    with open(ppm_path, "rb") as f:
        header = b""
        while True:
            line = f.readline()
            if not line:
                break
            if line.strip().startswith(b"#"):
                continue
            header += line
            parts = header.split()
            if len(parts) >= 4:
                break
        w = int(parts[1])
        h = int(parts[2])
        data = f.read(w * h * 3)

    row_stride = (w * 3 + 3) & ~3
    bmp_data_size = row_stride * h
    file_size = 54 + bmp_data_size

    header = struct.pack("<2sIHHI", b"BM", file_size, 0, 0, 54) + \
             struct.pack("<IIIHHIIIIII", 40, w, h, 1, 24, 0, bmp_data_size, 0, 0, 0, 0)

    rows = []
    pad = b"\x00" * (row_stride - w * 3)
    for y in range(h - 1, -1, -1):
        row_bytes = bytearray()
        row_slice = data[y * w * 3 : (y + 1) * w * 3]
        for x in range(w):
            r = row_slice[x * 3]
            g = row_slice[x * 3 + 1]
            b = row_slice[x * 3 + 2]
            row_bytes.extend((b, g, r))
        row_bytes.extend(pad)
        rows.append(bytes(row_bytes))

    with open(bmp_path, "wb") as f:
        f.write(header)
        f.write(b"".join(rows))

if __name__ == "__main__":
    ppm_to_bmp(sys.argv[1], sys.argv[2])
