#!/bin/sh
# Copyright © Aptos Foundation
# Parts of the project are originally copyright © Meta Platforms, Inc.
# SPDX-License-Identifier: Apache-2.0

has_command() {
  command -v "$1" > /dev/null 2>&1
}

# This script is used to set up a minimal environment for building the Aptos CLI and other tools.
# The `dev_setup.sh` script is way too complex, and too hard to figure out what is actually happening.  This script
# simplifies the process
if has_command wget; then
  wget https://raw.githubusercontent.com/gregnazario/universal-installer/refs/heads/main/scripts/install_pkg.sh
elif has_command curl; then
  curl -O https://raw.githubusercontent.com/gregnazario/universal-installer/refs/heads/main/scripts/install_pkg.sh
else
  echo "Unable to download install script, no wget or curl. Abort"
  exit 1
fi

# TODO: Do we need to add `ca-certificates`, `curl`, `unzip`, `wget`
# Install rustup
if ! has_command cargo; then
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
fi

OS="$(uname)"
case "$OS" in
  Linux)
    if has_command apt-get; then
      # Ubuntu / Debian based APT-GET
      sudo apt-get update
      sh install_pkg.sh build-essential pkgconf libssl-dev git libudev-dev lld libdw-dev clang llvm cmake
    elif has_command dnf; then
      # RHEL / CentOS based DNF
      sh install_pkg.sh gcc gcc-c++ make pkgconf openssl-devel git rust-libudev-devel lld elfutils-devel clang llvm cmake
    elif has_command yum; then
      # RHEL / CentOS based YUM
      sh install_pkg.sh gcc gcc-c++ make pkgconf openssl-devel git rust-libudev-devel lld elfutils-devel clang llvm cmake
    elif has_command pacman; then
      # Arch based PACMAN
      sh install_pkg.sh base-devel pkgconf openssl git lld clang llvm cmake
    elif has_command apk; then
      # Alpine based APK
      sh install_pkg.sh alpine-sdk coreutils pkgconfig openssl-dev git lld elfutils-dev clang llvm cmake libc-dev
    elif has_command zypper; then
      # OpenSUSE zypper
      sh install_pkg.sh gcc gcc-c++ make pkg-config libopenssl-devel git libudev-devel lld libdw-devel clang llvm cmake
    #elif has_command emerge; then
      # Gentoo Emerge
      # TODO: This doesn't quite work correctly yet
    #  sudo emerge --sync
    #  sh install_pkg.sh --skip-overrides sys-devel/gcc dev-libs/openssl dev-vcs/git dev-lang/rust
    elif has_command xbps-install; then
      # Void linux xbps
      sh install_pkg.sh gcc make pkg-config git lld elfutils-devel clang llvm cmake
    else
      echo "Unable to find supported Linux package manager (apt-get, dnf, yum, zypper, xbps or pacman). Abort"
      exit 1
    fi
  ;;
  Darwin)
    sh install_pkg.sh pkgconfig openssl git llvm cmake
  ;;
  FreeBSD)
    sh install_pkg.sh gcc gmake binutils pkgconf git openssl cmake llvm hidapi
  ;;
  *)
    echo "Unknown OS. Abort."
    exit 1
  ;;
esac
