import http.server
import socketserver
import subprocess
import os

PORT = 8000

class CompilerHandler(http.server.BaseHTTPRequestHandler):
    def do_POST(self):
        content_length = int(self.headers.get('Content-Length', 0))
        source_code = self.rfile.read(content_length).decode('utf-8')
        
        print(f"Received {len(source_code)} bytes of source code.")
        
        # Determine if it's C or Assembly based on simplistic heuristics
        is_asm = source_code.strip().startswith('.') or 'syscall' in source_code.lower()
        ext = '.s' if is_asm else '.c'
        
        source_file = f"temp_build{ext}"
        target_file = "temp_build.elf"
        
        with open(source_file, "w", encoding="utf-8") as f:
            f.write(source_code)
            
        # Clang command to build a freestanding x86_64 ELF binary
        cmd = [
            "clang",
            "-target", "x86_64-pc-none-elf",
            "-ffreestanding",
            "-nostdlib",
            "-static",
            "-fuse-ld=lld",
            "-Wl,-e,main" if not is_asm else "-Wl,-e,_start",
            "-Wl,-Ttext=0x400000",
            source_file,
            "-o", target_file
        ]
        
        print(f"Executing: {' '.join(cmd)}")
        
        try:
            # We use shell=True just in case they are on Windows and clang is via WSL or vice versa,
            # though direct list execution is safer if clang is in PATH. Let's try direct first.
            result = subprocess.run(cmd, capture_output=True, text=True)
            
            if result.returncode != 0:
                print(f"Compilation Failed:\n{result.stderr}")
                self.send_response(500)
                self.send_header('Content-type', 'text/plain')
                self.end_headers()
                self.wfile.write(f"COMPILER ERROR:\n{result.stderr}".encode('utf-8'))
                return
                
            print("Compilation successful. Sending binary...")
            
            with open(target_file, "rb") as f:
                binary_data = f.read()
                
            self.send_response(200)
            self.send_header('Content-type', 'application/octet-stream')
            self.end_headers()
            self.wfile.write(binary_data)
            
        except FileNotFoundError:
             # Fallback if clang is not in Windows PATH but is in WSL
             print("Clang not found natively. Trying via WSL...")
             wsl_cmd = ["wsl"] + cmd
             result = subprocess.run(wsl_cmd, capture_output=True, text=True)
             
             if result.returncode != 0:
                print(f"WSL Compilation Failed:\n{result.stderr}")
                self.send_response(500)
                self.send_header('Content-type', 'text/plain')
                self.end_headers()
                self.wfile.write(f"COMPILER ERROR:\n{result.stderr}".encode('utf-8'))
                return
             
             with open(target_file, "rb") as f:
                binary_data = f.read()
                
             self.send_response(200)
             self.send_header('Content-type', 'application/octet-stream')
             self.end_headers()
             self.wfile.write(binary_data)

        except Exception as e:
            print(f"Server Error: {e}")
            self.send_response(500)
            self.end_headers()
            self.wfile.write(str(e).encode('utf-8'))
        finally:
            if os.path.exists(source_file): os.remove(source_file)
            if os.path.exists(target_file): os.remove(target_file)

with socketserver.TCPServer(("", PORT), CompilerHandler) as httpd:
    print(f"LLVM Remote Backend running at port {PORT}")
    httpd.serve_forever()
