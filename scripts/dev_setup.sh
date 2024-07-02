#!/bin/bash
# -*- sh-basic-offset: 2 -*-
#
# Copyright © Aptos Foundation
# Parts of the project are originally copyright © Meta Platforms, Inc.
# SPDX-License-Identifier: Apache-2.0
#

# This script sets up the environment for the build by installing necessary dependencies.
#
# Usage ./dev_setup.sh <options>
#   v - verbose, print all statements

# Assumptions for nix systems:
# 1 The running user is the user who will execute the builds.
# 2 .profile will be used to configure the shell
# 3 ${HOME}/bin/, or ${INSTALL_DIR} is expected to be on the path - hashicorp tools/etc.  will be installed there on linux systems.

set -o pipefail # trace ERR through pipes
set -o errtrace # trace ERR through 'time command' and other functions
set -o nounset  ## set -u : exit the script if you try to use an uninitialised variable
set -o errexit  ## set -e : exit the script if any statement returns a non-true return value
# set -o xtrace    # print commands as they are executed

NODE_MAJOR_VERSION=20
PNPM_VERSION=8.2.0
SHELLCHECK_VERSION=0.7.1
GRCOV_VERSION=0.8.2
KUBECTL_VERSION=1.18.6
S5CMD_VERSION=2.2.2
TERRAFORM_VERSION=0.12.26
HELM_VERSION=3.2.4
VAULT_VERSION=1.5.0
Z3_VERSION=4.11.2
CVC5_VERSION=0.0.3
DOTNET_VERSION=6.0
BOOGIE_VERSION=3.0.9
ALLURE_VERSION=2.15.pr1135
# this is 3.21.4; the "3" is silent
PROTOC_VERSION=21.4
SOLC_VERSION="v0.8.11+commit.d7f03943"

SCRIPT_PATH="$(cd "$(dirname "$0")" >/dev/null 2>&1 && pwd)"
cd "${SCRIPT_PATH}/.." || die "SCRIPT_PATH not accessible: ${SCRIPT_PATH}"

function die() {
  echo "$@" >&2
  exit 1
}

function msg_info {
  if [[ ${BATCH_MODE} == "false" ]]; then
    echo "${@}"
  fi
}

function usage {
  echo "Usage:"
  echo "Installs or updates necessary dev tools for aptoslabs/aptos-core."
  echo "-b batch mode, no user interactions and minimal output"
  echo "-p update ${HOME}/.profile"
  echo "-t install build tools"
  echo "-r install protoc and related tools"
  echo "-o install operations tooling as well: helm, terraform, yamllint, vault, docker, kubectl, python3"
  echo "-y install or update Move Prover tools: z3, cvc5, dotnet, boogie"
  echo "-d install tools for the Move documentation generator: graphviz"
  echo "-a install tools for build and test api"
  echo "-P install PostgreSQL"
  echo "-J install js/ts tools"
  echo "-v verbose mode"
  echo "-i installs an individual tool by name"
  echo "-n will target the /opt/ dir rather than the $HOME dir.  /opt/bin/, /opt/rustup/, and /opt/dotnet/ rather than $HOME/bin/, $HOME/.rustup/, and $HOME/.dotnet/"
  echo "-k skip pre-commit"
  echo "If no toolchain component is selected with -t, -o, -y, -d, or -p, the behavior is as if -t had been provided."
  echo "This command must be called from the root folder of the Aptos-core project."
}

function add_to_profile {
  eval "$1"
  FOUND=$(grep -c "$1" <"${HOME}/.profile" || true) # grep error return would kill the script.
  if [ "$FOUND" == "0" ]; then
    echo "$1" >>"${HOME}"/.profile
  fi
}

# It is important to keep all path updates together to allow this script to work well when run in github actions
# inside of a docker image created using this script.   GHA wipes the home directory via docker mount options, so
# this profile needs built and sourced on every execution of a job using the docker image.   See the .github/actions/build-setup
# action in this repo, as well as docker/ci/github/Dockerfile.
function update_path_and_profile {
  touch "${HOME}"/.profile

  DOTNET_ROOT="$HOME/.dotnet"
  BIN_DIR="$HOME/bin"
  C_HOME="${HOME}/.cargo"
  if [[ ${OPT_DIR} == "true" ]]; then
    DOTNET_ROOT="/opt/dotnet"
    BIN_DIR="/opt/bin"
    C_HOME="/opt/cargo"
  fi

  mkdir -p "${BIN_DIR}"
  if [[ -n ${CARGO_HOME:-} ]]; then
    add_to_profile "export CARGO_HOME=\"${CARGO_HOME}\""
    add_to_profile "export PATH=\"${BIN_DIR}:${CARGO_HOME}/bin:\$PATH\""
  else
    add_to_profile "export PATH=\"${BIN_DIR}:${C_HOME}/bin:\$PATH\""
  fi
  if [[ ${INSTALL_PROTOC} == "true" ]]; then
    add_to_profile "export PATH=\$PATH:/usr/local/include"
  fi
  if [[ ${INSTALL_PROVER} == "true" ]]; then
    add_to_profile "export DOTNET_ROOT=\"${DOTNET_ROOT}\""
    add_to_profile "export PATH=\"${DOTNET_ROOT}/tools:\$PATH\""
    add_to_profile "export Z3_EXE=\"${BIN_DIR}/z3\""
    add_to_profile "export CVC5_EXE=\"${BIN_DIR}/cvc5\""
    add_to_profile "export BOOGIE_EXE=\"${DOTNET_ROOT}/tools/boogie\""
  fi
  add_to_profile "export SOLC_EXE=\"${BIN_DIR}/solc\""
}

