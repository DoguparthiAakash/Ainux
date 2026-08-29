import subprocess
import time
import sys

def main():
    print("Starting QEMU...")
    cmd = ["qemu-system-x86_64", "-m", "10M", "-drive", "file=ainux.iso,format=raw,if=ide,bus=1,unit=0,media=cdrom", "-drive", "file=disk3.img,format=raw,if=ide,bus=0,unit=0,media=disk", "-display", "none", "-serial", "stdio", "-boot", "d", "-device", "AC97"]
    
    p = subprocess.Popen(cmd, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True, bufsize=1)
    
    test_started = False
    
    timeout_time = time.time() + 60
    
    try:
        while True:
            if time.time() > timeout_time:
                print("TIMEOUT")
                break
                
            line = p.stdout.readline()
            if not line:
                break
                
            print(f"QEMU: {line.strip()}")
                
            if "verify fork subsystem" in line:
                test_started = True
                
            if test_started and "Child (pid" in line and "exited with status" in line:
                print(">>> TEST FORK SUCCESS <<<")
                
            if "Shutting down..." in line:
                print(">>> KERNEL SHUTDOWN DETECTED <<<")
                break
            
            if test_started and "PANIC" in line:
                print(">>> KERNEL PANIC <<<")
                break
    finally:
        p.terminate()
        p.kill()

if __name__ == '__main__':
    main()
