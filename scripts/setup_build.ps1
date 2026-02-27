<#
.SYNOPSIS
    setup_build.ps1 - Windows development environment setup for Aptos Core

.DESCRIPTION
    Installs the tools and libraries required to build, test, and develop
    Aptos Core on Windows.  Uses WinGet for most packages.

    Run with -Help for full usage information.

.NOTES
    Counterpart to scripts/setup_build.sh (Linux/macOS).
    To add a new tool, add a version constant, an install_* function,
    and wire it into the appropriate install group.
#>

[CmdletBinding()]
param(
    [Alias("b")]
    [switch]$Batch,

    [Alias("t")]
    [switch]$BuildTools,

    [Alias("y")]
    [switch]$Prover,

    [Alias("k")]
    [switch]$SkipPreCommit,

    [Alias("h")]
    [switch]$Help
)

$ErrorActionPreference = 'Stop'

# ============================================================================
# Tool Versions (pinned for reproducible builds)
# ============================================================================

# Protobuf compiler (https://github.com/protocolbuffers/protobuf)
$PROTOC_VERSION        = "29.3"

# grcov -- Rust code-coverage aggregator (https://github.com/mozilla/grcov)
$GRCOV_VERSION         = "0.10.5"

# cargo-sort -- sorts Cargo.toml dependency sections
$CARGO_SORT_VERSION    = "2.0.2"

# cargo-machete -- detects unused crate dependencies
$CARGO_MACHETE_VERSION = "0.9.1"

# cargo-nextest -- faster Rust test runner
$CARGO_NEXTEST_VERSION = "0.9.128"

# .NET SDK -- runtime for the Boogie verifier
$DOTNET_VERSION        = "8.0"

# Z3 -- SMT solver for the Move Prover
$Z3_VERSION            = "4.11.2"

# Boogie -- verification language for the Move Prover
$BOOGIE_VERSION        = "3.5.6"

# cvc5 -- alternative SMT solver for the Move Prover
$CVC5_VERSION          = "0.0.3"

# Node.js major version (LTS)
$NODE_MAJOR_VERSION    = "22"

# pnpm -- Node.js package manager
$PNPM_VERSION          = "10.6.4"

# ============================================================================
# Logging Utilities
# ============================================================================

function Write-Step  { param([string]$Msg) Write-Host "[STEP]  $Msg" -ForegroundColor Cyan }
function Write-Info  { param([string]$Msg) Write-Host "[INFO]  $Msg" -ForegroundColor Green }
function Write-Warn  { param([string]$Msg) Write-Host "[WARN]  $Msg" -ForegroundColor Yellow }
function Write-Err   { param([string]$Msg) Write-Host "[ERROR] $Msg" -ForegroundColor Red }

function Stop-WithError {
    param([string[]]$Messages)
    foreach ($m in $Messages) { Write-Err $m }
    exit 1
}

# ============================================================================
# Help
# ============================================================================

function Show-Help {
    Write-Host @"

NAME
    setup_build.ps1 - Windows development environment setup for Aptos Core

SYNOPSIS
    .\scripts\setup_build.ps1 [OPTIONS]

DESCRIPTION
    Installs or updates the tools needed to build, test, and develop
    aptos-core on Windows.  Uses WinGet for most packages.

    If no component flags (-t, -y) are provided, an interactive prompt
    is shown.  Use -b to suppress the prompt and default to -t.

OPTIONS
    -t, -BuildTools   Install core build tools:
                        Rust toolchain (stable + nightly, rustfmt, clippy)
                        MSVC Build Tools (C++ compiler, Windows SDK, CMake)
                        LLVM/Clang
                        OpenSSL
                        protoc (Protocol Buffers compiler + Rust plugins)
                        Python 3 + pip
                        Node.js + npm + pnpm
                        PostgreSQL
                        grcov, cargo-sort, cargo-machete, cargo-nextest

    -y, -Prover       Install Move Prover tools:
                        .NET SDK
                        Z3 (SMT solver)
                        Boogie (verification language)
                        Git (if not present)

    -b, -Batch        Batch/CI mode.  Suppresses interactive prompts.
                      Defaults to -t if no component flag is given.

    -k, -SkipPreCommit  (reserved for future use)

    -h, -Help         Show this help message and exit.

EXAMPLES
    # Interactive selection
    .\scripts\setup_build.ps1

    # CI: install build tools non-interactively
    .\scripts\setup_build.ps1 -b -t

    # Install Move Prover tools
    .\scripts\setup_build.ps1 -b -y

    # Install everything
    .\scripts\setup_build.ps1 -b -t -y
"@
}

