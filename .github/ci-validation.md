# Rebased CI validation

Measured on 2026-09-17 UTC. **Validation is not green:** normal workspace CI and
one large baseline sample, including its bounded rerun, failed on a prover output
mismatch. Measurement runs are finished; no further retries were launched.

## Revisions and method

Rebased onto main `aba7ebe8287390be4768c5845d7022b5ccdec512` without conflicts;
the three existing commits retained identical patches. Shipping branch:
`vk/ci-improve-1-validation-20260916`. No PR was opened.

Candidate `6de07db2c04bdd975e13f2e33003458c15dc32f6` also aligns all three
nextest inventory commands with `--profile ci`: newer main excludes Lean tests
through that profile. A regression test failed before the fix and passed after it.
Execution commands, assertions, retry settings and upstream exclusions are unchanged.

Targeted benchmarks use `2b8522e0f93c05d468b2372cb3f9599cebb03104`, with the
identical Git tree `b13ed12a01b41902e24b90b43c5a40bd82b64225`. The separate SHA
prevents concurrent normal CI from cancelling benchmarks. Each suite schedules
three baseline/candidate samples using the same source, packages and profiles.

[CLI/API comparisons](https://github.com/aptos-labs/aptos-core/actions/runs/35233992968)
use candidate test sources on both sides, main's baseline orchestration and its
prebuilt tools image, pinned to
`sha256:e64d0e85cd83b1f6383631a9216ffb9b3a909baf6b632dc6c4ba6c80ad295226`.
The branch does not change product Rust sources relative to that main revision.
Devnet/testnet/mainnet image digests were resolved once for all samples.
The [temporary comparison harness](https://github.com/aptos-labs/aptos-core/commit/7c742e7b5225f7364607752861be136de99cdfef)
is outside shipping history; its adaptations select images and collect diagnostics,
without changing assertions.

## Measurements

Three samples per variant, except large: only successful matched samples one and
three are compared below. Medians and full ranges include job
setup/cleanup/reporting, excluding queueing. CLI time is the slowest network job;
compute sums all three networks. Sharded targeted time is build plus slowest
shard; compute sums build and all shards. Small reporting/result-gate jobs are
excluded. Allocated vCPU-minutes are a proxy, **not billed dollars**.

| Workload | Baseline seconds: median (range) | Candidate seconds: median (range) | Median allocated vCPU-minutes: baseline → candidate | Result |
| --- | --- | --- | --- | --- |
| [Targeted small](https://github.com/aptos-labs/aptos-core/actions/runs/35234005745) | 133 (132–133) | 129 (128–129) | 141.9 → 137.6 | 3.0% less time/compute; modest difference |
| [Targeted medium](https://github.com/aptos-labs/aptos-core/actions/runs/35234419023) | 248 (225–274) | 247 (246–252) | 264.5 → 263.5 | Essentially flat |
| [Targeted large](https://github.com/aptos-labs/aptos-core/actions/runs/35235061862) | 1,927.5 (1,902–1,953) | 890.5 (853–928) | 2,056.0 → 1,819.7 | 53.8% faster; 11.5% less compute; **two successful pairs only** |
| CLI E2E | 526 (508–550) | 255 (236–410) | 561.1 → 99.1 | 51.5% faster; 82.3% less compute |
| API compatibility | 81 (77–91) | 95 (81–101) | 86.4 → 12.7 | 17.3% slower; 85.3% less compute |

Removing benchmark-only inventory collection gives **131s → 129s** for small
(1.5% faster) and **246s → 247s** for medium (0.4% slower). These small differences
are not evidence of a substantial speedup. Unlike the previous revision's run,
the medium gain and API latency improvement did not repeat.
For the two large pairs, removing inventory overhead gives **1,925.5s → 885s**
(54.0% faster) and **2,053.9 → 1,796.0 vCPU-minutes** (12.6% lower).
Including build-to-shard scheduling gaps gives 922s (884–960), 52.2% faster.
Including network scheduling and the result gate, CLI's median is 261s
(243–416), still 50.4% faster. Across all three samples, including the retry,
CLI's total allocated compute is 81.7% lower.

API's first spec-generation step, including image pull, took a median 37s → 44s;
checkout took 3s → 7s. All six YAML/JSON comparison jobs passed on their first
attempt. API latency remains variable; its repeatable benefit is lower allocation.

Small inventories matched nine tests in every sample. Medium matched 2,354 total
identities and 2,346 runnable tests, with eight CI-profile exclusions and no ignored
tests. No small/medium retries occurred. Large candidate inventories contain
3,651 identities: 29 ignored, eight CI-profile exclusions and 3,614 runnable
tests. Local checks of downloaded inventories verified exact baseline/candidate
identity and ignore-status equality for samples one and three, and exact,
disjoint runnable partition coverage for all three candidate samples. Sample
two has no baseline inventory because its test step failed; the remote
three-sample verifier therefore remains failed. All 24 candidate shards passed.
Across all three candidate samples, the median was 896s and 1,829.9 vCPU-minutes;
sample two is excluded from the paired comparison above, not counted as a third
successful pair.
Candidate sample three's prover test `choice.move` had output-baseline differences
on its first three attempts and passed on its fourth; retry time is included.
Baseline samples one and three each passed on a second attempt for `choice.move`
and `closures/inline/folds_of_idx.move`, respectively.
Baseline sample two's [initial attempt](https://github.com/aptos-labs/aptos-core/actions/runs/35235061862/attempts/1)
exhausted all four attempts on `choice.move`; its prover
`closures/amm_example.move` also needed a second attempt. The failed job consumed
**1,961s / 2,091.7 allocated vCPU-minutes**. The single
[rerun](https://github.com/aptos-labs/aptos-core/actions/runs/35235061862/attempts/2)
failed on the same test after four attempts: 3,613 passed, one failed, 37 skipped.
It consumed **2,120s / 2,261.3 allocated vCPU-minutes**. The mismatch adds a
redacted return value and a duplicate `simple_incorrect` trace frame, not a test
timeout. Both failed jobs are excluded from successful-pair medians but retained
as **4,353.1 additional allocated vCPU-minutes** of validation cost. The two
failed inventory verifiers add 28s / 0.9 vCPU-minutes. Reused successful jobs were
counted once. These success-conditioned, two-pair results have limited confidence;
they do not establish reliability or a clean three-sample benchmark.

All 18 CLI network suites passed, with 41 tests per network. One candidate
sample's testnet suite needed a second attempt:
`test_node_update_consensus_key` and `test_node_update_validator_network_address`
encountered `ERECONFIGURATION_IN_PROGRESS`. The retry passed, and all retry time
is included. Unchanged assertions do not establish that runner sizing has no
effect on flake frequency.

Candidate phase medians across all three samples (small / medium / large): prover
setup **8.3 / 8.0 / 8.2s**, build/archive **65.7 / 146.3 / 463.8s**, and inline/slowest-shard tests
**0.2 / 42.4 / 309.1s**. Large archives were 781 MB; shard downloads took a median
18s (15–48s), with peak sampled shard memory of 8.0 GB.
Phase medians are not additive. Targeted builders used c7i-flex.16xlarge;
candidate shards used c7i-flex.8xlarge.
CLI/API runner families can vary, so these are practical CI comparisons rather
than architecture-controlled CPU benchmarks.

### Smoke artifact transfer

The [transfer comparison](https://github.com/aptos-labs/aptos-core/actions/runs/35235523714)
reuses the exact freshly built rebased smoke binaries. All four file checksums
matched in all six consumers.

- Artifact bytes: **13.26 GB → 3.65 GB (72.5% smaller)**.
- Download/extraction, three samples per format: median **309s → 98s (68.3%
  faster)**; ranges 289–336s and 88–119s.
- Upload tradeoff, one controlled producer comparison: node **16s → 38s**;
  helpers/archive **42s → 119s**.

This compares artifact compression, not the full smoke workflow against its
former cache backend. Transfer percentages are not whole-smoke-check speedups.

## Correctness and remaining checks

- Local: **41 tests passed** (13 Node, 13 Python metrics, six CLI assertion,
  nine Rust), plus Cargo check, Clippy, formatting and workflow linting.
- The small benchmark passed Linux helper tests, exact inventories, and the live
  missing-artifact failure contract. Freshness now passes: merge base age zero
  days against a seven-day limit.
- [CLI dependency check](https://github.com/aptos-labs/aptos-core/actions/runs/35233362096)
  passed.
- [Normal CI](https://github.com/aptos-labs/aptos-core/actions/runs/35233358677)
  checks out rebased `30aed347bd06c2d4bb2fc1861ff1dcc18394acca`; its test actions
  are unchanged by the later inventory-only fix. General/Rust lints, licenses,
  batch-encryption, cached-package consistency, VM-feature checks and doctests
  passed (63 doctests passed, 36 ignored).
- **Workspace failure:** 12,822 passed, one failed, 171 skipped. `choice.move`
  exhausted all four attempts with output-baseline differences. The unchanged
  `ping_success_resets_fail_counter` timed out once and `folds_of_idx.move` hit
  prover resource/time limits once; each passed on its second attempt.
  The job took **60m 41s / 3,883.7 allocated vCPU-minutes** and was not rerun.
  Prover sources, unit-test action and nextest configuration are unchanged from
  main; the same `choice.move` discrepancy also occurred in the old-path benchmark.
  This does not establish a clean regression-free result; triage the flake before
  claiming full validation. No assertion, baseline or timeout was weakened.
- All eight smoke partitions passed: **143 executed tests, 48 ignored**.
  `test_swarm_with_bad_non_qs_node` twice timed out waiting for a ledger version,
  then passed on its third attempt. The slowest measured test phase was 714.5s;
  peak sampled host memory was 27.2 GB.

A real PR is still needed to exercise affected-package selection in PR context,
labels/skip conditions, required-check integration and PR-specific permissions.
No whole-PR speedup, universal regression-free result, or billed-dollar savings
is claimed. Warm native caches and the dependency-check runner change have no
isolated performance measurement.

See [ci-performance.md](ci-performance.md) for current behavior and reproduction
commands. Run benchmark suites sequentially; dispatches sharing a SHA share a
cancellation group. Metrics expire after seven days and build artifacts after one.
