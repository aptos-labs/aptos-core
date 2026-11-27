#!/bin/bash
# Fails the AMI build when a tool CI jobs rely on is missing for the runner user.
set -euo pipefail

# Check with the environment jobs get, not a hand-built PATH.
set -a
# shellcheck disable=SC1091
. /etc/environment
set +a

missing=()
for tool in git curl jq zip unzip gh aws gcloud kubectl docker python python3 pip3 \
  rustc cargo cargo-nextest protoc clang cmake pkg-config node boogie z3 cvc5; do
  if command -v "$tool" >/dev/null; then
    echo "ok: $tool -> $(command -v "$tool")"
  else
    missing+=("$tool")
  fi
done

if ! systemctl show rolaunch -p ExecStart | grep -q -- "-timeout 24h"; then
  missing+=("rolaunch job timeout")
fi
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
aws --version
rustc --version
cargo nextest --version
protoc --version
