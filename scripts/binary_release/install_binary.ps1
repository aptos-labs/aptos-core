# Copyright © Aptos Foundation
# SPDX-License-Identifier: Apache-2.0

########################################################
# Download and install a binary release from GitHub    #
########################################################
#
# Usage:
#   iwr https://raw.githubusercontent.com/aptos-labs/aptos-core/main/scripts/binary_release/install_binary.ps1 | iex
#   Or with parameters:
#   iwr https://raw.githubusercontent.com/aptos-labs/aptos-core/main/scripts/binary_release/install_binary.ps1 -OutFile install.ps1; .\install.ps1 -BinaryName aptos-node -Version 1.2.3
#
# Examples:
#   # Install latest version
#   .\install_binary.ps1 -BinaryName aptos-node
#
#   # Install specific version
#   .\install_binary.ps1 -BinaryName aptos-node -Version 1.2.3
#
#   # Install to custom directory
#   .\install_binary.ps1 -BinaryName aptos-node -BinDir "C:\Tools\bin"

param(
    [Parameter(Mandatory=$true)]
    [string]$BinaryName,

    [Parameter(Mandatory=$false)]
    [string]$Version = "latest",

    [Parameter(Mandatory=$false)]
    [string]$BinDir = "$env:USERPROFILE\.local\bin",

    [Parameter(Mandatory=$false)]
    [switch]$Force,

    [Parameter(Mandatory=$false)]
    [switch]$Yes,

    [Parameter(Mandatory=$false)]
    [string]$Repo = "aptos-labs/aptos-core"
)

$ErrorActionPreference = "Stop"

# ANSI color codes for terminal output
$ESC = [char]27
$RED = "$ESC[31m"
$GREEN = "$ESC[32m"
$YELLOW = "$ESC[33m"
$BLUE = "$ESC[34m"
$RESET = "$ESC[0m"

function Write-ColorOutput {
    param(
        [string]$Message,
        [string]$Color = $RESET
    )
    Write-Host "${Color}${Message}${RESET}"
}

function Write-Error-Message {
    param([string]$Message)
    Write-ColorOutput "Error: $Message" $RED
}

function Write-Success {
    param([string]$Message)
    Write-ColorOutput $Message $GREEN
}

function Write-Info {
    param([string]$Message)
    Write-ColorOutput $Message $BLUE
}

function Write-Warning-Message {
    param([string]$Message)
    Write-ColorOutput $Message $YELLOW
}

# Detect architecture
$ARCH = if ([Environment]::Is64BitOperatingSystem) { "x86_64" } else { "i686" }
$TARGET_TRIPLE = "$ARCH-pc-windows-msvc"

Write-Info "Installing $BinaryName for $TARGET_TRIPLE..."

# Get version if latest
if ($Version -eq "latest") {
    Write-Info "Fetching latest release version..."
    try {
        $releases = Invoke-RestMethod -Uri "https://api.github.com/repos/$Repo/releases"
        $latestRelease = $releases | Where-Object { $_.tag_name -like "$BinaryName-v*" } | Select-Object -First 1

        if ($null -eq $latestRelease) {
            Write-Error-Message "Could not determine latest version for $BinaryName"
            exit 1
        }

        $Version = $latestRelease.tag_name -replace "$BinaryName-v", ""
        Write-Info "Latest version: $Version"
    } catch {
        Write-Error-Message "Failed to fetch releases: $_"
        exit 1
    }
}

# Check if already installed
$InstalledPath = Join-Path $BinDir "$BinaryName.exe"
if ((Test-Path $InstalledPath) -and -not $Force) {
    try {
        $CurrentVersion = & $InstalledPath --version 2>$null | Select-String -Pattern '\d+\.\d+\.\d+' | ForEach-Object { $_.Matches.Value } | Select-Object -First 1

        if ($CurrentVersion -eq $Version) {
            Write-Success "$BinaryName $Version is already installed"
            exit 0
        } else {
            Write-Warning-Message "$BinaryName is already installed (version: $CurrentVersion)"
            if (-not $Yes) {
                $response = Read-Host "Do you want to upgrade to version $Version? [y/N]"
                if ($response -notmatch '^[Yy]') {
                    exit 0
                }
            }
        }
    } catch {
        Write-Warning-Message "Could not determine current version"
    }
}

