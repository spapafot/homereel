<#
.SYNOPSIS
  Generates synthetic test videos for the playback proof-of-concept matrix.

.DESCRIPTION
  Writes 3-minute 720p test-pattern clips with a burned-in timestamp and a tone, in the
  containers and codecs the PoC must cover, to poc-fixtures/ (gitignored). Upload the folder to
  Google Drive, then test each file with methods (a)-(d) on the /poc page. Real phone videos,
  an iPhone HEVC .mov and a file over 2 GB should be tested too; they are not synthesised here.
  Requires the ffmpeg sidecar (run scripts/fetch-ffmpeg.ps1 first).
#>
[CmdletBinding()]
param(
  [int]$DurationSeconds = 180
)

$ErrorActionPreference = 'Stop'

$RepoRoot = Split-Path -Parent $PSScriptRoot
$Ffmpeg = Join-Path $RepoRoot 'src-tauri\binaries\ffmpeg-x86_64-pc-windows-msvc.exe'
$OutDir = Join-Path $RepoRoot 'poc-fixtures'

if (-not (Test-Path $Ffmpeg)) {
  throw 'ffmpeg sidecar not found. Run scripts/fetch-ffmpeg.ps1 first.'
}
New-Item -ItemType Directory -Force -Path $OutDir | Out-Null

# A visible running clock makes seek accuracy easy to check by eye. The static ffmpeg build
# prints "Fontconfig error: Cannot load default config file"; drawtext still finds a font.
$Video = "testsrc2=size=1280x720:rate=30,drawtext=text='%{pts\:hms}':fontsize=72:fontcolor=white:box=1:boxcolor=black@0.6:x=(w-tw)/2:y=h-120"
$Audio = 'sine=frequency=440:sample_rate=48000'

$Fixtures = @(
  @{ Name = '01-h264-aac-faststart.mp4'; Args = @('-c:v', 'libx264', '-preset', 'veryfast', '-pix_fmt', 'yuv420p', '-c:a', 'aac', '-movflags', '+faststart') },
  # Without +faststart the moov atom is written at the end, like many camera and screen recordings.
  @{ Name = '02-h264-aac-moov-at-end.mp4'; Args = @('-c:v', 'libx264', '-preset', 'veryfast', '-pix_fmt', 'yuv420p', '-c:a', 'aac') },
  @{ Name = '03-h264-aac.mov'; Args = @('-c:v', 'libx264', '-preset', 'veryfast', '-pix_fmt', 'yuv420p', '-c:a', 'aac') },
  @{ Name = '04-hevc-aac.mov'; Args = @('-c:v', 'libx265', '-preset', 'fast', '-tag:v', 'hvc1', '-pix_fmt', 'yuv420p', '-c:a', 'aac') },
  @{ Name = '05-vp9-opus.webm'; Args = @('-c:v', 'libvpx-vp9', '-b:v', '1M', '-deadline', 'realtime', '-cpu-used', '8', '-c:a', 'libopus') },
  @{ Name = '06-h264-aac.mkv'; Args = @('-c:v', 'libx264', '-preset', 'veryfast', '-pix_fmt', 'yuv420p', '-c:a', 'aac') },
  @{ Name = '07-mpeg4-mp3.avi'; Args = @('-c:v', 'mpeg4', '-q:v', '5', '-c:a', 'libmp3lame') }
)

foreach ($Fixture in $Fixtures) {
  $Output = Join-Path $OutDir $Fixture.Name
  Write-Host "Creating $($Fixture.Name)"
  $FfmpegArgs = @(
    '-hide_banner', '-loglevel', 'error', '-y',
    '-f', 'lavfi', '-i', $Video,
    '-f', 'lavfi', '-i', $Audio,
    '-t', $DurationSeconds
  ) + $Fixture.Args + @($Output)
  & $Ffmpeg @FfmpegArgs
  if ($LASTEXITCODE -ne 0) {
    throw "ffmpeg failed for $($Fixture.Name)"
  }
}

Write-Host "Fixtures written to $OutDir"
