# CI caching and test distribution

## Design

Targeted unit tests use the same affected-package selector for `run` and
`archive`, including the four existing exclusions. A 64-vCPU job builds the
archive once. Selections with at most 3,000 tests execute there; larger selections
use eight 32-vCPU partitions. The threshold and runner sizes are provisional
until the Linux A/B measurements below are complete. This does not replace
workspace tests, doctests, VM-feature checks, or smoke tests.

Optional compilation caching and prover tools use GitHub's native cache service. GitHub enforces
branch access using the job's runtime credentials: pull requests can read the
default/base branch caches, but cannot write into the default branch's scope.
Changing a key or bypassing this repository's setup action does not grant that
write access. See [GitHub's cache restrictions](https://docs.github.com/en/actions/reference/workflows-and-actions/dependency-caching#restrictions-for-accessing-a-cache).

Compilation caching is **off by default**. Enable normal CI only by setting the
repository variable `CI_COMPILATION_CACHE_ENABLED=true` after the quota and
large warm-cache gates below pass. Explicit benchmark inputs can enable it
without changing repository settings. Cold smoke builds regressed materially;
the isolated small warm-cache win does not justify a broad default rollout.

When enabled, normal cache producers are pushes and manual runs on `main`. Other jobs restore
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
The single GHA backend uses synchronous `all` writes. The `ignore` multilevel
policy starts detached writes, which can be lost when an ephemeral runner exits;
its top-level write count is not evidence that those entries reached GitHub.
Benchmark seed and reader jobs also use the same `CARGO_TERM_COLOR`: sccache
hashes this environment variable even though it normalizes rustc's color flags.
The `all` policy also avoids `l0`'s extra permission probe before every write.
Cache diagnostics export error counts, never raw backend logs or signed URLs.

Targeted and smoke build outputs are **artifacts**, not shared caches. Consumers
download exact artifact IDs from their build jobs in the same workflow run.
Attempt-specific producer names avoid upload collisions; IDs allow failed-job
reruns to reuse a successful earlier build. Build artifacts expire after one day;
measurements after seven. Longer-delayed reruns must rerun their build jobs.
Smoke executables have their executable permissions restored after download.
Smoke uploads use fast compression for their large raw binaries; nextest-only
artifacts retain compression level zero because their contents are already zstd.

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

Repeat for `small`, `medium`, and `large`, each with `disabled`, `cold`, and `seeded` cache
modes. Each dispatch uses three fresh baseline runners and three fresh candidate
build runners. Run these experiments sequentially to avoid unnecessary runner
contention and expense. The fixed suites are:

- Small: `aptos-cargo-cli`.
- Medium: `move-compiler-v2`, `move-prover-boogie-backend`, and
  `move-prover-e2e-tests` (2,341 tests; originally called `large`). This suite
  completed too quickly to justify eight-way fan-out, motivating the raised cutoff.
- Large: the medium packages plus `e2e-move-tests`, `aptos-api`, and `move-prover`.
  These include the dominant test-time consumers in the historical targeted run.
  Verify that the resulting selection exceeds the sharding threshold.

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

With `benchmark-rerun=true`, a sharded experiment intentionally fails the last
shard of the first candidate after its tests finish. Run `gh run rerun RUN_ID --failed` to
verify that the rerun downloads the successful original build's exact artifact
ID. Keep first-attempt test timings and rerun-validation overhead separate.
The report retains all attempts without counting reused executions twice;
GitHub can assign new IDs to successful jobs that were not executed again.

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
  Child CPU/RSS omit work in a separately started sccache daemon. New records
  additionally measure host busy CPU from `/proc/stat`, including that daemon
  and other host processes; idle, I/O wait, and stolen time are excluded.
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

Keep the 3,000-test cutoff and eight-way fan-out only if measured overhead,
memory, and cost support them; otherwise tune them within this PR. Likewise keep
the smaller CLI/API and merge-base runners only after Linux runs pass without
material timeout or memory regressions. Delete only benchmark artifact/cache IDs
whose ownership has been verified; never clear repository caches globally.

## Results recorded on 2026-09-09

The branch is published and the comparisons below ran on fresh Linux runners.
Allocated vCPU-minutes are a compute-allocation proxy, **not billed dollars**.
Job execution excludes provisioning; check elapsed time below includes scheduling
and the candidate CLI result gate. Samples include retries and slow image pulls.
Independent smoke validation and some small experiments overlapped long-running
builds, each on fresh EC2 instances. Shared service or provider contention is
therefore part of the observed variation, not a controlled-away factor.

### CLI and API runner sizing

The first three CLI comparisons use identical reference images and test sources.
The fourth also includes the candidate's address-comparison fixes; it is a
combined-change validation, not a runner-only comparison.

| Run | CLI check elapsed, old / new | CLI vCPU-minutes, old / new | API execution, old / new | API vCPU-minutes, old / new |
| --- | ---: | ---: | ---: | ---: |
| [34406079480](https://github.com/aptos-labs/aptos-core/actions/runs/34406079480) | 818 / 448 s | 833.1 / 116.9 | 84 / 64 s | 89.6 / 8.5 |
| [34407408453](https://github.com/aptos-labs/aptos-core/actions/runs/34407408453) | 600 / 441 s | 599.5 / 115.7 | 92 / 75 s | 98.1 / 10.0 |
| [34408401763](https://github.com/aptos-labs/aptos-core/actions/runs/34408401763) | 706 / 660 s | 721.1 / 169.6 | 73 / 73 s | 77.9 / 9.7 |
| [34410026272](https://github.com/aptos-labs/aptos-core/actions/runs/34410026272) | 543 / 289 s | 546.1 / 100.4 | 77 / 192 s | 82.1 / 25.6 |

- CLI runner-only medians: **706 to 448 seconds (36.5% shorter)** and **721.1
  to 116.9 allocated vCPU-minutes (83.8% less)**. The small GitHub-hosted result
  gate has no explicit CPU label and is excluded from the allocation proxy.
- API, including all four samples: median execution **80.5 to 74 seconds**;
  median allocated compute **85.9 to 9.9 vCPU-minutes (88.5% less)**. Check
  elapsed medians are 114 / 104.5 seconds. The fourth candidate spent about
  122 seconds pulling the same tools-image digest; both spec checks passed on
  their first attempt. Do not promise a consistent API latency improvement.
- The fixed CLI suite passed on its first attempt on all three networks. Six
  regression tests also passed on each runner. Two earlier false failures
  compared equivalent short/padded account-address strings; comparisons now use
  the existing SDK's parsed account-address type. Existing retries still cover
  transient epoch-transition errors; production execution logic is unchanged.
- Sampled host used-memory peaks were about **3.23 GB for CLI** and **1.46 GB
  for API**, on 16-GB runners. These are one-second host samples, not process RSS
  or a guarantee about unobserved peaks. Neither runner cgroup peak counter was
  exposed. No out-of-memory failure occurred.

The observed 8-vCPU machines were c7a.2xlarge spot instances; the 64-vCPU
machines were generally c7i-flex.16xlarge on-demand. That pricing difference
makes a vCPU-only dollar conversion inappropriate. Actual AWS billing, storage,
transfer, runner provisioning, and the Runs-On fee have not been measured.

### Unit-test and cache experiments

- The first small disabled-cache run used shallow baseline checkout and is not
  a fair total-time comparison. Subsequent baselines fetch full history, matching
  the candidate. Inventory and correctness evidence from the first run remains
  valid.
- Small cold-cache run 34407408453: all three nine-test samples passed, with
  identical inventories. Baseline median job execution was **127 seconds**;
  candidate **152 seconds**. Each candidate recorded 350 cache misses and no
  hits. Cold caching did not improve this small workload.
- Corrected cache-disabled run
  [34413315139](https://github.com/aptos-labs/aptos-core/actions/runs/34413315139):
  all samples and inventories passed. Median baseline/candidate compilation
  phases were **63.08 / 62.02 seconds**, but full job execution was **124 / 155
  seconds**. Two candidates spent about a minute in database setup, mostly
  Docker-image transfer; the third completed in 120 seconds. Host CPU counters
  worked on Linux. There is no consistent tiny-job speedup in these samples.
- Seeded run 34408401763 was not genuinely warm: seed/readers had different
  `CARGO_TERM_COLOR`, and detached writes did not all finish. Run 34409394605
  fixed the environment and used synchronous `l0` writes, but persisted only
  105 compilation entries. Readers each obtained **107 hits / 243 misses**;
  baseline/candidate medians were **125 / 133 seconds**, excluding the seed's
  103-second execution. This is partial reuse, not evidence of a warm-cache win.
  These intermediate configurations are not the final implementation.
- Final `all` policy in
  [34412173055](https://github.com/aptos-labs/aptos-core/actions/runs/34412173055):
  the first seed persisted 201 compilation entries before upload throttling;
  readers got 203 hits / 147 misses. Rerunning only the seed and its dependents
  added the remaining 147 entries. All three second-pass readers then got
  **350 hits / zero misses**. Median build/archive time was **51.99 seconds**
  versus baseline build/run **63.03 seconds**; full job execution was
  **121 versus 126 seconds**. The two seed jobs took **99 + 100 seconds**
  (212.3 allocated vCPU-minutes) separately. The populated cache contained 349
  entries including the service check, totaling **266,333,880 bytes**. This is
  a modest small-job improvement after full population, not evidence of a
  large-workspace warm-cache gain or cheap first-time population.
- The original 2,341-test suite (now `medium`) passed in all three baselines and
  all 24 candidate partitions in run 34410026272. Baseline median check elapsed
  was **270 seconds**, versus **370 seconds** with eight-way sharding; allocated
  compute was **256 / 475.2 vCPU-minutes**. Archives were about 126.6 MB each.
  These fast compiler tests did not justify fan-out, so the inline cutoff was
  raised from 1,000 to 3,000. The expanded `large` suite exercises actual slow
  E2E/API/prover tests; its result must justify retaining the sharded path.
- The corrected inline path passed all three 2,341-test samples and exact
  inventories in
  [34412913212](https://github.com/aptos-labs/aptos-core/actions/runs/34412913212).
  Baseline/candidate median check elapsed was **276 / 273 seconds** and
  allocation **258.1 / 256.0 vCPU-minutes**: essentially neutral, without the
  earlier sharding regression. Candidate execution ranged 219–248 seconds;
  baseline execution ranged 237–260 seconds.
- That run deliberately failed one shard **after its tests passed**. A
  failed-jobs rerun successfully downloaded original artifact **10126978793**
  (`candidate-1-tests-1`) without rebuilding. The rerun added 57 seconds on one
  32-vCPU runner, plus inventory verification; it is excluded from the
  first-attempt comparison. Exact inventory and partition checks passed again.
- The Linux Apple-target dependency check passed in run
  [34407180664](https://github.com/aptos-labs/aptos-core/actions/runs/34407180664).
  The lightweight merge-base/helper checks passed on 2-vCPU Linux runners in
  runs 34408401763 and 34409394605 (201 / 181 seconds total).

All three completed experimental cache populations were removed by verified branch
entry IDs: 99 entries / 31,916,863 bytes from run 34408401763 and 106 entries /
46,987,893 bytes from run 34409394605, plus 349 entries / 266,333,880 bytes from
run 34412173055. No production cache entries were deleted.
Repository usage reached about 9.69 GB from other writers; querying its configured
storage limit requires administrator permission (HTTP 403). Capacity and a real
main-to-PR restore/write-denial exercise remain external pre-merge checks.
The real missing-artifact download failed as expected in run 34412173055;
the following assertion required that failure, and the Linux helper tests passed.

### Local validation

- `cargo check`, all nine `aptos-cargo-cli` unit tests, package Clippy with
  warnings denied, and package formatting pass.
- Ten Node and thirteen Python CI-helper tests cover cache endpoint/write policy, the checked-in required-check
  shell bodies, cache preflight fallback, threshold boundaries, partition
  completeness, explicit package selection, fail-closed dependency checks,
  missing/invalid artifact IDs, and correct rerun allocation accounting.
  Actionlint passes for the changed workflows; a whole-repository run also
  reports unrelated pre-existing workflow errors, which were left untouched.
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
  checkout and on Linux. Remote targeted samples exercised provisioning, Docker,
  PostgreSQL readiness, prover installation, archive transport, and all shards.

Historical GitHub job durations were re-read from the API. These are context,
**not matched before/after comparisons**:

| Existing job | Runner time | Allocated vCPU-minutes | Evidence |
| --- | ---: | ---: | --- |
| Targeted unit tests | 50.37 min | 3,223.5 | [Run 34253211093](https://github.com/aptos-labs/aptos-core/actions/runs/34253211093) |
| Sequential CLI network tests | 8.33 min | 533.3 | [Run 33703105368](https://github.com/aptos-labs/aptos-core/actions/runs/33703105368) |
| API compatibility | 1.18 min | 75.7 | [Run 33703105368](https://github.com/aptos-labs/aptos-core/actions/runs/33703105368) |
