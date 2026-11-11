param(
    [switch]$Release,
    [switch]$SkipDesktop,
    [switch]$SkipBrowser
)

$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot

if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
    $cargoBin = Join-Path $env:USERPROFILE '.cargo\bin'
    if (Test-Path (Join-Path $cargoBin 'cargo.exe')) {
        $env:Path = "$cargoBin;$env:Path"
    }
    else {
        throw 'cargo was not found. Install Rust from https://rustup.rs and reopen the terminal.'
    }
}

$cargoArgs = @('build')
if ($Release) {
    $cargoArgs += '--release'
}
$profile = if ($Release) { 'release' } else { 'debug' }

function Invoke-Cargo {
    param([string]$Directory, [string[]]$Arguments)
    Push-Location $Directory
    try {
        & cargo @Arguments
        if ($LASTEXITCODE -ne 0) {
            throw "cargo $($Arguments -join ' ') failed with exit code $LASTEXITCODE"
        }
    }
    finally {
        Pop-Location
    }
}

if (-not $SkipDesktop) {
    $desktop = Join-Path $root 'agent-desktop'
    Invoke-Cargo -Directory $desktop -Arguments ($cargoArgs + @('-p', 'agent-desktop'))
    $desktopExe = Join-Path $desktop "target\$profile\agent-desktop.exe"
    Write-Host "desktop: $desktopExe"
}

if (-not $SkipBrowser) {
    $browser = Join-Path $root 'agent-browser'
    Invoke-Cargo -Directory $browser -Arguments ($cargoArgs + @('--manifest-path', 'cli/Cargo.toml'))
    $browserExe = Join-Path $browser "cli\target\$profile\agent-browser.exe"

    $arch = switch ($env:PROCESSOR_ARCHITECTURE) {
        'ARM64' { 'arm64' }
        default { 'x64' }
    }
    $binDir = Join-Path $browser 'bin'
    New-Item -ItemType Directory -Force -Path $binDir | Out-Null
    $launcherExe = Join-Path $binDir "agent-browser-win32-$arch.exe"
    Copy-Item -Path $browserExe -Destination $launcherExe -Force
    Write-Host "browser: $browserExe"
    Write-Host "launcher: $launcherExe"
}

Write-Host ''
$versionExe = Join-Path (Join-Path $root 'agent-desktop\target') "$profile\agent-desktop.exe"
if (Test-Path $versionExe) {
    & $versionExe --version
}
