<#
.SYNOPSIS
  Downloads the pinned ffmpeg build and installs it as the Tauri sidecar.

.DESCRIPTION
  Fetches the gyan.dev "essentials" build from the versioned GyanD/codexffmpeg GitHub release,
  verifies its SHA-256, and copies ffmpeg.exe to
  src-tauri/binaries/ffmpeg-x86_64-pc-windows-msvc.exe (the target-triple name Tauri's
  externalBin expects). The binary is gitignored; FFMPEG_VERSION.txt records what was installed.
#>
[CmdletBinding()]
param(
  [switch]$Force
)

$ErrorActionPreference = 'Stop'

$Version = '9.0.2'
$AssetName = "ffmpeg-$Version-essentials_build.zip"
$Url = "https://github.com/GyanD/codexffmpeg/releases/download/$Version/$AssetName"
$ExpectedSha256 = '60f467265b1e312373dbcd92200c2618a74850f98d3d078e94296bb3fa2047ba'

$RepoRoot = Split-Path -Parent $PSScriptRoot
$BinariesDir = Join-Path $RepoRoot 'src-tauri\binaries'
$Target = Join-Path $BinariesDir 'ffmpeg-x86_64-pc-windows-msvc.exe'
$VersionFile = Join-Path $BinariesDir 'FFMPEG_VERSION.txt'

if ((Test-Path $Target) -and -not $Force) {
  Write-Host "ffmpeg sidecar already present at $Target (use -Force to replace)."
  exit 0
}

# Download into a fresh, empty directory so nothing else is executed from the archive location.
$WorkDir = Join-Path ([System.IO.Path]::GetTempPath()) ("drivetube-ffmpeg-" + [guid]::NewGuid())
New-Item -ItemType Directory -Path $WorkDir | Out-Null

try {
  $ZipPath = Join-Path $WorkDir $AssetName
  Write-Host "Downloading $Url"
  $ProgressPreference = 'SilentlyContinue'
  Invoke-WebRequest -Uri $Url -OutFile $ZipPath -UseBasicParsing

  $ActualSha256 = (Get-FileHash -Path $ZipPath -Algorithm SHA256).Hash.ToLowerInvariant()
  if ($ActualSha256 -ne $ExpectedSha256) {
    throw "SHA-256 mismatch for $AssetName. Expected $ExpectedSha256, got $ActualSha256."
  }
  Write-Host 'SHA-256 verified.'

  $ExtractDir = Join-Path $WorkDir 'extracted'
  Expand-Archive -Path $ZipPath -DestinationPath $ExtractDir
  $FfmpegExe = Get-ChildItem -Path $ExtractDir -Recurse -Filter 'ffmpeg.exe' |
    Where-Object { $_.Directory.Name -eq 'bin' } |
    Select-Object -First 1
  if (-not $FfmpegExe) {
    throw "ffmpeg.exe not found inside $AssetName."
  }

  New-Item -ItemType Directory -Force -Path $BinariesDir | Out-Null
  Copy-Item -Path $FfmpegExe.FullName -Destination $Target -Force
  $BinarySha256 = (Get-FileHash -Path $Target -Algorithm SHA256).Hash.ToLowerInvariant()
  @(
    "ffmpeg $Version (gyan.dev essentials build, GPLv3)",
    "source: $Url",
    "archive sha256: $ExpectedSha256",
    "ffmpeg.exe sha256: $BinarySha256"
  ) | Set-Content -Path $VersionFile -Encoding utf8

  Write-Host "Installed ffmpeg $Version sidecar at $Target"
}
finally {
  Remove-Item -Recurse -Force -Path $WorkDir -ErrorAction SilentlyContinue
}
