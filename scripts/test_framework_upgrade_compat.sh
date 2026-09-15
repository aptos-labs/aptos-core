#!/usr/bin/env bash
#
# Framework upgrade compatibility test.
#
# Asserts that a node running the PREVIOUS release's binary can execute the
# framework in THIS checkout. On mainnet, validators take a new binary first and
# the framework upgrade follows; fullnodes lag behind both, so when the upgrade
# lands a fullnode still on the old binary must execute it or it stops there.
# This reproduces that on a single node, without consensus or a Forge cluster.
#
#   old CLI  -> runs the localnet, i.e. supplies the VM under test
#   this repo -> supplies the framework, built into a governance bundle
#
# The upgrade goes through governance rather than a direct publish, and that is
# not ceremony: a normal transaction is capped at max_transaction_size_in_bytes
# (64 KiB), and only a script whose hash is in the on-chain ApprovedExecutionHashes
# gets max_transaction_size_in_bytes.gov (1 MiB). The aptos-framework package is
# far past 64 KiB, so nothing but an approved governance script can carry it.
# See aptos-move/aptos-vm/src/transaction_metadata.rs:66 and
# aptos-move/aptos-gas-schedule/src/gas_schedule/transaction.rs:77.
#
# Usage:
#   scripts/test_framework_upgrade_compat.sh --old-cli <path/to/aptos>
#                                            [--release-tool <path>]
#                                            [--control]
#
#   --control  rerun with the new binary as the node, as a positive control.
#              Old fails + control passes => a compatibility problem.
#              Both fail => the framework or this harness is broken instead.
#
# Note: the aptos CLI reads and writes ~/.aptos, and the localnet binds fixed
# ports (8080 API, 50051 stream), so only one run at a time on a host.

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

ROOT_ACCOUNT="0xa550c18"
API_URL="http://127.0.0.1:8080"
SEED="0000000000000000000000000000000000000000000000000000000000000001"

OLD_CLI=""
NEW_CLI="$(command -v aptos || true)"
RELEASE_TOOL="${REPO_ROOT}/target/debug/aptos-release-tool"
RUN_CONTROL=0

while [[ $# -gt 0 ]]; do
  case "$1" in
    --old-cli) OLD_CLI="$2"; shift 2 ;;
    --new-cli) NEW_CLI="$2"; shift 2 ;;
    --release-tool) RELEASE_TOOL="$2"; shift 2 ;;
    --control) RUN_CONTROL=1; shift ;;
    -h|--help) sed -n '2,32p' "${BASH_SOURCE[0]}"; exit 0 ;;
    *) echo "unknown argument: $1" >&2; exit 2 ;;
  esac
done

[[ -x "${OLD_CLI}" ]]      || { echo "--old-cli must be an executable aptos CLI" >&2; exit 2; }
[[ -x "${RELEASE_TOOL}" ]] || { echo "aptos-release-tool not found at ${RELEASE_TOOL}; build it with 'cargo build -p aptos-release-tool'" >&2; exit 2; }
# Only --control needs a second binary: it reruns the whole thing with the new
# CLI as the node. Everywhere else the CLI is just a client submitting to the
# node under test, so the node's own CLI is used.
if [[ "${RUN_CONTROL}" -eq 1 && ! -x "${NEW_CLI}" ]]; then
  echo "--control needs --new-cli to point at an executable aptos CLI (or one on PATH)" >&2
  exit 2
fi

WORK_DIR="$(mktemp -d "${TMPDIR:-/tmp}/fwup-compat.XXXXXX")"
LOCALNET_PID=""

cleanup() {
  if [[ -n "${LOCALNET_PID}" ]] && kill -0 "${LOCALNET_PID}" 2>/dev/null; then
    kill "${LOCALNET_PID}" 2>/dev/null || true
    wait "${LOCALNET_PID}" 2>/dev/null || true
  fi
  LOCALNET_PID=""
}
trap cleanup EXIT

log()  { printf '\n=== %s\n' "$*"; }
fail() { printf '\nFAIL: %s\n' "$*" >&2; exit 1; }

start_localnet() {
  local cli="$1" test_dir="$2" log_file="$3"

  "${cli}" node run-localnet \
    --test-dir "${test_dir}" --seed "${SEED}" --no-faucet --assume-yes \
    >"${log_file}" 2>&1 &
  LOCALNET_PID=$!

  for _ in $(seq 1 120); do
    if curl -sf "${API_URL}/v1" >/dev/null 2>&1; then
      [[ -f "${test_dir}/mint.key" ]] || fail "localnet is up but ${test_dir}/mint.key is missing"
      return 0
    fi
    kill -0 "${LOCALNET_PID}" 2>/dev/null || fail "localnet exited early; see ${log_file}"
    sleep 2
  done
  fail "localnet did not become ready within 240s; see ${log_file}"
}