# Construct download URLs
$ReleaseTag = "$BinaryName-v$Version"
$ArchiveName = "$BinaryName-v$Version-$TARGET_TRIPLE.zip"
$DownloadUrl = "https://github.com/$Repo/releases/download/$ReleaseTag/$ArchiveName"
$ChecksumUrl = "https://github.com/$Repo/releases/download/$ReleaseTag/$ArchiveName.sha256"

Write-Info "Downloading from: $DownloadUrl"

# Create temporary directory
$TmpDir = Join-Path $env:TEMP "binary-install-$(New-Guid)"
New-Item -ItemType Directory -Path $TmpDir | Out-Null

try {
    # Download archive
    $ArchivePath = Join-Path $TmpDir $ArchiveName
    try {
        Invoke-WebRequest -Uri $DownloadUrl -OutFile $ArchivePath -UseBasicParsing
    } catch {
        Write-Error-Message "Failed to download $ArchiveName"
        Write-Error-Message "URL: $DownloadUrl"
        Write-Info "Available releases: https://github.com/$Repo/releases"
        exit 1
    }

    # Download and verify checksum if available
    $ChecksumPath = Join-Path $TmpDir "$ArchiveName.sha256"
    try {
        Invoke-WebRequest -Uri $ChecksumUrl -OutFile $ChecksumPath -UseBasicParsing
        Write-Info "Verifying checksum..."

        $expectedHash = (Get-Content $ChecksumPath -Raw).Split()[0].Trim()
        $actualHash = (Get-FileHash -Path $ArchivePath -Algorithm SHA256).Hash.ToLower()

        if ($expectedHash -ne $actualHash) {
            Write-Error-Message "Checksum verification failed"
            Write-Error-Message "Expected: $expectedHash"
            Write-Error-Message "Got: $actualHash"
            exit 1
        }

        Write-Success "Checksum verified"
    } catch {
        Write-Warning-Message "Checksum not available, skipping verification"
    }

    # Extract archive
    Write-Info "Extracting archive..."
    Expand-Archive -Path $ArchivePath -DestinationPath $TmpDir -Force

    # Create bin directory if it doesn't exist
    if (-not (Test-Path $BinDir)) {
        New-Item -ItemType Directory -Path $BinDir -Force | Out-Null
    }

    # Install binary
    $BinaryPath = Join-Path $TmpDir "$BinaryName.exe"
    Write-Info "Installing to $InstalledPath..."
    Copy-Item -Path $BinaryPath -Destination $InstalledPath -Force

    # Verify installation
    if (Test-Path $InstalledPath) {
        Write-Success "Successfully installed $BinaryName $Version"

        # Check if bin directory is in PATH
        $currentPath = [Environment]::GetEnvironmentVariable("Path", "User")
        if ($currentPath -notlike "*$BinDir*") {
            Write-Warning-Message "$BinDir is not in your PATH"
            $response = if ($Yes) { "y" } else { Read-Host "Do you want to add it to your PATH? [y/N]" }

            if ($response -match '^[Yy]') {
                $newPath = "$BinDir;$currentPath"
                [Environment]::SetEnvironmentVariable("Path", $newPath, "User")
                $env:Path = "$BinDir;$env:Path"
                Write-Success "Added $BinDir to PATH"
                Write-Info "Please restart your terminal for PATH changes to take effect"
            } else {
                Write-Info "You can manually add it to your PATH later"
            }
        } else {
            Write-Info "Run '$BinaryName --version' to verify the installation"
        }
    } else {
        Write-Error-Message "Installation failed"
        exit 1
    }

} finally {
    # Clean up
    if (Test-Path $TmpDir) {
        Remove-Item -Path $TmpDir -Recurse -Force -ErrorAction SilentlyContinue
    }
}
