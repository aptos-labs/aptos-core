#!/bin/sh
# Copyright (c) Aptos Foundation
# Parts of the project are originally copyright (c) Meta Platforms, Inc.
# SPDX-License-Identifier: Apache-2.0
#
# setup_build.sh - Development environment setup for aptos-core
#
# Installs the tools and libraries required to build, test, and develop
# Aptos Core.  Written in POSIX sh for portability across Linux distros
# and macOS.
#
# Run with --help for full usage information.
#
# ADDING A NEW PACKAGE MANAGER:
#   1. Add detection logic in detect_package_manager()
#   2. Add name mappings in resolve_pkg_name()
#   3. Add install command in install_pkg()
#   That's it -- every other function uses these three.

set -e

# ============================================================================
# Tool Versions (pinned for reproducible builds)
#
# Every externally-downloaded tool is version-pinned here.  To upgrade a
# tool, change its version constant and (if needed) update the download
# URL pattern in the corresponding install_*() function.
# ============================================================================

# -- Build tools --

# Clang/LLVM -- C/C++ compiler used by Rust crates with native bindings
# (https://apt.llvm.org/ for apt; system repos for others)
CLANG_VERSION=21

# cargo-sort -- keeps Cargo.toml [dependencies] sections alphabetically sorted
# (https://crates.io/crates/cargo-sort)
CARGO_SORT_VERSION=2.0.2

# cargo-machete -- detects unused crate dependencies in Cargo.toml
# (https://crates.io/crates/cargo-machete)
CARGO_MACHETE_VERSION=0.9.1

# cargo-nextest -- faster Rust test runner with better output and retries
# (https://crates.io/crates/cargo-nextest)
CARGO_NEXTEST_VERSION=0.9.128

# grcov -- Rust source-code coverage aggregator (https://github.com/mozilla/grcov)
GRCOV_VERSION=0.10.5

# protoc -- Protocol Buffers compiler (https://github.com/protocolbuffers/protobuf)
PROTOC_VERSION=29.3

# -- Operations tools --

# ShellCheck -- static analysis for shell scripts (https://www.shellcheck.net/)
SHELLCHECK_VERSION=0.10.0

# kubectl -- Kubernetes cluster CLI (https://kubernetes.io/docs/reference/kubectl/)
KUBECTL_VERSION=1.32.3

# Terraform -- declarative infrastructure as code (https://www.terraform.io/)
TERRAFORM_VERSION=1.10.5

# Helm -- Kubernetes package manager (https://helm.sh/)
HELM_VERSION=3.17.4

# HashiCorp Vault -- secrets management (https://www.vaultproject.io/)
VAULT_VERSION=1.18.5

# s5cmd -- high-performance S3 file manager (https://github.com/peak/s5cmd)
S5CMD_VERSION=2.3.0

# Allure -- test-reporting framework used in CI dashboards
# (https://docs.qameta.io/allure/)  NOTE: pinned to diem fork release
ALLURE_VERSION=2.15.pr1135

# -- Move Prover tools --

# Z3 -- SMT solver, primary backend for the Move Prover
# (https://github.com/Z3Prover/z3)
# NOTE: pinned to 4.11.2 because newer releases require glibc >= 2.35,
# which would break CI containers on older distros.
Z3_VERSION=4.11.2

# cvc5 -- SMT solver, alternative backend for the Move Prover
# (https://cvc5.github.io/)
# NOTE: pinned to 0.0.3 because newer releases changed the binary
# distribution format (zip archives instead of bare binaries).
CVC5_VERSION=0.0.3

# .NET SDK -- runtime needed to execute the Boogie verifier
# (https://dotnet.microsoft.com/)
DOTNET_VERSION=8.0

# Boogie -- intermediate verification language used by the Move Prover
# (https://github.com/boogie-org/boogie)
BOOGIE_VERSION=3.5.6

# -- JS/TS tools --

# Node.js LTS major version -- used for JS/TS SDK development
NODE_MAJOR_VERSION=22

# pnpm -- fast, disk-efficient Node.js package manager (https://pnpm.io/)
PNPM_VERSION=10.6.4

# ============================================================================
# Resolve script location and cd to repo root
# ============================================================================

SCRIPT_PATH="$(cd "$(dirname "$0")" >/dev/null 2>&1 && pwd)"
cd "$SCRIPT_PATH/.." || {
    echo "ERROR: Could not cd to repository root from $SCRIPT_PATH" >&2
    exit 1
}

# ============================================================================
# Logging Utilities
# ============================================================================

# Enable colour when stderr is a terminal
if [ -t 2 ]; then
    _CLR_RED='\033[0;31m'
    _CLR_YEL='\033[0;33m'
    _CLR_GRN='\033[0;32m'
    _CLR_CYN='\033[0;36m'
    _CLR_RST='\033[0m'
else
    _CLR_RED='' _CLR_YEL='' _CLR_GRN='' _CLR_CYN='' _CLR_RST=''
fi

log_info()  { printf "${_CLR_GRN}[INFO]${_CLR_RST}  %s\n" "$*" >&2; }
log_warn()  { printf "${_CLR_YEL}[WARN]${_CLR_RST}  %s\n" "$*" >&2; }
log_error() { printf "${_CLR_RED}[ERROR]${_CLR_RST} %s\n" "$*" >&2; }
log_step()  { printf "${_CLR_CYN}[STEP]${_CLR_RST}  %s\n" "$*" >&2; }

# Print an error and exit.  Accepts multiple arguments; each is printed
# on its own line so callers can provide context and remediation hints.
die() {
    for _line in "$@"; do
        log_error "$_line"
    done
    exit 1
}

# ============================================================================
# Help / Usage
# ============================================================================

