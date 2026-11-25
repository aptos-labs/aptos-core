#!/bin/bash
# Fails the AMI build when a tool CI jobs rely on is missing for the runner user.
set -euo pipefail

export PATH="${HOME}/bin:${HOME}/.cargo/bin:/usr/local/bin:${PATH}"

missing=()
for tool in git curl jq zip unzip gh gcloud kubectl docker python python3 pip3 \
  rustc cargo cargo-nextest protoc clang cmake pkg-config node; do
  if command -v "$tool" >/dev/null; then
    echo "ok: $tool -> $(command -v "$tool")"
  else
    missing+=("$tool")
  fi
done

if ! pkg-config --exists libpq; then
  missing+=("libpq")
fi
if ! id -nG runner | tr ' ' '\n' | grep -qx docker; then
  missing+=("docker group membership")
fi
if ! sudo -n true; then
  missing+=("passwordless sudo")
fi
if ! pip3 config list | grep -q "break-system-packages='true'"; then
  missing+=("system pip installs")
fi

if ((${#missing[@]})); then
  echo "missing for runner: ${missing[*]}" >&2
  exit 1
fi
rustc --version
cargo nextest --version
protoc --version
