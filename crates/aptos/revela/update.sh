#!/bin/sh

cd "$(dirname "$0")"

# Ensure the user has shasum installed.
if ! which shasum > /dev/null; then
  echo "ERROR: Please install shasum"
  exit 1
fi

# Ensure the user has curl installed.
if ! which curl > /dev/null; then
  echo "ERROR: Please install curl"
  exit 1
fi

# Ensure the user has given a tag.
if [ -z "$1" ]; then
  echo "ERROR: Please provide a tag"
  exit 1
fi

TAG="$1"

set -e

# Download the tar.gz file.
curl -L --fail-with-body "https://github.com/verichains/revela/archive/refs/tags/$TAG.tar.gz" -o /tmp/source.tar.gz

# Get its sha256 hash.
SHA256=$(shasum -a 256 /tmp/source.tar.gz | awk '{print $1}')

# Update version.txt
echo $TAG > version.txt
echo $SHA256 >> version.txt

echo "Done!"
