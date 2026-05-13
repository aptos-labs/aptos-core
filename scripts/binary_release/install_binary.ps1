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
if (-not [Environment]::Is64BitOperatingSystem) {
    Write-Error-Message "32-bit Windows is not supported. Prebuilt binaries are only available for x86_64-pc-windows-msvc."
    exit 1
}

$ARCH = "x86_64"
$TARGET_TRIPLE = "$ARCH-pc-windows-msvc"

Write-Info "Installing $BinaryName for $TARGET_TRIPLE..."

# Get version if latest
if ($Version -eq "latest") {
    Write-Info "Fetching latest release version..."
    try {
        # Fetch with pagination support (100 per page, max 3 pages = 300 releases)
        $latestRelease = $null
        for ($page = 1; $page -le 3; $page++) {
            $releases = Invoke-RestMethod -Uri "https://api.github.com/repos/$Repo/releases?per_page=100&page=$page"
            $latestRelease = $releases | Where-Object { $_.tag_name -like "$BinaryName-v*" } | Select-Object -First 1

            if ($null -ne $latestRelease) {
                break
            }

            # Check if there are more pages
            if ($releases.Count -eq 0) {
                break
            }
        }

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
        $archiveRequestParams = @{
            Uri     = $DownloadUrl
            OutFile = $ArchivePath
        }
        if ((Get-Command Invoke-WebRequest).Parameters.ContainsKey('UseBasicParsing')) {
            $archiveRequestParams['UseBasicParsing'] = $true
        }
        Invoke-WebRequest @archiveRequestParams
    } catch {
        Write-Error-Message "Failed to download $ArchiveName"
        Write-Error-Message "URL: $DownloadUrl"
        Write-Info "Available releases: https://github.com/$Repo/releases"
        exit 1
    }

    # Download and verify checksum if available
    $ChecksumPath = Join-Path $TmpDir "$ArchiveName.sha256"
    try {
        $checksumRequestParams = @{
            Uri     = $ChecksumUrl
            OutFile = $ChecksumPath
        }
        if ((Get-Command Invoke-WebRequest).Parameters.ContainsKey('UseBasicParsing')) {
            $checksumRequestParams['UseBasicParsing'] = $true
        }
        Invoke-WebRequest @checksumRequestParams
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
    # Search recursively for the binary in case the archive contains a directory prefix
    $binaryMatches = Get-ChildItem -Path $TmpDir -Filter "$BinaryName.exe" -Recurse -File -ErrorAction SilentlyContinue

    if (-not $binaryMatches -or $binaryMatches.Count -eq 0) {
        Write-Error-Message "Failed to find $BinaryName.exe in extracted archive at $TmpDir"
        exit 1
    }

    if ($binaryMatches.Count -gt 1) {
        Write-Error-Message "Multiple $BinaryName.exe files found in extracted archive:"
        $binaryMatches | ForEach-Object { Write-Error-Message "  $($_.FullName)" }
        Write-Error-Message "Please ensure the archive contains a single $BinaryName.exe"
        exit 1
    }

    $BinaryPath = $binaryMatches[0].FullName
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
