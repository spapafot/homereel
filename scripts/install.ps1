<#
.SYNOPSIS
  Builds Homereel from source and installs it for the current Windows user.

.DESCRIPTION
  Checks the toolchain (Git, Node.js 24+, Visual Studio Build Tools with the C++ workload, Rust)
  and offers to install what is missing with winget. Then it builds the checkout this script is
  in, or clones the repository to -Dir (and pulls it on later runs), adds the Google OAuth client,
  fetches ffmpeg, builds the installer and runs it silently.

  From a checkout:
    powershell -ExecutionPolicy Bypass -File scripts/install.ps1 [-GoogleClient <path>]
  Without one:
    irm https://raw.githubusercontent.com/spapafot/homereel/main/scripts/install.ps1 | iex

  Piped to iex, the script takes its parameters from HOMEREEL_GOOGLE_CLIENT and HOMEREEL_DIR.
#>
[CmdletBinding()]
param(
  # The Desktop app OAuth client JSON downloaded from Google Cloud (docs/setup.md). Without it,
  # and without one already in the checkout, the app cannot connect to Google Drive.
  [string]$GoogleClient = $env:HOMEREEL_GOOGLE_CLIENT,
  # Where to clone the source when the script does not run from a checkout.
  [string]$Dir = $env:HOMEREEL_DIR
)

