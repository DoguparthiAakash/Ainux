import struct
import subprocess
import os
import sys

def convert_mp4_to_vid(input_file, output_file, width=160, height=120, fps=10):
    print(f"Converting {input_file} to {output_file}...")
    
    if not os.path.exists(input_file):
        print(f"Error: {input_file} not found.")
        return

    # FFmpeg command to extract raw RGB frames
    # -i input
    # -vf scale=w:h,fps=fps
    # -f rawvideo
    # -pix_fmt rgb24
    # -
    cmd = [
        "ffmpeg",
        "-i", input_file,
        "-vf", f"scale={width}:{height},fps={fps}",
        "-f", "rawvideo",
        "-pix_fmt", "rgb24",
        "-"
    ]
    
    try:
        # Run ffmpeg
        process = subprocess.Popen(cmd, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
        
        # Read all stdout (raw frames)
        # Warning: Large files might consume memory, but 160x120 is small.
        # w*h*3 = 57600 bytes per frame. 
        # 10s video = 576KB. 1min = 3.4MB. Safe.
        raw_data, stderr = process.communicate()
        
        if process.returncode != 0:
            print("FFmpeg Error:")
            print(stderr.decode())
            return
            
        total_bytes = len(raw_data)
        frame_size = width * height * 3
        if total_bytes % frame_size != 0:
            print(f"Warning: Data size {total_bytes} not multiple of frame size {frame_size}.")
            
        num_frames = total_bytes // frame_size
        print(f"Captured {num_frames} frames.")
        
        # Write VID1 Header and Data
        with open(output_file, "wb") as f:
            # Header: VID1 | W(2) | H(2) | Frames(4) | FPS(2)
            f.write(b"VID1")
            f.write(struct.pack("<HHIH", width, height, num_frames, fps))
            f.write(raw_data)
            
        print(f"Saved {output_file} ({os.path.getsize(output_file)} bytes).")

    except FileNotFoundError:
        print("Error: ffmpeg not found. Please install ffmpeg.")
    except Exception as e:
        print(f"Error: {e}")

if __name__ == "__main__":
    convert_mp4_to_vid("initrd/libs/video.mp4", "initrd/libs/video.vid", 160, 120, 10)
