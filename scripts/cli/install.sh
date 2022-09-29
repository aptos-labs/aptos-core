#!/bin/sh
set -eu

# aptos-cli's automatic install script.

usage() {
  arg0="$0"
  cath << EOF
Installs Aptos CLI.

Usage:

  $arg0 [--dry-run]

  --dry-run
      Echo the commands for the install process without running them.
EOF
}

echo_latest_version() {
  version="$(curl -fsSL https://api.github.com/repos/aptos-labs/aptos-core/releases | awk 'match($0,/.*"html_url": "(.*\/releases\/tag\/.*)".*/)' | head -n 1 | awk -F '"' '{print $4}')"
  version="${version#https://github.com/aptos-labs/aptos-core/releases/tag/}"
  version="${version#aptos-cli-v}"
  echo "$version"
}

echo_standalone_postinstall() {
  echoh
  cath << EOF
Standalone release has been installed into $STANDALONE_INSTALL_PREFIX/bin/aptos

Extend your path to use aptos cli:
  PATH="$STANDALONE_INSTALL_PREFIX/bin:\$PATH"
Then run with:
  aptos
EOF
}

main() {
  if [ "${TRACE-}" ]; then
    set -x
  fi

  unset \
    DRY_RUN

  while [ "$#" -gt 0 ]; do

    case "$1" in
      --dry-run)
        DRY_RUN=1
        ;;
      -h | --h | -help | --help)
        usage
        exit 0
        ;;
      -*)
        echoerr "Unknown flag $1"
        echoerr "Run with --help to see usage."
        exit 1
        ;;
    esac

    shift
  done

  # These are used by the various install_* functions that make use of GitHub
  # releases in order to download and unpack the right release.
  STANDALONE_INSTALL_PREFIX=${STANDALONE_INSTALL_PREFIX:-$HOME/.local}
  VERSION=${VERSION:-$(echo_latest_version)}
  OS=${OS:-$(os)}
  ARCH=${ARCH:-$(arch)}
  CACHE_DIR=$(echo_cache_dir)

  distro_name

  install_standalone
}


fetch() {
  URL="$1"
  FILE="$2"

  if [ -e "$FILE" ]; then
    echoh "+ Reusing $FILE"
    return
  fi

  sh_c mkdir -p "$CACHE_DIR"
  sh_c curl \
    -#fL \
    -o "$FILE.incomplete" \
    -C - \
    "$URL"
  sh_c mv "$FILE.incomplete" "$FILE"
}

install_standalone() {
  if [ ! has_standalone ]; then
    echoerr "No prebuilt CLI found for $OS-$ARCH"
    exit 1
  fi

  # Apple silicon
  if [ "$OS" = "macos" ] && [ "$ARCH" = "arm64" ]; then
    ARCH="x86_64"
  fi

  echoh "Installing v$VERSION of the $ARCH release from GitHub."
  echoh

  DISTRO=$(normalized_distro)

  preinstall

  fetch "https://github.com/aptos-labs/aptos-core/releases/download/aptos-cli-v$VERSION/aptos-cli-$VERSION-$DISTRO-$ARCH.zip" \
    "$CACHE_DIR/aptos-cli-$VERSION-$DISTRO-$ARCH.zip"

  # -w only works if the directory exists so try creating it first. If this
  # fails we can ignore the error as the -w check will then swap us to sudo.
  sh_c mkdir -p "$STANDALONE_INSTALL_PREFIX" 2> /dev/null || true

  sh_c="sh_c"
  if [ ! -w "$STANDALONE_INSTALL_PREFIX" ]; then
    sh_c="sudo_sh_c"
  fi

  if [ -e "$STANDALONE_INSTALL_PREFIX/lib/aptos-cli-$VERSION" ]; then
    echoh
    echoh "aptos-cli-$VERSION is already installed at $STANDALONE_INSTALL_PREFIX/lib/aptos-cli-$VERSION"
    echoh "Remove it to reinstall."
    exit 0
  fi

  "$sh_c" mkdir -p "$STANDALONE_INSTALL_PREFIX/bin"
  "$sh_c" unzip -d "$STANDALONE_INSTALL_PREFIX/bin" "$CACHE_DIR/aptos-cli-$VERSION-$DISTRO-$ARCH.zip"

  echo_standalone_postinstall
}

