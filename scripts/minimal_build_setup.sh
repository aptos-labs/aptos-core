#!/bin/bash
# Copyright © Aptos Foundation
# Parts of the project are originally copyright © Meta Platforms, Inc.
# SPDX-License-Identifier: Apache-2.0


# This script is used to set up a minimal environment for building the Aptos CLI and other tools.
# The `dev_setup.sh` script is way too complex, and too hard to figure out what is actually happening.  This script
# simplifies the process
curl -O https://raw.githubusercontent.com/gregnazario/universal-installer/refs/heads/main/scripts/install_pkg.sh

# TODO: Do we need to add `ca-certificates`, `curl`, `unzip`, `wget`

OS="$(uname)"
case "$OS" in
  Linux)
    if command -v apt-get &>/dev/null; then
      # Ubuntu / Debian based APT-GET
      sh install_pkg.sh build-essential pkg-config libssl-dev git rustup libudev-dev lld libdw-dev clang llvm cmake
    elif command -v apt &>/dev/null; then
      # Ubuntu / Debian based APT
      sh install_pkg.sh build-essential pkg-config libssl-dev git rustup libudev-dev lld libdw-dev clang llvm cmake
    elif command -v dnf &>/dev/null; then
      # RHEL / CentOS based DNF
      sh install_pkg.sh gcc gcc-c++ make pkg-config openssl-devel git rustup libudev-dev lld libdw-dev clang llvm cmake
    elif command -v yum &>/dev/null; then
      # RHEL / CentOS based YUM
      sh install_pkg.sh gcc make pkgconfig openssl-devel git rustup rust-libudev-devel lld libdwarf-devel clang llvm cmake
    elif command -v pacman &>/dev/null; then
      # Arch based PACMAN
      sh install_pkg.sh base-devel pkgconf openssl git rustup lld clang llvm cmake
    elif command -v apk &>/dev/null; then
      # Alpine based APK
      sh install_pkg.sh alpine-sdk coreutils pkgconfig openssl-dev git rustup libudev-dev lld libdw-dev clang llvm cmake
    else
      # TODO: Support more package managers?
      echo "Unable to find supported package manager (yum, apt-get, dnf, or pacman). Abort"
      exit 1
    fi
  ;;
  Darwin)
    # TODO: May need to do a different path for macports, but atm brew is expected here
    sh install_pkg.sh build-essential pkgconfig openssl git rustup clang llvm cmake
  ;;
  *)
    echo "Unknown OS. Abort."
    exit 1
  ;;
esac