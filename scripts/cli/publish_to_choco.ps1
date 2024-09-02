# Copyright © Aptos Foundation
# SPDX-License-Identifier: Apache-2.0

###########################################
# Publish Aptos Cli to Chocolatey.org #
###########################################

# Note: This must be run from the root of the aptos-core repository.

$NAME="aptos-cli"
$CRATE_NAME="aptos"
$CARGO_PATH="crates\$CRATE_NAME\Cargo.toml"
$VERSION = Get-Content $CARGO_PATH | Select-String -Pattern '^\w*version = "(\d*\.\d*.\d*)"' | % {"$($_.matches.groups[1])"}
$ExePath = "target\cli\$CRATE_NAME.exe"
$apiKey = $env:CHOCO_API_KEY
$ZIP_NAME="$NAME-$VERSION-Windows-x86_64.zip"

choco install checksum -y

$aptosHash = & checksum -t sha256 $ExePath

Set-Location -Path "chocolatey"

@"
Aptos Binary verification steps
1. Download https://github.com/aptos-labs/aptos-core/releases/download/aptos-cli-$VERSION/$ZIP_NAME
2. Extract aptos.exe
3. Verify binary: checksum.exe -t sha256 aptos.exe: $aptosHash

File 'LICENSE.txt' is obtained from: https://github.com/aptos-labs/aptos-core/blob/main/LICENSE
"@ | Out-File -FilePath "VERIFICATION.txt" -Encoding utf8 -Append

choco pack --version $VERSION configuration=release

choco apikey --api-key $apiKey --source https://push.chocolatey.org/

choco push aptos.$VERSION.nupkg --source https://push.chocolatey.org/

Set-Location -Path ".."