show_help() {
    cat <<'HELPTEXT'
NAME
    setup_build.sh - Development environment setup for Aptos Core

SYNOPSIS
    ./scripts/setup_build.sh [OPTIONS]

DESCRIPTION
    Installs or updates the tools and libraries needed to build, test, and
    develop aptos-core.  The script auto-detects your OS and package manager.

    If no component flags (-t, -o, -y, -d, -r, -P, -J) are provided, the
    default behavior is equivalent to -t (install build tools only).

COMPONENT FLAGS
    -t    Install core build tools.  This is the most common choice and is
          the default when no flags are given.  Includes:

            Rust toolchain    Stable + nightly, rustfmt, clippy.  The
                              specific channel is read from rust-toolchain.toml.
            CMake             Cross-platform build system generator used by
                              native C/C++ dependencies.
            Clang / LLVM      C/C++ compiler required by several Rust crates
                              that include C bindings (e.g. rocksdb, lz4).
            grcov             Aggregates Rust code-coverage data into reports.
            lcov              Generates line-coverage HTML reports.
            pkg-config        Resolves library paths so the Rust build system
                              can find native dependencies.
            OpenSSL dev libs  Headers and libraries for TLS/crypto, used by
                              network and API crates.
            protoc            Protocol Buffers compiler plus Rust codegen
                              plugins (prost, prost-serde, prost-crate).
            lld               Fast linker that significantly reduces link times
                              (Linux only).
            libdw             DWARF debug-info library for profiling and
                              backtraces (Linux only).
            libudev           Device-event library needed by the hidapi crate
                              for Aptos Ledger hardware wallet support
                              (Linux only).
            cargo-sort        Sorts Cargo.toml dependency sections.
            cargo-machete     Detects unused crate dependencies.
            cargo-nextest     Faster test runner with better output.
            git               Version control (usually pre-installed).

    -o    Install operations / infrastructure tools:

            yamllint          YAML file linter.
            python3           Python interpreter for scripts.
            jq                Command-line JSON processor.
            tidy / xsltproc   HTML/XML validation and transformation.
            shellcheck        Static analysis for shell scripts.
            vault             HashiCorp secrets management.
            helm              Kubernetes package manager.
            terraform         Infrastructure-as-code provisioning.
            kubectl           Kubernetes cluster CLI.
            AWS CLI           Amazon Web Services command-line interface.
            s5cmd             High-performance S3 file manager.
            allure            Test-reporting framework.
            coreutils         GNU core utilities (apt-get only).

    -y    Install Move Prover tools:

            z3                SMT solver (primary Prover backend).
            cvc5              SMT solver (alternative Prover backend).
            .NET SDK          Runtime for the Boogie verifier.
            Boogie            Intermediate verification language that the
                              Move Prover compiles down to.

    -d    Install documentation-generator tools:

            graphviz          Renders graphs and diagrams for generated docs.

    -r    Install protoc and related Cargo plugins only (without the full
          build-tools set):

            protoc               Protocol Buffers compiler.
            protoc-gen-prost     Rust protobuf code generation.
            protoc-gen-prost-serde   Serde support for generated types.
            protoc-gen-prost-crate   Crate-level code generation.

    -P    Install PostgreSQL development libraries (libpq-dev or equivalent)
          needed by the indexer and analytics components.

    -J    Install JavaScript / TypeScript tools:

            Node.js (v22)     JavaScript runtime (LTS).
            pnpm              Fast, disk-efficient Node.js package manager.

MODIFIER FLAGS
    -b    Batch mode.  Suppresses the interactive confirmation prompt and
          reduces informational output.  Recommended for CI/CD pipelines
          and Docker image builds.

    -p    Update ~/.profile with PATH entries for all installed tools.
          Adds CARGO_HOME, DOTNET_ROOT, Z3_EXE, CVC5_EXE, BOOGIE_EXE
          as appropriate.

    -n    Install to /opt/ instead of $HOME.  Uses /opt/bin/, /opt/rustup/,
          /opt/cargo/, /opt/dotnet/.  Useful for shared or containerized
          environments.

    -i NAME
          Install an individual tool by name.  May be specified multiple
          times (e.g. -i z3 -i protoc).  If NAME matches a built-in
          installer function (install_<NAME>), that function is called;
          otherwise the system package manager is used.

    -k    Skip pre-commit hook installation.

    -v    Verbose mode.  Prints every shell command as it executes (set -x).
          Automatically enabled when stderr is not a terminal (e.g. in CI).

    -h, --help
          Show this help message and exit.

SUPPORTED PACKAGE MANAGERS
    apt-get   Debian, Ubuntu, and derivatives
    yum       RHEL, CentOS, Amazon Linux
    dnf       Fedora (experimental)
    pacman    Arch Linux
    apk       Alpine Linux
    brew      macOS (Homebrew)

    To add support for a new package manager:
      1. Add detection logic in detect_package_manager()
      2. Add package-name mappings in resolve_pkg_name()
      3. Add the install command in install_pkg()

EXAMPLES
    # Interactive install of default build tools
    ./scripts/setup_build.sh

    # CI: non-interactive build tools + Move Prover
    ./scripts/setup_build.sh -b -t -y

    # Install only protoc and plugins
    ./scripts/setup_build.sh -b -r

    # Docker image build targeting /opt
    ./scripts/setup_build.sh -b -t -n

    # Install a single tool by name
    ./scripts/setup_build.sh -b -i z3

    # Full development setup (everything)
    ./scripts/setup_build.sh -b -t -o -y -d -P -J -p
HELPTEXT
}

# ============================================================================
# Privilege Helper
# ============================================================================

# Prints "sudo" when not running as root; empty otherwise.
# Usage:  $(sudo_if_needed) apt-get install ...
sudo_if_needed() {
    if [ "$(id -u)" != "0" ]; then
        echo "sudo"
    fi
}

# ============================================================================
# Package Manager Detection
#
# Checks for known package managers in a deterministic order.
# To add a new PM, insert a command -v check here and the matching
# install_pkg / resolve_pkg_name cases below.
# ============================================================================

detect_package_manager() {
    case "$(uname -s)" in
        Linux)
            if command -v apt-get >/dev/null 2>&1; then
                echo "apt-get"
            elif command -v yum >/dev/null 2>&1; then
                echo "yum"
            elif command -v pacman >/dev/null 2>&1; then
                echo "pacman"
            elif command -v apk >/dev/null 2>&1; then
                echo "apk"
            elif command -v dnf >/dev/null 2>&1; then
                log_warn "dnf package manager support is experimental"
                echo "dnf"
            else
                die "No supported Linux package manager found." \
                    "Looked for: apt-get, yum, pacman, apk, dnf." \
                    "Install one of these, or add support in detect_package_manager()."
            fi
            ;;
        Darwin)
            if command -v brew >/dev/null 2>&1; then
                echo "brew"
            else
                die "Homebrew is required on macOS but was not found." \
                    "Install it from https://brew.sh/ then re-run this script."
            fi
            ;;
        *)
            die "Unsupported operating system: $(uname -s)." \
                "This script supports Linux and macOS only."
            ;;
    esac
}

# ============================================================================
# Package Name Resolution
#
# Maps a canonical (generic) package name to the distro-specific name(s).
# Some generics expand to multiple packages (space-separated).
# Returns empty string when the package is not needed on that platform.
#
# >>> TO ADD A NEW PACKAGE MANAGER: add a case branch for it under each
#     generic name that differs from the default. <<<
# ============================================================================

resolve_pkg_name() {
    _generic="$1"
    _pm="$2"

    case "$_generic" in

        # Compiler toolchain (gcc/g++/make or equivalent meta-package)
        build-essentials)
            case "$_pm" in
                apt-get)     echo "build-essential" ;;
                pacman)      echo "base-devel" ;;
                apk)         echo "alpine-sdk coreutils" ;;
                yum|dnf)     echo "gcc gcc-c++ make" ;;
                brew)        echo "" ;;
                *)           echo "" ;;
            esac
            ;;

        # OpenSSL development headers -- needed by Rust TLS crates (rustls, openssl-sys)
        openssl-dev)
            case "$_pm" in
                apt-get)     echo "libssl-dev" ;;
                apk)         echo "openssl-dev" ;;
                yum|dnf)     echo "openssl-devel" ;;
                pacman|brew) echo "openssl" ;;
                *)           echo "" ;;
            esac
            ;;

        # pkg-config -- lets build systems locate installed libraries
        pkg-config)
            case "$_pm" in
                apt-get|dnf) echo "pkg-config" ;;
                pacman)      echo "pkgconf" ;;
                brew|apk|yum) echo "pkgconfig" ;;
                *)           echo "pkg-config" ;;
            esac
            ;;

        # PostgreSQL client development libraries (libpq)
        postgres-dev)
            case "$_pm" in
                apt-get|apk) echo "libpq-dev" ;;
                pacman|yum)  echo "postgresql-libs" ;;
                dnf)         echo "libpq-devel" ;;
                brew)        echo "postgresql" ;;
                *)           echo "" ;;
            esac
            ;;

        # libdw -- DWARF debug info (used for backtraces / profiling on Linux)
        libdw-dev)
            case "$_pm" in
                pacman)      echo "libelf" ;;
                brew)        echo "" ;;
                *)           echo "libdw-dev" ;;
            esac
            ;;

        # libudev -- device event library (needed by hidapi crate for Aptos Ledger)
        libudev-dev)
            case "$_pm" in
                pacman|brew) echo "" ;;
                *)           echo "libudev-dev" ;;
            esac
            ;;

        # Python 3 interpreter and dev headers
        python3)
            case "$_pm" in
                apt-get) echo "python3-all-dev python3-setuptools python3-pip" ;;
                apk)     echo "python3-dev" ;;
                *)       echo "python3" ;;
            esac
            ;;

        # pre-commit hook manager
        pre-commit)
            case "$_pm" in
                brew)   echo "pre-commit" ;;
                pacman) echo "python-pre-commit" ;;
                *)      echo "" ;;
            esac
            ;;

        # tidy -- HTML validator / pretty-printer
        tidy)
            case "$_pm" in
                apk) echo "tidyhtml" ;;
                *)   echo "tidy" ;;
            esac
            ;;

        # xsltproc -- XSLT command-line processor
        xsltproc)
            case "$_pm" in
                apt-get) echo "xsltproc" ;;
                *)       echo "libxslt" ;;
            esac
            ;;

        # Fallback: use the generic name as-is
        *)
            echo "$_generic"
            ;;
    esac
}

