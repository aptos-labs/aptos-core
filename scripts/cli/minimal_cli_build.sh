#!/bin/sh
# Copyright © Aptos Foundation
# Parts of the project are originally copyright © Meta Platforms, Inc.
# SPDX-License-Identifier: Apache-2.0

set -e

# Set to 1 after `emerge --sync` in this script (avoids running sync twice on Gentoo).
EMERGE_SYNC_DONE=0

has_command() {
  command -v "$1" > /dev/null 2>&1
}

# Run a command with root privileges when needed (no `sudo` when already root).
run_as_root() {
  if [ "$(id -u)" -eq 0 ]; then
    "$@"
  elif has_command sudo; then
    sudo "$@"
  else
    echo "Error: root privileges are required for: $*. Rerun as root or install sudo." 1>&2
    exit 1
  fi
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
    if ! has_command sudo; then
      echo "Error: sudo is required to install packages when not running as root. Rerun as root or install sudo." 1>&2
      exit 1
    fi
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
      $pre_cmd pacman -Sy --noconfirm
      $pre_cmd pacman -S --noconfirm --needed "$@"
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
      $pre_cmd port install "$@"
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

# If HTTPS clients are missing, install curl and wget (and CA bundle) via the OS package manager.
bootstrap_ssl_and_fetch_clients() {
  if has_command curl || has_command wget; then
    return 0
  fi

  os="$(uname -s)"
  case "$os" in
    Linux)
      if has_command apt-get; then
        run_as_root apt-get update
        minimal_install_pkgs apt-get ca-certificates curl wget unzip
      elif has_command dnf; then
        minimal_install_pkgs dnf ca-certificates curl wget unzip
      elif has_command yum; then
        minimal_install_pkgs yum ca-certificates curl wget unzip
      elif has_command pacman; then
        minimal_install_pkgs pacman ca-certificates curl wget unzip
      elif has_command apk; then
        minimal_install_pkgs apk ca-certificates curl wget unzip
      elif has_command zypper; then
        minimal_install_pkgs zypper ca-certificates curl wget unzip
      elif has_command emerge; then
        run_as_root emerge --sync
        EMERGE_SYNC_DONE=1
        minimal_install_pkgs emerge app-misc/ca-certificates net-misc/curl net-misc/wget app-arch/unzip
      elif has_command xbps-install; then
        minimal_install_pkgs xbps ca-certificates curl wget unzip
      else
        echo "Neither curl nor wget is installed, and no supported package manager was found to install them. Abort." 1>&2
        exit 1
      fi
      ;;
    Darwin)
      if has_command brew; then
        minimal_install_pkgs brew ca-certificates curl wget unzip
      elif has_command port; then
        minimal_install_pkgs port curl wget unzip
      else
        echo "Neither curl nor wget is installed, and neither Homebrew nor MacPorts was found to install them. Abort." 1>&2
        exit 1
      fi
      ;;
    FreeBSD)
      minimal_install_pkgs pkg ca_root_nss curl wget unzip
      ;;
    *)
      echo "Neither curl nor wget is installed, and automatic install is not implemented for this OS ($os). Abort." 1>&2
      exit 1
      ;;
  esac
}

# Install rustup when cargo is missing (prefers curl, then wget, then FreeBSD fetch).
# Download to a temp file so downloader failures are not masked by POSIX pipeline exit status.
ensure_rustup() {
  if has_command cargo; then
    return 0
  fi

  rustup_url="https://sh.rustup.rs"
  tmp="$(mktemp "${TMPDIR:-/tmp}/rustup-init.XXXXXX")"
  trap 'rm -f "$tmp"' 0

  if has_command curl; then
    curl --proto '=https' --tlsv1.2 -sSf "$rustup_url" -o "$tmp"
  elif has_command wget; then
    wget -qO "$tmp" "$rustup_url"
  elif has_command fetch && [ "$(uname -s)" = "FreeBSD" ]; then
    fetch -o "$tmp" "$rustup_url"
  else
    echo "Cannot install rustup: need curl or wget (install one with your package manager). Abort." 1>&2
    exit 1
  fi

  sh "$tmp" -y
  rm -f "$tmp"
  trap - 0
}

