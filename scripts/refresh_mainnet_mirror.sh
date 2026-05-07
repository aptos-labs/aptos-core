#!/usr/bin/env bash
# Copyright (c) Aptos Foundation
# SPDX-License-Identifier: Apache-2.0
#
# One-button refresh for the mainnet-mirror data pipeline:
#   1. failure_metrics.json   (Prometheus → fp_7d_avg / fp_30d_max per peer_id)
#   2. stake_distribution.json (mainnet REST API → ValidatorSet voting power)
#   3. mainnet_validator_snapshot.json (enriched output: stake + location + class)
#
# Both Python scripts can also be run individually if you only need one piece.
#
# Required env vars:
#   GRAFANA_TOKEN  — service-account token for the Aptos Labs Grafana
# Optional:
#   IPINFO_TOKEN   — ipinfo.io token (avoids the unauthenticated rate limit
#                    of ~50 lookups/min during the geolocation step)
#
# Usage:
#   GRAFANA_TOKEN=... ./scripts/refresh_mainnet_mirror.sh

set -euo pipefail

if [[ -z "${GRAFANA_TOKEN:-}" ]]; then
    echo "error: GRAFANA_TOKEN env var required" >&2
    echo "       (used to query mainnet failure metrics from Prometheus)" >&2
    exit 1
fi

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$REPO_ROOT"

echo "==> [1/2] Refreshing failure_metrics.json from live Prometheus"
python3 scripts/pull_mainnet_failure_metrics.py
echo

echo "==> [2/2] Refreshing stake_distribution.json + mainnet_validator_snapshot.json"
echo "         (live REST API for stake; ipinfo.io for geolocation)"
python3 scripts/pull_mainnet_validator_snapshot.py --from-rest-api --refresh-input
echo

echo "==> Done. Updated files:"
echo "    testsuite/testcases/src/data/mainnet-mirror/failure_metrics.json"
echo "    testsuite/testcases/src/data/mainnet-mirror/stake_distribution.json"
echo "    testsuite/testcases/src/data/mainnet-mirror/mainnet_validator_snapshot.json"
echo
echo "Review with: git diff testsuite/testcases/src/data/mainnet-mirror/"