# ============================================================================
# Package Installation
#
# install_pkg  <exact-name> <pm>   -- install a single distro-specific package
# install_generic <canonical-name>  -- resolve name then install
#
# >>> TO ADD A NEW PACKAGE MANAGER: add a case branch in install_pkg(). <<<
# ============================================================================

install_pkg() {
    _pkg="$1"
    _pm="$2"

    if [ -z "$_pkg" ]; then
        return 0
    fi

    if command -v "$_pkg" >/dev/null 2>&1; then
        log_info "$_pkg is already installed"
        return 0
    fi

    log_info "Installing $_pkg via $_pm"
    _sudo="$(sudo_if_needed)"

    case "$_pm" in
        apt-get)
            # shellcheck disable=SC2086
            $_sudo apt-get install "$_pkg" --no-install-recommends -y || {
                log_error "apt-get failed to install '$_pkg'."
                log_error "Hint: run 'sudo apt-get update' and retry, or check the package name."
                return 1
            }
            ;;
        yum)
            # shellcheck disable=SC2086
            $_sudo yum install "$_pkg" -y || {
                log_error "yum failed to install '$_pkg'."
                log_error "Hint: run 'sudo yum makecache' and retry."
                return 1
            }
            ;;
        dnf)
            # shellcheck disable=SC2086
            $_sudo dnf install "$_pkg" -y || {
                log_error "dnf failed to install '$_pkg'."
                log_error "Hint: run 'sudo dnf makecache' and retry."
                return 1
            }
            ;;
        pacman)
            # shellcheck disable=SC2086
            $_sudo pacman -Syu "$_pkg" --noconfirm || {
                log_error "pacman failed to install '$_pkg'."
                log_error "Hint: run 'sudo pacman -Sy' and retry."
                return 1
            }
            ;;
        apk)
            # shellcheck disable=SC2086
            $_sudo apk --update add --no-cache "$_pkg" || {
                log_error "apk failed to install '$_pkg'."
                log_error "Hint: run 'apk update' and retry."
                return 1
            }
            ;;
        brew)
            brew install "$_pkg" || {
                log_error "brew failed to install '$_pkg'."
                log_error "Hint: run 'brew update' and retry."
                return 1
            }
            ;;
        *)
            die "Unknown package manager '$_pm'." \
                "Cannot install '$_pkg'." \
                "Add support for '$_pm' in install_pkg()."
            ;;
    esac
}

# Resolve a canonical name to distro-specific name(s) and install each.
install_generic() {
    _generic="$1"
    _pm="${2:-$PACKAGE_MANAGER}"
    _resolved="$(resolve_pkg_name "$_generic" "$_pm")"

    if [ -z "$_resolved" ]; then
        log_info "Package '$_generic' is not required on $_pm -- skipping"
        return 0
    fi

    for _p in $_resolved; do
        install_pkg "$_p" "$_pm"
    done
}

# ============================================================================
# Clang / LLVM Installer
#
# apt-get needs the official LLVM repository for recent versions;
# all other PMs install from their default repos.
# ============================================================================

install_clang() {
    _pm="$1"
    _version="${2:-$CLANG_VERSION}"

    if [ "$_pm" = "apt-get" ]; then
        log_step "Installing Clang $_version from the LLVM apt repository"
        _sudo="$(sudo_if_needed)"
        # shellcheck disable=SC2086
        $_sudo apt-get install -y gnupg lsb-release software-properties-common wget || \
            die "Failed to install prerequisites for the LLVM apt repository." \
                "Ensure apt sources are configured and network is available."
        # The upstream LLVM install script uses bash-specific syntax
        # shellcheck disable=SC2086
        $_sudo bash -c "$(wget -O - https://apt.llvm.org/llvm.sh)" llvm.sh "$_version" || \
            die "Failed to run the LLVM apt setup script for Clang $_version." \
                "Check https://apt.llvm.org/ for supported distro/version combinations."
        # shellcheck disable=SC2086
        $_sudo update-alternatives --install /usr/bin/clang clang "/usr/bin/clang-${_version}" 100
        # shellcheck disable=SC2086
        $_sudo update-alternatives --install /usr/bin/clang++ clang++ "/usr/bin/clang++-${_version}" 100
    else
        install_pkg clang "$_pm"
        install_pkg llvm "$_pm"
    fi
}

# ============================================================================
# Rust Toolchain
# ============================================================================

install_rustup() {
    log_step "Installing Rust via rustup"
    _batch="$1"

    if [ "$OPT_DIR" = "true" ]; then
        export RUSTUP_HOME=/opt/rustup/
        mkdir -p "$RUSTUP_HOME" 2>/dev/null || true
        export CARGO_HOME=/opt/cargo/
        mkdir -p "$CARGO_HOME" 2>/dev/null || true
    fi

    if command -v rustup >/dev/null 2>&1; then
        _ver="$(rustup --version 2>/dev/null || echo "unknown")"
        log_info "Rustup already installed: $_ver"
    else
        log_info "Downloading rustup installer from https://sh.rustup.rs"
        curl https://sh.rustup.rs -sSf | sh -s -- -y --default-toolchain stable || \
            die "Failed to install rustup." \
                "Check your network connection and try again." \
                "Manual install: https://rustup.rs/"
        if [ -n "${CARGO_HOME}" ]; then
            PATH="${CARGO_HOME}/bin:${PATH}"
            export PATH
        else
            PATH="${HOME}/.cargo/bin:${PATH}"
            export PATH
        fi
    fi
}

install_toolchain() {
    _version="$1"
    if rustup show | grep -q "$_version" 2>/dev/null; then
        log_info "Rust toolchain '$_version' already installed"
    else
        log_info "Installing Rust toolchain: $_version"
        rustup install "$_version" || \
            die "Failed to install Rust toolchain '$_version'." \
                "Run 'rustup toolchain list' to see available toolchains."
    fi
}

install_rustup_components_and_nightly() {
    log_step "Updating Rust toolchains and installing components"

    # Update all installed toolchains
    rustup update || log_warn "rustup update reported warnings"

    # Ensure latest stable is available (even if repo pins an older channel)
    rustup toolchain install stable || \
        die "Failed to install the stable Rust toolchain."

    # rustfmt -- code formatter (used in CI lint checks)
    rustup component add rustfmt || \
        die "Failed to add the rustfmt component."

    # clippy -- Rust linter (used in CI lint checks)
    rustup component add clippy || \
        die "Failed to add the clippy component."

    # Nightly toolchain -- required for strict rustfmt formatting rules
    log_info "Installing nightly toolchain (needed for strict formatting)"
    if ! rustup toolchain install nightly; then
        if [ "$(uname -s)" = "Linux" ]; then
            log_warn "Nightly install failed; falling back to nightly-2023-06-01"
            rustup toolchain install nightly-2023-06-01 || \
                die "Failed to install fallback nightly toolchain (nightly-2023-06-01)." \
                    "This may indicate a corrupted rustup installation."
            # Rename to plain "nightly" (workaround for https://github.com/rust-lang/rustup/issues/1299)
            if [ -d "$HOME/.rustup/toolchains/nightly-2023-06-01-x86_64-unknown-linux-gnu" ]; then
                mv "$HOME/.rustup/toolchains/nightly-2023-06-01-x86_64-unknown-linux-gnu" \
                   "$HOME/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu"
            fi
        else
            log_error "Failed to install nightly toolchain. Manual installation required."
            log_error "Try: rustup toolchain install nightly"
        fi
    fi

    # rustfmt on nightly (best-effort; used by cargo +nightly fmt)
    rustup component add rustfmt --toolchain nightly 2>/dev/null || \
        log_warn "Could not add rustfmt to the nightly toolchain"

    log_info "Rust toolchain summary:"
    rustup --version
    rustup show
    rustup toolchain list -v
}

# ============================================================================
# Cargo Tool Installers
#
# Each cargo tool is version-pinned and installed with --locked to ensure
# reproducible dependency resolution.
# ============================================================================

