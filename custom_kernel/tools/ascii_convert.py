import sys

try:
    from PIL import Image
except ImportError:
    print("PIL is not installed. Please install it using 'pip install Pillow'")
    sys.exit(1)

def image_to_ascii(image_path, width=30):
    try:
        img = Image.open(image_path).convert('L')
    except Exception as e:
        print(f"Error opening image: {e}")
        return

    # Calculate height to preserve aspect ratio, adjusting for font height/width ratio
    aspect_ratio = img.height / img.width
    height = int(width * aspect_ratio * 0.5)
    img = img.resize((width, height))

    # ASCII characters from darker to lighter
    ascii_chars = ["█", "▓", "▒", "░", "≡", "=", "+", ":", "-", ".", " "]
    
    pixels = img.getdata()
    ascii_str = ""
    for pixel in pixels:
        ascii_str += ascii_chars[pixel // 25]

    for i in range(0, len(ascii_str), width):
        sys.stdout.buffer.write((ascii_str[i:i+width] + '\n').encode('utf-8'))

if __name__ == "__main__":
    if len(sys.argv) > 1:
        image_to_ascii(sys.argv[1])
    else:
        print("Usage: python ascii_convert.py <image_path>")