function install_build_essentials {
  case ${PACKAGE_MANAGER} in
    apt-get)
      install_pkg build-essential;;
    pacman)
      install_pkg base-devel;;
    apk)
      install_pkg alpine-sdk coreutils;;
    yum | dnf)
      install_pkg gcc gcc-c++ make;;
  esac
}

function install_protoc {
  local protoc_pkg tmpdir

  if command -v "${INSTALL_DIR}/protoc" &>/dev/null && [[ "$("${INSTALL_DIR}/protoc" --version || true)" =~ .*${PROTOC_VERSION}.* ]]; then
    echo "protoc 3.${PROTOC_VERSION} already installed"
    return
  fi

  case ${OS} in
    linux)
      protoc_pkg="protoc-${PROTOC_VERSION}-linux-x86_64";;
    darwin)
      protoc_pkg="protoc-${PROTOC_VERSION}-osx-universal_binary";;
    *)
      echo "protoc support not configured for this platform (uname=${OS})"
      return;;
  esac

  echo "Installing protoc and plugins"
  tmpdir=$(mktemp -d)
  (
    cd "${tmpdir}" || die "Temporary directory not accessible: ${tmpdir}"
    curl -LOs "https://github.com/protocolbuffers/protobuf/releases/download/v${PROTOC_VERSION}/${protoc_pkg}.zip" --retry 3
    sudo unzip -o ${protoc_pkg}.zip -d /usr/local bin/protoc
    sudo unzip -o ${protoc_pkg}.zip -d /usr/local 'include/*'
    sudo chmod +x /usr/local/bin/protoc
  )
  rm -rf "${tmpdir}"

  # Install the cargo plugins
  if ! command -v protoc-gen-prost &>/dev/null; then
    cargo install protoc-gen-prost --locked
  fi
  if ! command -v protoc-gen-prost-serde &>/dev/null; then
    cargo install protoc-gen-prost-serde --locked
  fi
  if ! command -v protoc-gen-prost-crate &>/dev/null; then
    cargo install protoc-gen-prost-crate --locked
  fi
}

function install_rustup {
  local installed_version
  if [[ ${OPT_DIR} == "true" ]]; then
    export RUSTUP_HOME=/opt/rustup
    mkdir -p "${RUSTUP_HOME}"
    export CARGO_HOME=/opt/cargo
    mkdir -p "${CARGO_HOME}"
  fi

  # Install Rust
  msg_info "Installing Rust......"
  installed_version="$(rustup --version 2>/dev/null || true)"
  if [[ -n ${installed_version:-} ]]; then
    msg_info "Rustup is already installed, version: ${installed_version}"
  else
    curl https://sh.rustup.rs -sSf | sh -s -- -y --default-toolchain stable
    if [[ -n ${CARGO_HOME:-} ]]; then
      PATH="${CARGO_HOME}/bin:${PATH}"
    else
      PATH="${HOME}/.cargo/bin:${PATH}"
    fi
  fi
}

function install_vault {
  local installed_version march archive
  installed_version=$("${INSTALL_DIR}"/vault --version 2>/dev/null || true)
  if [[ ${installed_version} != "Vault v${VAULT_VERSION}" ]]; then
    [[ ${MACHINE} == "x86_64" ]] && march="amd64" || march=${MACHINE}
    archive="vault_${VAULT_VERSION}_${OS}_${march}.zip"
    curl -sL -o "${archive}" "https://releases.hashicorp.com/vault/${VAULT_VERSION}/${archive}"
    unzip -qq -d "${INSTALL_DIR}" "${archive}"
    rm "${archive}"
    chmod +x "${INSTALL_DIR}"/vault
  fi
  vault --version
}

