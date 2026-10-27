[CmdletBinding()]
param(
    [string]$Target = 'x86_64-unknown-linux-gnu',
    [string]$GlibcVersion = '2.31',
    [switch]$InstallMissing,
    [switch]$SkipFmt,
    [switch]$SkipPackageAssets
)

$ErrorActionPreference = 'Stop'

$root = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path
Set-Location $root

function Require-Command {
    param(
        [Parameter(Mandatory = $true)][string]$Name,
        [Parameter(Mandatory = $true)][string]$InstallHint
    )

    $command = Get-Command $Name -ErrorAction SilentlyContinue
    if (-not $command) {
        throw "$Name was not found. $InstallHint"
    }
    return $command
}

function Get-RustTarget {
    param([string]$Value)

    if ($Value -match '^(.*-linux-gnu)\.\d+(\.\d+)?$') {
        return $Matches[1]
    }
    return $Value
}

function Get-ZigTarget {
    param(
        [string]$RustTarget,
        [string]$RequestedTarget,
        [string]$RequestedGlibcVersion
    )

    if ($RequestedTarget -ne $RustTarget) {
        return $RequestedTarget
    }
    if ($RustTarget.EndsWith('-linux-gnu') -and -not [string]::IsNullOrWhiteSpace($RequestedGlibcVersion)) {
        return "$RustTarget.$RequestedGlibcVersion"
    }
    return $RustTarget
}

function Install-RustTargetIfRequested {
    param([string]$RustTarget)

    $installedTargets = @(rustup target list --installed)
    if ($installedTargets -contains $RustTarget) {
        return
    }

    if ($InstallMissing) {
        rustup target add $RustTarget
        return
    }

    throw "Rust target $RustTarget is not installed. Run: rustup target add $RustTarget"
}

function Install-CargoZigbuildIfRequested {
    $command = Get-Command cargo-zigbuild -ErrorAction SilentlyContinue
    if ($command) {
        return
    }

    if ($InstallMissing) {
        cargo install cargo-zigbuild --locked
        return
    }

    throw 'cargo-zigbuild was not found. Run: cargo install cargo-zigbuild --locked'
}

function Install-ZigIfRequested {
    $command = Get-Command zig -ErrorAction SilentlyContinue
    if ($command) {
        return
    }

    if ($InstallMissing) {
        $winget = Get-Command winget -ErrorAction SilentlyContinue
        if ($winget) {
            winget install -e --id zig.zig
            $command = Get-Command zig -ErrorAction SilentlyContinue
            if ($command) {
                return
            }
        }
    }

    throw 'zig was not found. Install Zig from https://ziglang.org/download/ and ensure zig is on PATH.'
}

function Reset-DistDirectory {
    param([string]$Dist)

    $distParent = Split-Path $Dist -Parent
    New-Item -ItemType Directory -Path $distParent -Force | Out-Null
    if (Test-Path -LiteralPath $Dist) {
        $resolvedDist = (Resolve-Path -LiteralPath $Dist).Path
        $resolvedRoot = (Resolve-Path -LiteralPath $root).Path
        if (-not $resolvedDist.StartsWith($resolvedRoot, [System.StringComparison]::OrdinalIgnoreCase)) {
            throw "Refusing to clear directory outside workspace: $resolvedDist"
        }
        Remove-Item -LiteralPath $resolvedDist -Recurse -Force
    }
    New-Item -ItemType Directory -Path $Dist -Force | Out-Null
}

function Copy-DirectoryIfPresent {
    param(
        [string]$Source,
        [string]$Destination
    )

    if (Test-Path -LiteralPath $Source) {
        Copy-Item -LiteralPath $Source -Destination $Destination -Recurse -Force
    }
}

function Copy-FileIfPresent {
    param(
        [string]$Source,
        [string]$Destination
    )

    if (Test-Path -LiteralPath $Source) {
        Copy-Item -LiteralPath $Source -Destination $Destination -Force
    }
}

function Find-BuiltExecutable {
    param(
        [string]$TargetDir,
        [string]$RustTarget,
        [string]$ZigTarget
    )

    $candidates = @(
        (Join-Path $TargetDir "$ZigTarget\release\click_automation"),
        (Join-Path $TargetDir "$RustTarget\release\click_automation"),
        (Join-Path $TargetDir 'release\click_automation')
    )

    foreach ($candidate in $candidates) {
        if (Test-Path -LiteralPath $candidate) {
            return (Resolve-Path -LiteralPath $candidate).Path
        }
    }

    throw "Linux executable was not found under $TargetDir"
}

function Write-Launcher {
    param([string]$Dist)

    $launcher = @'
#!/usr/bin/env bash
set -euo pipefail
DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
export LD_LIBRARY_PATH="$DIR:${LD_LIBRARY_PATH:-}"
export RAPIDOCR_MODEL_DIR="${RAPIDOCR_MODEL_DIR:-$DIR/models/rapidocr}"
exec "$DIR/click_automation" "$@"
'@
    $path = Join-Path $Dist 'run.sh'
    Set-Content -LiteralPath $path -Value $launcher -Encoding utf8
}

$rustTarget = Get-RustTarget $Target
$zigTarget = Get-ZigTarget $rustTarget $Target $GlibcVersion
$targetDir = Join-Path $root "target\linux-$rustTarget"
$distName = if ($rustTarget -eq 'x86_64-unknown-linux-gnu') { 'linux-x86_64' } else { "linux-$rustTarget" }
$dist = Join-Path $root "dist\$distName"

Require-Command rustup 'Install Rust from https://rustup.rs/.' | Out-Null
Require-Command cargo 'Install Rust from https://rustup.rs/.' | Out-Null
Install-RustTargetIfRequested $rustTarget
Install-ZigIfRequested
Install-CargoZigbuildIfRequested

if (-not $SkipFmt) {
    cargo fmt -- --check
}

Write-Host "Building Linux release: rustTarget=$rustTarget zigTarget=$zigTarget"
try {
    cargo zigbuild --release --locked --target $zigTarget --target-dir $targetDir
} catch {
    Write-Error @"
Linux release build failed. If the linker output mentions X11, xcb, GL, EGL, fontconfig,
or other native libraries, install the matching target sysroot development packages or
perform this release build on a Linux machine with the documented packages installed.
"@
    throw
}

Reset-DistDirectory $dist
$builtExe = Find-BuiltExecutable $targetDir $rustTarget $zigTarget
Copy-Item -LiteralPath $builtExe -Destination (Join-Path $dist 'click_automation') -Force

if (-not $SkipPackageAssets) {
    Copy-FileIfPresent (Join-Path $root 'config.yaml') (Join-Path $dist 'config.yaml')
    Copy-DirectoryIfPresent (Join-Path $root 'models') (Join-Path $dist 'models')
    Copy-DirectoryIfPresent (Join-Path $root 'patches') (Join-Path $dist 'patches')
    Copy-FileIfPresent (Join-Path $root 'notosansscregular.otf') (Join-Path $dist 'notosansscregular.otf')
}

$ortRuntimes = Get-ChildItem -LiteralPath $targetDir -Recurse -File -ErrorAction SilentlyContinue |
    Where-Object { $_.Name -like 'libonnxruntime*.so*' }
foreach ($runtime in $ortRuntimes) {
    Copy-Item -LiteralPath $runtime.FullName -Destination (Join-Path $dist $runtime.Name) -Force
}

Write-Launcher $dist
Write-Host "Linux package generated: $dist"
Write-Host 'On Linux, run: chmod +x click_automation run.sh && ./run.sh'