# This script is used to set up a minimal environment for building the Aptos CLI and other tools.
# The `dev_setup.sh` script is way too complex, and too hard to figure out what is actually happening.  This script
# simplifies the process

OS="$(uname)"
bootstrap_ssl_and_fetch_clients

case "$OS" in
  Linux)
    if has_command apt-get; then
      # Ubuntu / Debian based APT-GET
      run_as_root apt-get update
      minimal_install_pkgs apt-get ca-certificates curl wget unzip build-essential pkgconf libssl-dev git libudev-dev lld libdw-dev clang llvm cmake
    elif has_command dnf; then
      # RHEL / CentOS based DNF
      # This depends on the OS!
      # Source the os-release file to parse it
      . /etc/os-release

      # Check if it's Rocky
      if [ "$ID" = "rocky" ]; then
        echo "Rocky Linux detected"
        minimal_install_pkgs dnf ca-certificates curl wget unzip gcc gcc-c++ make pkgconf openssl-devel git systemd-devel lld elfutils-devel clang llvm cmake
      else
        minimal_install_pkgs dnf ca-certificates curl wget unzip gcc gcc-c++ make pkgconf openssl-devel git rust-libudev-devel lld elfutils-devel clang llvm cmake
      fi
    elif has_command yum; then
      # RHEL / CentOS based YUM
      minimal_install_pkgs yum ca-certificates curl wget unzip gcc gcc-c++ make pkgconf openssl-devel git rust-libudev-devel lld elfutils-devel clang llvm cmake
    elif has_command pacman; then
      # Arch based PACMAN
      minimal_install_pkgs pacman ca-certificates curl wget unzip base-devel pkgconf openssl git lld clang llvm cmake
    elif has_command apk; then
      # Alpine based APK
      minimal_install_pkgs apk ca-certificates curl wget unzip alpine-sdk coreutils pkgconfig openssl-dev git lld elfutils-dev clang llvm cmake libc-dev
    elif has_command zypper; then
      # OpenSUSE zypper
      minimal_install_pkgs zypper ca-certificates curl wget unzip gcc gcc-c++ make pkg-config libopenssl-devel git libudev-devel lld libdw-devel clang llvm cmake
    elif has_command emerge; then
      # Gentoo Emerge
      if [ "$EMERGE_SYNC_DONE" != 1 ]; then
        run_as_root emerge --sync
      fi
      minimal_install_pkgs emerge app-misc/ca-certificates net-misc/curl net-misc/wget app-arch/unzip sys-devel/gcc dev-libs/openssl dev-vcs/git dev-lang/rust llvm-core/clang
    elif has_command xbps-install; then
      # Void linux xbps
      minimal_install_pkgs xbps ca-certificates curl wget unzip gcc make pkg-config git lld elfutils-devel clang llvm cmake
    else
      echo "Unable to find supported Linux package manager (apt-get, dnf, yum, zypper, xbps or pacman). Abort"
      exit 1
    fi
  ;;
  Darwin)
    # macOS (Homebrew or MacPorts)
    if has_command brew; then
      minimal_install_pkgs brew ca-certificates curl wget unzip pkg-config openssl git llvm cmake
    elif has_command port; then
      minimal_install_pkgs port curl wget unzip pkgconfig openssl git llvm cmake
    else
      echo "Missing package manager Homebrew (https://brew.sh/) or MacPorts (https://www.macports.org/). Abort." 1>&2
      exit 1
    fi
  ;;
  FreeBSD)
    # FreeBSD
    minimal_install_pkgs pkg ca_root_nss curl wget unzip gcc gmake binutils pkgconf git openssl cmake llvm hidapi
  ;;
  *)
    echo "Unknown OS. Abort."
    exit 1
  ;;
esac

ensure_rustup