function install_helm {
  local march archive tmpdir
  if command -v helm &>/dev/null; then
    echo "Helm already installed"
    return
  fi
  case ${PACKAGE_MANAGER} in
    brew)
      install_pkg helm brew;;
    *)
      [[ ${MACHINE} == "x86_64" ]] && march="amd64" || march=${MACHINE}
      tmpdir=$(mktemp -d)
      archive="helm-v${HELM_VERSION}-${OS}-${march}.tar.gz"
      curl -sL -o "${tmpdir}"/out.tar.gz "https://get.helm.sh/${archive}"
      tar -zxvf "${tmpdir}"/out.tar.gz -C "${tmpdir}/"
      cp "${tmpdir}/${OS}-${march}/helm" "${INSTALL_DIR}/helm"
      rm -rf "$tmpdir"
      chmod +x "${INSTALL_DIR}"/helm;;
  esac
}

function install_terraform {
  local installed_version march archive
  installed_version=$(terraform --version 2>/dev/null | head -1 || true)
  if [[ ${installed_version} != "Terraform v${TERRAFORM_VERSION}" ]]; then
    case ${PACKAGE_MANAGER} in
      brew)
        install_pkg tfenv brew
        tfenv install ${TERRAFORM_VERSION}
        tfenv use ${TERRAFORM_VERSION};;
      *)
        [[ ${MACHINE} == "x86_64" ]] && march="amd64" || march=${MACHINE}
        archive="terraform_${TERRAFORM_VERSION}_${OS}_${march}.zip"
        curl -sL -o "${archive}" "https://releases.hashicorp.com/terraform/${TERRAFORM_VERSION}/${archive}"
        unzip -qq -d "${INSTALL_DIR}" "${archive}"
        rm "${archive}"
        chmod +x "${INSTALL_DIR}"/terraform
        terraform --version;;
    esac
  fi
}

function install_kubectl {
  local installed_version march
  installed_version=$(kubectl version client --short=true 2>/dev/null | head -1 || true)
  if [[ ${installed_version} != "Client Version: v${KUBECTL_VERSION}" ]]; then
    case ${PACKAGE_MANAGER} in
      brew)
        install_pkg kubectl brew;;
      *)
        [[ ${MACHINE} == "x86_64" ]] && march="amd64" || march=${MACHINE}
        curl -sL -o "${INSTALL_DIR}"/kubectl "https://storage.googleapis.com/kubernetes-release/release/v${KUBECTL_VERSION}/bin/${OS}/${march}/kubectl"
        chmod +x "${INSTALL_DIR}"/kubectl;;
    esac
  fi
  kubectl version client --short=true | head -1 || true
}

function install_awscli {
  local tmpdir
  if ! command -v aws &>/dev/null; then
    case ${PACKAGE_MANAGER} in
      brew)
        install_pkg awscli brew;;
      apk)
        apk add --no-cache python3 py3-pip
        pip3 install --upgrade pip
        pip3 install awscli;;
      *)
        tmpdir=$(mktemp -d)
        mkdir -p "${tmpdir}"/work/
        curl -sL -o "${tmpdir}"/aws.zip "https://awscli.amazonaws.com/awscli-exe-${OS}-${MACHINE}.zip"
        unzip -qq -d "${tmpdir}"/work/ "${tmpdir}"/aws.zip
        TARGET_DIR="${HOME}"/.local/
        if [[ ${OPT_DIR} == "true" ]]; then
          TARGET_DIR="/opt/aws/"
        fi
        mkdir -p "${TARGET_DIR}"
        "${tmpdir}"/work/aws/install -i "${TARGET_DIR}" -b "${INSTALL_DIR}"
        "${INSTALL_DIR}"/aws --version
        rm -rf "${tmpdir}";;
    esac
  fi
}

function install_s5cmd {
  local tmpdir
  if command -v s5cmd &>/dev/null; then
    echo "s5cmd exists, remove before reinstalling."
    return
  fi

  case ${OS} in
    darwin)
      install_pkg peak/tap/s5cmd brew;;
    linux)
      case ${MACHINE} in
        x86_64)
          suffix="64bit";;
        i386 | i686)
          suffix="32bit";;
        aarch64_be | aarch64 | armv8b | armv8l)
          suffix="arm64";;
        arm)
          suffix="armv6";;
        *)
          die "No good way to install s5cmd";;
      esac
      tmpdir=$(mktemp -d)
      mkdir -p "${tmpdir}"/work/
      curl -sL -o "${tmpdir}"/s5cmd.tar.gz https://github.com/peak/s5cmd/releases/download/v${S5CMD_VERSION}/s5cmd_${S5CMD_VERSION}_Linux-${suffix}.tar.gz
      tar -C "${tmpdir}"/work -xzvf "${tmpdir}"/s5cmd.tar.gz
      mv "${tmpdir}"/work/s5cmd "${INSTALL_DIR}"/
      "${INSTALL_DIR}"/s5cmd version
      rm -rf "${tmpdir}";;
  esac
}

function install_pkg {
  echo "Installing $*."
  case ${PACKAGE_MANAGER} in
    apt-get)
      sudo apt-get install --no-install-recommends -y "${@}"
      echo "apt-get install result code: $?";;
    yum)
      sudo yum install -y "${@}";;
    pacman)
      sudo pacman -Syu --noconfirm "${@}";;
    apk)
      apk --update add --no-cache "${@}";;
    dnf)
      dnf install "${@}";;
    brew)
      brew install "${@}";;
  esac
}

