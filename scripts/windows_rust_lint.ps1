# Mirror of rust_lint.sh just for windows

# Check if we're in the root of the aptos-core repo
if (-not (Test-Path ".github")) {
  Write-Host "Please run this from the root of aptos-core!"
  exit 1
}

# Set CHECK_ARG if the first argument is --check
$CHECK_ARG = ""
if ($args.Count -ge 1 -and $args[0] -eq "--check") {
  $CHECK_ARG = "--check"
}

# Enable verbose and error handling (like set -e -x in bash)
$ErrorActionPreference = "Stop"
$VerbosePreference = "Continue"

# Run clippy with the aptos-core specific configuration
Write-Host "Running cargo xclippy..."
cargo xclippy

# Run the formatter (nightly required for stricter formatting)
Write-Host "Running cargo +nightly fmt $CHECK_ARG..."
cargo +nightly fmt $CHECK_ARG

# Run cargo-sort with grouping and workspace support
Write-Host "Running cargo sort --grouped --workspace $CHECK_ARG..."
cargo sort --grouped --workspace $CHECK_ARG

# Check for unused dependencies with cargo machete
Write-Host "Running cargo machete..."
cargo machete