#!/bin/bash
# Copyright © Aptos Foundation
# Parts of the project are originally copyright © Meta Platforms, Inc.
# SPDX-License-Identifier: Apache-2.0


# This script is used to set up a minimal environment for developing on Aptos core.
./minimal_setup.sh

# TODO: add lcov

OS="$(uname)"
case "$OS" in
  Linux)
    if command -v apt-get &>/dev/null; then
      # Ubuntu / Debian based APT-GET
      sh install_pkg.sh lcov
    elif command -v apt &>/dev/null; then
      # Ubuntu / Debian based APT
      sh install_pkg.sh lcov
    elif command -v yum &>/dev/null; then
      # RHEL / CentOS based YUM
      sh install_pkg.sh lcov
    elif command -v dnf &>/dev/null; then
      # RHEL / CentOS based DNF
      sh install_pkg.sh lcov
    elif command -v pacman &>/dev/null; then
      # Arch based PACMAN
      echo "Pacman doesn't support lcov"
    elif command -v apk &>/dev/null; then
      # Alpine based APK
      echo "APK doesn't support lcov"
    else
      # TODO: Support more package managers?
      echo "Unable to find supported package manager (yum, apt-get, dnf, or pacman). Abort"
      exit 1
    fi
  ;;
  Darwin)
    # TODO: May need to do a different path for macports, but atm brew is expected here
    sh install_pkg.sh lcov
  ;;
  *)
    echo "Unknown OS. Abort."
    exit 1
  ;;
esac

rustup install nightly
cargo install cargo-sort
cargo install cargo-machete --locked --version 0.7.0
cargo install cargo-nextest --locked --version 0.9.85
cargo install grcov --version=0.8.2 --locked
