#!/bin/bash
# Copyright © Aptos Foundation
# Parts of the project are originally copyright © Meta Platforms, Inc.
# SPDX-License-Identifier: Apache-2.0


# This script is used to set up a minimal environment for building the Aptos CLI and other tools.
# The `dev_setup.sh` script is way too complex, and too hard to figure out what is actually happening.  This script
# simplifies the process
if command -v wget &>/dev/null; then
  wget https://raw.githubusercontent.com/gregnazario/universal-installer/refs/heads/main/scripts/install_pkg.sh
elif command -v curl &>/dev/null; then
  curl -O https://raw.githubusercontent.com/gregnazario/universal-installer/refs/heads/main/scripts/install_pkg.sh
else
  echo "Unable to download install script, no wget or curl. Abort"
  exit 1
fi

# TODO: Do we need to add `ca-certificates`, `curl`, `unzip`, `wget`
# Install rustup
if ! command -v rustup&>/dev/null; then
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
fi

OS="$(uname)"
case "$OS" in
  Linux)
    if command -v apt-get &>/dev/null; then
      # Ubuntu / Debian based APT-GET
      sudo apt-get update
      sh install_pkg.sh build-essential pkgconf libssl-dev git libudev-dev lld libdw-dev clang llvm cmake
    elif command -v dnf &>/dev/null; then
      # RHEL / CentOS based DNF
      sh install_pkg.sh gcc gcc-c++ make pkgconf openssl-devel git rust-libudev-devel lld elfutils-devel clang llvm cmake
    elif command -v yum &>/dev/null; then
      # RHEL / CentOS based YUM
      sh install_pkg.sh gcc gcc-c++ make pkgconf openssl-devel git rust-libudev-devel lld elfutils-devel clang llvm cmake
    elif command -v pacman &>/dev/null; then
      # Arch based PACMAN
      sh install_pkg.sh base-devel pkgconf openssl git lld clang llvm cmake
    elif command -v apk &>/dev/null; then
      # Alpine based APK
      sh install_pkg.sh alpine-sdk coreutils pkgconfig openssl-dev git lld elfutils-dev clang llvm cmake libc-dev
    elif command -v zypper &>/dev/null; then
      # OpenSUSE zypper
      sh install_pkg.sh gcc gcc-c++ make pkg-config libopenssl-devel git libudev-devel lld libdw-devel clang llvm cmake
    #elif command -v emerge &>/dev/null; then
      # Gentoo Emerge
      # TODO: This doesn't quite work correctly yet
    #  sudo emerge --sync
    #  sh install_pkg.sh --skip-overrides sys-devel/gcc dev-libs/openssl dev-vcs/git dev-lang/rust
    elif command -v xbps-install &>/dev/null; then
      # Void linux xbps
      sh install_pkg.sh gcc make pkg-config git lld elfutils-devel clang llvm cmake
    else
      # TODO: Support more package managers?
      echo "Unable to find supported package manager (apt-get, dnf, yum, zypper, xbps or pacman). Abort"
      exit 1
    fi
  ;;
  Darwin)
    # TODO: May need to do a different path for macports, but atm brew is expected here
    sh install_pkg.sh pkgconfig openssl git llvm cmake
  ;;
  # TODO: Does not work yet
  #FreeBSD)
  #  sh install_pkg.sh gcc gmake binutils pkgconf git openssl cmake llvm
  #;;
  *)
    echo "Unknown OS. Abort."
    exit 1
  ;;
esac

# TODO: Determine how best to install on it's own
#git clone https://github.com/aptos-labs/aptos-core.git
#cd aptos-core || exit 1

# If cargo is installed correctly use it, but otherwise, you may need to initialize rustup, depends on OS
#if command -v cargo &>/dev/null; then
#  cargo build -p aptos --profile cli
#else
  # if you use rustup init, you may need to use the .cargo/bin cargo
#  /usr/bin/rustup-init -y
#  ~/.cargo/bin/cargo build -p aptos --profile cli
#fi