#!/bin/sh
# Copyright © Aptos Foundation
# Parts of the project are originally copyright © Meta Platforms, Inc.
# SPDX-License-Identifier: Apache-2.0

set -e

has_command() {
  command -v "$1" > /dev/null 2>&1
}

# Install native packages for the given package manager (self-contained; no external install scripts).
minimal_install_pkgs() {
  pm="$1"
  shift
  if [ "$#" -eq 0 ]; then
    return 0
  fi

  pre_cmd=""
  if [ "$(id -u)" -ne 0 ]; then
    pre_cmd="sudo"
  fi

  echo "Installing packages with $pm: $*"
  case "$pm" in
    apt-get)
      $pre_cmd apt-get install -y --no-install-recommends "$@"
      ;;
    dnf)
      $pre_cmd dnf install -y "$@"
      ;;
    yum)
      $pre_cmd yum install -y "$@"
      ;;
    pacman)
      $pre_cmd pacman -Syu --noconfirm "$@"
      ;;
    apk)
      $pre_cmd apk --update add --no-cache "$@"
      ;;
    zypper)
      $pre_cmd zypper install -y "$@"
      ;;
    emerge)
      $pre_cmd emerge "$@"
      ;;
    brew)
      brew install "$@"
      ;;
    port)
      port install "$@"
      ;;
    xbps)
      $pre_cmd xbps-install -y "$@"
      ;;
    pkg)
      $pre_cmd pkg install -y "$@"
      ;;
    *)
      echo "Unsupported package manager: $pm" 1>&2
      exit 1
      ;;
  esac
}

# This script is used to set up a minimal environment for building the Aptos CLI and other tools.
# The `dev_setup.sh` script is way too complex, and too hard to figure out what is actually happening.  This script
# simplifies the process
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
      minimal_install_pkgs apt-get build-essential pkgconf libssl-dev git libudev-dev lld libdw-dev clang llvm cmake
    elif has_command dnf; then
      # RHEL / CentOS based DNF
      # This depends on the OS!
      # Source the os-release file to parse it
      . /etc/os-release

      # Check if it's Rocky
      if [ "$ID" = "rocky" ]; then
        echo "Rocky Linux detected"
        minimal_install_pkgs dnf gcc gcc-c++ make pkgconf openssl-devel git systemd-devel lld elfutils-devel clang llvm cmake
      else
        minimal_install_pkgs dnf gcc gcc-c++ make pkgconf openssl-devel git rust-libudev-devel lld elfutils-devel clang llvm cmake
      fi
    elif has_command yum; then
      # RHEL / CentOS based YUM
      minimal_install_pkgs yum gcc gcc-c++ make pkgconf openssl-devel git rust-libudev-devel lld elfutils-devel clang llvm cmake
    elif has_command pacman; then
      # Arch based PACMAN
      minimal_install_pkgs pacman base-devel pkgconf openssl git lld clang llvm cmake
    elif has_command apk; then
      # Alpine based APK
      minimal_install_pkgs apk alpine-sdk coreutils pkgconfig openssl-dev git lld elfutils-dev clang llvm cmake libc-dev
    elif has_command zypper; then
      # OpenSUSE zypper
      minimal_install_pkgs zypper gcc gcc-c++ make pkg-config libopenssl-devel git libudev-devel lld libdw-devel clang llvm cmake
    elif has_command emerge; then
      # Gentoo Emerge
      sudo emerge --sync
      minimal_install_pkgs emerge sys-devel/gcc dev-libs/openssl dev-vcs/git dev-lang/rust llvm-core/clang
    elif has_command xbps-install; then
      # Void linux xbps
      minimal_install_pkgs xbps gcc make pkg-config git lld elfutils-devel clang llvm cmake
    else
      echo "Unable to find supported Linux package manager (apt-get, dnf, yum, zypper, xbps or pacman). Abort"
      exit 1
    fi
  ;;
  Darwin)
    # macOS (Homebrew or MacPorts)
    if has_command brew; then
      minimal_install_pkgs brew pkg-config openssl git llvm cmake
    elif has_command port; then
      minimal_install_pkgs port pkgconfig openssl git llvm cmake
    else
      echo "Missing package manager Homebrew (https://brew.sh/) or MacPorts (https://www.macports.org/). Abort." 1>&2
      exit 1
    fi
  ;;
  FreeBSD)
    # FreeBSD
    minimal_install_pkgs pkg gcc gmake binutils pkgconf git openssl cmake llvm hidapi
  ;;
  *)
    echo "Unknown OS. Abort."
    exit 1
  ;;
esac