# Determine if we have standalone releases on GitHub for the system's arch.
has_standalone() {
  case $ARCH in
    amd64) return 0 ;;
    # We only have amd64 for macOS.
    arm64)
      [ "$(distro)" != macos ]
      return
      ;;
    *) return 1 ;;
  esac
}

preinstall() {
  distro="$(distro)"
  case $distro in
    debian)
      sudo_sh_c apt-get install unzip
      if ! command -v openssl &> /dev/null; then
        sudo_sh_c apt-get install libssl1.1
      fi
      ;;
  esac
}

normalized_distro() {
  distro="$(distro)"
  case $distro in
    macos) echo MacOSX ;;
    debian) echo Ubuntu ;;
    windows) echo Windows ;;
    *) echo "$distro" ;;
  esac
}

os() {
  uname="$(uname)"
  case $uname in
    Linux) echo linux ;;
    Darwin) echo macos ;;
    FreeBSD) echo freebsd ;;
    *) echo "$uname:-windows" ;;
  esac
}

# Print the detected Linux distro, otherwise print the OS name.
#
# Example outputs:
# - macos -> macos
# - ubuntu, ... -> debian
# - windows -> windows
#
# Inspired by https://github.com/docker/docker-install/blob/26ff363bcf3b3f5a00498ac43694bf1c7d9ce16c/install.sh#L111-L120.
distro() {
  if [ "$OS" = "macos" ] ; then
    echo "$OS"
    return
  fi

  if [ -f /etc/os-release ]; then
    (
      . /etc/os-release
      if [ "${ID_LIKE-}" ]; then
        for id_like in $ID_LIKE; do
          case "$id_like" in debian | fedora | opensuse)
            echo "$id_like"
            return
            ;;
          esac
        done
      fi

      echo "$ID"
    )
    return
  fi

  echo "windows"
}

# Print a human-readable name for the OS/distro.
distro_name() {
  if [ "$(uname)" = "Darwin" ]; then
    echo "macOS v$(sw_vers -productVersion)"
    return
  fi

  if [ -f /etc/os-release ]; then
    (
      . /etc/os-release
      echo "$PRETTY_NAME"
    )
    return
  fi

  if [ "$OS" = "windows" ] ; then
    echo "Windows"
  else
    # Prints something like: Linux 4.19.0-9-amd64
    uname -sr
  fi
}

arch() {
  uname_m=$(uname -m)
  case $uname_m in
    aarch64) echo arm64 ;;
    *) echo "$uname_m" ;;
  esac
}

command_exists() {
  if [ ! "$1" ]; then return 1; fi
  command -v "$@" > /dev/null
}

sh_c() {
  echoh "+ $*"
  if [ ! "${DRY_RUN-}" ]; then
    sh -c "$*"
  fi
}

sudo_sh_c() {
  if [ "$(id -u)" = 0 ]; then
    sh_c "$@"
  elif command_exists sudo; then
    sh_c "sudo $*"
  elif command_exists su; then
    sh_c "su root -c '$*'"
  else
    echoh
    echoerr "This script needs to run the following command as root."
    echoerr "  $*"
    echoerr "Please install sudo or su."
    exit 1
  fi
}

echo_cache_dir() {
  if [ "${XDG_CACHE_HOME-}" ]; then
    echo "$XDG_CACHE_HOME/aptos-cli"
  elif [ "${HOME-}" ]; then
    echo "$HOME/.cache/aptos-cli"
  else
    echo "/tmp/aptos-cli"
  fi
}

echoh() {
  echo "$@" | humanpath
}

cath() {
  humanpath
}

echoerr() {
  echoh "$@" >&2
}

# humanpath replaces all occurrences of " $HOME" with " ~"
# and all occurrences of '"$HOME' with the literal '"$HOME'.
humanpath() {
  sed "s# $HOME# ~#g; s#\"$HOME#\"\$HOME#g"
}

main "$@"