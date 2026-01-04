import struct
import math

def create_vid(filename="video.vid", width=320, height=240, frames=60, fps=10):
    # Header: "VID1", w(2), h(2), frames(4), fps(2)
    with open(filename, "wb") as f:
        f.write(b"VID1")
        f.write(struct.pack("<HHIH", width, height, frames, fps))
        
        # Generate Frames
        for i in range(frames):
            # Create a frame (RGB)
            # Example: Moving vertical bars + Bouncing Ball
            data = bytearray(width * height * 3)
            
            t = i / frames
            
            # Background: Gradient
            r_base = int(128 + 127 * math.sin(t * 6.28))
            
            # Ball
            bx = int(width/2 + (width/2 - 20) * math.sin(t * 10))
            by = int(height/2 + (height/2 - 20) * math.cos(t * 8))
            br = 20
            
            for y in range(height):
                for x in range(width):
                    idx = (y * width + x) * 3
                    
                    # Gradient BG
                    data[idx] = (x * 255) // width
                    data[idx+1] = (y * 255) // height
                    data[idx+2] = r_base
                    
                    # Ball
                    dx = x - bx
                    dy = y - by
                    if dx*dx + dy*dy < br*br:
                        data[idx] = 255
                        data[idx+1] = 255
                        data[idx+2] = 0
            
            f.write(data)
            print(f"Frame {i+1}/{frames}")

if __name__ == "__main__":
    create_vid("initrd/libs/video.vid", 160, 120, 60, 10)
    print("Done.")