# cargo-sort: keeps [dependencies] sections in Cargo.toml alphabetically sorted
install_cargo_sort() {
    if command -v cargo-sort >/dev/null 2>&1; then
        log_info "cargo-sort already installed"
    else
        log_info "Installing cargo-sort v${CARGO_SORT_VERSION} (sorts Cargo.toml dependency sections)"
        cargo install cargo-sort --locked --version "${CARGO_SORT_VERSION}" || \
            die "Failed to install cargo-sort." \
                "Ensure Rust is installed and cargo is in your PATH."
    fi
}

# cargo-machete: detects unused crate dependencies in Cargo.toml
install_cargo_machete() {
    if command -v cargo-machete >/dev/null 2>&1; then
        log_info "cargo-machete already installed"
    else
        log_info "Installing cargo-machete v${CARGO_MACHETE_VERSION} (detects unused crate dependencies)"
        cargo install cargo-machete --locked --version "${CARGO_MACHETE_VERSION}" || \
            die "Failed to install cargo-machete."
    fi
}

# cargo-nextest: faster, more ergonomic Rust test runner
install_cargo_nextest() {
    if command -v cargo-nextest >/dev/null 2>&1; then
        log_info "cargo-nextest already installed"
    else
        log_info "Installing cargo-nextest v${CARGO_NEXTEST_VERSION} (faster Rust test runner)"
        cargo install cargo-nextest --locked --version "${CARGO_NEXTEST_VERSION}" || \
            die "Failed to install cargo-nextest."
    fi
}

# grcov: collects and aggregates Rust code-coverage data
install_grcov() {
    if command -v grcov >/dev/null 2>&1; then
        log_info "grcov already installed"
    else
        log_info "Installing grcov v${GRCOV_VERSION} (Rust code coverage)"
        cargo install grcov --version="${GRCOV_VERSION}" --locked || \
            die "Failed to install grcov v${GRCOV_VERSION}."
    fi
}

# ============================================================================
# protoc -- Protocol Buffers Compiler
#
# Downloads a pre-built binary from the protobuf GitHub releases and also
# installs Rust codegen plugins via cargo.
# ============================================================================

install_protoc() {
    INSTALL_PROTOC_DONE="true"
    log_step "Installing protoc (Protocol Buffers compiler) and Rust plugins"

    _skip_protoc=""
    if command -v "${INSTALL_DIR}protoc" >/dev/null 2>&1; then
        _current="$("${INSTALL_DIR}protoc" --version 2>/dev/null || echo "")"
        case "$_current" in
            *"${PROTOC_VERSION}"*)
                log_info "protoc v${PROTOC_VERSION} already installed"
                _skip_protoc="true"
                ;;
        esac
    fi

    if [ "$_skip_protoc" != "true" ]; then
        case "$(uname -s)" in
            Linux)  _protoc_pkg="protoc-${PROTOC_VERSION}-linux-x86_64" ;;
            Darwin) _protoc_pkg="protoc-${PROTOC_VERSION}-osx-universal_binary" ;;
            *)
                log_warn "protoc: no pre-built binary for $(uname -s) -- skipping"
                return 0
                ;;
        esac

        _tmpdir="$(mktemp -d)"
        (
            cd "$_tmpdir" || exit 1
            _url="https://github.com/protocolbuffers/protobuf/releases/download/v${PROTOC_VERSION}/${_protoc_pkg}.zip"
            curl -LOs "$_url" --retry 3 || \
                die "Failed to download protoc from $_url." \
                    "Check network connectivity and GitHub availability."
            _sudo="$(sudo_if_needed)"
            # shellcheck disable=SC2086
            $_sudo unzip -o "${_protoc_pkg}.zip" -d /usr/local bin/protoc || \
                die "Failed to extract protoc binary."
            # shellcheck disable=SC2086
            $_sudo unzip -o "${_protoc_pkg}.zip" -d /usr/local 'include/*' || \
                die "Failed to extract protoc include files."
            # shellcheck disable=SC2086
            $_sudo chmod +x /usr/local/bin/protoc
        )
        rm -rf "$_tmpdir"
    fi

    # Rust codegen plugins for protobuf
    for _plugin in protoc-gen-prost protoc-gen-prost-serde protoc-gen-prost-crate; do
        if ! command -v "$_plugin" >/dev/null 2>&1; then
            log_info "Installing $_plugin (protobuf Rust codegen plugin)"
            cargo install "$_plugin" --locked || \
                log_warn "Failed to install $_plugin -- protobuf codegen may not work"
        else
            log_info "$_plugin already installed"
        fi
    done
}

# ============================================================================
# lcov -- Line-coverage report generation
# ============================================================================

install_lcov() {
    _pm="$1"
    log_info "Installing lcov (line-coverage HTML report generator)"

    case "$_pm" in
        apk)
            _sudo="$(sudo_if_needed)"
            # shellcheck disable=SC2086
            $_sudo apk --update add --no-cache \
                -X https://dl-cdn.alpinelinux.org/alpine/edge/testing lcov
            ;;
        apt-get|yum|dnf|brew)
            install_pkg lcov "$_pm"
            ;;
        pacman)
            log_warn "lcov is not available in the official pacman repositories."
            log_warn "To install manually from AUR:"
            log_warn "  git clone https://aur.archlinux.org/lcov.git"
            log_warn "  cd lcov && makepkg -si --noconfirm"
            ;;
        *)
            log_warn "No lcov installation method known for $_pm"
            ;;
    esac
}

# ============================================================================
# ShellCheck -- Static analysis for shell scripts
# ============================================================================

install_shellcheck() {
    if command -v shellcheck >/dev/null 2>&1; then
        log_info "shellcheck already installed"
        return 0
    fi

    log_info "Installing shellcheck v${SHELLCHECK_VERSION} (shell script linter)"

    if [ "$(uname -s)" = "Darwin" ]; then
        install_pkg shellcheck brew
        return 0
    fi

    install_generic xz
    _machine="$(uname -m)"
    _tmpdir="$(mktemp -d)"
    _url="https://github.com/koalaman/shellcheck/releases/download/v${SHELLCHECK_VERSION}/shellcheck-v${SHELLCHECK_VERSION}.linux.${_machine}.tar.xz"
    curl -sL -o "${_tmpdir}/out.xz" "$_url" || {
        rm -rf "$_tmpdir"
        die "Failed to download shellcheck from:" \
            "  $_url" \
            "Check that the URL is reachable and the architecture (${_machine}) is correct."
    }
    tar -xf "${_tmpdir}/out.xz" -C "${_tmpdir}/"
    cp "${_tmpdir}/shellcheck-v${SHELLCHECK_VERSION}/shellcheck" "${INSTALL_DIR}/shellcheck"
    chmod +x "${INSTALL_DIR}/shellcheck"
    rm -rf "$_tmpdir"
}

# ============================================================================
# HashiCorp Vault -- Secrets management
# ============================================================================

install_vault() {
    log_info "Installing Vault v${VAULT_VERSION} (secrets management)"

    _current="$("${INSTALL_DIR}/vault" --version 2>/dev/null || echo "")"
    if [ "$_current" = "Vault v${VAULT_VERSION}" ]; then
        log_info "Vault ${VAULT_VERSION} already installed"
        return 0
    fi

    _machine="$(uname -m)"
    case "$_machine" in x86_64) _machine="amd64" ;; esac
    _os="$(uname -s | tr '[:upper:]' '[:lower:]')"

    _tmpfile="$(mktemp)"
    curl -sL -o "$_tmpfile" \
        "https://releases.hashicorp.com/vault/${VAULT_VERSION}/vault_${VAULT_VERSION}_${_os}_${_machine}.zip" || {
        rm -f "$_tmpfile"
        die "Failed to download Vault ${VAULT_VERSION} for ${_os}/${_machine}."
    }
    unzip -qq -d "$INSTALL_DIR" "$_tmpfile"
    rm -f "$_tmpfile"
    chmod +x "${INSTALL_DIR}/vault"
    "${INSTALL_DIR}/vault" --version
}