# ============================================================================
# Resolve script location and cd to repo root
# ============================================================================

$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
Set-Location (Join-Path $ScriptDir "..") -ErrorAction Stop

if (-not (Test-Path "rust-toolchain.toml")) {
    Stop-WithError "Cannot find rust-toolchain.toml in $(Get-Location).",
                   "This script must be run from the aptos-core repository root."
}

# ============================================================================
# Handle -Help
# ============================================================================

if ($Help) {
    Show-Help
    exit 0
}

# ============================================================================
# Architecture detection
# ============================================================================

$Arch = if ([Environment]::Is64BitOperatingSystem) { "64" } else { "86" }

# ============================================================================
# OS check
# ============================================================================

$OsCaption = (Get-CimInstance Win32_OperatingSystem).Caption
if ($OsCaption -notmatch "Windows (10|11|Server 20)") {
    Stop-WithError "Unsupported OS: $OsCaption",
                   "This script supports Windows 10, 11, and Server 2019+."
}

# ============================================================================
# WinGet bootstrap
# ============================================================================

function Ensure-WinGet {
    if (Get-Command winget -ErrorAction SilentlyContinue) { return }
    if (Test-Path "$env:LOCALAPPDATA\Microsoft\WindowsApps\winget.exe") {
        $env:Path += ";$env:LOCALAPPDATA\Microsoft\WindowsApps"
        return
    }
    Stop-WithError "WinGet is not installed.",
                   "Install it from https://aka.ms/getwinget or the Microsoft Store,",
                   "then re-run this script."
}

function Reload-Path {
    $env:Path = [Environment]::GetEnvironmentVariable("PATH", "User") + ";" +
                [Environment]::GetEnvironmentVariable("PATH", "Machine")
}

# ============================================================================
# Package helpers
# ============================================================================

function Install-WinGetPackage {
    param(
        [string]$Id,
        [string]$Name
    )
    $list = winget list --id $Id 2>&1 | Out-String
    if ($list -match "No installed package") {
        Write-Info "Installing $Name..."
        winget install --id $Id --silent --accept-source-agreements --accept-package-agreements
    } else {
        Write-Info "$Name is already installed."
    }
}

function Safe-Download {
    param(
        [string]$Url,
        [string]$Destination
    )
    if (Test-Path $Destination) { Remove-Item $Destination -Force }
    Write-Info "Downloading $Url"
    $ProgressPreference = 'SilentlyContinue'
    Invoke-WebRequest -Uri $Url -OutFile $Destination -UseBasicParsing
}

# ============================================================================
# Build Tool Installers
# ============================================================================

function Install-MSVCBuildTools {
    Write-Step "Installing MSVC Build Tools"
    $sdkComponent = if ($OsCaption -match "Windows 11") {
        "Microsoft.VisualStudio.Component.Windows11SDK.22621"
    } else {
        "Microsoft.VisualStudio.Component.Windows10SDK.20348"
    }
    Install-WinGetPackage -Id "Microsoft.VisualStudio.2022.BuildTools" -Name "Visual Studio Build Tools"
    # Ensure C++ workload components are present
    $list = winget list --id "Microsoft.VisualStudio.2022.BuildTools" 2>&1 | Out-String
    if ($list -notmatch "No installed package") {
        Write-Info "Ensuring C++ components are configured..."
        $vsInstaller = "${env:ProgramFiles(x86)}\Microsoft Visual Studio\Installer\vs_installer.exe"
        if (Test-Path $vsInstaller) {
            & $vsInstaller modify --installPath (Get-CimInstance -ClassName "MSFT_VSInstance" -ErrorAction SilentlyContinue |
                Select-Object -First 1 -ExpandProperty InstallLocation) `
                --add "Microsoft.VisualStudio.Component.VC.Tools.x86.x64" `
                --add $sdkComponent `
                --add "Microsoft.VisualStudio.Component.VC.CMake.Project" `
                --quiet --wait 2>$null
        }
    }
}

function Install-Rustup {
    Write-Step "Installing Rust toolchain"
    Install-WinGetPackage -Id "Rustlang.Rustup" -Name "Rustup"
    Reload-Path
    if (Get-Command rustup -ErrorAction SilentlyContinue) {
        Write-Info "Configuring Rust toolchain..."
        rustup update
        rustup component add rustfmt
        rustup component add clippy
        rustup toolchain install nightly
        rustup component add rustfmt --toolchain nightly
    } else {
        Write-Warn "rustup not found in PATH after install. You may need to restart your shell."
    }
}

