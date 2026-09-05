#!/bin/bash
set -e

# Create new root directories
mkdir -p arch/x86_64/{cpu,mm,asm,syscall}
mkdir -p core/{mm,process,fs,syscall,ipc,manager,microkernel,object,security,debug}
mkdir -p drivers
mkdir -p net
mkdir -p lib
mkdir -p apps/{nux,engine}
mkdir -p games
mkdir -p c_src
mkdir -p include
mkdir -p scripts
mkdir -p docs
mkdir -p tools

# Safe move function using cp and rm
safe_mv() {
    if [ -e "$1" ] || [ -d "$1" ]; then
        if [ -d "$1" ]; then
            cp -R "$1/"* "$2/" 2>/dev/null || true
            rm -rf "$1"
        else
            mv "$1" "$2"
        fi
    fi
}

safe_mv_dir() {
    if [ -d "$1" ]; then
        cp -R "$1/"* "$2/" 2>/dev/null || true
        rm -rf "$1"
    fi
}

# 1. Arch
safe_mv_dir src/asm arch/x86_64/asm
safe_mv_dir src/cpu arch/x86_64/cpu

# 2. Core
safe_mv src/main.rs core/
safe_mv src/config.rs core/
safe_mv src/sysctl.rs core/
safe_mv src/shell.rs core/
safe_mv_dir src/process core/process
safe_mv_dir src/fs core/fs
safe_mv_dir src/syscall core/syscall
safe_mv_dir src/ipc core/ipc
safe_mv_dir src/manager core/manager
safe_mv_dir src/microkernel core/microkernel
safe_mv_dir src/object core/object
safe_mv_dir src/security core/security
safe_mv_dir src/debug core/debug
safe_mv_dir src/mm core/mm

# 3. Drivers
safe_mv_dir src/drivers drivers

# 4. Net
safe_mv_dir src/net net
safe_mv_dir src/bsd_rust net/bsd_rust

# 5. Lib
safe_mv_dir src/lib lib
safe_mv_dir src/semantic lib/semantic

# 6. Apps & Games
safe_mv_dir src/apps apps
safe_mv_dir src/gui apps/gui
safe_mv_dir src/games games
safe_mv src/tui_fm.rs apps/
safe_mv src/tui_pm.rs apps/
safe_mv_dir src/nux apps/nux
safe_mv_dir src/engine apps/engine
safe_mv_dir src/sem apps/sem

# 7. C Sources and Includes
if [ -d src/c/include ]; then
    cp -R src/c/include/* include/ 2>/dev/null || true
    rm -rf src/c/include
fi
safe_mv_dir src/c c_src
safe_mv_dir src/bsd_compat c_src/bsd_compat
safe_mv_dir src/bsd_external c_src/bsd_external

# 8. Scripts
safe_mv build_only.sh scripts/
safe_mv build_iso.sh scripts/
safe_mv run.sh scripts/

# Cleanup empty src dirs
find src -type d -empty -delete || true

echo "Restructure script complete."
