# =============================================================================
#  Mithl OS — Windows PowerShell Launcher
# =============================================================================
$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$Iso = Join-Path $ScriptDir "ainux.iso"
$Disk = Join-Path $ScriptDir "disk4.img"
$SerialLog = Join-Path $ScriptDir "serial.log"
$Qemu = "C:\Program Files\qemu\qemu-system-x86_64.exe"

if (-not (Test-Path $Qemu)) {
    Write-Host "❌ QEMU not found at $Qemu" -ForegroundColor Red
    exit 1
}

if (-not (Test-Path $Iso)) {
    Write-Host "❌ ISO not found at $Iso" -ForegroundColor Red
    exit 1
}

$DiskArg = @()
if (Test-Path $Disk) {
    $DiskArg = @("-drive", "file=$Disk,format=raw,index=0,media=disk")
}

# Check WHPX
$Accel = "-accel", "whpx"
Write-Host "⚡ Acceleration: WHPX enabled" -ForegroundColor Green
Write-Host "🔊 Audio: AC97 (Host DirectSound/SDL) + PC Speaker" -ForegroundColor Cyan

Write-Host ""
Write-Host "  ╔══════════════════════════════════════╗" -ForegroundColor Yellow
Write-Host "  ║         Mithl OS Bootloader          ║" -ForegroundColor Yellow
Write-Host "  ╚══════════════════════════════════════╝" -ForegroundColor Yellow
Write-Host ""
Write-Host "  Ctrl+Alt   → Release mouse"
Write-Host "  Ctrl+Alt+F → Toggle fullscreen"
Write-Host ""

& $Qemu `
    -name "Mithl OS" `
    -M pc,pcspk-audiodev=snd0 `
    -smp 4 `
    -m 2G `
    -cdrom $Iso `
    -boot d `
    @DiskArg `
    @Accel `
    -vga std `
    -audiodev sdl,id=snd0 `
    -device AC97,audiodev=snd0 `
    -serial "file:$SerialLog" `
    -net nic,model=rtl8139 -net user,hostfwd=tcp::8080-:8080,hostfwd=tcp::9000-:9000
