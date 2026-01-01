from PIL import Image
import struct
import math
import os

def create_vid(output_file, width=160, height=120, fps=10, duration_sec=10):
    images = []
    # Load available images
    for name in ["initrd/libs/image.png", "initrd/libs/image.bmp", "initrd/libs/FyVZv.png"]:
        if os.path.exists(name):
            try:
                img = Image.open(name).convert("RGB")
                images.append(img)
                print(f"Loaded {name}: {img.size}")
            except Exception as e:
                print(f"Failed to load {name}: {e}")
                
    if not images:
        print("No images found to create video!")
        return

    frames = fps * duration_sec
    
    with open(output_file, "wb") as f:
        # Header
        f.write(b"VID1")
        f.write(struct.pack("<HHIH", width, height, frames, fps))
        
        # Animate
        # Cycle through images
        frames_per_img = frames // len(images)
        
        for idx, img in enumerate(images):
            # Ken Burns Effect
            # Start: Center, Scale 1.0
            # End: Pan to random corner, Scale 1.2
            # Or simple pan
            
            iw, ih = img.size
            aspect = iw / ih
            target_aspect = width / height
            
            # Crop to cover
            if aspect > target_aspect:
                 # Image wider
                 nh = ih
                 nw = int(ih * target_aspect)
                 ox = (iw - nw) // 2
                 oy = 0
            else:
                 # Image taller
                 nw = iw
                 nh = int(iw / target_aspect)
                 ox = 0
                 oy = (ih - nh) // 2
                 
            # Base Crop
            base_img = img.crop((ox, oy, ox+nw, oy+nh))
            
            # Generate frames
            count = frames_per_img
            if idx == len(images) - 1:
                count = frames - (frames_per_img * idx)
                
            for i in range(count):
                t = i / count
                # Zoom in slightly: 1.0 -> 1.2
                scale = 1.0 + (0.2 * t)
                
                cw = int(nw / scale)
                ch = int(nh / scale)
                
                # Pan Center
                cx = nw // 2
                cy = nh // 2
                
                # TopLeft of crop
                x = cx - cw // 2
                y = cy - ch // 2
                
                # Crop from base
                frame_img = base_img.crop((x, y, x+cw, y+ch))
                # Resize to target
                frame_img = frame_img.resize((width, height), Image.Resampling.BILINEAR)
                
                # Write RGB
                f.write(frame_img.tobytes())
                
    print(f"Generated {output_file} from {len(images)} images.")

if __name__ == "__main__":
    create_vid("initrd/libs/video.vid", 160, 120, 10, 10)
