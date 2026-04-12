[CmdletBinding()]
param(
  [switch]$SkipInstall,
  [switch]$SkipFrontendBuild,
  [switch]$SkipWebBuild,
  [switch]$SkipServiceBuild,
  [switch]$DryRun
)

$ErrorActionPreference = "Stop"

$scriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$root = Split-Path -Parent $scriptDir
$appsRoot = Join-Path $root "apps"
$distIndex = Join-Path $appsRoot "out\index.html"
$webBinary = Join-Path $root "target\debug\codexmanager-web.exe"
$serviceBinary = Join-Path $root "target\debug\codexmanager-service.exe"
$startBinary = Join-Path $root "target\debug\codexmanager-start.exe"

function Write-Step {
  param([string]$Message)
  Write-Output $Message
}

function Assert-Command {
  param(
    [string]$Name,
    [string]$Reason
  )

  if (-not (Get-Command $Name -ErrorAction SilentlyContinue)) {
    throw "$Name not found in PATH. $Reason"
  }
}

function Invoke-Step {
  param(
    [string]$CommandLine,
    [scriptblock]$Action
  )

  if ($DryRun) {
    Write-Step "DRY RUN: $CommandLine"
    return
  }

  & $Action
  if ($LASTEXITCODE -ne 0) {
    throw "command failed: $CommandLine"
  }
}

function Assert-Artifact {
  param(
    [string]$Path,
    [string]$Label
  )

  if ($DryRun) {
    Write-Step "DRY RUN: verify $Label exists -> $Path"
    return
  }

  if (-not (Test-Path $Path)) {
    throw "$Label missing: $Path"
  }
}

function Stop-MatchingProcess {
  param(
    [string]$ImageName,
    [string]$ExpectedPath,
    [string]$Label
  )

  $expectedFullPath = [System.IO.Path]::GetFullPath($ExpectedPath)
  $processes = Get-CimInstance Win32_Process -Filter "Name = '$ImageName'" -ErrorAction SilentlyContinue |
    Where-Object {
      $_.ExecutablePath -and
      ([System.IO.Path]::GetFullPath($_.ExecutablePath) -eq $expectedFullPath)
    }

  foreach ($process in $processes) {
    if ($DryRun) {
      Write-Step "DRY RUN: stop $Label process -> PID $($process.ProcessId) ($expectedFullPath)"
      continue
    }

    Write-Step "stop $Label process -> PID $($process.ProcessId) ($expectedFullPath)"
    Stop-Process -Id $process.ProcessId -Force
  }
}

Write-Step "repo root: $root"
Write-Step "apps root: $appsRoot"
Write-Step "frontend index: $distIndex"
Write-Step "expected web binary: $webBinary"
Write-Step "expected service binary: $serviceBinary"
Write-Step "expected start binary: $startBinary"

if (-not (Test-Path $appsRoot)) {
  throw "apps directory not found: $appsRoot"
}

Assert-Command -Name "git" -Reason "This script is intended to run from the repository workspace."
Assert-Command -Name "cargo" -Reason "Rust builds are required for codexmanager-start and its siblings."

if ((-not $SkipInstall) -or (-not $SkipFrontendBuild)) {
  Assert-Command -Name "pnpm" -Reason "Frontend install/build is required unless both -SkipInstall and -SkipFrontendBuild are set."
}

Stop-MatchingProcess -ImageName "codexmanager-start.exe" -ExpectedPath $startBinary -Label "start"
Stop-MatchingProcess -ImageName "codexmanager-web.exe" -ExpectedPath $webBinary -Label "web"
Stop-MatchingProcess -ImageName "codexmanager-service.exe" -ExpectedPath $serviceBinary -Label "service"

Push-Location $root
try {
  if (-not $SkipInstall) {
    Invoke-Step -CommandLine "pnpm -C `"$appsRoot`" install" -Action {
      & pnpm -C $appsRoot install
    }
  }

  if (-not $SkipFrontendBuild) {
    Invoke-Step -CommandLine "pnpm -C `"$appsRoot`" run build:desktop" -Action {
      & pnpm -C $appsRoot run build:desktop
    }
  }

  Assert-Artifact -Path $distIndex -Label "frontend artifact"

  if (-not $SkipWebBuild) {
    Invoke-Step -CommandLine "cargo build -p codexmanager-web --features embedded-ui" -Action {
      & cargo build -p codexmanager-web --features embedded-ui
    }
  }

  if (-not $SkipServiceBuild) {
    Invoke-Step -CommandLine "cargo build -p codexmanager-service" -Action {
      & cargo build -p codexmanager-service
    }
  }

  Invoke-Step -CommandLine "cargo build -p codexmanager-start" -Action {
    & cargo build -p codexmanager-start
  }
} finally {
  Pop-Location
}

Assert-Artifact -Path $webBinary -Label "web binary"
Assert-Artifact -Path $serviceBinary -Label "service binary"
Assert-Artifact -Path $startBinary -Label "start binary"

Write-Step "done"
Write-Step "start launcher ready: $startBinary"
