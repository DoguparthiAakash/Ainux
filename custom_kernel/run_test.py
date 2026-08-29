import subprocess
import time

def main():
    print("Starting QEMU...")
    cmd = ["qemu-system-x86_64", "-m", "10M", "-cdrom", "ainux.iso", "-drive", "file=disk3.img,format=raw,index=0,media=disk", "-display", "none", "-serial", "stdio", "-boot", "d", "-device", "AC97"]
    
    p = subprocess.Popen(cmd, stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True, bufsize=1)
    
    timeout_time = time.time() + 60
    
    commands = ["nuxc c/hello.c", "ls", "exec a.elf", "shutdown"]
    cmd_idx = 0
    ready_for_cmd = False
    
    try:
        while True:
            if time.time() > timeout_time:
                print("TIMEOUT")
                break
                
            line = p.stdout.readline()
            if not line:
                break
                
            print(f"QEMU: {line.strip()}")
            
            if "Ainux Shell" in line or ">" in line:
                ready_for_cmd = True
                
            if ready_for_cmd and cmd_idx < len(commands):
                time.sleep(0.5)
                cmd_to_send = commands[cmd_idx]
                print(f"Sending: {cmd_to_send}")
                p.stdin.write(cmd_to_send + "\n")
                p.stdin.flush()
                cmd_idx += 1
                ready_for_cmd = False # wait for next prompt
                
            if "Shutting down..." in line or "Halting" in line:
                print(">>> KERNEL SHUTDOWN DETECTED <<<")
                break
    finally:
        p.terminate()
        p.kill()

if __name__ == '__main__':
    main()