# ============================================================================
# Helm -- Kubernetes package manager
# ============================================================================

install_helm() {
    if command -v helm >/dev/null 2>&1; then
        log_info "helm already installed"
        return 0
    fi

    log_info "Installing Helm v${HELM_VERSION} (Kubernetes package manager)"

    if [ "$(uname -s)" = "Darwin" ]; then
        install_pkg helm brew
        return 0
    fi

    _machine="$(uname -m)"
    case "$_machine" in x86_64) _machine="amd64" ;; esac
    _os="$(uname -s | tr '[:upper:]' '[:lower:]')"

    _tmpdir="$(mktemp -d)"
    curl -sL -o "${_tmpdir}/helm.tar.gz" \
        "https://get.helm.sh/helm-v${HELM_VERSION}-${_os}-${_machine}.tar.gz" || {
        rm -rf "$_tmpdir"
        die "Failed to download Helm ${HELM_VERSION} for ${_os}/${_machine}."
    }
    tar -zxf "${_tmpdir}/helm.tar.gz" -C "${_tmpdir}/"
    cp "${_tmpdir}/${_os}-${_machine}/helm" "${INSTALL_DIR}/helm"
    chmod +x "${INSTALL_DIR}/helm"
    rm -rf "$_tmpdir"
}

# ============================================================================
# Terraform -- Infrastructure as code
# ============================================================================

install_terraform() {
    _current="$(terraform --version 2>/dev/null | head -n 1 || echo "")"
    if [ "$_current" = "Terraform v${TERRAFORM_VERSION}" ]; then
        log_info "Terraform ${TERRAFORM_VERSION} already installed"
        return 0
    fi

    log_info "Installing Terraform v${TERRAFORM_VERSION} (infrastructure as code)"

    if [ "$(uname -s)" = "Darwin" ]; then
        install_pkg tfenv brew
        tfenv install "${TERRAFORM_VERSION}"
        tfenv use "${TERRAFORM_VERSION}"
        return 0
    fi

    _machine="$(uname -m)"
    case "$_machine" in x86_64) _machine="amd64" ;; esac
    _os="$(uname -s | tr '[:upper:]' '[:lower:]')"

    _tmpfile="$(mktemp)"
    curl -sL -o "$_tmpfile" \
        "https://releases.hashicorp.com/terraform/${TERRAFORM_VERSION}/terraform_${TERRAFORM_VERSION}_${_os}_${_machine}.zip" || {
        rm -f "$_tmpfile"
        die "Failed to download Terraform ${TERRAFORM_VERSION} for ${_os}/${_machine}."
    }
    unzip -qq -d "${INSTALL_DIR}" "$_tmpfile"
    rm -f "$_tmpfile"
    chmod +x "${INSTALL_DIR}/terraform"
    terraform --version
}

# ============================================================================
# kubectl -- Kubernetes CLI
# ============================================================================

install_kubectl() {
    _current="$(kubectl version --client 2>/dev/null | head -n 1 || echo "")"
    case "$_current" in
        *"v${KUBECTL_VERSION}"*)
            log_info "kubectl ${KUBECTL_VERSION} already installed"
            return 0
            ;;
    esac

    log_info "Installing kubectl v${KUBECTL_VERSION} (Kubernetes CLI)"

    if [ "$(uname -s)" = "Darwin" ]; then
        install_pkg kubectl brew
        return 0
    fi

    _machine="$(uname -m)"
    case "$_machine" in x86_64) _machine="amd64" ;; esac
    _os="$(uname -s | tr '[:upper:]' '[:lower:]')"

    curl -sL -o "${INSTALL_DIR}/kubectl" \
        "https://dl.k8s.io/release/v${KUBECTL_VERSION}/bin/${_os}/${_machine}/kubectl" || \
        die "Failed to download kubectl ${KUBECTL_VERSION} for ${_os}/${_machine}."
    chmod +x "${INSTALL_DIR}/kubectl"
    kubectl version --client 2>/dev/null | head -n 1 || true
}

# ============================================================================
# AWS CLI
# ============================================================================

install_awscli() {
    if command -v aws >/dev/null 2>&1; then
        log_info "AWS CLI already installed"
        return 0
    fi

    log_info "Installing AWS CLI"

    if [ "$(uname -s)" = "Darwin" ]; then
        install_pkg awscli brew
        return 0
    fi

    if [ "$PACKAGE_MANAGER" = "apk" ]; then
        apk add --no-cache python3 py3-pip
        pip3 install --upgrade pip
        pip3 install awscli
        return 0
    fi

    _machine="$(uname -m)"
    _os="$(uname -s | tr '[:upper:]' '[:lower:]')"

    _tmpdir="$(mktemp -d)"
    curl -sL -o "${_tmpdir}/aws.zip" \
        "https://awscli.amazonaws.com/awscli-exe-${_os}-${_machine}.zip" || {
        rm -rf "$_tmpdir"
        die "Failed to download AWS CLI for ${_os}/${_machine}."
    }
    unzip -qq -d "${_tmpdir}" "${_tmpdir}/aws.zip"

    _target_dir="${HOME}/.local/"
    if [ "$OPT_DIR" = "true" ]; then
        _target_dir="/opt/aws/"
    fi
    mkdir -p "$_target_dir"

    "${_tmpdir}/aws/install" -i "$_target_dir" -b "$INSTALL_DIR" || \
        die "AWS CLI installer failed." \
            "Target dir: $_target_dir, bin dir: $INSTALL_DIR"
    "${INSTALL_DIR}aws" --version
    rm -rf "$_tmpdir"
}

# ============================================================================
# s5cmd -- High-performance S3 file manager
# ============================================================================

install_s5cmd() {
    if command -v s5cmd >/dev/null 2>&1; then
        log_info "s5cmd already installed (remove it first to reinstall)"
        return 0
    fi

    log_info "Installing s5cmd v${S5CMD_VERSION} (fast S3 file manager)"

    if [ "$(uname -s)" = "Darwin" ]; then
        install_pkg peak/tap/s5cmd brew
        return 0
    fi

    if [ "$(uname -s)" = "Linux" ]; then
        _machine="$(uname -m | tr '[:upper:]' '[:lower:]')"
        _suffix=""
        case "$_machine" in
            x86_64)                           _suffix="64bit" ;;
            i386|i686)                        _suffix="32bit" ;;
            aarch64_be|aarch64|armv8b|armv8l) _suffix="arm64" ;;
            arm)                              _suffix="armv6" ;;
        esac

        if [ -n "$_suffix" ]; then
            _tmpdir="$(mktemp -d)"
            curl -sL -o "${_tmpdir}/s5cmd.tar.gz" \
                "https://github.com/peak/s5cmd/releases/download/v${S5CMD_VERSION}/s5cmd_${S5CMD_VERSION}_Linux-${_suffix}.tar.gz" || {
                rm -rf "$_tmpdir"
                die "Failed to download s5cmd v${S5CMD_VERSION} for Linux/${_suffix}."
            }
            tar -C "$_tmpdir" -xzf "${_tmpdir}/s5cmd.tar.gz"
            mv "${_tmpdir}/s5cmd" "${INSTALL_DIR}/"
            rm -rf "$_tmpdir"
            "${INSTALL_DIR}s5cmd" version
            return 0
        fi
    fi

    log_warn "s5cmd: no pre-built binary for $(uname -s)/$(uname -m) -- skipping"
}

# ============================================================================
# Allure -- Test reporting framework
# ============================================================================

