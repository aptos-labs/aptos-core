# Final CI validation

Measured on 2026-09-22–23 UTC. **Full Lint+Test CI and all benchmark workflows
passed.** No workflow/job reruns were launched. Existing test-level
retries remain enabled and their time is included below.

## Tested revisions

Shipping branch: `vk/ci-improve-1-validation-20260916`.
Tested code: [`7dcf3cf36b`](https://github.com/aptos-labs/aptos-core/commit/7dcf3cf36b471ea8ba1115546aadb73f0de6348d),
based on main `aba7ebe8287390be4768c5845d7022b5ccdec512`. No PR was opened.
The subsequent validation-report commit changes only this document.

Targeted comparisons use `bf4774c576dd26972e17bfd0f17783b4922d2b7a`, whose
Git tree is identical to the tested code. Its separate SHA prevents benchmark
dispatches from cancelling normal CI. All targeted baseline/candidate pairs build the
same fixed source with the same packages, profiles, exclusions, and retry policy.

## Measured performance

Three samples per variant; all samples completed successfully. Values are
medians, with full ranges in parentheses. Times include job setup, cleanup,
reporting, and test retries, but exclude queueing and small result-gate jobs.
Large targeted time is build plus the slowest shard; CLI time is the slowest
network job. Compute sums every builder/shard or network job.
**Allocated vCPU-minutes are an allocation proxy, not billed dollars.**

| Workload | Baseline seconds | Candidate seconds | Allocated vCPU-minutes: baseline → candidate | Result |
| --- | --- | --- | --- | --- |
| [Targeted small](https://github.com/aptos-labs/aptos-core/actions/runs/35799635052) | 116 (114–125) | 122 (118–129) | 123.7 → 130.1 | 5.2% slower; 5.2% more allocation |
| [Targeted medium](https://github.com/aptos-labs/aptos-core/actions/runs/35799885461) | 218 (212–220) | 219 (218–235) | 232.5 → 233.6 | Essentially flat |
| [Targeted large](https://github.com/aptos-labs/aptos-core/actions/runs/35800277593) | 1,898 (1,890–1,921) | 879 (863–1,030) | 2,024.5 → 1,848.0 | 53.7% faster; 8.7% less allocation |
| [CLI E2E](https://github.com/aptos-labs/aptos-core/actions/runs/35799660366) | 518 (513–524) | 419 (254–431) | 552.5 → 123.3 | 19.1% faster; 77.7% less allocation |
| [API compatibility](https://github.com/aptos-labs/aptos-core/actions/runs/35799660366) | 85 (82–88) | 85 (80–87) | 90.7 → 11.3 | Same median time; 87.5% less allocation |

Removing benchmark-only inventory collection gives small **114s → 122s**
(7.0% slower), medium **216s → 219s** (1.4% slower), and large
**1,896s → 873s** (54.0% faster). Large allocation becomes
**2,022.4 → 1,821.9 vCPU-minutes** (9.9% lower).
Including build-to-shard scheduling gaps, large's median is **908s**, still
52.2% faster than the baseline. Including network scheduling and the result gate,
CLI's median is **424s**, 18.1% faster. Small's added time is concentrated in database
startup/readiness; build times were nearly identical. Small and medium do not
show a meaningful speed benefit.

### Smoke artifact transfer

The [controlled transfer comparison](https://github.com/aptos-labs/aptos-core/actions/runs/35801025680)
uses the exact fresh smoke binaries from normal CI. All four file checksums
matched in all six consumers.

- Artifact bytes: **13.26 GB → 3.65 GB (72.5% smaller)**.
- Download/extraction: **299s → 100s (66.6% faster)**; three samples per format,
  ranges 261–303s and 100–162s.
- Upload tradeoff, one producer comparison: node **16s → 36s**;
  test/helper bundle **44s → 120s**.

This isolates compression, not the full smoke workflow or the former cache
backend. Transfer savings are not whole-smoke-check speedups.

## Correctness and retries

- [Full Lint+Test](https://github.com/aptos-labs/aptos-core/actions/runs/35799538493):
  **12,826 workspace tests passed, 171 skipped**; 63 doctests passed, 36 ignored.
  All eight smoke partitions passed (**143 executed tests**), as did lints,
  licenses, batch-encryption tests, cached-package consistency, and result gates.
  The workspace job took 61m 41s; this is not a matched workspace speed comparison.
- [CLI dependency check](https://github.com/aptos-labs/aptos-core/actions/runs/35799540643)
  passed. The small comparison also passed Linux helper tests, merge-base
  freshness, and the missing-artifact failure contract.
- Every targeted sample passed exact baseline/candidate inventory matching.
  Small: nine runnable tests. Medium: 2,354 identities, 2,346 runnable tests,
  eight CI-profile exclusions. Large: 3,654 identities, 3,617 runnable tests,
  29 ignored and eight CI-profile exclusions. All 24 large shards passed, with
  exact, non-overlapping partition coverage.
- `choice.move` passed on its first attempt in **all seven executions**: six
  large benchmark executions plus full workspace CI. The three new tracing
  regression tests also passed in both modes and full CI.
  The `choice.move` source and golden output are unchanged; the fix suppresses synthetic
  return-local traces while preserving actual result traces.
- Small/medium and all three large baselines were retry-free. Large candidate
  sample one's `folds_of_idx.move` hit prover resource/time limits twice, then
  passed on its third attempt; workspace CI passed that test on its second attempt.
- Two smoke tests passed on their second attempts: `test_swarm_with_bad_non_qs_node`
  timed out waiting for a ledger version; `optimistic_verification` failed its
  randomness-progress assertion. No test or timeout was changed to make them pass.
- All 18 CLI network suites and six API comparison jobs passed. Two of nine
  candidate CLI network suites (sample two testnet and sample three mainnet)
  needed second attempts after validator-update tests encountered
  `ERECONFIGURATION_IN_PROGRESS`; all nine baseline suites passed first try.
  These retries prevent a blanket no-regression claim for runner downsizing or sharding.

No assertion, golden output, timeout, or retry policy was weakened for validation.

## Scope and reproduction

The [temporary CLI/API and transfer harness](https://github.com/aptos-labs/aptos-core/commit/d6ca61110d851c2263cd86b1c56f63d5bb5c20e0)
is outside shipping history. CLI/API comparisons use current test sources,
baseline/candidate orchestration, and identical prebuilt tools images. The main
tools image is pinned to
`sha256:e64d0e85cd83b1f6383631a9216ffb9b3a909baf6b632dc6c4ba6c80ad295226`;
network digests were resolved once for the entire batch. These comparisons
isolate workflow performance; they do not validate a freshly built candidate
Docker image containing the prover fix. Normal workspace tests, targeted
benchmarks, and smoke binaries build the fixed source.

These are three-sample practical CI comparisons, not architecture-controlled
CPU benchmarks or a whole-PR speedup. Warm native caches and the dependency-check
runner change have no isolated performance measurement. A real PR is still
needed to exercise affected-package selection, labels/skip conditions,
required-check integration, and PR-specific permissions.

See [ci-performance.md](ci-performance.md) for current behavior and reproduction
commands. Run benchmark suites sequentially because dispatches sharing a SHA
share a cancellation group. Metrics expire after seven days and build artifacts
after one day.
