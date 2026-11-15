#!/usr/bin/env bash
set -euo pipefail
# Cache the selected compiler, not unrelated toolchains installed in the image.
if [[ "$*" == 'rustup toolchain list --quiet' ]]; then
  exit 0
fi
exec "$@"