install_allure() {
    _current="$(allure --version 2>/dev/null || echo "")"
    if [ "$_current" = "${ALLURE_VERSION}" ]; then
        log_info "Allure ${ALLURE_VERSION} already installed"
        return 0
    fi

    log_info "Installing Allure v${ALLURE_VERSION} (test reporting)"
    _sudo="$(sudo_if_needed)"

    if [ "$PACKAGE_MANAGER" = "apt-get" ]; then
        # shellcheck disable=SC2086
        $_sudo apt-get install default-jre -y --no-install-recommends
        _deb="${HOME}/allure_${ALLURE_VERSION}-1_all.deb"
        curl -sL -o "$_deb" \
            "https://github.com/diem/allure2/releases/download/${ALLURE_VERSION}/allure_${ALLURE_VERSION}-1_all.deb" || \
            die "Failed to download Allure .deb package."
        # shellcheck disable=SC2086
        $_sudo dpkg -i "$_deb"
        rm -f "$_deb"
    elif [ "$PACKAGE_MANAGER" = "apk" ]; then
        apk --update add --no-cache \
            -X https://dl-cdn.alpinelinux.org/alpine/edge/community openjdk11
    else
        log_warn "No automated Allure install method for $PACKAGE_MANAGER."
        log_warn "Install Allure manually: https://docs.qameta.io/allure/#_installing_a_commandline"
    fi
}

# ============================================================================
# Move Prover Tools
# ============================================================================

# Z3 -- SMT solver (primary backend for the Move Prover)
install_z3() {
    log_step "Installing Z3 v${Z3_VERSION} (SMT solver for Move Prover)"

    if command -v /usr/local/bin/z3 >/dev/null 2>&1; then
        log_warn "z3 already exists at /usr/local/bin/z3"
        log_warn "This install targets ${INSTALL_DIR}z3 -- consider removing the system copy."
    fi

    if command -v "${INSTALL_DIR}z3" >/dev/null 2>&1; then
        _current="$("${INSTALL_DIR}z3" --version 2>/dev/null || echo "")"
        case "$_current" in
            *"${Z3_VERSION}"*)
                log_info "Z3 ${Z3_VERSION} already installed"
                return 0
                ;;
        esac
    fi

    case "$(uname -s)" in
        Linux)  _z3_pkg="z3-${Z3_VERSION}-x64-glibc-2.31" ;;
        Darwin)
            case "$(uname -m)" in
                arm64) _z3_pkg="z3-${Z3_VERSION}-arm64-osx-11.0" ;;
                *)     _z3_pkg="z3-${Z3_VERSION}-x64-osx-10.16" ;;
            esac
            ;;
        *)
            log_warn "Z3: no pre-built binary for $(uname -s) -- skipping"
            return 0
            ;;
    esac

    _tmpdir="$(mktemp -d)"
    (
        cd "$_tmpdir" || exit 1
        curl -LOs "https://github.com/Z3Prover/z3/releases/download/z3-${Z3_VERSION}/${_z3_pkg}.zip" || \
            die "Failed to download Z3 ${Z3_VERSION}." \
                "URL: https://github.com/Z3Prover/z3/releases/download/z3-${Z3_VERSION}/${_z3_pkg}.zip"
        unzip -q "${_z3_pkg}.zip"
        cp "${_z3_pkg}/bin/z3" "${INSTALL_DIR}"
        chmod +x "${INSTALL_DIR}z3"
    )
    rm -rf "$_tmpdir"
}

# cvc5 -- SMT solver (alternative Move Prover backend)
install_cvc5() {
    log_step "Installing cvc5 v${CVC5_VERSION} (SMT solver for Move Prover)"

    if command -v /usr/local/bin/cvc5 >/dev/null 2>&1; then
        log_warn "cvc5 already exists at /usr/local/bin/cvc5"
        log_warn "This install targets ${INSTALL_DIR}cvc5 -- consider removing the system copy."
    fi

    if command -v "${INSTALL_DIR}cvc5" >/dev/null 2>&1; then
        _current="$("${INSTALL_DIR}cvc5" --version 2>/dev/null || echo "")"
        case "$_current" in
            *"${CVC5_VERSION}"*)
                log_info "cvc5 ${CVC5_VERSION} already installed"
                return 0
                ;;
        esac
    fi

    case "$(uname -s)" in
        Linux)  _cvc5_pkg="cvc5-Linux" ;;
        Darwin) _cvc5_pkg="cvc5-macOS" ;;
        *)
            log_warn "cvc5: no pre-built binary for $(uname -s) -- skipping"
            return 0
            ;;
    esac

    _tmpdir="$(mktemp -d)"
    (
        cd "$_tmpdir" || exit 1
        curl -LOs "https://github.com/cvc5/cvc5/releases/download/cvc5-${CVC5_VERSION}/${_cvc5_pkg}" || {
            log_warn "Failed to download cvc5 -- skipping"
            exit 0
        }
        cp "$_cvc5_pkg" "${INSTALL_DIR}cvc5" || true
        chmod +x "${INSTALL_DIR}cvc5" || true
    )
    rm -rf "$_tmpdir"
}

# .NET SDK -- runtime for the Boogie verifier
install_dotnet() {
    log_step "Installing .NET SDK ${DOTNET_VERSION} (runtime for Boogie verifier)"
    mkdir -p "${DOTNET_INSTALL_DIR}" 2>/dev/null || true

    if [ -x "${DOTNET_INSTALL_DIR}/dotnet" ]; then
        _count="$("${DOTNET_INSTALL_DIR}/dotnet" --list-sdks 2>/dev/null | grep -c "^${DOTNET_VERSION}" || echo "0")"
        if [ "$_count" != "0" ]; then
            log_info ".NET ${DOTNET_VERSION} already installed"
            return 0
        fi
    fi

    # Prerequisites vary by distro
    if [ "$(uname -s)" = "Linux" ]; then
        case "$PACKAGE_MANAGER" in
            apk)
                for _p in icu zlib libintl libcurl; do
                    install_pkg "$_p" "$PACKAGE_MANAGER"
                done
                ;;
            apt-get)
                for _p in gettext zlib1g; do
                    install_pkg "$_p" "$PACKAGE_MANAGER"
                done
                ;;
            yum|dnf|pacman)
                for _p in icu zlib; do
                    install_pkg "$_p" "$PACKAGE_MANAGER"
                done
                ;;
        esac
    fi

    wget --tries 10 --retry-connrefused --waitretry=5 \
        https://dot.net/v1/dotnet-install.sh -O dotnet-install.sh || \
        die "Failed to download .NET install script." \
            "URL: https://dot.net/v1/dotnet-install.sh"
    chmod +x dotnet-install.sh
    ./dotnet-install.sh --channel "$DOTNET_VERSION" --install-dir "${DOTNET_INSTALL_DIR}" --version latest || \
        die "Failed to install .NET SDK ${DOTNET_VERSION}." \
            "Check the .NET install log above for details."
    rm -f dotnet-install.sh
}

# Boogie -- intermediate verification language for the Move Prover
install_boogie() {
    log_step "Installing Boogie v${BOOGIE_VERSION} (verification language)"
    mkdir -p "${DOTNET_INSTALL_DIR}tools/" 2>/dev/null || true

    if [ -x "${DOTNET_INSTALL_DIR}dotnet" ]; then
        _installed="$("${DOTNET_INSTALL_DIR}dotnet" tool list --tool-path "${DOTNET_INSTALL_DIR}tools/" 2>/dev/null || echo "")"
        case "$_installed" in
            *"boogie"*"${BOOGIE_VERSION}"*)
                log_info "Boogie ${BOOGIE_VERSION} already installed"
                return 0
                ;;
        esac
    fi

    "${DOTNET_INSTALL_DIR}dotnet" tool update --tool-path "${DOTNET_INSTALL_DIR}tools/" \
        Boogie --version "$BOOGIE_VERSION" || \
        die "Failed to install Boogie ${BOOGIE_VERSION}." \
            "Ensure .NET SDK is installed at ${DOTNET_INSTALL_DIR}."
}

# ============================================================================
# Node.js / JavaScript / TypeScript Tools
# ============================================================================

install_nodejs() {
    log_step "Installing Node.js v${NODE_MAJOR_VERSION}"
    _sudo="$(sudo_if_needed)"

    if [ "$PACKAGE_MANAGER" = "apt-get" ]; then
        curl -fsSL "https://deb.nodesource.com/setup_${NODE_MAJOR_VERSION}.x" -o nodesource_setup.sh || \
            die "Failed to download NodeSource setup script for Node.js v${NODE_MAJOR_VERSION}."
        if [ -n "$_sudo" ]; then
            # shellcheck disable=SC2086
            $_sudo -E sh nodesource_setup.sh
        else
            sh nodesource_setup.sh
        fi
        rm -f nodesource_setup.sh
    fi

    install_pkg nodejs "$PACKAGE_MANAGER"
    install_pkg npm "$PACKAGE_MANAGER"
}

