#!/bin/sh
# Copyright © Aptos Foundation
# Parts of the project are originally copyright © Meta Platforms, Inc.
# SPDX-License-Identifier: Apache-2.0

# This script is used to set up a minimal environment for building the Aptos CLI and other tools.
# The `dev_setup.sh` script is way too complex, and too hard to figure out what is actually happening. This script
# simplifies the process.

set -e

has_command() {
  command -v "$1" > /dev/null 2>&1
}

die() {
  echo "Error: $1" >&2
  exit 1
}

as_root() {
  if [ "$(id -u)" -eq 0 ]; then
    "$@"
  elif has_command sudo; then
    sudo "$@"
  else
    die "Root privileges or sudo are required to run: $*"
  fi
}

is_package_installed() {
  pm="$1"
  package="$2"

  case "$pm" in
    apt-get)
      dpkg-query -W -f='${Status}\n' "$package" 2> /dev/null | grep -q '^install ok installed$'
      ;;
    dnf | yum | zypper)
      rpm -q "$package" > /dev/null 2>&1
      ;;
    pacman)
      pacman -Q "$package" > /dev/null 2>&1
      ;;
    apk)
      apk info -e "$package" > /dev/null 2>&1
      ;;
    emerge)
      set -- "/var/db/pkg/$package"-[0-9]*
      [ -d "$1" ]
      ;;
    xbps)
      xbps-query "$package" > /dev/null 2>&1
      ;;
    brew)
      brew list "$package" > /dev/null 2>&1
      ;;
    port)
      port installed "$package" 2> /dev/null | grep -q ' (active)'
      ;;
    pkg)
      pkg info -e "$package" > /dev/null 2>&1
      ;;
    *)
      return 1
      ;;
  esac
}

install_pkgs() {
  pm="$1"
  shift

  count=$#
  i=0
  while [ "$i" -lt "$count" ]; do
    pkg="$1"
    shift
    if is_package_installed "$pm" "$pkg"; then
      echo "Package $pkg is already installed"
    else
      set -- "$@" "$pkg"
    fi
    i=$((i + 1))
  done

  if [ "$#" -eq 0 ]; then
    return 0
  fi

  echo "Installing $* using $pm..."
  case "$pm" in
    apt-get)
      as_root apt-get update
      as_root env DEBIAN_FRONTEND=noninteractive apt-get install --no-install-recommends -y "$@"
      ;;
    dnf)
      as_root dnf install -y "$@"
      ;;
    yum)
      as_root yum install -y "$@"
      ;;
    pacman)
      as_root pacman -Syu --needed --noconfirm "$@"
      ;;
    apk)
      as_root apk --update add --no-cache "$@"
      ;;
    zypper)
      as_root zypper --non-interactive install -y "$@"
      ;;
    emerge)
      as_root emerge --sync
      as_root emerge --noreplace "$@"
      ;;
    xbps)
      as_root xbps-install -Sy "$@"
      ;;
    brew)
      brew install "$@"
      ;;
    port)
      as_root port -N install "$@"
      ;;
    pkg)
      as_root pkg install -y "$@"
      ;;
    *)
      die "Unsupported package manager: $pm"
      ;;
  esac
}

# TODO: Do we need to add `ca-certificates`, `curl`, `unzip`, `wget`
# Install rustup
if ! has_command cargo; then
  if has_command curl; then
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
  elif has_command wget; then
    wget -qO- https://sh.rustup.rs | sh -s -- -y
  else
    die "Unable to install rustup, no curl or wget."
  fi
fi

OS="$(uname)"
case "$OS" in
  Linux)
    if has_command apt-get; then
      # Ubuntu / Debian based APT-GET
      install_pkgs apt-get build-essential pkgconf libssl-dev git libudev-dev lld libdw-dev clang llvm cmake
    elif has_command dnf; then
      # RHEL / CentOS based DNF
      # This depends on the OS!
      if [ -r /etc/os-release ]; then
        # shellcheck disable=SC1091
        . /etc/os-release
      fi

      # Check if it's Rocky
      if [ "${ID:-}" = "rocky" ]; then
        echo "Rocky Linux detected"
        install_pkgs dnf gcc gcc-c++ make pkgconf openssl-devel git systemd-devel lld elfutils-devel clang llvm cmake
      else
        install_pkgs dnf gcc gcc-c++ make pkgconf openssl-devel git rust-libudev-devel lld elfutils-devel clang llvm cmake
      fi
    elif has_command yum; then
      # RHEL / CentOS based YUM
      install_pkgs yum gcc gcc-c++ make pkgconf openssl-devel git rust-libudev-devel lld elfutils-devel clang llvm cmake
    elif has_command pacman; then
      # Arch based PACMAN
      install_pkgs pacman base-devel pkgconf openssl git lld clang llvm cmake
    elif has_command apk; then
      # Alpine based APK
      install_pkgs apk alpine-sdk coreutils pkgconfig openssl-dev git lld elfutils-dev clang llvm cmake libc-dev
    elif has_command zypper; then
      # OpenSUSE zypper
      install_pkgs zypper gcc gcc-c++ make pkg-config libopenssl-devel git libudev-devel lld libdw-devel clang llvm cmake
    elif has_command emerge; then
      # Gentoo Emerge
      install_pkgs emerge sys-devel/gcc dev-libs/openssl dev-vcs/git dev-lang/rust llvm-core/clang
    elif has_command xbps-install; then
      # Void linux xbps
      install_pkgs xbps gcc make pkg-config git lld elfutils-devel clang llvm cmake
    else
      die "Unable to find supported Linux package manager (apt-get, dnf, yum, pacman, apk, zypper, emerge, or xbps-install)."
    fi
    ;;
  Darwin)
    # macOS
    if has_command brew; then
      install_pkgs brew pkgconfig openssl git llvm cmake
    elif has_command port; then
      install_pkgs port pkgconfig openssl git llvm cmake
    else
      die "Missing package manager Homebrew (https://brew.sh/) or MacPorts (https://www.macports.org/)."
    fi
    ;;
  FreeBSD)
    # FreeBSD
    if has_command pkg; then
      install_pkgs pkg gcc gmake binutils pkgconf git openssl cmake llvm hidapi
    else
      die "Missing package manager pkg (https://www.freebsd.org/ports/)."
    fi
    ;;
  *)
    die "Unknown OS: $OS."
    ;;
esac
