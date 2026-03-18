# Copyright © Aptos Foundation
# SPDX-License-Identifier: Apache-2.0

#########################################################
# Build and package a binary release for any executable #
#########################################################
# Example:
# .\build_binary_release.ps1 -BinaryName "my-tool" -CrateName "my-crate" -BuildProfile "tool" -Version "1.0.0"
#

# Note: This must be run from the root of the aptos-core repository.

param(
    [Parameter(Mandatory=$true)]
    [string]$BinaryName,

    [Parameter(Mandatory=$true)]
    [string]$CrateName,

    [Parameter(Mandatory=$true)]
    [ValidateSet("tool", "performance")]
    [string]$BuildProfile,

    [Parameter(Mandatory=$true)]
    [string]$Version,

    [Parameter(Mandatory=$false)]
    [bool]$SkipChecks = $false
)

$TARGET_TRIPLE = "x86_64-pc-windows-msvc"
$Env:VCPKG_ROOT = 'C:\vcpkg\'

# Determine the cargo path - try common locations
$CARGO_PATH = $null
$possiblePaths = @(
    "crates\$CrateName\Cargo.toml",
    "$CrateName\Cargo.toml",
    "aptos-move\$CrateName\Cargo.toml"
)

foreach ($path in $possiblePaths) {
    if (Test-Path $path) {
        $CARGO_PATH = $path
        break
    }
}

if ($null -eq $CARGO_PATH) {
    Write-Error "Could not find Cargo.toml for crate $CrateName"
    exit 1
}

# Get the version from Cargo.toml
$DETECTED_VERSION = Get-Content $CARGO_PATH | Select-String -Pattern '^\w*version = "(\d+\.\d+\.\d+)"' | ForEach-Object {"$($_.Matches.Groups[1])"}

if (-not $SkipChecks) {
    # Check that the version matches
    if ($Version -ne $DETECTED_VERSION) {
        Write-Error "Wanted to release for $Version, but Cargo.toml says the version is $DETECTED_VERSION"
        exit 2
    }
} else {
    Write-Warning "Skipping version checks!"
}

# Install the developer tools
Write-Host "Installing developer tools"
PowerShell -ExecutionPolicy Bypass -File scripts/windows_dev_setup.ps1

# Note: This is required to bypass openssl issue on Windows.
Write-Host "Installing OpenSSL"
vcpkg install openssl:x64-windows-static-md --clean-after-build

# Build the binary
Write-Host "Building release $Version of $BinaryName for $TARGET_TRIPLE using profile '$BuildProfile'"
cargo build -p $CrateName --profile $BuildProfile

# Determine the output directory based on profile
$BUILD_DIR = "target\$BuildProfile"

# Check if the binary exists
$binaryPath = "$BUILD_DIR\$BinaryName.exe"
$cratePath = "$BUILD_DIR\$CrateName.exe"

if (Test-Path $binaryPath) {
    $finalBinaryPath = $binaryPath
} elseif (Test-Path $cratePath) {
    # If binary name differs from crate name, copy it
    Copy-Item $cratePath $binaryPath
    $finalBinaryPath = $binaryPath
} else {
    Write-Error "Could not find binary $BinaryName.exe or $CrateName.exe in $BUILD_DIR"
    exit 1
}

# Compress the binary with 'v' prefix in version
$ZIP_NAME = "$BinaryName-v$Version-$TARGET_TRIPLE.zip"
Write-Host "Compressing binary to $ZIP_NAME"
Compress-Archive -Path $finalBinaryPath -DestinationPath $ZIP_NAME -Force

# Generate SHA256 checksum
Write-Host "Generating SHA256 checksum"
$hash = Get-FileHash -Path $ZIP_NAME -Algorithm SHA256
"$($hash.Hash.ToLower())  $ZIP_NAME" | Out-File -FilePath "$ZIP_NAME.sha256" -Encoding ASCII

# Clean up temporary copy if we made one
if ((Test-Path $binaryPath) -and (Test-Path $cratePath) -and ($binaryPath -ne $cratePath)) {
    Remove-Item $binaryPath
}