# pnpm -- fast, disk-efficient Node.js package manager
install_pnpm() {
    log_info "Installing pnpm v${PNPM_VERSION} (Node.js package manager)"
    _shell_path="$(command -v sh)"
    curl -fsSL https://get.pnpm.io/install.sh | PNPM_VERSION="${PNPM_VERSION}" SHELL="$_shell_path" sh - || \
        log_warn "pnpm installation had issues -- you may need to install it manually"
}

# ============================================================================
# Profile / PATH Management
# ============================================================================

add_to_profile() {
    _line="$1"
    eval "$_line"
    _found="$(grep -c "$_line" "${HOME}/.profile" 2>/dev/null || echo "0")"
    if [ "$_found" = "0" ]; then
        echo "$_line" >> "${HOME}/.profile"
    fi
}

update_path_and_profile() {
    log_step "Updating ~/.profile with tool paths"
    touch "${HOME}/.profile"

    DOTNET_ROOT="$HOME/.dotnet"
    BIN_DIR="$HOME/bin"
    C_HOME="${HOME}/.cargo"
    if [ "$OPT_DIR" = "true" ]; then
        DOTNET_ROOT="/opt/dotnet"
        BIN_DIR="/opt/bin"
        C_HOME="/opt/cargo"
    fi

    mkdir -p "${BIN_DIR}"
    if [ -n "$CARGO_HOME" ]; then
        add_to_profile "export CARGO_HOME=\"${CARGO_HOME}\""
        add_to_profile "export PATH=\"${BIN_DIR}:${CARGO_HOME}/bin:\$PATH\""
    else
        add_to_profile "export PATH=\"${BIN_DIR}:${C_HOME}/bin:\$PATH\""
    fi

    if [ "$INSTALL_PROTOC_DONE" = "true" ]; then
        add_to_profile "export PATH=\$PATH:/usr/local/include"
    fi

    if [ "$INSTALL_PROVER" = "true" ]; then
        add_to_profile "export DOTNET_ROOT=\"${DOTNET_ROOT}\""
        add_to_profile "export PATH=\"${DOTNET_ROOT}/tools:\$PATH\""
        add_to_profile "export Z3_EXE=\"${BIN_DIR}/z3\""
        add_to_profile "export CVC5_EXE=\"${BIN_DIR}/cvc5\""
        add_to_profile "export BOOGIE_EXE=\"${DOTNET_ROOT}/tools/boogie\""
    fi
}

# ============================================================================
# Welcome Banner (shown in interactive mode)
# ============================================================================

welcome_message() {
    cat <<EOF
============================================================
  Aptos Core -- Development Environment Setup
============================================================

This script will install the dependencies needed to build,
test, and develop Aptos Core.

Selected components:
EOF

    if [ "$INSTALL_BUILD_TOOLS" = "true" ]; then
        echo "  [BUILD]    Rust, CMake, Clang, protoc, lld, cargo tools"
    fi
    if [ "$OPERATIONS" = "true" ]; then
        echo "  [OPS]      Helm, Terraform, kubectl, Vault, AWS CLI, etc."
    fi
    if [ "$INSTALL_PROVER" = "true" ]; then
        echo "  [PROVER]   Z3, cvc5, .NET SDK, Boogie"
    fi
    if [ "$INSTALL_DOC" = "true" ]; then
        echo "  [DOCS]     graphviz"
    fi
    if [ "$INSTALL_PROTOC_FLAG" = "true" ]; then
        echo "  [PROTOC]   Protocol Buffers compiler and plugins"
    fi
    if [ "$INSTALL_POSTGRES" = "true" ]; then
        echo "  [POSTGRES] PostgreSQL development libraries"
    fi
    if [ "$INSTALL_JSTS" = "true" ]; then
        echo "  [JS/TS]    Node.js, pnpm"
    fi
    if [ "$INSTALL_PROFILE" = "true" ]; then
        echo "  [PROFILE]  ~/.profile will be updated"
    fi

    cat <<EOF

Press Ctrl-C to abort, or answer below to continue.
============================================================
EOF
}

# ============================================================================
# Argument Parsing
# ============================================================================

# Defaults
BATCH_MODE=false
VERBOSE=false
INSTALL_BUILD_TOOLS=false
OPERATIONS=false
INSTALL_PROFILE=false
INSTALL_PROVER=false
INSTALL_DOC=false
INSTALL_PROTOC_FLAG=false
INSTALL_PROTOC_DONE=false
INSTALL_POSTGRES=false
INSTALL_JSTS=false
INSTALL_INDIVIDUAL=false
INSTALL_PACKAGES=""
OPT_DIR=false
SKIP_PRE_COMMIT=false

# Auto-enable verbose when stderr is not a terminal (CI / piped output)
if [ ! -t 2 ]; then
    VERBOSE=true
fi

# Handle --help before getopts (POSIX getopts does not support long options)
for _arg in "$@"; do
    case "$_arg" in
        --help) show_help; exit 0 ;;
    esac
done

while getopts "btoprvydaPJhi:nk" _opt; do
    case "$_opt" in
        b) BATCH_MODE=true ;;
        t) INSTALL_BUILD_TOOLS=true ;;
        o) OPERATIONS=true ;;
        p) INSTALL_PROFILE=true ;;
        r) INSTALL_PROTOC_FLAG=true ;;
        v) VERBOSE=true ;;
        y) INSTALL_PROVER=true ;;
        d) INSTALL_DOC=true ;;
        P) INSTALL_POSTGRES=true ;;
        J) INSTALL_JSTS=true ;;
        h) show_help; exit 0 ;;
        i)
            INSTALL_INDIVIDUAL=true
            INSTALL_PACKAGES="$INSTALL_PACKAGES $OPTARG"
            ;;
        n) OPT_DIR=true ;;
        k) SKIP_PRE_COMMIT=true ;;
        *)
            show_help
            exit 1
            ;;
    esac
done

if [ "$VERBOSE" = "true" ]; then
    set -x
fi

# Default: install build tools when no component flag is given
if [ "$INSTALL_BUILD_TOOLS" = "false" ] && \
   [ "$OPERATIONS" = "false" ] && \
   [ "$INSTALL_PROFILE" = "false" ] && \
   [ "$INSTALL_PROVER" = "false" ] && \
   [ "$INSTALL_DOC" = "false" ] && \
   [ "$INSTALL_POSTGRES" = "false" ] && \
   [ "$INSTALL_JSTS" = "false" ] && \
   [ "$INSTALL_INDIVIDUAL" = "false" ]; then
    INSTALL_BUILD_TOOLS=true
fi

# ============================================================================
# Pre-flight Checks
# ============================================================================

if [ ! -f rust-toolchain.toml ]; then
    die "Cannot find rust-toolchain.toml in $(pwd)." \
        "This script must be run from the aptos-core repository root." \
        "" \
        "Usage:  cd /path/to/aptos-core && ./scripts/setup_build.sh [OPTIONS]" \
        "Help:   ./scripts/setup_build.sh --help"
fi

# ============================================================================
# Compute Install Directory
# ============================================================================

INSTALL_DIR="${HOME}/bin/"
if [ "$OPT_DIR" = "true" ]; then
    INSTALL_DIR="/opt/bin/"
fi
mkdir -p "$INSTALL_DIR" 2>/dev/null || true

# ============================================================================
# Detect Package Manager
# ============================================================================

PACKAGE_MANAGER="$(detect_package_manager)"
log_info "Detected package manager: $PACKAGE_MANAGER"

# ============================================================================
# Interactive Confirmation
# ============================================================================

if [ "$BATCH_MODE" = "false" ]; then
    welcome_message
    printf "Proceed with installation? (y/N) > "
    read -r _input
    case "$_input" in
        y*|Y*) ;;
        *)
            echo "Exiting without changes."
            exit 0
            ;;
    esac
