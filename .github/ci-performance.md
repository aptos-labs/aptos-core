# CI caching and test distribution

## Design

Targeted unit tests use the same affected-package selector for `run` and
`archive`, including the four existing exclusions. A 64-vCPU job builds the
archive once. Selections with at most 1,000 tests execute there; larger selections
use eight 32-vCPU partitions. The threshold and runner sizes are provisional
until the Linux A/B measurements below are complete. This does not replace
workspace tests, doctests, VM-feature checks, or smoke tests.

Compilation and prover tools use GitHub's native cache service. GitHub enforces
branch access using the job's runtime credentials: pull requests can read the
default/base branch caches, but cannot write into the default branch's scope.
Changing a key or bypassing this repository's setup action does not grant that
write access. See [GitHub's cache restrictions](https://docs.github.com/en/actions/reference/workflows-and-actions/dependency-caching#restrictions-for-accessing-a-cache).

Normal cache producers are pushes and manual runs on `main`. Other jobs restore
only. `sccache` is pinned, uses only its GHA backend, and includes compiler inputs
in its keys; its namespace is not rotated weekly. Prover keys include OS,
architecture, and the hash of `scripts/dev_setup.sh`. The installer checks the
requested versions after both hits and misses. No cache contains credentials.

Setup rejects non-GitHub cache endpoints, including shared cache proxies. Missing
credentials, download failures, or cache-server initialization failures disable
caching rather than bypassing tests. The explicit startup check is necessary:
`SCCACHE_IGNORE_SERVER_IO_ERROR` alone does not cover initialization failures.
The upstream sccache post-job annotation hook is disabled because its statistics
failure can fail a job; our statistics collection is best-effort instead.
The single GHA backend uses synchronous `l0` writes. The `ignore` multilevel
policy starts detached writes, which can be lost when an ephemeral runner exits;
its top-level write count is not evidence that those entries reached GitHub.

Targeted and smoke build outputs are **artifacts**, not shared caches. Consumers
download exact artifact IDs from their build jobs in the same workflow run.
Attempt-specific producer names avoid upload collisions; IDs allow failed-job
reruns to reuse a successful earlier build. Build artifacts expire after one day;
measurements after seven. Longer-delayed reruns must rerun their build jobs.
Smoke executables have their executable permissions restored after download.

The required check names remain unchanged. Failures, cancellations, and
unexpected skips fail the result gates. Only the explicit skip conditions are
accepted. PostgreSQL readiness is checked before archived tests start.

CLI network tests run independently on three 8-vCPU runners. API compatibility
uses an 8-vCPU runner. The dependency-graph check runs on Linux while resolving
the Apple target; it does not claim to validate actual Mach-O linkage. Cargo
resolution failures fail the check rather than being treated as absent packages.

## Pre-merge benchmark procedure

The existing `Lint+Test` workflow is the dispatch entry point, so the benchmark
can run from an unmerged, published branch without first adding a new workflow
to `main`. Its benchmark-only mode skips normal lint/test jobs. It calls the
candidate reusable workflow, not the production `@main` version.

```bash
gh workflow run lint-test.yaml --ref vk/ci-improve-1 \
  -f ci-benchmark=true -f benchmark-suite=small \
  -f benchmark-cache=disabled -f benchmark-e2e=true
```

Repeat for `small` and `large`, each with `disabled`, `cold`, and `seeded` cache
modes. Each dispatch uses three fresh baseline runners and three fresh candidate
build runners. Run these experiments sequentially to avoid unnecessary runner
contention and expense. The fixed suites are:

- Small: `aptos-cargo-cli`.
- Large: `move-compiler-v2`, `move-prover-boogie-backend`, and
  `move-prover-e2e-tests`. Verify that it actually exceeds the sharding threshold.

Both paths check out the dispatch's exact SHA, select the same packages, and use
the same nextest profile/retry policy. The baseline retains the original tool
setup and build-and-run command. The candidate uses the production archive and
partition workflow. The verifier compares complete test identities and ignore
status, checks all eight partition inventories, and rejects duplicates or
omissions. Baseline inventory collection occurs after its timed test command.

`seeded` populates a separate `benchmark-RUN_ID` cache in the dispatching branch's
scope, then starts fresh read-only consumers. Its seed time and cost are reported
separately. This tests warm-cache reuse **without granting a branch main-write
access**; it is not evidence that a real main-to-PR restore has occurred. A repeat
of a PR-only change can still miss because ordinary PRs do not publish results.
No benchmark writes to the production compilation-cache namespace.

With `benchmark-e2e=true`, both old and new CLI/API workflows use the prebuilt
image at `893d1ffea49dcfa933f0421b19fc6e31a9c808ab`. The candidate CLI checks out
the dispatch SHA for its Python assertion fixes; the baseline CLI and both API
jobs keep the reference source. The first three runner-size comparisons were
dispatched before those fixes and use reference test sources on both sides.
Record resolved network-image digests; moving tags can confound comparisons.

The large, cache-disabled experiment intentionally fails the last shard of the
first candidate after its tests finish. Run `gh run rerun RUN_ID --failed` to
verify that the rerun downloads the successful original build's exact artifact
ID. Keep first-attempt test timings and rerun-validation overhead separate.
The report retains all attempts without counting reused job IDs twice.

After each completed run:

```bash
python3 .github/scripts/ci_metrics.py report RUN_ID > run.json
gh run download RUN_ID --pattern 'ci-metrics-*' --dir measurements/RUN_ID
```

Collect and compare:

- Required-check completion time, job start/completion timestamps, setup,
  compilation, archive listing/extraction, upload/download, and test execution.
  Job timestamps also include scheduling/dependency effects; distinguish those
  from execution time. Exclude benchmark-only inventory collection when comparing
  production paths, and retain raw job totals alongside adjusted phase totals.
- Median and range across the three samples; slowest shard, shard imbalance,
  retries, timeouts, and failures. Never drop unsuccessful samples silently.
- sccache hits, misses, errors, writes, and non-cacheable compilations; prover
  cache hit/miss and installation time. A disabled cache is not a cold-cache hit.
- Runner CPU allocation and memory. Phase records report child CPU time and the
  largest child RSS, **not** aggregate parallel-process memory. They also record
  the runner cgroup memory high-water mark when available. Use runner telemetry
  for aggregate memory and utilization when that counter is unavailable.
- Archive sizes and transfer time, native cache usage/evictions, retained artifact
  storage, and request throttling. At the initial measurement there were five
  repository caches totaling 7,115,327,502 bytes; confirm the actual storage limit
  before enabling broad production cache population.
- Allocated vCPU-minutes and actual runner rates. Compute dollars from runner
  billing, provisioning/billing minima, storage, requests, and transfer charges.
  vCPU-minutes alone are not dollar savings. Account for trusted cache-seed costs.

Before merging, also exercise a fresh Linux inline run and all shards with
PostgreSQL/prover tools; empty selection; missing artifact; compilation/test
failure; cancellation; expected/unexpected skip; and failed-jobs reruns. Verify
that a PR token cannot publish into the main cache scope, including when the
workflow's read-only policy is bypassed. Check real default-branch restore
behavior using a trusted cache entry. Do not weaken the backend boundary to
perform this test.

Keep the 1,000-test cutoff and eight-way fan-out only if measured overhead,
memory, and cost support them; otherwise tune them within this PR. Likewise keep
the smaller CLI/API and merge-base runners only after Linux runs pass without
material timeout or memory regressions. Delete only benchmark artifact/cache IDs
whose ownership has been verified; never clear repository caches globally.

## Results recorded on 2026-09-08

Local validation is complete for the checks below. Remote A/B execution, genuine
main-to-PR cache tests, runner-size tuning, and dollar-cost measurement are still
pending; no speedup or cost-saving percentage is established by these results.

- `cargo check`, all nine `aptos-cargo-cli` unit tests, package Clippy with
  warnings denied, and package formatting pass.
- CI-helper tests cover cache endpoint/write policy, the checked-in required-check
  shell bodies, cache preflight fallback, threshold boundaries, partition
  completeness, explicit package selection, and fail-closed dependency checks.
- The actual CLI creates a 1,631,443-byte archive. Normal and archived inventories
  contain the same nine tests; all eight partitions pass with exact disjoint
  coverage. An empty sixteenth partition succeeds with a warning.
- An excluded-only package selection writes no archive; a missing archive and an
  invalid build profile both fail rather than being treated as empty selections.
- Three warm-local-Cargo samples on macOS/ARM64, using the default debug build
  profile and nextest's `ci` execution profile: old run median **3.35 s**, archive
  construction median **3.35 s**, archived execution median **0.45 s**. These are
  small-path sanity measurements, not Linux CI speedups; archive planning and
  network transfer are not included in the archived-execution timing.
- A real pinned sccache binary failed startup against an unavailable local GHA
  endpoint, even with `SCCACHE_IGNORE_SERVER_IO_ERROR=1`. The corrected preflight
  disabled the wrapper and the same Rust probe compiled successfully. GHA was
  selected even with an S3 bucket environment variable present.
- The dependency check passes against the real Apple-target Cargo graph on this
  checkout. Live Linux runner provisioning, Docker, and prover installation have
  not been exercised locally.

Historical GitHub job durations were re-read from the API. These are context,
**not matched before/after comparisons**:

| Existing job | Runner time | Allocated vCPU-minutes | Evidence |
| --- | ---: | ---: | --- |
| Targeted unit tests | 50.37 min | 3,223.5 | [Run 34253211093](https://github.com/aptos-labs/aptos-core/actions/runs/34253211093) |
| Sequential CLI network tests | 8.33 min | 533.3 | [Run 33703105368](https://github.com/aptos-labs/aptos-core/actions/runs/33703105368) |
| API compatibility | 1.18 min | 75.7 | [Run 33703105368](https://github.com/aptos-labs/aptos-core/actions/runs/33703105368) |