function install_xz {
  case ${PACKAGE_MANAGER} in
    apt-get)
      install_pkg xz-utils;;
    *)
      install_pkg xz;;
  esac
}

function install_pkg_config {
  local package=""
  #Differently named packages for pkg-config
  case ${PACKAGE_MANAGER} in
    apt-get | dnf)
      package="pkg-config";;
    pacman)
      package="pkgconf";;
    brew | apk | yum)
      package="pkgconfig";;
    *)
      return;;
  esac
  install_pkg ${package}
}

function install_shellcheck {
  local archive tmpdir
  if command -v shellcheck &>/dev/null; then
    echo "Shellcheck already installed"
    return
  fi
  case ${PACKAGE_MANAGER} in
    brew)
      install_pkg shellcheck brew;;
    *)
      tmpdir=$(mktemp -d)
      archive="shellcheck-v${SHELLCHECK_VERSION}.${OS}.${MACHINE}.tar.xz"
      curl -sL -o "${tmpdir}"/out.tar.xz "https://github.com/koalaman/shellcheck/releases/download/v${SHELLCHECK_VERSION}/${archive}"
      tar -xf "${tmpdir}"/out.tar.xz -C "${tmpdir}"/
      cp "${tmpdir}/shellcheck-v${SHELLCHECK_VERSION}/shellcheck" "${INSTALL_DIR}/shellcheck"
      rm -rf "${tmpdir}"
      chmod +x "${INSTALL_DIR}"/shellcheck;;
  esac
}

function install_openssl_dev {
  #Differently named packages for openssl dev
  case ${PACKAGE_MANAGER} in
    apk)
      install_pkg openssl-dev;;
    apt-get)
      install_pkg libssl-dev;;
    yum | dnf)
      install_pkg openssl-devel;;
    pacman | brew)
      install_pkg openssl;;
  esac
}

function install_lcov {
  #Differently named packages for lcov with different sources.
  case ${PACKAGE_MANAGER} in
    apk)
      apk --update add --no-cache -X http://dl-cdn.alpinelinux.org/alpine/edge/testing lcov;;
    apt-get | yum | dnf | brew)
      install_pkg lcov;;
    pacman)
      echo "nope no lcov for you."
      echo "You can try installing it yourself from sources:"
      echo "git clone https://aur.archlinux.org/lcov.git"
      echo "cd lcov && makepkg -si --noconfirm";;
  esac
}

function install_tidy {
  #Differently named packages for tidy
  case ${PACKAGE_MANAGER} in
    apk)
      apk --update add --no-cache -X http://dl-cdn.alpinelinux.org/alpine/edge/testing tidyhtml;;
    *)
      install_pkg tidy;;
  esac
}

function install_toolchain {
  local version=$1
  if ! rustup show | grep "${version}" >/dev/null; then
    echo "Installing ${version} of rust toolchain"
    rustup install "${version}"
  else
    echo "${version} rust toolchain already installed"
  fi
}

function install_rustup_components_and_nightly {
  echo "Printing the rustup version and toolchain list"
  rustup --version
  rustup show
  rustup toolchain list -v

  echo "Updating rustup and installing rustfmt & clippy"
  rustup update
  rustup component add rustfmt
  rustup component add clippy

  # We require nightly for strict rust formatting
  echo "Installing the nightly toolchain and rustfmt nightly"
  if ! rustup toolchain install nightly; then
    if [[ ${OS} == "linux" ]]; then
      # TODO: remove this once we have an answer: https://github.com/rust-lang/rustup/issues/3390
      echo "Failed to install the nightly toolchain using rustup! Falling back to an older linux build at 2023-06-01."
      rustup toolchain install nightly-2023-06-01 # Fix the date to avoid flakiness

      # Rename the toolchain to nightly (crazy... see: https://github.com/rust-lang/rustup/issues/1299).
      # Note: this only works for linux. The primary purpose is to unblock CI/CD on flakes.
      mv ~/.rustup/toolchains/nightly-2023-06-01-x86_64-unknown-linux-gnu ~/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu
    else
      echo "Failed to install the nightly toolchain using rustup! Manual installation is required!"
    fi
  fi

  if ! rustup component add rustfmt --toolchain nightly; then
    echo "Failed to install rustfmt nightly using rustup."
  fi
}

function install_cargo_sort {
  if ! command -v cargo-sort &>/dev/null; then
    cargo install cargo-sort --locked
  fi
}

function install_cargo_machete {
  if ! command -v cargo-machete &>/dev/null; then
    cargo install cargo-machete --locked
  fi
}

function install_cargo_nextest {
  if ! command -v cargo-nextest &>/dev/null; then
    cargo install cargo-nextest --locked
  fi
}

