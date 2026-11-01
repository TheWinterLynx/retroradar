[CmdletBinding()]
param()
$ErrorActionPreference = 'Stop'
Push-Location (Split-Path $PSScriptRoot -Parent)
try {
    # cargo metadata includes optional unselected dependencies. cargo tree reflects
    # the enabled features actually built, including local tests and build tools.
    $lines = @(cargo tree --locked --target x86_64-pc-windows-msvc --edges normal,build,dev --prefix none --format '{p}')
    if ($LASTEXITCODE -ne 0) { throw 'Cannot resolve active Windows dependencies.' }
    $names = @($lines | ForEach-Object { if ($_ -match '^([A-Za-z0-9_-]+) v') { $Matches[1] } } | Sort-Object -Unique)
    $forbidden = @('ring', 'aws-lc-rs', 'aws-lc-sys', 'openssl', 'openssl-sys', 'native-tls', 'boring', 'boring-sys', 'libz-sys', 'bzip2-sys', 'zstd-sys', 'cmake')
    foreach ($name in $names) {
        if ($name -in $forbidden) { throw "Forbidden active Windows dependency: $name" }
    }
    if ('cc' -in $names) {
        $consumers = @(cargo tree --locked --target x86_64-pc-windows-msvc --edges normal,build,dev --invert cc --depth 1 --prefix none --format '{p}')
        if ($LASTEXITCODE -ne 0) { throw 'Cannot inspect native compiler consumers.' }
        foreach ($line in $consumers) {
            if ($line -match '^([A-Za-z0-9_-]+) v') {
                if ($Matches[1] -notin @('cc', 'libsqlite3-sys')) { throw "Unapproved native compiler consumer: $line" }
            }
        }
    }
    Write-Host 'R01 DEPENDENCY GATE GREEN: ring inactive; SQLite is the only approved native compiler consumer.' -ForegroundColor Green
} finally { Pop-Location }
