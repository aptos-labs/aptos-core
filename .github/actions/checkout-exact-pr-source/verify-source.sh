#!/usr/bin/env bash

set -euo pipefail

mode="${1:-}"
source_sha="${2:-}"

if [[ ! "$source_sha" =~ ^[0-9a-f]{40}$ ]]; then
  echo "::error::SOURCE_SHA must be a full lowercase commit SHA" >&2
  exit 1
fi

case "$mode" in
  validate)
    ;;
  verify)
    actual_sha="$(git rev-parse HEAD)"
    if [[ "$actual_sha" != "$source_sha" ]]; then
      echo "::error::Checked-out HEAD does not match SOURCE_SHA" >&2
      exit 1
    fi
    git checkout -B "ci-pr-${GITHUB_RUN_ID:?GITHUB_RUN_ID is required}" "$source_sha"
    ;;
  *)
    echo "::error::Expected validate or verify mode" >&2
    exit 1
    ;;
esac