function install_grcov {
  if ! command -v grcov &>/dev/null; then
    cargo install grcov --version="${GRCOV_VERSION}" --locked
  fi
}

function install_dotnet {
  echo "Installing .Net"
  mkdir -p "${DOTNET_INSTALL_DIR}" || true
  if [[ $("${DOTNET_INSTALL_DIR}/dotnet" --list-sdks | grep -c "^${DOTNET_VERSION}" || true) == "0" ]]; then
    if [[ ${OS} == "linux" ]]; then
      # Install various prerequisites for .dotnet. There are known bugs
      # in the dotnet installer to warn even if they are present. We try
      # to install anyway based on the warnings the dotnet installer creates.
      case ${PACKAGE_MANAGER} in
        apk)
          install_pkg icu zlib libintl libcurl;;
        apt-get)
          install_pkg gettext zlib1g;;
        yum | dnf)
          install_pkg icu zlib;;
        pacman)
          install_pkg icu zlib;;
      esac
    fi
    # Below we need to (a) set TERM variable because the .net installer expects it and it is not set
    # in some environments (b) use bash not sh because the installer uses bash features.
    # NOTE: use wget to better follow the redirect
    wget --tries 10 --retry-connrefused --waitretry=5 https://dot.net/v1/dotnet-install.sh -O dotnet-install.sh
    chmod +x dotnet-install.sh
    ./dotnet-install.sh --channel ${DOTNET_VERSION} --install-dir "${DOTNET_INSTALL_DIR}" --version latest
    rm dotnet-install.sh
  else
    echo Dotnet already installed.
  fi
}

function install_boogie {
  echo "Installing boogie"
  mkdir -p "${DOTNET_INSTALL_DIR}/tools" || true
  if [[ "$("${DOTNET_INSTALL_DIR}/dotnet" tool list --tool-path "${DOTNET_INSTALL_DIR}/tools/")" =~ .*boogie.*${BOOGIE_VERSION}.* ]]; then
    echo "Boogie $BOOGIE_VERSION already installed"
  else
    "${DOTNET_INSTALL_DIR}/dotnet" tool update --tool-path "${DOTNET_INSTALL_DIR}/tools/" Boogie --version $BOOGIE_VERSION
  fi
}

function install_z3 {
  local tmpdir
  echo "Installing Z3"
  if command -v /usr/local/bin/z3 &>/dev/null; then
    echo "z3 already exists at /usr/local/bin/z3"
    echo "but this install will go to ${INSTALL_DIR}/z3."
    echo "you may want to remove the shared instance to avoid version confusion"
  fi
  if command -v "${INSTALL_DIR}/z3" &>/dev/null && [[ "$("${INSTALL_DIR}/z3" --version || true)" =~ .*${Z3_VERSION}.* ]]; then
    echo "Z3 ${Z3_VERSION} already installed"
    return
  fi
  case ${OS} in
    linux)
      Z3_PKG="z3-$Z3_VERSION-x64-glibc-2.31";;
    darwin)
      if [[ ${MACHINE} == "arm64" ]]; then
        Z3_PKG="z3-${Z3_VERSION}-arm64-osx-11.0"
      else
        Z3_PKG="z3-${Z3_VERSION}-x64-osx-10.16"
      fi;;
    *)
      echo "Z3 support not configured for this platform (uname=${OS})"
      return;;
  esac
  tmpdir=$(mktemp -d)
  (
    cd "${tmpdir}" || die "Temporary directory not accessible: ${tmpdir}"
    curl -LOs "https://github.com/Z3Prover/z3/releases/download/z3-$Z3_VERSION/$Z3_PKG.zip"
    unzip -q "$Z3_PKG.zip"
    cp "$Z3_PKG/bin/z3" "${INSTALL_DIR}"
    chmod +x "${INSTALL_DIR}/z3"
  )
  rm -rf "${tmpdir}"
}

function install_cvc5 {
  local tmpdir
  echo "Installing cvc5"
  if command -v /usr/local/bin/cvc5 &>/dev/null; then
    echo "cvc5 already exists at /usr/local/bin/cvc5"
    echo "but this install will go to $${INSTALL_DIR}/cvc5."
    echo "you may want to remove the shared instance to avoid version confusion"
  fi
  if command -v "${INSTALL_DIR}/cvc5" &>/dev/null && [[ "$("${INSTALL_DIR}/cvc5" --version || true)" =~ .*${CVC5_VERSION}.* ]]; then
    echo "cvc5 ${CVC5_VERSION} already installed"
    return
  fi
  case ${OS} in
    linux)
      CVC5_PKG="cvc5-Linux";;
    darwin)
      CVC5_PKG="cvc5-macOS";;
    *)
      echo "cvc5 support not configured for this platform (uname=${OS})"
      return;;
  esac
  tmpdir=$(mktemp -d)
  (
    cd "${tmpdir}" || die "Temporary directory not accessible: ${tmpdir}"
    curl -LOs "https://github.com/cvc5/cvc5/releases/download/cvc5-${CVC5_VERSION}/${CVC5_PKG}" || true
    cp "$CVC5_PKG" "${INSTALL_DIR}/cvc5" || true
    chmod +x "${INSTALL_DIR}/cvc5" || true
  )
  rm -rf "${tmpdir}"
}

