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
#                                            [--new-cli <path>]
#                                            [--release-tool <path>]
#                                            [--control]
#
#   --control  first run the whole thing with the new binary as the node, as a
#              positive control, and only then with the old one. If the control
#              fails, the framework or this harness is broken and compatibility
#              is not the question; if it passes and the old run then fails,
#              that is a compatibility break.
#
# Note: the aptos CLI reads and writes ~/.aptos, and the localnet binds fixed
# ports (8080 API, 50051 stream), so only one run at a time on a host.

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

ROOT_ACCOUNT="0xa550c18"
API_URL="http://127.0.0.1:8080"
# Fixed so a failing run can be reproduced with the same keys.
SEED="0000000000000000000000000000000000000000000000000000000000000001"

OLD_CLI=""
NEW_CLI=""
RELEASE_TOOL="${REPO_ROOT}/target/debug/aptos-release-tool"
RUN_CONTROL=0

while [[ $# -gt 0 ]]; do
  case "$1" in
    --old-cli) OLD_CLI="$2"; shift 2 ;;
    --new-cli) NEW_CLI="$2"; shift 2 ;;
    --release-tool) RELEASE_TOOL="$2"; shift 2 ;;
    --control) RUN_CONTROL=1; shift ;;
    -h|--help) sed -n '/^# Usage:/,/^$/p' "${BASH_SOURCE[0]}"; exit 0 ;;
    *) echo "unknown argument: $1" >&2; exit 2 ;;
  esac
done

[[ -x "${OLD_CLI}" ]]      || { echo "--old-cli must be an executable aptos CLI" >&2; exit 2; }
[[ -x "${RELEASE_TOOL}" ]] || { echo "aptos-release-tool not found at ${RELEASE_TOOL}; build it with 'cargo build -p aptos-release-tool'" >&2; exit 2; }

# Only --control needs a second binary. Elsewhere the CLI is just a client
# submitting to the node under test, so the node's own CLI is used.
if [[ "${RUN_CONTROL}" -eq 1 ]]; then
  [[ -n "${NEW_CLI}" ]] || NEW_CLI="$(command -v aptos || true)"
  [[ -x "${NEW_CLI}" ]] || { echo "--control needs --new-cli to point at an executable aptos CLI (or one on PATH)" >&2; exit 2; }
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
  local cli="$1" base="$2"
  local log_file="${base}-localnet.log"

  "${cli}" node run-localnet \
    --test-dir "${base}" --seed "${SEED}" --no-faucet --assume-yes \
    >"${log_file}" 2>&1 &
  LOCALNET_PID=$!

  local attempt
  for ((attempt = 0; attempt < 120; attempt++)); do
    if curl -sf "${API_URL}/v1" >/dev/null 2>&1; then
      [[ -f "${base}/mint.key" ]] || fail "localnet is up but ${base}/mint.key is missing"
      return 0
    fi
    kill -0 "${LOCALNET_PID}" 2>/dev/null || fail "localnet exited early; see ${log_file}"
    sleep 2
  done
  fail "localnet did not become ready within 240s; see ${log_file}"
}

# The localnet's validator, whose stake proposes and votes.
validator_field() {
  local base=$1
  local field=$2
  yq ".${field}" < "${base}/0/private-identity.yaml"
}

# One bundle serves every run: it is built from this checkout and deploy-testnet
# only reads it. No Gas entry -- a gas change is a release concern, and this test
# only asks whether the old VM can execute the new framework.
generate_bundle() {
  local config="${WORK_DIR}/bundle-config.yaml"
  BUNDLE="${WORK_DIR}/bundle"

  cat >"${config}" <<'YAML'
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

  "${RELEASE_TOOL}" generate-bundle --release-config "${config}" --bundle "${BUNDLE}" \
    >"${WORK_DIR}/generate.log" 2>&1 \
    || { tail -30 "${WORK_DIR}/generate.log" >&2; fail "could not generate the bundle"; }
}

run_upgrade() {
  local label="$1" cli="$2"
  local base="${WORK_DIR}/${label}"

  log "[${label}] starting localnet with $("${cli}" --version 2>/dev/null | head -1)"
  start_localnet "${cli}" "${base}"

  log "[${label}] deploying the bundle through governance"
  # --skip-signoff: the sign-off checkboxes are a human review gate, not applicable here.
  # --mint-to-validator: funds the validator's gas; refused on testnet, intended for
  #   throwaway networks exactly like this one.
  # --skip-simulation: deploy-testnet simulates in its own VM, which is built from
  #   THIS checkout -- i.e. the new binary. Simulating would assert against the wrong
  #   VM and mask the node under test; the real execution below is the assertion.
  #
  # Keys go through the environment rather than argv, so they stay out of the
  # process table; deploy-testnet declares them hide_env_values, so they stay out
  # of its diagnostics too.
  ROOT_KEY="$(xxd -p -c 64 "${base}/mint.key" | tr -d '\n' | cut -c3-)" \
  VALIDATOR_ADDRESS="$(validator_field "${base}" account_address)" \
  VALIDATOR_KEY="$(validator_field "${base}" account_private_key)" \
  "${RELEASE_TOOL}" deploy-testnet \
    --bundle "${BUNDLE}" \
    --network "${API_URL}" \
    --mint-to-validator \
    --skip-signoff \
    --skip-simulation \
    >"${base}-deploy.log" 2>&1 \
    || { tail -40 "${base}-deploy.log" >&2; fail "[${label}] the upgrade did not execute -- this binary cannot run the new framework"; }

  # The publish is only half of it: every later transaction has to load the new
  # modules, so a node that accepted the upgrade can still fail afterwards.
  # Submitted with the node's own CLI: the client version is irrelevant here,
  # what matters is the node executing against the framework it just took.
  log "[${label}] executing a transaction against the upgraded framework"
  local post="${base}-post-upgrade.json"
  "${cli}" move run \
    --function-id 0x1::aptos_account::transfer \
    --args "address:${ROOT_ACCOUNT}" u64:1 \
    --sender-account "${ROOT_ACCOUNT}" \
    --private-key-file "${base}/mint.key" \
    --encoding bcs \
    --url "${API_URL}" \
    --max-gas 2000000 \
    --assume-yes \
    >"${post}" 2>"${base}-post-upgrade.err" \
    || { cat "${base}-post-upgrade.err" "${post}" >&2; fail "[${label}] could not submit a transaction after the upgrade"; }

  # Parsed rather than grepped, so a rejected transaction is distinguishable from
  # a CLI that never ran, and so vm_status survives into the failure message.
  jq -e '.Result.success == true' "${post}" >/dev/null 2>&1 \
    || fail "[${label}] a transaction failed after the upgrade: $(jq -r '.Result.vm_status // .Error // "no result"' "${post}" 2>/dev/null)"

  cleanup
  log "[${label}] PASS"
}

for tool in jq yq; do
  command -v "${tool}" >/dev/null || { echo "${tool} is required" >&2; exit 2; }
done

log "work dir: ${WORK_DIR}"

log "generating the governance bundle from this checkout"
generate_bundle

# The control runs first, deliberately: fail() aborts the script, so running it
# after the old binary would only exercise it when the old binary had already
# passed -- the case needing no comparison. Failing here means the new binary
# cannot publish its own framework, so the fault is in the framework or in this
# script, not in backward compatibility.
if [[ "${RUN_CONTROL}" -eq 1 ]]; then
  run_upgrade control "${NEW_CLI}"
fi

run_upgrade old "${OLD_CLI}"

log "PASS: the old binary executed this checkout's framework"