fi

# ============================================================================
# Bootstrap: refresh package index and install fundamental utilities
# ============================================================================

if [ "$PACKAGE_MANAGER" = "apt-get" ]; then
    log_step "Updating apt-get package index"
    _sudo="$(sudo_if_needed)"
    # shellcheck disable=SC2086
    $_sudo apt-get update || \
        die "apt-get update failed." \
            "Check /etc/apt/sources.list and network connectivity."
    install_pkg ca-certificates "$PACKAGE_MANAGER"
fi

# These three are needed by many installers below
install_pkg curl "$PACKAGE_MANAGER"
install_pkg unzip "$PACKAGE_MANAGER"
install_pkg wget "$PACKAGE_MANAGER"

# ============================================================================
# Profile updates (-p)
# ============================================================================

if [ "$INSTALL_PROFILE" = "true" ]; then
    update_path_and_profile
fi

# ============================================================================
# Build Tools (-t)
# ============================================================================

if [ "$INSTALL_BUILD_TOOLS" = "true" ]; then
    log_step "========== Installing build tools =========="

    # C/C++ compiler toolchain
    install_generic build-essentials
    # CMake: build-system generator for native dependencies
    install_pkg cmake "$PACKAGE_MANAGER"
    # Clang/LLVM: C/C++ compiler used by native Rust crate dependencies
    install_clang "$PACKAGE_MANAGER"

    # OpenSSL headers: required by Rust TLS/crypto crates
    install_generic openssl-dev
    # pkg-config: lets build.rs scripts find native libraries
    install_generic pkg-config

    # lld: LLVM linker -- dramatically speeds up Rust link times (Linux only)
    if [ "$(uname -s)" = "Linux" ]; then
        install_pkg lld "$PACKAGE_MANAGER"
    fi

    # libdw: DWARF debug-info library for profiling/backtraces (Linux only)
    if [ "$(uname -s)" = "Linux" ]; then
        install_generic libdw-dev
    fi

    # Rust toolchain
    install_rustup "$BATCH_MODE"

    # Install the exact channel pinned in rust-toolchain.toml
    _rust_channel="$(grep channel ./rust-toolchain.toml | sed 's/.*"\([^"]*\)".*/\1/')"
    if [ -z "$_rust_channel" ]; then
        die "Could not parse Rust channel from rust-toolchain.toml." \
            "Expected a line like: channel = \"1.XX.Y\""
    fi
    install_toolchain "$_rust_channel"
    install_rustup_components_and_nightly

    # Cargo developer tools
    install_cargo_sort
    install_cargo_machete
    install_cargo_nextest
    install_grcov

    # git: should be pre-installed, but ensure it's there
    install_pkg git "$PACKAGE_MANAGER"
    # lcov: line-coverage report generation
    install_lcov "$PACKAGE_MANAGER"
    # Protocol Buffers compiler + Rust codegen plugins
    install_protoc
fi

# ============================================================================
# Standalone protoc (-r, only when -t was not also given)
# ============================================================================

if [ "$INSTALL_PROTOC_FLAG" = "true" ] && [ "$INSTALL_BUILD_TOOLS" = "false" ]; then
    install_pkg unzip "$PACKAGE_MANAGER"
    install_protoc
fi

# ============================================================================
# Operations Tools (-o)
# ============================================================================

if [ "$OPERATIONS" = "true" ]; then
    log_step "========== Installing operations tools =========="

    # YAML linter for config file validation
    install_pkg yamllint "$PACKAGE_MANAGER"
    # Python 3 for scripts and tooling
    install_pkg python3 "$PACKAGE_MANAGER"
    # jq: lightweight JSON processor (used in shell scripts)
    install_pkg jq "$PACKAGE_MANAGER"
    # git: version control
    install_pkg git "$PACKAGE_MANAGER"
    # HTML validator / XSLT processor
    install_generic tidy
    install_generic xsltproc
    # coreutils: GNU timeout, etc. (apt-get only; other distros include it)
    if [ "$PACKAGE_MANAGER" = "apt-get" ]; then
        install_pkg coreutils "$PACKAGE_MANAGER"
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

# ============================================================================
# Individual Tool Install (-i)
# ============================================================================

if [ "$INSTALL_INDIVIDUAL" = "true" ]; then
    log_step "========== Installing individual tools =========="
    for _pkg in $INSTALL_PACKAGES; do
        if type "install_${_pkg}" >/dev/null 2>&1; then
            # Pass $PACKAGE_MANAGER so functions like install_clang and
            # install_lcov that expect it as $1 work correctly.
            "install_${_pkg}" "$PACKAGE_MANAGER"
        else
            install_pkg "$_pkg" "$PACKAGE_MANAGER"
        fi
    done
fi

# ============================================================================
# Move Prover Tools (-y)
# ============================================================================

if [ "$INSTALL_PROVER" = "true" ]; then
    log_step "========== Installing Move Prover tools =========="

    DOTNET_INSTALL_DIR="${HOME}/.dotnet/"
    if [ "$OPT_DIR" = "true" ]; then
        DOTNET_INSTALL_DIR="/opt/dotnet/"
        mkdir -p "$DOTNET_INSTALL_DIR" 2>/dev/null || true
    fi
    export DOTNET_INSTALL_DIR

    install_pkg unzip "$PACKAGE_MANAGER"
    install_z3
    install_cvc5
    install_dotnet
    install_boogie
fi

# ============================================================================
# Documentation Tools (-d)
# ============================================================================

if [ "$INSTALL_DOC" = "true" ]; then
    log_step "========== Installing documentation tools =========="
    # graphviz: renders dependency graphs and diagrams in generated docs
    install_pkg graphviz "$PACKAGE_MANAGER"
fi

# ============================================================================
# PostgreSQL (-P)
# ============================================================================

if [ "$INSTALL_POSTGRES" = "true" ]; then
    log_step "========== Installing PostgreSQL dev libraries =========="
    install_generic postgres-dev
fi

# ============================================================================
# JavaScript / TypeScript (-J)
# ============================================================================

if [ "$INSTALL_JSTS" = "true" ]; then
    log_step "========== Installing JavaScript/TypeScript tools =========="
    install_nodejs
    install_pnpm
fi

# ============================================================================
# Always-installed: libudev, Python 3, pre-commit
# ============================================================================

# libudev-dev: needed by the hidapi crate for Aptos Ledger support (Linux only)
if [ "$(uname -s)" = "Linux" ]; then
    install_generic libudev-dev
fi

# Python 3: required by various scripts and the pre-commit framework
install_generic python3

if [ "$SKIP_PRE_COMMIT" = "false" ]; then
    log_step "Setting up pre-commit hooks"
    _pre_commit_pkg="$(resolve_pkg_name pre-commit "$PACKAGE_MANAGER")"
    if [ -n "$_pre_commit_pkg" ]; then
        install_pkg "$_pre_commit_pkg" "$PACKAGE_MANAGER"
    else
        pip3 install pre-commit || \
            log_warn "Failed to install pre-commit via pip3." \
                     "You can install it manually: pip3 install pre-commit"
    fi

    if command -v pre-commit >/dev/null 2>&1; then
        pre-commit install || log_warn "pre-commit install failed"
    elif [ -x "${HOME}/.local/bin/pre-commit" ]; then
        "${HOME}/.local/bin/pre-commit" install || log_warn "pre-commit install failed"
    else
        log_warn "pre-commit not found in PATH after installation." \
                 "You may need to add ~/.local/bin to your PATH."
    fi
fi

# ============================================================================
# Done
# ============================================================================

if [ "$BATCH_MODE" = "false" ]; then
    cat <<EOF

============================================================
  Setup complete!
============================================================

You should now be able to build the project:

    cargo build

For a quick compile check (faster, no codegen):

    cargo check

To run tests for a specific package:

    cargo test -p <package>

For full lint checks:

    ./scripts/rust_lint.sh

============================================================
EOF
fi

exit 0
