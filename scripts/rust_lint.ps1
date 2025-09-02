# This assumes you have already installed cargo-sort and cargo machete:
# cargo install cargo-sort
# cargo install cargo-machete
#
# The best way to do this however is to run scripts/dev_setup.sh
#
# If you want to run this from anywhere in aptos-core, try adding the wrapper
# script to your path:
# https://gist.github.com/banool/e6a2b85e2fff067d3a215cbfaf808032

# Make sure we're in the root of the repo.
if (-not (Test-Path ".github")) {
    Write-Error "Please run this from the root of aptos-core!"
    exit 1
}

# Run in check mode if requested.
$CHECK_ARG = ""
if ($args[0] -eq "--check") {
    $CHECK_ARG = "--check"
}

# Set appropriate script flags.
$ErrorActionPreference = "Stop"
$VerbosePreference = "Continue"

# Run clippy with the aptos-core specific configuration.
cargo xclippy

# Run the formatter. Note: we require the nightly
# build of cargo fmt to provide stricter rust formatting.
cargo +nightly fmt $CHECK_ARG

# Once cargo-sort correctly handles workspace dependencies,
# we can move to cleaner workspace dependency notation.
# See: https://github.com/DevinR528/cargo-sort/issues/47
cargo install cargo-sort --locked --version 1.0.7
cargo sort --grouped --workspace $CHECK_ARG

# Check for unused rust dependencies.;
cargo install cargo-machete --locked --version 0.7.0
cargo machete
