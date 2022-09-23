#! /bin/bash

# Create extension release or exit if command fails
set -eu

printf "\n📦 Creating extension release zip...\n"

echo "${PWD##*/}"

cd apps/extension

zip -r release.zip ./build || { printf "\n⛔ Unable to create %s extension release.\n"; }

printf "\n✔ Successfully created %s extension release.\n"
