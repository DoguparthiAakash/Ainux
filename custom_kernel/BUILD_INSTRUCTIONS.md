# Nux Cross-Platform Build Instructions

## Prerequisites

You need Rust installed on your Linux system to build for other platforms.

## Quick Build (Linux Only)

If you just want to build for Linux:
```bash
cd nux_portable
cargo build --release
```

The binary will be at `target/release/nux`

## Cross-Platform Build

### 1. Install Cross-Compilation Targets

```bash
# For Windows
rustup target add x86_64-pc-windows-gnu
sudo apt-get install mingw-w64  # On Ubuntu/Debian

# For macOS (requires macOS SDK - see notes below)
rustup target add x86_64-apple-darwin
rustup target add aarch64-apple-darwin
```

### 2. Run Build Script

```bash
mkdir -p builds
./build_cross_platform.sh
```

This will create:
- `builds/nux-linux-x86_64` - Linux binary
- `builds/nux-windows-x86_64.exe` - Windows binary
- `builds/nux-macos-x86_64` - macOS Intel binary
- `builds/nux-macos-arm64` - macOS Apple Silicon binary

## Alternative: Use GitHub Actions

For easier cross-platform builds, I recommend using GitHub Actions which can build on actual Windows and macOS machines.

Create `.github/workflows/build.yml`:

```yaml
name: Build Nux

on: [push, pull_request]

jobs:
  build:
    strategy:
      matrix:
        os: [ubuntu-latest, windows-latest, macos-latest]
    runs-on: ${{ matrix.os }}
    
    steps:
    - uses: actions/checkout@v3
    - uses: actions-rust-lang/setup-rust-toolchain@v1
    
    - name: Build
      run: |
        cd nux_portable
        cargo build --release
    
    - name: Upload artifact
      uses: actions/upload-artifact@v3
      with:
        name: nux-${{ matrix.os }}
        path: nux_portable/target/release/nux*
```

## Distribution

### Linux
```bash
# Just copy the binary
cp builds/nux-linux-x86_64 /usr/local/bin/nux
chmod +x /usr/local/bin/nux
```

### Windows
1. Copy `nux-windows-x86_64.exe` to a folder
2. Rename to `nux.exe`
3. Add folder to PATH environment variable

### macOS
```bash
# Copy to /usr/local/bin
sudo cp builds/nux-macos-arm64 /usr/local/bin/nux
sudo chmod +x /usr/local/bin/nux
```

## Notes

- **macOS Cross-Compilation**: Building for macOS from Linux requires the macOS SDK, which has licensing restrictions. It's easier to use GitHub Actions or build on an actual Mac.
- **Windows**: The MinGW toolchain works well for cross-compilation from Linux.
- **All binaries are standalone** - no Rust installation needed to run them!

## Testing

After building, test each binary:

```bash
# Linux
./builds/nux-linux-x86_64 version

# Windows (on Windows or Wine)
wine builds/nux-windows-x86_64.exe version

# macOS (on macOS)
./builds/nux-macos-arm64 version
```