function install_allure {
  local installed_version
  installed_version="$(allure --version 2>/dev/null || true)"
  if [[ ${installed_version} != "${ALLURE_VERSION}" ]]; then
    case ${PACKAGE_MANAGER} in
      apt-get)
        sudo apt-get install default-jre -y --no-install-recommends
        export ALLURE=${HOME}/allure_"${ALLURE_VERSION}"-1_all.deb
        curl -sL -o "$ALLURE" "https://github.com/diem/allure2/releases/download/${ALLURE_VERSION}/allure_${ALLURE_VERSION}-1_all.deb"
        sudo dpkg -i "${ALLURE}"
        rm "${ALLURE}";;
      apk)
        apk --update add --no-cache -X http://dl-cdn.alpinelinux.org/alpine/edge/community openjdk11;;
      *)
        echo "No good way to install allure";;
    esac
  fi
}

function install_xsltproc {
  case ${PACKAGE_MANAGER} in
    apt-get)
      install_pkg xsltproc;;
    *)
      install_pkg libxslt;;
  esac
}

function install_nodejs {
  case ${PACKAGE_MANAGER} in
    apt-get)
      curl -fsSL "https://deb.nodesource.com/setup_${NODE_MAJOR_VERSION}.x" -o nodesource_setup.sh
      chmod +x nodesource_setup.sh
      sudo -E bash ./nodesource_setup.sh;;
  esac
  install_pkg nodejs
  # The nodejs Debian package already bundles npm
  if [[ ${PACKAGE_MANAGER} != "apt-get" ]]; then
    install_pkg npm
  fi
}

function install_solidity {
  local solc_bin
  echo "Installing Solidity compiler"
  if [ -f "${INSTALL_DIR}/solc" ]; then
    echo "Solidity already installed at ${INSTALL_DIR}/solc"
    return
  fi
  # We fetch the binary from  https://binaries.soliditylang.org
  case ${OS} in
    linux)
      solc_bin="linux-amd64/solc-linux-amd64-${SOLC_VERSION}";;
    darwin)
      solc_bin="macosx-amd64/solc-macosx-amd64-${SOLC_VERSION}";;
    *)
      echo "Solidity support not configured for this platform (uname=${OS})"
      return;;
  esac
  curl -o "${INSTALL_DIR}/solc" "https://binaries.soliditylang.org/${solc_bin}"
  chmod +x "${INSTALL_DIR}/solc"
}

function install_pnpm {
  curl -fsSL https://get.pnpm.io/install.sh | sudo env PNPM_VERSION=${PNPM_VERSION} SHELL="$(which bash)" bash -
}

function install_python3 {
  case ${PACKAGE_MANAGER} in
    apt-get)
      install_pkg python3-all-dev python3-setuptools python3-pip;;
    apk)
      install_pkg python3-dev;;
    *)
      install_pkg python3;;
  esac
}

function install_postgres {
  case ${PACKAGE_MANAGER} in
    apt-get | apk)
      install_pkg libpq-dev;;
    pacman | yum)
      install_pkg postgresql-libs;;
    dnf)
      install_pkg libpq-devel;;
    brew)
      install_pkg postgresql;;
  esac
}

function install_lld {
  # Right now, only install lld for linux
  if [[ ${OS} == "linux" ]]; then
    install_pkg lld
  fi
}

function install_libdw {
  # Right now, only install libdw for linux
  if [[ ${OS} == "linux" ]]; then
    install_pkg libdw-dev
  fi
}

# this is needed for hdpi crate from aptos-ledger
function install_libudev-dev {
  # Need to install libudev-dev for linux
  if [[ ${OS} == "linux" && ${PACKAGE_MANAGER} != "pacman" ]]; then
    install_pkg libudev-dev
  fi
}

