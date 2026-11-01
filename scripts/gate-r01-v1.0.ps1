[CmdletBinding()]
param([switch]$SkipGpu)
$ErrorActionPreference = 'Stop'
Set-Location (Split-Path $PSScriptRoot -Parent)
function Step([string]$Name, [scriptblock]$Run) {
    Write-Host "[R01] $Name" -ForegroundColor Cyan
    & $Run
    if ($LASTEXITCODE -ne 0) { throw "R01 FAILED: $Name (exit $LASTEXITCODE)" }
}
if ($env:OS -ne 'Windows_NT') { throw 'R01 Windows acceptance requires Windows.' }
if (-not (Get-Command git -ErrorAction SilentlyContinue)) { throw 'Install Git for Windows first.' }
if (-not (Get-Command rustup -ErrorAction SilentlyContinue)) { throw 'Install Rust via rustup and Visual Studio Build Tools: Desktop development with C++ + Windows SDK.' }
Step 'Clean checkout' { git diff --exit-code; if ($LASTEXITCODE -ne 0) { throw 'Tracked edits present.' }; git diff --cached --exit-code; if ($LASTEXITCODE -ne 0) { throw 'Staged edits present.' }; $pending = git ls-files --others --exclude-standard; if ($LASTEXITCODE -ne 0) { throw 'Cannot inspect checkout.' }; if ($pending) { throw 'Untracked files present; move or commit them first.' } }
Step 'Validated head' { git rev-parse HEAD }
Step 'Toolchain' { rustup show active-toolchain }
$hostInfo = rustc -vV
if ($LASTEXITCODE -ne 0 -or -not ($hostInfo -match 'host: .*windows-msvc')) { throw 'Use a Windows MSVC Rust toolchain; install Build Tools C++ and Windows SDK before compiling.' }
Write-Host '[R01] MSVC and Windows SDK prerequisites' -ForegroundColor Cyan
$vswhere = Join-Path ${env:ProgramFiles(x86)} 'Microsoft Visual Studio\Installer\vswhere.exe'
if (-not (Test-Path $vswhere)) { throw 'Install Visual Studio 2022 Build Tools with Desktop development with C++.' }
$vsInstall = & $vswhere -latest -products '*' -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath
if ($LASTEXITCODE -ne 0 -or -not $vsInstall) { throw 'MSVC C++ build tools are missing.' }
$sdkLib = Join-Path ${env:ProgramFiles(x86)} 'Windows Kits\10\Lib\*\um\x64\kernel32.lib'
if (-not (Test-Path $sdkLib)) { throw 'Install a Windows SDK with x64 libraries using Visual Studio Installer.' }
Step 'Formatting' { cargo fmt --all -- --check }
Step 'Rust dependency policy (SQLite exception)' { & powershell -NoProfile -ExecutionPolicy Bypass -File (Join-Path $PSScriptRoot 'check-rust-deps-v1.0.ps1') }
$previousFlags = $env:RUSTFLAGS
try {
    $env:RUSTFLAGS = '-Dwarnings'
    Step 'Portable Windows paths' { cargo test --locked -p retroradar --test portable }
    Step 'Globe overlay clipping' { cargo test --locked -p retroradar-gui sun_at_the_globe_edge }
    Step 'Workspace regressions' { cargo test --locked --workspace --all-targets }
    Step 'Lockfile unchanged after Cargo' { git diff --exit-code -- Cargo.lock }
    Step 'Release binary' { cargo build --locked --release --bin retroradar }
    $portable = Join-Path ([System.IO.Path]::GetTempPath()) ('retroradar-r01-' + [guid]::NewGuid().ToString('N'))
    Step 'Offline doctor' { & .\target\release\retroradar.exe --data-dir $portable doctor }
    if (Test-Path $portable) { throw 'Doctor unexpectedly created files.' }
    if (-not $SkipGpu) {
        try {
            Step 'Windows GPU/window smoke (120 frames)' { & .\target\release\retroradar.exe --data-dir $portable --offline --no-record --size 1280x720 --exit-after-frames 120 }
        } finally {
            if (Test-Path $portable) { Remove-Item -Recurse -Force $portable }
        }
    }
} finally { $env:RUSTFLAGS = $previousFlags }
if ($SkipGpu) { Write-Host 'R01 CODE GATE GREEN - GPU acceptance pending' -ForegroundColor Yellow }
else { Write-Host 'R01 AUTOMATED GATE GREEN - inspect the live UI before reporting R01 FULL GATE GREEN' -ForegroundColor Green }
