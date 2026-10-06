#!/usr/bin/env bash
# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0

image_tag_prefix() {
  local pr_number="${1-}"
  local profile="${2-release}"
  local features="${3-}"
  local prefix=""
  local normalized_features

  if [[ -n "$pr_number" ]]; then
    prefix="pr-${pr_number}_"
  fi
  if [[ "$profile" != "release" ]]; then
    prefix="${prefix}${profile}_"
  fi
  if [[ -n "$features" ]]; then
    normalized_features="$(printf '%s' "$features" | sed -e 's/[^a-zA-Z0-9]/_/g')"
    prefix="${prefix}${normalized_features}_"
  fi

  printf '%s' "$prefix"
}
