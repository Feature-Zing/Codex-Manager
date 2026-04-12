<#
.SYNOPSIS
  构建 service/web/start 三个模块并打包为 zip 压缩包。

.DESCRIPTION
  完整流程:
    1. 安装前端依赖 (pnpm install)
    2. 构建前端产物 (pnpm build:desktop -> apps/out/)
    3. Release 编译 codexmanager-web (带 embedded-ui 特性，内嵌前端)
    4. Release 编译 codexmanager-service
    5. Release 编译 codexmanager-start
    6. 将三个 exe 打包为 zip，放在项目根目录

.PARAMETER SkipInstall
  跳过 pnpm install 步骤

.PARAMETER SkipFrontendBuild
  跳过前端构建步骤（需要 apps/out/index.html 已存在）

.PARAMETER SkipRustBuild
  跳过 Rust 编译步骤（需要 target/release/ 下的二进制已存在）

.PARAMETER DebugBuild
  使用 debug 模式编译（默认 release）

.PARAMETER DryRun
  仅打印将要执行的命令，不实际执行

.EXAMPLE
  .\scripts\package-service.ps1
  # 完整构建并打包

.EXAMPLE
  .\scripts\package-service.ps1 -SkipInstall -SkipFrontendBuild
  # 跳过前端步骤，仅编译 Rust 并打包
#>

[CmdletBinding()]
param(
  [switch]$SkipInstall,
  [switch]$SkipFrontendBuild,
  [switch]$SkipRustBuild,
  [switch]$DebugBuild,
  [switch]$DryRun
)

$ErrorActionPreference = "Stop"

# ---------------------------------------------------------------------------
# 路径定义
# ---------------------------------------------------------------------------
$PKG_SCRIPT_DIR = Split-Path -Parent $MyInvocation.MyCommand.Path
$PKG_ROOT = Split-Path -Parent $PKG_SCRIPT_DIR
$PKG_APPS_ROOT = Join-Path $PKG_ROOT "apps"
$PKG_DIST_INDEX = Join-Path $PKG_APPS_ROOT "out\index.html"

# 根据编译模式选择输出目录
$PKG_BUILD_PROFILE = "release"
if ($DebugBuild) { $PKG_BUILD_PROFILE = "debug" }
$PKG_RELEASE_DIR = Join-Path (Join-Path $PKG_ROOT "target") $PKG_BUILD_PROFILE

# 二进制名称列表
$PKG_BINARY_NAMES = @("codexmanager-service", "codexmanager-web", "codexmanager-start")
$PKG_BINARY_EXT = ".exe"

# 版本号（从 Cargo.toml 读取）
$PKG_VERSION = "unknown"
$cargoTomlPath = Join-Path $PKG_ROOT "Cargo.toml"
if (Test-Path $cargoTomlPath) {
  $cargoContent = Get-Content $cargoTomlPath -Raw
  if ($cargoContent -match 'version\s*=\s*"([^"]+)"') {
    $PKG_VERSION = $Matches[1]
  }
}

# 平台与架构（硬编码 Windows x86_64）
$PKG_PLATFORM = "windows"
$PKG_ARCH = "x86_64"

# 输出 zip 文件名和路径
$PKG_PACKAGE_NAME = "CodexManager-service-$PKG_PLATFORM-$PKG_ARCH"
$PKG_ZIP_FILE_NAME = "$PKG_PACKAGE_NAME.zip"
$PKG_ZIP_FILE_PATH = Join-Path $PKG_ROOT $PKG_ZIP_FILE_NAME

# release 编译标志
$PKG_RELEASE_FLAG = "--release"
if ($DebugBuild) { $PKG_RELEASE_FLAG = "" }

# ---------------------------------------------------------------------------
# 调试输出：确认变量值
# ---------------------------------------------------------------------------
Write-Host ""
Write-Host "============================================" -ForegroundColor Green
Write-Host "  CodexManager Service Package Builder" -ForegroundColor Green
Write-Host "  Version: $PKG_VERSION" -ForegroundColor Green
Write-Host "  Profile: $PKG_BUILD_PROFILE" -ForegroundColor Green
Write-Host "  Platform: $PKG_PLATFORM-$PKG_ARCH" -ForegroundColor Green
Write-Host "  Package: $PKG_PACKAGE_NAME" -ForegroundColor Green
Write-Host "============================================" -ForegroundColor Green
Write-Host ""

