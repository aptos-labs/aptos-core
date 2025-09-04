# Copyright © Velor Foundation
# SPDX-License-Identifier: Apache-2.0

###########################################
# Build and package a release for the CLI #
###########################################

# Note: This must be run from the root of the velor-core repository.

# Set up basic variables.
$NAME="velor-cli"
$CRATE_NAME="velor"
$CARGO_PATH="crates\$CRATE_NAME\Cargo.toml"
$Env:VCPKG_ROOT = 'C:\vcpkg\'

# Get the version of the CLI from its Cargo.toml.
$VERSION = Get-Content $CARGO_PATH | Select-String -Pattern '^\w*version = "(\d*\.\d*.\d*)"' | % {"$($_.matches.groups[1])"}

# Install the developer tools
echo "Installing developer tools"
PowerShell -ExecutionPolicy Bypass -File scripts/windows_dev_setup.ps1

# Note: This is required to bypass openssl isssue on Windows.
echo "Installing OpenSSL"
vcpkg install openssl:x64-windows-static-md --clean-after-build

# Build the CLI.
echo "Building release $VERSION of $NAME for Windows"
cargo build -p $CRATE_NAME --profile cli

# Compress the CLI.
$ZIP_NAME="$NAME-$VERSION-Windows-x86_64.zip"
echo "Compressing CLI to $ZIP_NAME"
Compress-Archive -Path target\cli\$CRATE_NAME.exe -DestinationPath $ZIP_NAME

