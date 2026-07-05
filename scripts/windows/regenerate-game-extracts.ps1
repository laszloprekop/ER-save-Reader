# Regenerate the "decompiled game files" corpus from the ELDEN RING install
# using WitchyBND. Run this on the Windows machine.
#
# Prerequisites:
#   1. WitchyBND: https://github.com/ividyon/WitchyBND/releases (unzip anywhere).
#      Needs .NET Desktop Runtime (modern Windows 10/11 usually has it; the
#      release page says which version otherwise).
#   2. ELDEN RING install (game version should be 1.16.x / exe ProductVersion 2.6.x
#      to match the save-era evidence; check Steam betas if the game auto-updated).
#
# What this produces (in -OutDir):
#   regulation-bin\*.param.xml     <- the critical corpus (ItemLotParam, ShopLineup,
#                                     BonfireWarpParam, WorldMapPointParam, Magic, ...)
#   map\mapstudio\*.msb.xml        <- optional (-IncludeMsb), region names only
#
# NOT needed here: event\*.emevd.dcx — those are DCX-decompressed natively on the
# Mac (ooz); EMEVD parsing happens in the knowledge pipeline.
#
# Afterwards: copy -OutDir to the Mac at
#   '/Users/laszloprekop/dev/Elden Ring stuff/Elden Ring decompiled game files'
# then in ER-save-Editor: change the `game-extracts` catalog entry kind from
# "missing" to "directory" (root pointing at that folder) and run
#   er-save-editor knowledge catalog-update
#
# Usage (PowerShell):
#   .\regenerate-game-extracts.ps1 -WitchyExe 'C:\tools\WitchyBND\WitchyBND.exe'
#   .\regenerate-game-extracts.ps1 -WitchyExe ... -IncludeMsb
#   .\regenerate-game-extracts.ps1 -WitchyExe ... -GameDir 'D:\SteamLibrary\steamapps\common\ELDEN RING\Game'

param(
    [Parameter(Mandatory = $true)]
    [string]$WitchyExe,

    [string]$GameDir = 'C:\Program Files (x86)\Steam\steamapps\common\ELDEN RING\Game',

    [string]$OutDir = "$PSScriptRoot\decompiled-game-files",

    [switch]$IncludeMsb
)

$ErrorActionPreference = 'Stop'

function Assert-Path([string]$p, [string]$what) {
    if (-not (Test-Path $p)) { throw "$what not found: $p" }
}

Assert-Path $WitchyExe 'WitchyBND.exe'
Assert-Path $GameDir 'Game directory'
Assert-Path (Join-Path $GameDir 'regulation.bin') 'regulation.bin'

New-Item -ItemType Directory -Force -Path $OutDir | Out-Null

# Invoke WitchyBND on a batch of paths. Tries silent mode (-s) first; if the
# installed version rejects the flag, retries without it (interactive prompts
# may then appear once).
$script:UseSilent = $true
function Invoke-Witchy([string[]]$paths) {
    if ($paths.Count -eq 0) { return }
    if ($script:UseSilent) {
        & $WitchyExe -s @paths
        if ($LASTEXITCODE -ne 0) {
            Write-Warning "WitchyBND with -s exited $LASTEXITCODE; retrying without -s"
            $script:UseSilent = $false
            & $WitchyExe @paths
        }
    } else {
        & $WitchyExe @paths
    }
    if ($LASTEXITCODE -ne 0) { throw "WitchyBND failed (exit $LASTEXITCODE) on: $($paths[0]) ..." }
}

# Process a file list in chunks (command-line length safety).
function Invoke-WitchyChunked([System.Collections.IEnumerable]$files, [int]$chunk = 40) {
    $batch = @()
    foreach ($f in $files) {
        $batch += $f
        if ($batch.Count -ge $chunk) { Invoke-Witchy $batch; $batch = @() }
    }
    Invoke-Witchy $batch
}

Write-Host '=== Step 1: regulation.bin -> .param files ==='
$reg = Join-Path $OutDir 'regulation.bin'
Copy-Item (Join-Path $GameDir 'regulation.bin') $reg -Force
Invoke-Witchy @($reg)

$regBinDir = Join-Path $OutDir 'regulation-bin'
Assert-Path $regBinDir 'regulation-bin unpack folder (WitchyBND output)'
$paramFiles = Get-ChildItem $regBinDir -Filter '*.param' -File
Write-Host ("unpacked {0} .param files" -f $paramFiles.Count)

Write-Host '=== Step 2: .param -> .param.xml ==='
Invoke-WitchyChunked ($paramFiles | ForEach-Object { $_.FullName })
$xmlCount = (Get-ChildItem $regBinDir -Filter '*.xml' -File).Count
Write-Host ("serialized {0} XML files" -f $xmlCount)
if ($xmlCount -eq 0) { throw 'No XML produced - check WitchyBND game detection (it may prompt for the game/regulation version without -s).' }

if ($IncludeMsb) {
    Write-Host '=== Step 3 (optional): MSBs -> XML ==='
    $msbSrc = Join-Path $GameDir 'map\mapstudio'
    Assert-Path $msbSrc 'map\mapstudio'
    $msbDst = Join-Path $OutDir 'map\mapstudio'
    New-Item -ItemType Directory -Force -Path $msbDst | Out-Null
    Copy-Item (Join-Path $msbSrc '*.msb.dcx') $msbDst -Force
    Invoke-WitchyChunked (Get-ChildItem $msbDst -Filter '*.msb.dcx' -File | ForEach-Object { $_.FullName })
}

Write-Host ''
Write-Host '=== DONE ==='
Write-Host "Output: $OutDir"
Write-Host "Copy this folder to the Mac as:"
Write-Host "  '/Users/laszloprekop/dev/Elden Ring stuff/Elden Ring decompiled game files'"
Write-Host "then update the evidence catalog (game-extracts entry -> kind 'directory')"
Write-Host "and run: er-save-editor knowledge catalog-update"