Write-Host "[package] project root: $PKG_ROOT" -ForegroundColor Cyan
Write-Host "[package] release dir: $PKG_RELEASE_DIR" -ForegroundColor Cyan
Write-Host "[package] output zip: $PKG_ZIP_FILE_PATH" -ForegroundColor Cyan

# 检查必要工具
if (-not (Get-Command "git" -ErrorAction SilentlyContinue)) {
  throw "git not found in PATH. This script must run from the repository workspace."
}
if (-not (Get-Command "cargo" -ErrorAction SilentlyContinue)) {
  throw "cargo not found in PATH. Rust toolchain is required for compilation."
}
if ((-not $SkipInstall) -or (-not $SkipFrontendBuild)) {
  if (-not (Get-Command "pnpm" -ErrorAction SilentlyContinue)) {
    throw "pnpm not found in PATH. Use -SkipInstall -SkipFrontendBuild to skip frontend steps."
  }
}

Push-Location $PKG_ROOT
try {
  # -----------------------------------------------------------------------
  # 步骤 1: 安装前端依赖
  # -----------------------------------------------------------------------
  if (-not $SkipInstall) {
    Write-Host "[package] pnpm install (frontend dependencies)" -ForegroundColor Cyan
    if (-not $DryRun) {
      & pnpm -C $PKG_APPS_ROOT install
      if ($LASTEXITCODE -ne 0) { throw "pnpm install failed (exit code: $LASTEXITCODE)" }
    } else {
      Write-Host "  [DRY RUN] skipped" -ForegroundColor Yellow
    }
  } else {
    Write-Host "[package] SKIP: pnpm install" -ForegroundColor Cyan
  }

  # -----------------------------------------------------------------------
  # 步骤 2: 构建前端产物
  # -----------------------------------------------------------------------
  if (-not $SkipFrontendBuild) {
    Write-Host "[package] pnpm build:desktop (frontend build)" -ForegroundColor Cyan
    if (-not $DryRun) {
      & pnpm -C $PKG_APPS_ROOT run build:desktop
      if ($LASTEXITCODE -ne 0) { throw "pnpm build:desktop failed (exit code: $LASTEXITCODE)" }
    } else {
      Write-Host "  [DRY RUN] skipped" -ForegroundColor Yellow
    }
  } else {
    Write-Host "[package] SKIP: frontend build" -ForegroundColor Cyan
  }

  # 验证前端产物存在（web 的 embedded-ui 特性需要它）
  if ((-not $DryRun) -and (-not (Test-Path $PKG_DIST_INDEX -PathType Leaf))) {
    throw "frontend artifact not found: $PKG_DIST_INDEX"
  }

  # -----------------------------------------------------------------------
  # 步骤 3: 编译 Rust 二进制
  # -----------------------------------------------------------------------
  if (-not $SkipRustBuild) {
    # codexmanager-web (with embedded-ui)
    Write-Host "[package] cargo build: codexmanager-web (with embedded-ui)" -ForegroundColor Cyan
    if (-not $DryRun) {
      if ($PKG_RELEASE_FLAG) {
        & cargo build -p codexmanager-web --features embedded-ui $PKG_RELEASE_FLAG
      } else {
        & cargo build -p codexmanager-web --features embedded-ui
      }
      if ($LASTEXITCODE -ne 0) { throw "cargo build codexmanager-web failed (exit code: $LASTEXITCODE)" }
    } else {
      Write-Host "  [DRY RUN] skipped" -ForegroundColor Yellow
    }

    # codexmanager-service
    Write-Host "[package] cargo build: codexmanager-service" -ForegroundColor Cyan
    if (-not $DryRun) {
      if ($PKG_RELEASE_FLAG) {
        & cargo build -p codexmanager-service $PKG_RELEASE_FLAG
      } else {
        & cargo build -p codexmanager-service
      }
      if ($LASTEXITCODE -ne 0) { throw "cargo build codexmanager-service failed (exit code: $LASTEXITCODE)" }
    } else {
      Write-Host "  [DRY RUN] skipped" -ForegroundColor Yellow
    }

    # codexmanager-start
    Write-Host "[package] cargo build: codexmanager-start" -ForegroundColor Cyan
    if (-not $DryRun) {
      if ($PKG_RELEASE_FLAG) {
        & cargo build -p codexmanager-start $PKG_RELEASE_FLAG
      } else {
        & cargo build -p codexmanager-start
      }
      if ($LASTEXITCODE -ne 0) { throw "cargo build codexmanager-start failed (exit code: $LASTEXITCODE)" }
    } else {
      Write-Host "  [DRY RUN] skipped" -ForegroundColor Yellow
    }
  } else {
    Write-Host "[package] SKIP: Rust build" -ForegroundColor Cyan
  }

  # 验证所有二进制存在
  if (-not $DryRun) {
    foreach ($binName in $PKG_BINARY_NAMES) {
      $binPath = Join-Path $PKG_RELEASE_DIR ($binName + $PKG_BINARY_EXT)
      if (-not (Test-Path $binPath -PathType Leaf)) {
        throw "binary not found: $binPath"
      }
      $binSize = [math]::Round((Get-Item $binPath).Length / 1MB, 2)
      Write-Host "[package] $binName OK ($binSize MB)" -ForegroundColor Cyan
    }
  }

  # -----------------------------------------------------------------------
  # 步骤 4: 打包为 zip
  # -----------------------------------------------------------------------
  Write-Host "[package] packaging binaries into zip..." -ForegroundColor Cyan

  # 创建临时打包目录
  $stagingDir = Join-Path $PKG_ROOT "stage"
  $packageDir = Join-Path $stagingDir $PKG_PACKAGE_NAME

  if (-not $DryRun) {
    # 清理旧的临时目录和 zip
    if (Test-Path $packageDir) { Remove-Item -Recurse -Force $packageDir }
    if (Test-Path $PKG_ZIP_FILE_PATH) { Remove-Item -Force $PKG_ZIP_FILE_PATH }

    New-Item -ItemType Directory -Force $packageDir | Out-Null

    # 复制二进制到打包目录
    foreach ($binName in $PKG_BINARY_NAMES) {
      $srcPath = Join-Path $PKG_RELEASE_DIR ($binName + $PKG_BINARY_EXT)
      $dstPath = Join-Path $packageDir ($binName + $PKG_BINARY_EXT)
      Copy-Item -Force $srcPath $dstPath
      Write-Host "[package]   copied: $binName$PKG_BINARY_EXT" -ForegroundColor Cyan
    }

    # 创建 zip 压缩包
    Compress-Archive -Path $packageDir -DestinationPath $PKG_ZIP_FILE_PATH -Force

    # 清理临时目录
    Remove-Item -Recurse -Force $stagingDir

    # 验证 zip 文件
    if (-not (Test-Path $PKG_ZIP_FILE_PATH -PathType Leaf)) {
      throw "output zip not found: $PKG_ZIP_FILE_PATH"
    }
    $zipSize = [math]::Round((Get-Item $PKG_ZIP_FILE_PATH).Length / 1MB, 2)
    Write-Host "[package] output zip OK ($zipSize MB)" -ForegroundColor Cyan
  } else {
    Write-Host "  [DRY RUN] would create $PKG_ZIP_FILE_PATH" -ForegroundColor Yellow
  }

} finally {
  Pop-Location
}

# ---------------------------------------------------------------------------
# 完成
# ---------------------------------------------------------------------------
Write-Host ""
Write-Host "============================================" -ForegroundColor Green
Write-Host "  Package complete!" -ForegroundColor Green
Write-Host "  Output: $PKG_ZIP_FILE_PATH" -ForegroundColor Green
Write-Host "============================================" -ForegroundColor Green
Write-Host ""