# The body runs in a child scope so that, piped to iex, it leaves the caller's session settings
# alone. It throws instead of calling exit, which would close the caller's window.
& {
  $ErrorActionPreference = 'Stop'

  $RepoUrl = 'https://github.com/spapafot/homereel.git'
  $MinNodeMajor = 24

  function Write-Step([string]$Message) {
    Write-Host ''
    Write-Host "==> $Message" -ForegroundColor Cyan
  }

  # Windows PowerShell does not stop on a native command's exit code. Where it turns the command's
  # stderr into error records (ISE, redirected output), Stop would abort on the first progress line.
  function Invoke-Checked {
    $ErrorActionPreference = 'Continue'
    $Command = $args[0]
    $Arguments = @($args | Select-Object -Skip 1)
    & $Command @Arguments
    if ($LASTEXITCODE -ne 0) {
      throw "'$($args -join ' ')' failed with exit code $LASTEXITCODE."
    }
  }

  # Picks up tools installed after this terminal opened, including the ones winget installs below.
  function Update-SessionPath {
    $Entries = @($env:Path -split ';') +
      @([Environment]::GetEnvironmentVariable('Path', 'Machine') -split ';') +
      @([Environment]::GetEnvironmentVariable('Path', 'User') -split ';')
    $env:Path = ($Entries | Where-Object { $_ } | Select-Object -Unique) -join ';'
  }

  function Get-NodeMajor {
    if (-not (Get-Command node -ErrorAction SilentlyContinue)) {
      return 0
    }
    return [int](& node --version).TrimStart('v').Split('.')[0]
  }

  # A rustup without a default toolchain still puts cargo on the PATH, so run it.
  function Test-Cargo {
    $ErrorActionPreference = 'Continue'
    if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
      return $false
    }
    & cargo --version 2>$null | Out-Null
    return $LASTEXITCODE -eq 0
  }

  function Test-CppBuildTools {
    $VsWhere = Join-Path ${env:ProgramFiles(x86)} 'Microsoft Visual Studio\Installer\vswhere.exe'
    if (-not (Test-Path $VsWhere)) {
      return $false
    }
    $Installs = & $VsWhere -products '*' -property installationPath `
      -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64
    return [bool]$Installs
  }

  # Build Tools come before Rust, whose installer otherwise asks to install them itself.
  $Tools = @(
    @{
      Name = 'Git'
      WingetId = 'Git.Git'
      Test = { [bool](Get-Command git -ErrorAction SilentlyContinue) }
    }
    @{
      Name = "Node.js $MinNodeMajor or later"
      WingetId = 'OpenJS.NodeJS.LTS'
      Test = { (Get-NodeMajor) -ge $MinNodeMajor }
    }
    @{
      Name = 'Visual Studio Build Tools with the C++ workload'
      WingetId = 'Microsoft.VisualStudio.2022.BuildTools'
      Override = '--wait --passive --includeRecommended ' +
        '--add Microsoft.VisualStudio.Workload.VCTools'
      Test = { Test-CppBuildTools }
    }
    @{
      Name = 'Rust'
      WingetId = 'Rustlang.Rustup'
      Test = { Test-Cargo }
    }
  )

  $ToolchainHelp = 'Install them as described in docs/setup.md#toolchain'

  function Install-Toolchain {
    Update-SessionPath
    $Missing = @($Tools | Where-Object { -not (& $_.Test) })
    if ($Missing.Count -eq 0) {
      return
    }

    Write-Step 'Missing tools'
    $Missing | ForEach-Object { Write-Host "  - $($_.Name)" }
    if (-not (Get-Command winget -ErrorAction SilentlyContinue)) {
      throw "winget is not available. $ToolchainHelp and run this again."
    }
    if ($Missing | Where-Object { $_.Override }) {
      Write-Host 'The Build Tools download several GB and ask for administrator approval.'
    }
    $Answer = Read-Host 'Install them with winget now? [Y/n]'
    if ($Answer -match '^\s*n') {
      throw "$ToolchainHelp and run this again."
    }

    foreach ($Tool in $Missing) {
      Write-Step "Installing $($Tool.Name)"
      $WingetArgs = @(
        'install', '--id', $Tool.WingetId, '--exact', '--source', 'winget',
        '--accept-package-agreements', '--accept-source-agreements'
      )
      if ($Tool.Override) {
        $WingetArgs += @('--override', $Tool.Override)
      }
      # winget also fails when the package is installed but too old or incomplete, so the checks
      # below decide.
      & winget @WingetArgs
    }

    Update-SessionPath
    if ((Get-Command rustup -ErrorAction SilentlyContinue) -and -not (Test-Cargo)) {
      Invoke-Checked rustup default stable
    }
    $StillMissing = @($Missing | Where-Object { -not (& $_.Test) })
    if ($StillMissing.Count -gt 0) {
      $Names = ($StillMissing | ForEach-Object { $_.Name }) -join ', '
      throw "Still missing: $Names. $ToolchainHelp, open a new terminal and run this again."
    }
  }

  function Copy-GoogleClient([string]$Source, [string]$Target) {
    if (-not (Test-Path -LiteralPath $Source -PathType Leaf)) {
      throw "Google OAuth client file not found: $Source"
    }
    $Client = Get-Content -LiteralPath $Source -Raw | ConvertFrom-Json
    if (-not $Client.installed.client_id) {
      throw "$Source is not a Desktop app OAuth client. Create one as described in docs/setup.md."
    }
    if ((Resolve-Path -LiteralPath $Source).ProviderPath -ne $Target) {
      Copy-Item -LiteralPath $Source -Destination $Target -Force
    }
    Write-Host "Using the Google OAuth client from $Source"
  }

  if ([System.Runtime.InteropServices.RuntimeInformation]::OSArchitecture -ne 'X64') {
    throw 'Homereel builds for x64 Windows only.'
  }

  Install-Toolchain

  $ScriptCheckout = if ($PSScriptRoot) { Split-Path -Parent $PSScriptRoot }
  $IsScriptCheckout = $ScriptCheckout -and
    (Test-Path (Join-Path $ScriptCheckout 'src-tauri\tauri.conf.json'))
  if ($IsScriptCheckout) {
    $RepoRoot = $ScriptCheckout
    Write-Step "Building the checkout at $RepoRoot"
  } else {
    if (-not $Dir) {
      $Dir = Join-Path $HOME 'homereel'
    }
    $RepoRoot = $ExecutionContext.SessionState.Path.GetUnresolvedProviderPathFromPSPath($Dir)
    if (Test-Path (Join-Path $RepoRoot 'src-tauri\tauri.conf.json')) {
      Write-Step "Updating $RepoRoot"
      Invoke-Checked git -C $RepoRoot pull --ff-only
    } elseif (Get-ChildItem -Force $RepoRoot -ErrorAction SilentlyContinue) {
      throw "$RepoRoot exists and is not a Homereel checkout. Set HOMEREEL_DIR to another folder."
    } else {
      Write-Step "Cloning $RepoUrl to $RepoRoot"
      Invoke-Checked git clone $RepoUrl $RepoRoot
    }
  }

  $ClientTarget = Join-Path $RepoRoot 'src-tauri\google-oauth-client.json'
  $HasClient = (Test-Path $ClientTarget) -or [bool]$env:HOMEREEL_GOOGLE_CLIENT_ID
  if (-not $GoogleClient -and -not $HasClient) {
    Write-Step 'Google Drive access'
    Write-Host 'Homereel reads Google Drive with your own OAuth client. docs/setup.md shows how to'
    Write-Host 'create one (about 10 minutes). Videos in folders on this computer work without it.'
    $Answer = Read-Host 'Path to its JSON file, or Enter to build without Google Drive'
    $GoogleClient = $Answer.Trim().Trim('"')
  }
  if ($GoogleClient) {
    Copy-GoogleClient $GoogleClient $ClientTarget
    $HasClient = $true
  }

  Push-Location $RepoRoot
  try {
    Write-Step 'Installing npm packages'
    # npm.cmd, because npm.ps1 does not run where the execution policy blocks scripts.
    Invoke-Checked npm.cmd ci
    Write-Step 'Fetching ffmpeg'
    Invoke-Checked powershell.exe -NoProfile -ExecutionPolicy Bypass -File scripts\fetch-ffmpeg.ps1
    Write-Step 'Building Homereel (the first build takes several minutes)'
    Invoke-Checked npm.cmd run build
  } finally {
    Pop-Location
  }

  $BundleDir = Join-Path $RepoRoot 'src-tauri\target\release\bundle\nsis'
  $Setup = Get-ChildItem -Path $BundleDir -Filter '*-setup.exe' |
    Sort-Object LastWriteTime -Descending |
    Select-Object -First 1
  if (-not $Setup) {
    throw "No installer found in $BundleDir."
  }
  Write-Step "Installing $($Setup.Name)"
  # Silent mode installs for the current user and closes a running Homereel first.
  $Installer = Start-Process -FilePath $Setup.FullName -ArgumentList '/S' -Wait -PassThru
  if ($Installer.ExitCode -ne 0) {
    throw "The installer failed with exit code $($Installer.ExitCode)."
  }

  Write-Host ''
  Write-Host 'Homereel is installed. Start it from the Start menu.' -ForegroundColor Green
  if (-not $HasClient) {
    Write-Warning ('This build cannot connect to Google Drive. Create the OAuth client ' +
      '(docs/setup.md) and run this again.')
  }
  if (-not $IsScriptCheckout) {
    Write-Host "To update, run the same command again. The source stays in $RepoRoot."
  }
}