function Install-CargoPlugins {
    Write-Step "Installing Cargo plugins"
    if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
        Write-Warn "cargo not found -- skipping Cargo plugin installation"
        return
    }
    cargo install protoc-gen-prost --locked 2>$null
    cargo install protoc-gen-prost-serde --locked 2>$null
    cargo install protoc-gen-prost-crate --locked 2>$null
    cargo install grcov --version $GRCOV_VERSION --locked 2>$null
    cargo install cargo-sort --version $CARGO_SORT_VERSION --locked 2>$null
    cargo install cargo-machete --version $CARGO_MACHETE_VERSION --locked 2>$null
    cargo install cargo-nextest --version $CARGO_NEXTEST_VERSION --locked 2>$null
}

function Install-Protoc {
    Write-Step "Installing protoc v$PROTOC_VERSION"
    if (Get-Command protoc -ErrorAction SilentlyContinue) {
        $current = protoc --version 2>$null
        if ($current -match $PROTOC_VERSION) {
            Write-Info "protoc v$PROTOC_VERSION already installed"
            return
        }
    }
    $zip = "protoc-${PROTOC_VERSION}-win${Arch}.zip"
    $url = "https://github.com/protocolbuffers/protobuf/releases/download/v${PROTOC_VERSION}/$zip"
    $dest = Join-Path $env:USERPROFILE "Downloads\$zip"
    $extractDir = Join-Path $env:USERPROFILE "protoc-$PROTOC_VERSION"

    Safe-Download -Url $url -Destination $dest
    Expand-Archive -Path $dest -DestinationPath $extractDir -Force
    Remove-Item $dest -Force

    $binDir = Join-Path $extractDir "bin"
    if ($env:Path -notmatch [regex]::Escape($binDir)) {
        [Environment]::SetEnvironmentVariable("PATH", "$env:PATH;$binDir", "User")
        $env:Path += ";$binDir"
    }
    Write-Info "protoc installed to $binDir"
}

function Install-LLVM {
    Write-Step "Installing LLVM/Clang"
    Install-WinGetPackage -Id "LLVM.LLVM" -Name "LLVM"
}

function Install-OpenSSL {
    Write-Step "Installing OpenSSL"
    Install-WinGetPackage -Id "ShiningLight.OpenSSL" -Name "OpenSSL"
}

function Install-NodeJS {
    Write-Step "Installing Node.js"
    Install-WinGetPackage -Id "OpenJS.NodeJS" -Name "Node.js"
}

function Install-Pnpm {
    Write-Step "Installing pnpm"
    Install-WinGetPackage -Id "pnpm.pnpm" -Name "pnpm"
}

function Install-PostgreSQL {
    Write-Step "Installing PostgreSQL"
    Install-WinGetPackage -Id "PostgreSQL.PostgreSQL.15" -Name "PostgreSQL 15"
    # Add psql to PATH if not already there
    $pgDir = "$env:PROGRAMFILES\PostgreSQL\15\bin"
    if ((Test-Path $pgDir) -and ($env:Path -notmatch [regex]::Escape($pgDir))) {
        [Environment]::SetEnvironmentVariable("PATH", "$env:PATH;$pgDir", "User")
        $env:Path += ";$pgDir"
    }
}

function Install-Python {
    Write-Step "Installing Python"
    Install-WinGetPackage -Id "Python.Python.3.11" -Name "Python 3.11"
    Reload-Path
    # On Windows the Microsoft Store "python" app alias can intercept the
    # command even when the real Python is installed.  Check for the actual
    # executable rather than the alias.
    $realPython = Get-Command python.exe -ErrorAction SilentlyContinue |
        Where-Object { $_.Source -notmatch 'WindowsApps' } |
        Select-Object -First 1
    if ($realPython) {
        try { & $realPython.Source -m pip install --upgrade pip 2>$null }
        catch { Write-Warn "pip upgrade failed (non-fatal): $_" }
    } else {
        Write-Warn "python not found in PATH after install (Microsoft Store alias may be in the way)."
        Write-Warn "Disable the 'python' app execution alias in Settings > Apps > Advanced app settings."
    }
}

# ============================================================================
# Move Prover Tool Installers
# ============================================================================

function Install-DotNet {
    Write-Step "Installing .NET SDK $DOTNET_VERSION"
    $sdkMajor = $DOTNET_VERSION.Split('.')[0]
    Install-WinGetPackage -Id "Microsoft.DotNet.SDK.$sdkMajor" -Name ".NET SDK $sdkMajor"
    Reload-Path
    if ($env:Path -notmatch "dotnet") {
        [Environment]::SetEnvironmentVariable("DOTNET_ROOT", "$env:PROGRAMFILES\dotnet", "User")
        [Environment]::SetEnvironmentVariable("PATH",
            "$env:PATH;$env:PROGRAMFILES\dotnet;$env:USERPROFILE\.dotnet\tools", "User")
        $env:Path += ";$env:PROGRAMFILES\dotnet;$env:USERPROFILE\.dotnet\tools"
    }
}

