import os

# Ainux Unified ABI WASM (Step 3.1)
# Imports: (import "env" "cap_call" (func $cap_call (param i32 i32 i32 i32)))

WASM_CAP = (
    b"\x00\x61\x73\x6d"  # Magic
    b"\x01\x00\x00\x00"  # Version
    # Type section
    b"\x01\x0b\x02"
    b"\x60\x00\x01\x7f"        # Type 0: () -> i32
    b"\x60\x04\x7f\x7f\x7f\x7f\x00" # Type 1: (i32, i32, i32, i32) -> ()
    # Import section: "env" "cap_call"
    b"\x02\x10\x01"
    b"\x03\x65\x6e\x76" # "env"
    b"\x08\x63\x61\x70\x5f\x63\x61\x6c\x6c" # "cap_call"
    b"\x00\x01"         # Func index 0, Type 1
    # Function section
    b"\x03\x02\x01\x00" # Func index 1, Type 0
    # Memory section: 1 page
    b"\x05\x03\x01\x00\x01"
    # Export section: "main", "memory"
    b"\x07\x13\x02"
    b"\x04\x6d\x61\x69\x6e\x00\x01"
    b"\x06\x6d\x65\x6d\x6f\x72\x79\x02\x00"
    # Code section
    b"\x0a\x1a\x01"
    b"\x18\x00"          # Body size, locals (0)
    b"\x41\x00"          # i32.const 0 (handle)
    b"\x41\x01"          # i32.const 1 (op: OP_WRITE)
    b"\x41\x00"          # i32.const 0 (ptr)
    b"\x41\x0f"          # i32.const 15 (len)
    b"\x10\x00"          # call 0 (env.cap_call)
    b"\x41\x00"          # return 0
    b"\x0b"              # end
    # Data section: "Ainux CapCall!!"
    b"\x0b\x13\x01"
    b"\x00\x41\x00\x0b"  # Offset 0
    b"\x0f\x41\x69\x6e\x75\x78\x20\x43\x61\x70\x43\x61\x6c\x6c\x21\x21" # "Ainux CapCall!!"
)

def generate():
    path = os.path.join(os.path.dirname(__file__), "hello.wasm")
    with open(path, "wb") as f:
        f.write(WASM_CAP)
    print(f"Generated {path}")

if __name__ == "__main__":
    generate()
