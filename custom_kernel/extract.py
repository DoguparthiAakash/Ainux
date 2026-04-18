import cv2
import sys
import os

vid_path = "Screen Recording 2026-04-18 225200.mp4"
if not os.path.exists(vid_path):
    print("Video not found:", vid_path)
    sys.exit(1)

vidcap = cv2.VideoCapture(vid_path)
total_frames = int(vidcap.get(cv2.CAP_PROP_FRAME_COUNT))

vidcap.set(cv2.CAP_PROP_POS_FRAMES, max(0, total_frames - 5))
success, image = vidcap.read()
if success:
    cv2.imwrite("crash_frame.jpg", image)
    print("Extracted crash_frame.jpg")
else:
    print("Failed to read frame")
