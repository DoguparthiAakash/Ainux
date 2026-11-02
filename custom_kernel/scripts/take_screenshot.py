import socket
import json
import time
import subprocess
import os

qmp_port = 4455
serial_log_path = "/tmp/serial_test.log"
if os.path.exists(serial_log_path):
    try: os.remove(serial_log_path)
    except: pass
cmd = [
    "qemu-system-x86_64",
    "-name", "Mithl OS",
    "-M", "pc,pcspk-audiodev=snd0",
    "-smp", "4",
    "-m", "2G",
    "-cdrom", "mithl.iso",
    "-boot", "d",
    "-vga", "std",
    "-vnc", ":99",
    "-audiodev", "none,id=snd0",
    "-device", "AC97,audiodev=snd0",
    "-drive", "file=mithl,format=raw,index=0,media=disk",
    "-serial", f"file:{serial_log_path}",
    "-qmp", f"tcp:localhost:{qmp_port},server,nowait"
]

if os.path.exists("/dev/kvm") and os.access("/dev/kvm", os.R_OK):
    cmd.extend(["-enable-kvm", "-cpu", "host"])
else:
    cmd.extend(["-cpu", "qemu64"])

print("Starting QEMU...")
proc = subprocess.Popen(cmd)
time.sleep(1)

s = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
for _ in range(10):
    try:
        s.connect(("localhost", qmp_port))
        break
    except Exception:
        time.sleep(0.5)

def qmp_cmd(command, args=None):
    payload = {"execute": command}
    if args:
        payload["arguments"] = args
    s.sendall(json.dumps(payload).encode() + b"\n")
    return s.recv(4096)

# Read greeting & negotiate capabilities
greeting = s.recv(4096)
qmp_cmd("qmp_capabilities")

print(f"Waiting for desktop to initialize (monitoring {serial_log_path})...")
start_t = time.time()
while time.time() - start_t < 90:
    if os.path.exists(serial_log_path):
        with open(serial_log_path, "r", errors="ignore") as f:
            if "Desktop: Initial frame rendered" in f.read():
                print("Desktop rendered! Waiting 4 seconds for window composite...")
                time.sleep(4)
                break
    time.sleep(1)

ppm_path = "/tmp/mithl_desktop.ppm"
res = qmp_cmd("screendump", {"filename": ppm_path})
print("Screendump response:", res)

qmp_cmd("quit")
proc.wait()
os.system(f"cp {ppm_path} /mnt/e/lh/lsr/Ainux/custom_kernel/mithl_desktop.ppm")
os.system(f"python3 /mnt/e/lh/lsr/Ainux/custom_kernel/scripts/ppm2bmp.py /mnt/e/lh/lsr/Ainux/custom_kernel/mithl_desktop.ppm /mnt/e/lh/lsr/Ainux/custom_kernel/mithl_desktop.bmp")
print("QEMU session finished.")
