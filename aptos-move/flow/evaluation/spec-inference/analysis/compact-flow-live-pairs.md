# Compact Flow live-pair analysis

Date: 2026-09-11  
Branch: `wrwg/inf-codex`  
Source commit: `f8538c4e08ef3c59ca96c4b65ff0a4a1af23212c`  
Model: `gpt-5.6-sol`, high effort  
New `move-flow` SHA-256: `06eafaeabd8f2f43718763d42a2232ea37226e084005db733efc7ab6f0d10443`

## Question

Do the compact Flow queries, bounded WP diagnostics/conditions, minimized status guidance,
apparatus-hash memoization, and improved stale-condition diagnostic reduce agent context and
cost without reducing specification quality?

This is a targeted, post-hoc diagnostic, not a confirmatory arm comparison. `QP-part-025` was
selected because guided was expensive in the previous round. `BA-base-012` was added because
partition is a textbook algorithm; BA is private Etna code and requires an invented recursive
sum and monotonicity lemma.

Each target ran two fresh replicas of `agent_only` (AO) and `hybrid_guided` (HG), with balanced
arm order and concurrency 2. Both rounds used newly rendered Codex Flow plugins from the new
binary. The visible refutation set was enabled, while final scoring used the disjoint held-out
mutant set.

## Outcome and first-pass quality

| target | arm | first-pass success | eventual strict success | mean first-pass output tokens | mean eventual output tokens | mean aggregate input tokens | mean controller wall time |
|---|---|---:|---:|---:|---:|---:|---:|
| QP | AO | 2/2 | 2/2 | 12,471 | 12,471 | 789,330 | 9.9 min |
| QP | HG | 1/2 | 2/2 | 22,179 | 28,458 | 1,457,901 | 20.1 min |
| BA | AO | 2/2 | 2/2 | 12,134 | 12,134 | 518,978 | 4.5 min |
| BA | HG | 2/2 | 2/2 | 14,160 | 14,160 | 759,258 | 6.0 min |

QP HG replica 1 failed visible-mutant refutation after 15,679 output tokens and recovered with
another 12,558, for 28,237 total. The agent was not told in advance that a repair attempt would
be offered, so the first attempt can be treated as a no-retry failure. QP HG replica 2 passed on
its first refutation attempt at 28,678 tokens. Both AO cells passed first attempt at 12,323 and
12,619 tokens.

BA controls for this quality problem: all four cells passed first attempt and all four final
contracts killed all three held-out mutants. HG was cheaper in replica 1 (10,251 versus 12,842,
-20.2%) and more expensive in replica 2 (18,068 versus 11,425, +58.1%). On the two-replica mean,
HG used 16.7% more output, 46.3% more aggregate input, and 34.2% more wall time than AO.

## Did compact output reduce context pressure?

Yes, strongly at the Flow boundary and directionally in Codex input usage.

| target | arm | old mean Flow reply bytes | new mean Flow reply bytes | change | old mean input tokens | new mean input tokens | change |
|---|---|---:|---:|---:|---:|---:|---:|
| QP | AO | 1,367,371 | 29,827 | -97.8% | 1,091,305 | 789,330 | -27.7% |
| QP | HG | 873,172 | 8,978 | -99.0% | 1,710,379 | 1,457,901 | -14.8% |
| BA | AO | 1,219,434 | 16,510 | -98.6% | 641,316 | 518,978 | -19.1% |
| BA | HG | 1,049,127 | 31,292 | -97.0% | 987,207 | 759,258 | -23.1% |

The old means use the four corresponding replicas from
`corpus3.2-run7-codex-sol56-high`; the new means use two replicas. These are historical-cohort
comparisons, not paired random draws, so the percentages describe the observed apparatus shift
rather than a precise causal effect.

The behavior change is unambiguous:

- The eight new cells made **zero** `move_package_status` calls. The corresponding old QP/BA
  AO+HG cells made 46 status calls across 16 cells.
- Every new structural query was a function-scoped `function_usage` query. No new cell called
  package-wide `facts` or `module_summary`.
- BA query replies fell to 404 bytes per run from roughly 1.0–1.2 MB per run in the old cohorts.
- On QP HG, mean WP reply bytes fell from 40,535 to 4,534 (-88.8%), showing that the WP bounds
  matter on the generated-condition-heavy target. BA's WP replies were already small.
- No new QP WP output was a stale/no-op write; all three successful WP writes recorded a
  nonzero condition count.

Codex reports aggregate input over its internal model calls, not the peak context of a single
call. The CLI does not emit a direct peak-context measurement. The smaller Flow replies are
therefore the direct context-pressure measure; the 15–28% reduction in aggregate input is
consistent with it. Most input was cache-read input. On BA, uncached input fell only 2.9% for AO
and 7.7% for HG, while cache-read input fell 20.6% and 24.1%, respectively. This is the expected
shape if repeated context became smaller.

## Interpretation

The apparatus changes achieved their immediate purpose: redundant compilation/status work and
package-wide structural dumps disappeared, and the agent-visible Flow payload became roughly
two orders of magnitude smaller. BA then retained 4/4 first-pass and 4/4 strict success, so its
input reduction is not an early-failure artifact.

The pilot does **not** establish that HG is now cheaper than AO. BA's HG mean remains higher on
output, aggregate input, and time, with substantial two-replica variance. QP remains a poor
efficiency example: one of two HG cells required recovery and HG was much more expensive even
when it passed first attempt. QP is useful as a stress test for bounded WP output, but not by
itself as evidence for hybrid efficiency.

The miner's `cost USD` field is not usable for these Codex subscription runs: Codex telemetry
does not emit invoice or API-equivalent USD cost. A rendered `0.00` means missing cost data, not
free execution. Token, reply-byte, and wall-time measures are the valid efficiency measures here.

## Recommendation

Rerun the selected corpus under a new round ID with this binary and these freshly rendered
skills. There is a concrete reason: the measured apparatus now feeds dramatically less text to
the model while preserving strict success on the non-textbook BA check. The old round cannot be
silently reinterpreted because both binary and skill hashes changed.

Treat a four-replica corpus rerun as an exploratory replacement comparable to run 7. If the goal
is to resolve a roughly 25% arm difference rather than observe direction, retain the design
warning that approximately 8–10 replicas per task-arm cell are needed at the observed variance.
Report per-task first-pass success and cost before pooling, and preserve eventual recovery as a
separate outcome.

## Artifacts

- QP round: `evaluation-artifacts/corpus3.2-qp-paired-f853-sol56-high-r2/`
- BA round: `evaluation-artifacts/corpus3.2-ba-paired-f853-sol56-high-r2/`
- Both preflights passed, both audits report zero infrastructure-invalid runs, and both held-out
  mutation summaries report 4/4 strict successes.
