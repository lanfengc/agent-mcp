param(
    [switch]$Release,
    [switch]$SkipDesktop,
    [switch]$SkipBrowser,
    [switch]$SkipAndroid
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
$binDir = Join-Path $root 'bin'
New-Item -ItemType Directory -Force -Path $binDir | Out-Null

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
    $desktopEntry = Join-Path $binDir 'agent-desktop.exe'
    Copy-Item -Path $desktopExe -Destination $desktopEntry -Force
    Write-Host "desktop: $desktopExe"
    Write-Host "entry:   $desktopEntry"
}

if (-not $SkipBrowser) {
    $browser = Join-Path $root 'agent-browser'
    Invoke-Cargo -Directory $browser -Arguments ($cargoArgs + @('--manifest-path', 'cli/Cargo.toml'))
    $browserExe = Join-Path $browser "cli\target\$profile\agent-browser.exe"

    $arch = switch ($env:PROCESSOR_ARCHITECTURE) {
        'ARM64' { 'arm64' }
        default { 'x64' }
    }
    $browserBin = Join-Path $browser 'bin'
    New-Item -ItemType Directory -Force -Path $browserBin | Out-Null
    $launcherExe = Join-Path $browserBin "agent-browser-win32-$arch.exe"
    Copy-Item -Path $browserExe -Destination $launcherExe -Force
    $browserEntry = Join-Path $binDir 'agent-browser.exe'
    Copy-Item -Path $browserExe -Destination $browserEntry -Force
    foreach ($name in 'skill-data', 'skills') {
        $skillCopy = Join-Path $binDir $name
        if (Test-Path $skillCopy) {
            if ((Get-Item -Force $skillCopy).Attributes -band [IO.FileAttributes]::ReparsePoint) {
                throw "$skillCopy is a link, not a copy. Remove the link and run the build again."
            }
            Remove-Item -Path $skillCopy -Recurse -Force
        }
        Copy-Item -Path (Join-Path $browser $name) -Destination $skillCopy -Recurse
    }
    Write-Host "browser: $browserExe"
    Write-Host "entry:   $browserEntry"
    Write-Host "skills:  $(Join-Path $binDir 'skill-data')"
    Write-Host "launcher: $launcherExe"
}

if (-not $SkipAndroid) {
    $android = Join-Path $root 'agent-android'
    Invoke-Cargo -Directory $android -Arguments $cargoArgs
    $androidExe = Join-Path $android "target\$profile\agent-android.exe"
    $androidEntry = Join-Path $binDir 'agent-android.exe'
    Copy-Item -Path $androidExe -Destination $androidEntry -Force
    Write-Host "android: $androidExe"
    Write-Host "entry:   $androidEntry"
}

Write-Host ''
$built = @()
if (-not $SkipDesktop) { $built += 'agent-desktop.exe' }
if (-not $SkipBrowser) { $built += 'agent-browser.exe' }
if (-not $SkipAndroid) { $built += 'agent-android.exe' }
foreach ($name in $built) {
    & (Join-Path $binDir $name) --version
}