# mint.key is BCS: a 0x20 length prefix followed by the 32-byte key. deploy-testnet
# wants it hex-encoded, so drop the prefix.
root_key_hex() {
  xxd -p -c 64 "$1/mint.key" | tr -d '\n' | cut -c3-
}

# The localnet's validator, whose stake proposes and votes.
validator_field() {
  grep "^$2:" "$1/0/private-identity.yaml" | awk '{print $2}' | tr -d '"'
}

# One package set, one proposal. No Gas entry: a gas change is a release concern,
# and this test is only asking whether the old VM can execute the new framework.
write_bundle_config() {
  cat >"$1" <<'YAML'
---
name: "framework-upgrade-compat"
metadata:
  title: "Framework upgrade compatibility test"
  description: "Publishes this checkout's framework onto a localnet running the previous release's binary."
update_sequence:
  - Framework:
      bytecode_version: 10
      git_hash: ~
YAML
}

run_upgrade() {
  local label="$1" cli="$2"
  local test_dir="${WORK_DIR}/${label}"       ; mkdir -p "${test_dir}"
  local bundle="${WORK_DIR}/${label}-bundle"
  local config="${WORK_DIR}/${label}-config.yaml"

  log "[${label}] starting localnet with $("${cli}" --version 2>/dev/null | head -1)"
  start_localnet "${cli}" "${test_dir}" "${WORK_DIR}/${label}-localnet.log"

  log "[${label}] generating the governance bundle from this checkout"
  write_bundle_config "${config}"
  "${RELEASE_TOOL}" generate-bundle --release-config "${config}" --bundle "${bundle}" \
    >"${WORK_DIR}/${label}-generate.log" 2>&1 \
    || { tail -30 "${WORK_DIR}/${label}-generate.log" >&2; fail "[${label}] could not generate the bundle"; }

  log "[${label}] deploying it through governance"
  # --skip-signoff: the sign-off checkboxes are a human review gate, not applicable here.
  # --mint-to-validator: funds the validator's gas; refused on testnet, intended for
  #   throwaway networks exactly like this one.
  #
  # Keys go through the environment rather than argv so they do not sit in the
  # process table. deploy-testnet declares these with hide_env_values, so they
  # are also kept out of its own diagnostics. They only authorise this
  # throwaway localnet -- the seed is fixed, so they are reproducible by anyone
  # reading this script -- but argv is a bad habit to set.
  ROOT_KEY="$(root_key_hex "${test_dir}")" \
  VALIDATOR_ADDRESS="$(validator_field "${test_dir}" account_address)" \
  VALIDATOR_KEY="$(validator_field "${test_dir}" account_private_key)" \
  "${RELEASE_TOOL}" deploy-testnet \
    --bundle "${bundle}" \
    --network "${API_URL}" \
    --mint-to-validator \
    --skip-signoff \
    >"${WORK_DIR}/${label}-deploy.log" 2>&1 \
    || { tail -40 "${WORK_DIR}/${label}-deploy.log" >&2; fail "[${label}] the upgrade did not execute -- this binary cannot run the new framework"; }

  # The publish is only half of it: every later transaction has to load the new
  # modules, so a node that accepted the upgrade can still fail afterwards.
  # Submitted with the node's own CLI: the client version is irrelevant here,
  # what matters is the node executing against the framework it just took.
  log "[${label}] executing a transaction against the upgraded framework"
  local post="${WORK_DIR}/${label}-post-upgrade.log"
  "${cli}" move run \
    --function-id 0x1::aptos_account::transfer \
    --args "address:${ROOT_ACCOUNT}" u64:1 \
    --sender-account "${ROOT_ACCOUNT}" \
    --private-key-file "${test_dir}/mint.key" \
    --encoding bcs \
    --url "${API_URL}" \
    --max-gas 2000000 \
    --assume-yes \
    >"${post}" 2>&1 || true
  grep -q '"success": true' "${post}" \
    || { cat "${post}" >&2; fail "[${label}] transactions fail after the upgrade"; }

  cleanup
  log "[${label}] PASS"
}

log "work dir: ${WORK_DIR}"
run_upgrade old "${OLD_CLI}"
[[ "${RUN_CONTROL}" -eq 1 ]] && run_upgrade control "${NEW_CLI}"

log "PASS: the old binary executed this checkout's framework"
