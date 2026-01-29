# Copyright © Aptos Foundation
# SPDX-License-Identifier: Apache-2.0

###########################################
# Build and package a release for the CLI #
###########################################

# Note: This must be run from the root of the aptos-core repository.

# Set up basic variables.
$NAME="aptos-cli"
$CRATE_NAME="aptos"
$CARGO_PATH="crates\$CRATE_NAME\Cargo.toml"
$Env:VCPKG_ROOT = 'C:\vcpkg\'

# Detect processor architecture
$PROC_ARCH = [System.Environment]::GetEnvironmentVariable("PROCESSOR_ARCHITECTURE")
if ($PROC_ARCH -eq "ARM64") {
    $ARCH_NAME = "ARM64"
    $VCPKG_TRIPLET = "arm64-windows-static-md"
} else {
    $ARCH_NAME = "x86_64"
    $VCPKG_TRIPLET = "x64-windows-static-md"
}

echo "Detected architecture: $ARCH_NAME (triplet: $VCPKG_TRIPLET)"

# Get the version of the CLI from its Cargo.toml.
$VERSION = Get-Content $CARGO_PATH | Select-String -Pattern '^\w*version = "(\d*\.\d*.\d*)"' | % {"$($_.matches.groups[1])"}

# Install the developer tools
echo "Installing developer tools"
PowerShell -ExecutionPolicy Bypass -File scripts/windows_dev_setup.ps1

# Note: This is required to bypass openssl isssue on Windows.
echo "Installing OpenSSL for $ARCH_NAME"
vcpkg install openssl:$VCPKG_TRIPLET --clean-after-build

# Build the CLI.
echo "Building release $VERSION of $NAME for Windows $ARCH_NAME"
cargo build -p $CRATE_NAME --profile cli

# Compress the CLI.
$ZIP_NAME="$NAME-$VERSION-Windows-$ARCH_NAME.zip"
echo "Compressing CLI to $ZIP_NAME"
Compress-Archive -Path target\cli\$CRATE_NAME.exe -DestinationPath $ZIP_NAME