function welcome_message {
  cat <<EOF
Welcome to Aptos!

This script will download and install the necessary dependencies needed to
build, test and inspect Aptos Core.

Based on your selection, these tools will be included:
EOF

  if [[ ${INSTALL_BUILD_TOOLS} == "true" ]]; then
    cat <<EOF
Build tools (since -t or no option was provided):
  * Rust (and the necessary components, e.g. rust-fmt, clippy)
  * CMake
  * Clang
  * grcov
  * lcov
  * pkg-config
  * libssl-dev
  * lld (only for Linux)
EOF
  fi

  if [[ ${INSTALL_PROTOC} == "true" ]]; then
    cat <<EOF
protoc and related plugins (since -r or -t was provided):
  * protoc
EOF
  fi

  if [[ ${OPERATIONS} == "true" ]]; then
    cat <<EOF
Operation tools (since -o was provided):
  * yamllint
  * python3
  * docker
  * vault
  * terraform
  * kubectl
  * helm
  * aws cli
  * s5cmd
  * allure
EOF
  fi

  if [[ ${INSTALL_PROVER} == "true" ]]; then
    cat <<EOF
Move prover tools (since -y was provided):
  * z3
  * cvc5
  * dotnet
  * boogie
EOF
  fi

  if [[ ${INSTALL_DOC} == "true" ]]; then
    cat <<EOF
tools for the Move documentation generator (since -d was provided):
  * graphviz
EOF
  fi

  if [[ ${INSTALL_API_BUILD_TOOLS} == "true" ]]; then
    cat <<EOF
API build and testing tools (since -a was provided):
  * Python3 (schemathesis)
EOF
  fi

  if [[ ${INSTALL_POSTGRES} == "true" ]]; then
    cat <<EOF
PostgreSQL database (since -P was provided):
EOF
  fi

  if [[ ${INSTALL_JSTS} == "true" ]]; then
    cat <<EOF
Javascript/TypeScript tools (since -J was provided):
  * node.js
  * pnpm
  * solidity
EOF
  fi

  if [[ ${INSTALL_PROFILE} == "true" ]]; then
    cat <<EOF
Moreover, ~/.profile will be updated (since -p was provided).
EOF
  fi

  cat <<EOF
If you'd prefer to install these dependencies yourself, please exit this script
now with Ctrl-C.
EOF
}

BATCH_MODE=false
# set verbose if not interactive.
VERBOSE=false
[[ ! (-t 2) ]] && VERBOSE=true
INSTALL_BUILD_TOOLS=false
OPERATIONS=false
INSTALL_PROFILE=false
INSTALL_PROVER=false
INSTALL_DOC=false
INSTALL_PROTOC=false
INSTALL_API_BUILD_TOOLS=false
INSTALL_POSTGRES=false
INSTALL_JSTS=false
INSTALL_INDIVIDUAL=false
INSTALL_PACKAGES=()
INSTALL_DIR=${HOME}/bin
OPT_DIR=false
SKIP_PRE_COMMIT=false

#parse args
while getopts "btoprvydaPJh:i:nk" arg; do
  case "${arg}" in
  b)
    BATCH_MODE=true;;
  t)
    INSTALL_BUILD_TOOLS=true
    INSTALL_PROTOC=true;;
  o)
    OPERATIONS=true;;
  p)
    INSTALL_PROFILE=true;;
  r)
    INSTALL_PROTOC=true;;
  v)
    VERBOSE=true;;
  y)
    INSTALL_PROVER=true;;
  d)
    INSTALL_DOC=true;;
  a)
    INSTALL_API_BUILD_TOOLS=true;;
  P)
    INSTALL_POSTGRES=true;;
  J)
    INSTALL_JSTS=true;;
  i)
    INSTALL_INDIVIDUAL=true
    echo "${OPTARG}"
    INSTALL_PACKAGES+=("${OPTARG}");;
  n)
    OPT_DIR=true;;
  k)
    SKIP_PRE_COMMIT=true;;
  *)
    usage
    exit 0;;
  esac
done

if [[ ${VERBOSE} == "true" ]]; then
  set -x
fi

if [[ ${INSTALL_BUILD_TOOLS} == "false" &&
        ${OPERATIONS} == "false" &&
        ${INSTALL_PROFILE} == "false" &&
        ${INSTALL_PROVER} == "false" &&
        ${INSTALL_DOC} == "false" &&
        ${INSTALL_API_BUILD_TOOLS} == "false" &&
        ${INSTALL_POSTGRES} == "false" &&
        ${INSTALL_JSTS} == "false" &&
        ${INSTALL_INDIVIDUAL} == "false" ]]; then
  INSTALL_BUILD_TOOLS=true
  INSTALL_PROTOC=true
fi

if [[ ! -f rust-toolchain.toml ]]; then
  die "Unknown location. Please run this from the aptos-core repository. Abort."
fi

if [[ ${OPT_DIR} == "true" ]]; then
  INSTALL_DIR="/opt/bin"
fi
mkdir -p "${INSTALL_DIR}" || true

#############################################
#                                           #
#            Detect OS environment          #
#                                           #
#############################################

OS=$(uname -s | tr '[:upper:]' '[:lower:]')
MACHINE=$(uname -m | tr '[:upper:]' '[:lower]')

