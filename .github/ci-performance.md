# CI test execution and measurements

## Test execution

- [Targeted unit tests](workflows/targeted-unit-tests.yaml) use the same
  affected-package selector and exclusions for direct execution and archive
  creation. A 64-vCPU job builds the test archive. Selections with at most 3,000
  tests execute in that job; larger selections use eight 32-vCPU partitions.
  An empty package selection completes without scheduling partitions.
- [Smoke tests](workflows/smoke-tests.yaml) build the node and test/helper binaries
  in two parallel 64-vCPU jobs, then execute eight partitions on 16-vCPU runners.
- [CLI E2E tests](workflows/cli-e2e-tests.yaml) run devnet, testnet, and mainnet
  independently on 8-vCPU runners. Assertion regression tests run when their
  directory exists in the checked-out source.
- [API compatibility](workflows/node-api-compatibility-tests.yaml) uses an 8-vCPU
  runner. Merge-base freshness and CLI dependency checks use 2-vCPU Linux runners.
  The dependency check resolves the Apple-target Cargo graph; it does not inspect
  Mach-O linkage and fails on Cargo resolution errors.

Workspace tests, doctests, and VM-feature checks run separately from targeted
tests. The result gates expose `rust-targeted-unit-tests`, `rust-smoke-tests`,
and `run-cli-tests`. They reject failures, cancellation, missing results, and
skips outside their explicit skip conditions.

## Artifacts and setup

Targeted and smoke consumers download exact artifact IDs from their producer
jobs in the same workflow run. Attempt-specific artifact names avoid upload
collisions. Failed-job reruns reuse successful producers' artifacts while those
artifacts remain available. Build artifacts have one-day retention; expired or
deleted artifacts require rebuilding.

Smoke uploads use compression level 1 for raw binaries, and consumers restore
executable permissions after download. Targeted nextest archives use compression
level 0 because their contents are already compressed.

[PostgreSQL setup](actions/postgres-start/action.yaml) checks TCP readiness before
tests run. Targeted test jobs install prover tools because nextest archives do
not include external executables.

Prover-tool and merge-base dependency caches use the shared
[GitHub cache setup](actions/github-cache-setup/action.yaml). It accepts only
native GitHub cache endpoints with runtime credentials. Repository policy allows
cache writes only for pushes and manual runs on `main`; other jobs restore only.
Prover cache keys include OS, architecture, and the installer hash. The installer
checks tool versions after both cache hits and misses. The merge-base helper uses
a separate `merge-base-v1` Cargo cache.

## Targeted-test benchmark

Dispatch [Lint+Test](workflows/lint-test.yaml) on a published branch containing
the workflow changes. Replace `YOUR_BRANCH` with that branch name:

```bash
gh workflow run lint-test.yaml --ref YOUR_BRANCH \
  -f ci-benchmark=true -f benchmark-suite=small
```

Benchmark mode invokes [CI Benchmark](workflows/ci-benchmark.yaml) instead of the
normal lint/test jobs. Each run compares three baseline and three candidate
samples at the dispatched SHA, with matching package selections and nextest
profiles/retry settings:

- The baseline runs the setup script followed by direct build-and-test execution.
- The candidate uses the targeted archive/partition workflow.
- The verifier requires matching test inventories, including ignore status, and
  complete, non-overlapping partition coverage for sharded selections.

Choose a suite with `benchmark-suite`:

- `small`: `aptos-cargo-cli`.
- `medium`: `move-compiler-v2`, `move-prover-boogie-backend`, and
  `move-prover-e2e-tests`.
- `large`: the medium packages plus `e2e-move-tests`, `aptos-api`, and
  `move-prover`.

Suite sizes depend on the checked-out revision. Confirm the selected suite
exercises the intended inline or sharded path. Run comparisons sequentially to
limit contention and expense. The small suite also runs Linux helper checks and
verifies that a missing-artifact download fails.

## Collecting and interpreting measurements

Instrumented commands record phase timings, exit codes, child CPU time, largest
child RSS, sampled host memory, and available host CPU/cgroup counters.
[Measurement reporting](actions/ci-metrics-report/action.yaml) uploads these
records and runner configuration under `ci-metrics-*` with seven-day retention.
Post-test reporting and CLI/API resource sampling are best-effort; test commands,
required artifact transfers, and benchmark inventory verification remain blocking.

Collect results from the repository root, replacing `RUN_ID` with the workflow
run ID:

```bash
python3 .github/scripts/ci_metrics.py report RUN_ID > run.json
gh run download RUN_ID --pattern 'ci-metrics-*' --dir measurements/RUN_ID
```

The report includes job attempts, artifact metadata, and allocated vCPU-minutes
where runner labels specify CPU counts. It counts reused job executions once.
Compare medians and ranges, setup/build/transfer/test phases, slowest partitions,
retries, failures, and artifact sizes. Separate benchmark inventory overhead and
scheduling delays from test execution.

Largest child RSS is not aggregate memory across parallel processes; host samples
include other runner processes, and cgroup peaks are not per-phase peaks.
Allocated vCPU-minutes are a compute-allocation proxy, not billed dollars.
Billing also depends on provisioning, runner rates, storage, and transfer charges.

## Validation

Run the CI helper tests from the repository root:

```bash
node --test .github/scripts/*.test.cjs
python3 -m unittest discover -s .github/scripts -p 'test_*.py'
```

The tests cover cache policy, result gates, artifact IDs, PostgreSQL readiness,
measurement failure handling, inline/shard selection, inventory coverage, and
rerun accounting. Workflow changes also require Linux CI validation; manual
benchmarks do not replace the normal required PR checks.
