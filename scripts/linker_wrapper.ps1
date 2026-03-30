# Copyright (c) Aptos Foundation
# SPDX-License-Identifier: Apache-2.0

$ErrorActionPreference = "Stop"

function Resolve-LinkerFlavor {
    if ($env:APTOS_LINKER) {
        return $env:APTOS_LINKER.ToLowerInvariant()
    }

    if (Get-Command mold -ErrorAction SilentlyContinue) {
        return "mold"
    }

    if (Get-Command lld -ErrorAction SilentlyContinue -or Get-Command ld.lld -ErrorAction SilentlyContinue) {
        return "lld"
    }

    return "system"
}

$linkerFlavor = Resolve-LinkerFlavor
$clangArgs = @()

switch ($linkerFlavor) {
    "lld" {
        $clangArgs += "-fuse-ld=lld"
    }
    "mold" {
        $clangArgs += "-fuse-ld=mold"
    }
    "system" {
        # default linker
    }
    default {
        Write-Error "Unsupported APTOS_LINKER='$linkerFlavor'. Use one of: mold, lld, system."
        exit 2
    }
}

$clangArgs += $args

& clang @clangArgs
exit $LASTEXITCODE