PACKAGE_MANAGER=""
case ${OS} in
  linux)
    if command -v yum &>/dev/null; then
      PACKAGE_MANAGER="yum"
    elif command -v apt-get &>/dev/null; then
      PACKAGE_MANAGER="apt-get"
    elif command -v pacman &>/dev/null; then
      PACKAGE_MANAGER="pacman"
    elif command -v apk &>/dev/null; then
      PACKAGE_MANAGER="apk"
    elif command -v dnf &>/dev/null; then
      echo "WARNING: dnf package manager support is experimental"
      PACKAGE_MANAGER="dnf"
    else
      die "Unable to find supported package manager (yum, apt-get, dnf, or pacman). Abort"
    fi;;
  darwin)
      if command -v brew &>/dev/null; then
        PACKAGE_MANAGER="brew"
      else
        die "Missing package manager Homebrew (https://brew.sh/). Abort"
      fi;;
  *)
    die "Unknown OS. Abort.";;
esac

#############################################
#                                           #
#            Start installation             #
#                                           #
#############################################

if [[ ${BATCH_MODE} == "false" ]]; then
  welcome_message
  printf "Proceed with installing necessary dependencies? (y/N) > "
  read -e -r input
  if [[ "$input" != "y"* ]]; then
    echo "Exiting..."
    exit 0
  fi
fi

#
# Update package repositories
#

if [[ ${PACKAGE_MANAGER} == "apt-get" ]]; then
  msg_info "Updating apt-get......"
  sudo apt-get update
  msg_info "Installing ca-certificates......"
  install_pkg ca-certificates
fi

#
# Update shell profile with tool paths
#

if [[ ${INSTALL_PROFILE} == "true" ]]; then
  update_path_and_profile
fi
export PATH=${INSTALL_DIR}:${PATH}

#
# Install essentials
#

install_pkg curl wget unzip
install_xz

if [[ ${INSTALL_BUILD_TOOLS} == "true" ]]; then
  install_build_essentials
  install_pkg cmake clang llvm git

  install_openssl_dev
  install_pkg_config

  install_lld
  install_libdw

  install_rustup
  install_toolchain "$(grep channel ./rust-toolchain.toml | grep -o '"[^"]\+"' | sed 's/"//g')" # TODO: Fix me. This feels hacky.
  install_rustup_components_and_nightly

  install_cargo_sort
  install_cargo_machete
  install_cargo_nextest
  install_grcov
  install_lcov
fi

#
# Protoc
#

if [[ ${INSTALL_PROTOC} == "true" ]]; then
  install_protoc
fi

#
# Command line utilities
#

if [[ ${OPERATIONS} == "true" ]]; then
  install_pkg python3 jq git yamllint
  install_tidy
  install_xsltproc
  #for timeout
  if [[ ${PACKAGE_MANAGER} == "apt-get" ]]; then
    install_pkg coreutils
  fi
  install_shellcheck
  install_vault
  install_helm
  install_terraform
  install_kubectl
  install_awscli
  install_s5cmd
  install_allure
fi

#
# Packages requested on the command line
#

if [[ ${INSTALL_INDIVIDUAL} == "true" ]]; then
  for ((i = 0; i < ${#INSTALL_PACKAGES[@]}; i++)); do
    PACKAGE=${INSTALL_PACKAGES[$i]}
    if ! command -v "install_${PACKAGE}" &>/dev/null; then
      install_pkg "${PACKAGE}"
    else
      "install_${PACKAGE}"
    fi
  done
fi

#
# Prover
#

if [[ ${INSTALL_PROVER} == "true" ]]; then
  export DOTNET_INSTALL_DIR="${HOME}/.dotnet"
  if [[ ${OPT_DIR} == "true" ]]; then
    export DOTNET_INSTALL_DIR="/opt/dotnet"
  fi
  mkdir -p "${DOTNET_INSTALL_DIR}"
  install_z3
  install_cvc5
  install_dotnet
  install_boogie
fi

#
# Graphviz
#

if [[ ${INSTALL_DOC} == "true" ]]; then
  install_pkg graphviz
fi

#
# API tools
#

if [[ ${INSTALL_API_BUILD_TOOLS} == "true" ]]; then
  # python and tools
  install_python3
  sudo python3 -m pip install schemathesis
fi

#
# Postgres
#

if [[ ${INSTALL_POSTGRES} == "true" ]]; then
  install_postgres
fi

#
# Node.js and Javascript tools
#

if [[ ${INSTALL_JSTS} == "true" ]]; then
  # javascript and typescript tools
  install_nodejs
  install_pnpm
  install_solidity
fi

#
# Python3
#
install_python3
if [[ ${SKIP_PRE_COMMIT} == "false" ]]; then
  if [[ ${PACKAGE_MANAGER} != "pacman" ]]; then
    pip3 install pre-commit
    install_libudev-dev
  else
    install_pkg python-pre-commit
  fi

  # For now best effort install, will need to improve later
  if command -v pre-commit; then
    pre-commit install
  else
    ~/.local/bin/pre-commit install
  fi
fi

msg_info <<EOF
Finished installing all dependencies.

You should now be able to build the project by running:
	cargo build
EOF

exit 0
