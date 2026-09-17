# CI validation results

Measured on 2026-09-17 UTC. Measurements are complete; merge gates remain below.

## Revision and method

Timed candidate: `3be0111fe8a6d36c22aaad8972d44512b582e515`, on
`vk/ci-improve-1-validation-20260916`. The original remote branch was preserved;
no PR was opened.

Follow-up `ceafa814722d3cd2292b6d186c2ff25880aeebb1` fixes a validation gap:
ignored tests are filtered in nextest listings, so build-inventory comparison
must include filtered entries. Shard coverage still compares runnable tests only.
Regression tests reproduced the omission before the fix. Timed commands are
unchanged; the [corrected verifier passed on Linux](https://github.com/aptos-labs/aptos-core/actions/runs/35175437620)
over all nine recorded samples, along with all 12 Node and 13 Python helper tests.

Targeted benchmarks use `50599562104473fcb4dd4a535d1d643a69590638`, an empty
child commit with the identical Git tree
`4ef92740ac59f882810ce7be5b91e384dc350122`. This isolates their cancellation
group from normal CI without changing any benchmarked file. Each suite compares
three baseline/candidate samples with matching source, packages, profiles and
retry settings; inventories include ignore status.

[CLI/API comparison](https://github.com/aptos-labs/aptos-core/actions/runs/35170870724)
uses candidate test sources on both sides, baseline workflow logic from
`893d1ffea49dcfa933f0421b19fc6e31a9c808ab`, and immutable image digests resolved
once for all samples. The tools image is from that unchanged product-code
revision. The [temporary harness](https://github.com/aptos-labs/aptos-core/commit/ea77e689a8a6e99fa26cda254fbc266bbc8b4bce)
changes image selection, workflow sizing/orchestration and diagnostics, not test
assertions. It is on a separate measurements branch, outside shipping history.

## Measured changes

Three samples per variant. Times are workload-job durations, including
setup/cleanup/reporting but excluding queueing. CLI time is the slowest network
job; its compute sums all three networks. For sharded targeted tests, time is
build plus slowest shard; compute sums build and all shards. Small reporting/gate
jobs are excluded. Allocated vCPU-minutes are a proxy, **not billed dollars**.

| Workload | Baseline seconds: median (range) | Candidate seconds: median (range) | Median allocated vCPU-minutes: baseline → candidate | Result |
| --- | --- | --- | --- | --- |
| [Targeted small](https://github.com/aptos-labs/aptos-core/actions/runs/35172638902) | 131 (128–134) | 132 (128–134) | 139.7 → 140.8 | Neutral; nine identities matched |
| [Targeted medium](https://github.com/aptos-labs/aptos-core/actions/runs/35172874065) | 262 (253–273) | 240 (224–249) | 279.5 → 256.0 | 8.4% less time/compute; 2,341 identities matched |
| [Targeted large](https://github.com/aptos-labs/aptos-core/actions/runs/35173246819) | 1,877 (1,851–1,961) | 979 (949–1,003) | 2,002.1 → 1,902.4 | 47.8% less time; 5.0% less compute |
| CLI E2E | 498 (493–514) | 254 (241–263) | 531.2 → 98.1 | 49.0% less time; 81.5% less compute |
| API compatibility | 131 (73–141) | 72 (69–83) | 139.7 → 9.6 | 45.0% less time; 93.1% less compute |

Removing benchmark-only inventory collection changes the small comparison to
129s → 132s (+2.3%, still overlapping ranges) and medium to 260s → 240s
(7.7% faster). Including its result gate and scheduling gaps, CLI's candidate
median is 259s (246–269), still 48.0% faster than the baseline.
For large, removing inventory overhead yields 1,874s → 972s and 1,998.9 →
1,877.9 vCPU-minutes (48.1% less time, 6.1% less compute). Including inter-job
scheduling gaps, candidate elapsed time is 1,016s (978–1,032), a 45.9% reduction.
All three large samples matched 3,609 total tests, including 29 ignored tests,
with exact, disjoint coverage of 3,580 runnable tests across each eight-way split.

CLI ran 41 tests per network: all 18 network suites across both variants passed
on the first attempt. Both API spec comparisons passed in all six jobs, without
retries. Medium samples had no retries. One large candidate prover test,
`folds_of_idx.move`, hit resource/timeouts twice before passing on attempt three;
baseline `folds_of_multi.move` passed on its second attempt. Both sides' retry
time is included in the measurements. Targeted builders used
c7i-flex.16xlarge and shards c7i-flex.8xlarge. CLI/API assignments varied among
c7i, c7i-flex and c7a; these are practical CI comparisons, not universally
architecture-controlled CPU benchmarks. Observed latency gains are not guarantees.

Candidate phase medians (small / medium / large): prover setup **8.1 / 7.8 /
8.2s**, build/archive **69.2 / 137.1 / 492.4s**, and inline/slowest-shard test
phase **0.2 / 40.5 / 375.9s**. Large-shard archive download was a 21s median
(17–61s); each archive was 767 MB. Peak sampled shard memory was 8.4 GB.
These phase summaries are not additive medians and exclude other job overhead.

### Smoke artifacts

The [transfer comparison](https://github.com/aptos-labs/aptos-core/actions/runs/35171739994)
reused exact binaries from normal CI. All four file checksums matched in all six
consumers.

- Artifact bytes: **12.57 GB → 3.47 GB (72.4% smaller)**.
- Download/extraction, three consumers per format: median **332s → 158s (52.4%
  faster)**; ranges 310–353s and 104–278s.
- Upload tradeoff, one controlled producer comparison: node **14s → 28s**;
  helper/archive **40s → 94s**.

This compares artifact compression, not the full smoke workflow against the old
cache backend. Faster downloads do not imply the same percentage reduction in
overall smoke-check time.

## Correctness, interruptions and remaining gates

- Local: 40 tests passed, plus Cargo check, Clippy, formatting and workflow linting.
  [Linux CLI dependency check](https://github.com/aptos-labs/aptos-core/actions/runs/35170614809)
  passed.
- [Normal CI](https://github.com/aptos-labs/aptos-core/actions/runs/35170613312):
  general/Rust lints, license checks, batch-encryption tests, cached-package
  consistency and doctests passed (63 doctests, 36 ignored).
- All eight smoke partitions passed: 143 executed tests and 48 ignored.
  `test_changing_working_consensus` failed a no-progress assertion and
  `optimistic_verification` failed a chain-resumption assertion; each passed on
  its second attempt. Their sources and retry policies are unchanged. The slowest
  measured shard took 624.7s; peak sampled host memory was 26.3 GB. No OOM was reported.
- The workspace step was interrupted after about 24 minutes, then stuck
  finalizing. Normal cancellation and force-cancellation released the run;
  GitHub provides no log for that job, so the initial cause remains unconfirmed.
  Its reported allocation is 2,413.9 vCPU-minutes, including stalled finalization,
  not verified billed consumption.
- The [isolated workspace retry](https://github.com/aptos-labs/aptos-core/actions/runs/35173112084)
  passed using the exact candidate source and unchanged action: VM-feature checks,
  doctests, and **12,611 unit tests passed, 162 skipped**. `test_mint_nft` overflowed
  while corrupting a signature byte, then passed on its second attempt; that test
  is unchanged. The job took **55m 28s / 3,549.9 allocated vCPU-minutes**. This is a
  correctness run, not a matched workspace-performance comparison.
- **Rebase before merging:** the merge base is 14 days old; the policy permits
  seven. This failed normal CI and the small benchmark's freshness helper.
  Benchmark execution and inventory verification passed; the freshness check
  was not weakened. Small-run Linux helper/missing-artifact steps after that check
  were skipped, then covered by the successful Linux helper replay and a separate
  [live missing-artifact check](https://github.com/aptos-labs/aptos-core/actions/runs/35175637233).
- A real PR is still needed to validate affected-package selection in PR context,
  labels/skip conditions, required-check integration and PR-specific permissions.
  Manual runs do not establish those properties or prove every workload regression-free.

There is no whole-PR speedup or dollar-savings claim, and no isolated measurement
of warm native caches, merge-base checking or the dependency-check runner change.
See [ci-performance.md](ci-performance.md) for current behavior and reproduction
commands. Run benchmark suites sequentially: dispatches sharing a SHA also share
a cancellation group. Preserve metrics before their seven-day retention expires.
