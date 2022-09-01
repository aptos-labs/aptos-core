#!/bin/sh

# This script publishes the package to npm.js, first perfoming validity checks.
# This script can be used locally or in CI safely.
# It assumes the package has already been installed, built, and tested.

set -e

# Make sure everything is valid.
. ./check.sh

# Finally, publish the package. We assume it has been built
yarn publish --non-interactive