function Install-Z3 {
    Write-Step "Installing Z3 v$Z3_VERSION"
    if ($env:Z3_EXE -and (Test-Path $env:Z3_EXE)) {
        Write-Info "Z3 already installed at $env:Z3_EXE"
        return
    }
    $zip = "z3-${Z3_VERSION}-x${Arch}-win.zip"
    $url = "https://github.com/Z3Prover/z3/releases/download/z3-${Z3_VERSION}/$zip"
    $dest = Join-Path $env:USERPROFILE "Downloads\$zip"
    $extractDir = Join-Path $env:USERPROFILE "z3-${Z3_VERSION}-x${Arch}-win"

    Safe-Download -Url $url -Destination $dest
    Expand-Archive -Path $dest -DestinationPath $env:USERPROFILE -Force
    Remove-Item $dest -Force

    $z3Exe = Join-Path $extractDir "bin\z3.exe"
    [Environment]::SetEnvironmentVariable("Z3_EXE", $z3Exe, "User")
    $env:Z3_EXE = $z3Exe
    Write-Info "Z3 installed; Z3_EXE=$z3Exe"
}

function Install-Boogie {
    Write-Step "Installing Boogie v$BOOGIE_VERSION"
    if ($env:BOOGIE_EXE -and (Test-Path $env:BOOGIE_EXE)) {
        Write-Info "Boogie already installed at $env:BOOGIE_EXE"
        return
    }
    if (-not (Get-Command dotnet -ErrorAction SilentlyContinue)) {
        Write-Warn "dotnet not found -- cannot install Boogie"
        return
    }
    dotnet tool install --global Boogie --version $BOOGIE_VERSION
    $boogieExe = Join-Path $env:USERPROFILE ".dotnet\tools\boogie.exe"
    [Environment]::SetEnvironmentVariable("BOOGIE_EXE", $boogieExe, "User")
    $env:BOOGIE_EXE = $boogieExe
    Write-Info "Boogie installed; BOOGIE_EXE=$boogieExe"
}

function Install-Git {
    Write-Step "Ensuring Git is installed"
    if (Get-Command git -ErrorAction SilentlyContinue) {
        Write-Info "Git is already installed."
        return
    }
    Install-WinGetPackage -Id "Git.Git" -Name "Git"
}

# ============================================================================
# Install Groups
# ============================================================================

function Install-AllBuildTools {
    Write-Step "========== Installing build tools =========="
    Install-MSVCBuildTools
    Install-LLVM
    Install-OpenSSL
    Install-Python
    Install-Protoc
    Install-Rustup
    Install-CargoPlugins
    Install-NodeJS
    Install-Pnpm
    Install-PostgreSQL
}

function Install-AllProverTools {
    Write-Step "========== Installing Move Prover tools =========="
    Install-Git
    Install-DotNet
    Install-Z3
    Install-Boogie
}

# ============================================================================
# Main
# ============================================================================

Ensure-WinGet
Reload-Path

if ($BuildTools -or $Prover) {
    if ($BuildTools) { Install-AllBuildTools }
    if ($Prover)     { Install-AllProverTools }
} elseif ($Batch) {
    # In batch mode with no flags, default to build tools
    Install-AllBuildTools
} else {
    # Interactive mode
    Write-Host ""
    Write-Host "============================================================"
    Write-Host "  Aptos Core -- Windows Development Environment Setup"
    Write-Host "============================================================"
    Write-Host ""
    Write-Host "Select what to install:"
    Write-Host "  t  Build tools (Rust, MSVC, LLVM, protoc, Node.js, etc.)"
    Write-Host "  y  Move Prover tools (Z3, Boogie, .NET)"
    Write-Host ""
    $selection = Read-Host "Selection (t/y)"
    switch ($selection) {
        't' { Install-AllBuildTools }
        'y' { Install-AllProverTools }
        default {
            Write-Err "Invalid selection '$selection'. Use 't' or 'y'."
            exit 1
        }
    }
}

Reload-Path

Write-Host ""
Write-Host "============================================================" -ForegroundColor Green
Write-Host "  Setup complete!" -ForegroundColor Green
Write-Host "============================================================" -ForegroundColor Green
Write-Host ""
Write-Host "Open a new PowerShell session to pick up PATH changes."
Write-Host "Then you should be able to build: cargo build"
Write-Host ""
