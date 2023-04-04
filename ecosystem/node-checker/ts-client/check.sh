#!/bin/sh

# This script performs various validity checks prior to publishing a package
# to npm.js, such as checking the version and the changelog.

set -e

# Get the latest version of the package on npm.js
PUBLISHED_VERSION=`npm show aptos-node-checker-client version`

# Get the version from the local package.json file.
NEW_VERSION=`node -p -e "require('./package.json').version"`

# Exit happily if the version is the same.
if [ "$NEW_VERSION" = "$PUBLISHED_VERSION" ]; then
    echo "Version is the same. Exiting gracefully."
    exit 0
fi

# Functions to help check if the version went backwards.
verlte() {
    [ "$1" = "$(printf "$1\n$2" | sort -V | head -n1)" ]
}

# Ensure the version didn't go backwards.
if verlte $NEW_VERSION $PUBLISHED_VERSION; then
    echo "ERROR: The version number went backwards. Aborting."
    exit 1
fi

# Ensure there is an entry for the new version in the changelog.
if ! grep -q "# $NEW_VERSION" CHANGELOG.md; then
    echo "ERROR: The changelog does not contain an entry for the new version. Aborting."
    exit 1
fi

echo "Version and changelog look good"
