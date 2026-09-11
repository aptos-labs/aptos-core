# Corpus 3.2: Opus 5, Sol 5.6, and Terra 5.6

This report compares the latest full corpus-v3.2 rounds for Claude Opus 5,
GPT-5.6 Sol, and GPT-5.6 Terra. Each round contains the same 20 tasks, three
inference arms, and four replicas, for 240 scheduled cells. All used `high`
effort and acceptance-only generation; a contract refuted by a withheld mutant
received no model retry.

The original Terra schedule accidentally omitted the ordinary-mutant
disqualification gate. Its published 238/240 result was therefore incomplete.
The unchanged Terra candidates have now been scored against the omitted set.
For every task, the mutant manifest matched the same SHA-256 independently
bound before generation by both Opus and Sol. The corrected Terra result is
226/240 strict successes. The recovery archive is
[`corpus3.2-run9-codex-terra56-posthoc-disqualification.tar.gz`](corpus3.2-run9-codex-terra56-posthoc-disqualification.tar.gz).

## Result at a glance

| model round | strict / scheduled | disqualified | unmeasured | generation cost | mean / scheduled cell | cost / strict success | mean wall time |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| Opus 5 Foundry | 230/240 (95.83%) | 6 | 4 | $115.6532 | $0.48189 | $0.50284 | 123.8 s |
| GPT-5.6 Sol | 231/240 (96.25%) | 9 | 0 | $118.5750 | $0.49406 | $0.51331 | 140.6 s |
| GPT-5.6 Terra | 226/240 (94.17%) | 10 | 4 | $47.8752–$72.6770 | $0.19948–$0.30282 | $0.21184–$0.32158 | 118.4 s |

The observed quality rates differ by at most 2.08 percentage points. Failures
are concentrated in two tasks and the rounds contain only four replicas, so
these counts do not establish a reliable model-quality ordering. Sol has the
highest observed strict count and complete scoring coverage. Opus is one strict
success behind Sol. Terra is five behind Sol after the missing gate is restored.

Terra is the clear cost outlier. Its API-equivalent total is 37.2–58.6% below
Opus and 38.7–59.6% below Sol. Its cost per strict success is 36.0–57.9% below
Opus and 37.4–58.7% below Sol. Opus costs 2.5% less than Sol overall despite
Opus's higher nominal output rate.

## What is comparable

The scored mutant manifests are identical task by task across all three rounds.
The ordinary disqualification manifests were bound in the Opus and Sol
schedules. Terra recorded null digests, but the post-hoc pass verified its live
manifests against both independent bound copies before scoring. No Terra model
session was resumed, no candidate changed, and no mutant feedback was exposed.

The comparison is still descriptive rather than a controlled provider
ablation:

- Opus ran through the Claude Agent SDK on Microsoft Foundry with explicit
  `claude-opus-5` and a 1M-token context. Sol and Terra ran through Codex CLI
  0.153.2.
- Sol and Terra used the same Move Flow binary. Opus used a newer round-local
  binary. Controller harness identities also differ among the rounds.
- Opus and Sol bound the ordinary gate before generation. Terra's identical
  content was verified and applied afterward because of the scheduling error.
- The reported costs are list-price estimates. Terra and Sol were accessed by
  Codex subscription, whose observed marginal token charge was zero. Opus's
  estimate excludes any Foundry contract or invoice adjustment.

## Quality by model and arm

`U` means that no quality verdict was available; it is kept separate from a
conclusive mutant survival.

| model | arm | strict | disqualified | U | generation cost | mean / cell | cost / strict success |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Opus 5 | `agent_only` | 75/80 (93.75%) | 3 | 2 | $37.3875 | $0.46734 | $0.49850 |
| Opus 5 | `hybrid_flexible` | 77/80 (96.25%) | 2 | 1 | $38.6501 | $0.48313 | $0.50195 |
| Opus 5 | `hybrid_guided` | 78/80 (97.50%) | 1 | 1 | $39.6157 | $0.49520 | $0.50789 |
| Sol 5.6 | `agent_only` | 75/80 (93.75%) | 5 | 0 | $36.1884 | $0.45236 | $0.48251 |
| Sol 5.6 | `hybrid_flexible` | 78/80 (97.50%) | 2 | 0 | $43.8193 | $0.54774 | $0.56179 |
| Sol 5.6 | `hybrid_guided` | 78/80 (97.50%) | 2 | 0 | $38.5672 | $0.48209 | $0.49445 |
| Terra 5.6 | `agent_only` | 76/80 (95.00%) | 3 | 1 | $16.9452–$25.5731 | $0.21181–$0.31966 | $0.22296–$0.33649 |
| Terra 5.6 | `hybrid_flexible` | 75/80 (93.75%) | 4 | 1 | $17.0387–$27.1586 | $0.21298–$0.33948 | $0.22718–$0.36211 |
| Terra 5.6 | `hybrid_guided` | 75/80 (93.75%) | 3 | 2 | $13.8913–$19.9452 | $0.17364–$0.24932 | $0.18522–$0.26594 |

The hybrid arms improved the observed strict count for Opus and Sol. For Sol,
guided matched flexible's 78 strict successes at 12.0% lower cost. Opus's three
arms were within 6.0% of one another in total cost, and guided had the highest
quality count as well as the highest cost.

Terra shows a different pattern. Guided remained the cheapest arm by 18.0% at
ordinary rates and had the shortest mean wall time, but all three arms ended
within one strict success after the missing gate was applied. The apparent
pre-recovery advantage for Terra agent-only came from complete measurement,
not stronger held-out behavior.

## Where quality differed

Each entry is `strict / disqualified / unmeasured` over 12 cells.

| task | Opus 5 | Sol 5.6 | Terra 5.6 |
| --- | ---: | ---: | ---: |
| `BA-base-012` | 12/0/0 | 12/0/0 | 11/0/1 |
| `BK-bucket-016` | 12/0/0 | 12/0/0 | 12/0/0 |
| `LP-price-021` | 12/0/0 | 12/0/0 | 12/0/0 |
| `MD-median-015` | 12/0/0 | 12/0/0 | 12/0/0 |
| `MM-min-013` | 12/0/0 | 12/0/0 | 12/0/0 |
| `OV-order-006` | 12/0/0 | 7/5/0 | 12/0/0 |
| `PM-curve-027` | 12/0/0 | 12/0/0 | 12/0/0 |
| `QP-part-025` | 6/6/0 | 8/4/0 | 2/10/0 |
| `SM-select-022` | 8/0/4 | 12/0/0 | 10/0/2 |
| `TL-lev-020` | 12/0/0 | 12/0/0 | 12/0/0 |
| `TR-cancel-026` | 12/0/0 | 12/0/0 | 12/0/0 |
| `TR-discard-011` | 12/0/0 | 12/0/0 | 11/0/1 |
| `TR-order-010` | 12/0/0 | 12/0/0 | 12/0/0 |
| `TS-trial-019` | 12/0/0 | 12/0/0 | 12/0/0 |
| `UC-credits-008` | 12/0/0 | 12/0/0 | 12/0/0 |
| `VS-contrib-003` | 12/0/0 | 12/0/0 | 12/0/0 |
| `VS-fees-001` | 12/0/0 | 12/0/0 | 12/0/0 |
| `VS-redeem-004` | 12/0/0 | 12/0/0 | 12/0/0 |
| `VS-shares-002` | 12/0/0 | 12/0/0 | 12/0/0 |
| `WU-consume-023` | 12/0/0 | 12/0/0 | 12/0/0 |

All conclusive Opus failures were `QP-part-025-lost-element`. Terra failed the
same mutant in ten of twelve cells; only agent-only replica 1 and guided replica
4 killed it. Sol failed `lost-element` in four QP cells and
`OV-order-006-reorder-guards` in five OV cells. Every conclusive failure thus
came from the ordinary disqualification set; all measured scoring-set mutants
were killed.

The missing outcomes are apparatus limitations:

- Opus retained four `SM-select-022` cells as unmeasured after sequential clean
  solver retries.
- Terra had one terminal BA infrastructure failure. Two SM candidates and one
  TR-discard candidate could not receive complete gate verdicts because of
  mutation-anchor incompatibility or a repeated solver inconclusive.
- Sol finished with all 240 cells measured after clean retries for one session
  collision and three scoring-apparatus failures.

Across corresponding task/arm/replica cells, Opus and Sol were both strict in
222 cases; Opus alone was strict in 8, Sol alone in 5, one measured cell failed
both, and four Opus cells were missing. Opus and corrected Terra were both
strict in 222 cases; among cells measured for both, Opus alone was strict in 5,
Terra alone in 1, and 5 failed both. Sol and Terra were both strict in 220;
Sol alone was strict in 7, Terra alone in 6, and 3 failed both. These paired
counts reinforce that the small aggregate difference is driven by concentrated,
model-specific failures rather than a broad ordering over tasks.

## Cost accounting

### Token and component level

| model | input/cache counters | billed output | list-price components | total |
| --- | --- | ---: | --- | ---: |
| Opus 5 | 4,710 fresh; 6,391,781 5m cache writes; 66,056,581 cache reads | 1,706,110 | fresh $0.0236; cache writes $39.9486; cache reads $33.0283; output $42.6528 | $115.6532 |
| Sol 5.6 | 9,094,972 fresh; 54,875,904 cached | 1,193,345 | fresh $52.5094; cached $35.1435; visible output $21.1050; reasoning output $9.8170 | $118.5750 |
| Terra 5.6 | 10,140,640 fresh; 63,392,000 cached | 1,242,961 | ordinary-rate fresh $20.2813; cached $12.6784; output $14.9155; possible long-context premium $0–$24.8017 | $47.8752–$72.6770 |

Opus pricing was $5/M fresh input, $0.50/M cache reads, $6.25/M 5-minute
cache writes, and $25/M output. Thinking is already included in Opus output and
is counted once. Opus used 43% more billed output than Sol, but its low cache-read
rate and high first-check acceptance kept its total slightly below Sol.

Sol pricing was $4/M fresh input, $0.40/M cached input, and $20/M generated
output. Requests above 272K input use 2× input and 1.5× output rates. The total
includes visible plus reasoning output exactly once. The long-context premium
was $36.3778; without it, retained usage would have cost $82.1971.

Terra pricing was $2/M fresh input, $0.20/M cached input, and $12/M generated
output with the same long-context multipliers. Codex receipts aggregate an
entire turn rather than each underlying request, so exact surcharge attribution
is unavailable. The lower value applies ordinary rates everywhere. The upper
value applies the multiplier to every aggregate turn above 272K. This is an
accounting bound, not a confidence interval. The omitted gate and its recovery
use only the prover and add no model cost.

### Work included in the totals

Opus's total covers 240 finalized canonical sessions. Sol's comparable total is
also the retained 240-session cost; four completed excluded replica-5 sessions,
known invalid attempts, and one metered interruption raise its recorded
all-work minimum to $121.6417, with four other interrupted sessions lacking
terminal usage. Terra's total includes all 244 completed attempts: 240 canonical
attempts plus four quarantined infrastructure attempts. These accounting
perimeters should be retained when quoting small Opus-versus-Sol differences.

### Task level

The table reports mean model cost per scheduled cell. Terra remains a range for
the long-context attribution reason above.

| task | Opus 5 | Sol 5.6 | Terra 5.6 |
| --- | ---: | ---: | ---: |
| `BA-base-012` | $0.6608 | $1.1652 | $0.5681–$1.0190 |
| `BK-bucket-016` | $0.3035 | $0.2528 | $0.1553–$0.1956 |
| `LP-price-021` | $0.4421 | $0.2650 | $0.1373–$0.1503 |
| `MD-median-015` | $0.2477 | $0.1887 | $0.0968 |
| `MM-min-013` | $0.2830 | $0.3908 | $0.1639–$0.2109 |
| `OV-order-006` | $0.2512 | $0.1999 | $0.0955 |
| `PM-curve-027` | $0.2654 | $0.1989 | $0.1052 |
| `QP-part-025` | $0.7200 | $1.1166 | $0.2839–$0.4833 |
| `SM-select-022` | $0.7740 | $0.8728 | $0.4220–$0.7287 |
| `TL-lev-020` | $0.4647 | $0.5219 | $0.2043–$0.3603 |
| `TR-cancel-026` | $0.5990 | $0.7772 | $0.2344–$0.4033 |
| `TR-discard-011` | $0.4919 | $0.4390 | $0.1742–$0.2686 |
| `TR-order-010` | $0.3092 | $0.3357 | $0.1884–$0.2799 |
| `TS-trial-019` | $0.6784 | $0.2322 | $0.1106 |
| `UC-credits-008` | $0.3137 | $0.4737 | $0.1745–$0.2474 |
| `VS-contrib-003` | $0.4206 | $0.2489 | $0.1326–$0.1848 |
| `VS-fees-001` | $0.6370 | $0.7879 | $0.2141–$0.3171 |
| `VS-redeem-004` | $1.0390 | $0.8562 | $0.2920–$0.5340 |
| `VS-shares-002` | $0.4552 | $0.3437 | $0.1196–$0.1481 |
| `WU-consume-023` | $0.2814 | $0.2140 | $0.1170 |

Task identity dominates the within-model cost spread. BA, QP, SM, and
VS-redeem recur among the expensive tasks. Terra's ten QP disqualifications are
therefore especially costly failures even though Terra's absolute QP spend is
lower than the other models'.

## What the run logs show

The following counts come from all canonical `flow-events.jsonl`,
`controller-events.jsonl`, and final workspace diffs, rather than from the
headline summaries.

| diagnostic | Opus 5 | Sol 5.6 | Terra 5.6 |
| --- | ---: | ---: | ---: |
| candidate checks | 301 | 334 | 401 |
| first recorded check accepted | 210/240 (87.5%) | 172/240 (71.7%) | 152/240 (63.3%) |
| failed candidate checks | 34 | 92 | 155 |
| compile failures during refinement | 2 | 38 | 49 |
| prover failures during refinement | 25 | 34 | 60 |
| forbidden weakenings | 6 | 14 | 21 |
| incomplete contracts | 0 | 3 | 15 |
| policy violations | 0 | 1 | 7 |
| WP engine calls | 218 | 295 | 301 |
| explicit prover calls | 47 | 43 | 62 |
| mean added specification lines | 31.6 | 25.0 | 21.6 |
| byte-distinct final trees | 235/240 | 183/240 | 177/239 operational |

Opus usually reached an accepted candidate in one check and almost never
produced syntax or compilation errors. Sol needed more repair, and Terra needed
the most: 401 checks and 155 rejected candidates. The contrast is sharp on the
hard tasks. Mean checks per cell for BA were 1.33 Opus, 2.92 Sol, and 3.25 Terra;
for SM they were 1.42, 1.75, and 2.83; for VS-redeem they were 1.42, 2.00, and
2.92. One Terra BA agent-only cell required 12 candidate checks.

Those extra iterations did not translate into a higher model bill for Terra,
because its token rates are substantially lower. They also did not guarantee
semantic completeness: Opus averaged only one candidate check on QP yet failed
the hidden lost-element mutant in half the cells, while Terra averaged 2.58
checks and failed it in ten cells. Operational acceptance proves the candidate
against the original implementation; the disjoint mutant gates are what expose
these missing behavioral constraints.

The hybrid tools changed efficiency differently by model. Sol and Terra
agent-only had the lowest first-check acceptance within their rounds. Terra
guided reduced mean candidate checks from 2.03 in agent-only to 1.38 and was the
only Terra arm to reduce both cost and wall time materially. Opus flexible had
the best first-check acceptance, while guided used more WP calls and slightly
more output, producing its small cost premium.

Repeated exact final trees were more common for Sol and Terra than for Opus.
This shows that the Codex runs more often converged on byte-identical contracts;
it does not by itself imply better or worse specifications.

## Wall time and operational events

| model | mean | median | p95 | maximum | summed cell time |
| --- | ---: | ---: | ---: | ---: | ---: |
| Opus 5 | 123.8 s | 93.8 s | 288.4 s | 691.2 s | 8.25 h |
| Sol 5.6 | 140.6 s | 119.0 s | 304.7 s | 575.4 s | 9.37 h |
| Terra 5.6 | 118.4 s | 98.5 s | 270.5 s | 732.7 s | 7.90 h |

These are controller durations and overlap under concurrent dispatch. They are
useful diagnostics, not elapsed round durations or provider-latency estimates.
All three rounds shared a cooperative gate capped at two machine-wide Boogie
process groups.

Opus was interrupted after four cells because the SDK incorrectly reported a
200K context window for the explicit 1M Opus deployment. It resumed after that
metadata was shown stale; a second launch correction restored the pinned
round-local Flow binary. Sol changed generation concurrency from three to two,
recovered one cross-container session-ID collision and three scoring-apparatus
failures, and finished all 240 scores. Terra suffered a dispatcher outage after
192 cells; three of four affected cells recovered, while one BA cell remained
invalid. Terra later raised dispatch concurrency from three to five. None of
these clean infrastructure recoveries disclosed mutant feedback or counted as
a model repair.

## Evidence

- [Opus 5 round report](corpus3.2-run9-opus5-foundry.md) and
  [archive](corpus3.2-run9-opus5-foundry.tar.gz)
- [Sol 5.6 round report](corpus3.2-run8-codex-sol56.md) and
  [archive](corpus3.2-run8-codex-sol56.tar.gz)
- [Corrected Terra 5.6 report](corpus3.2-run9-codex-terra56-high.md),
  [original archive](corpus3.2-run9-codex-terra56-high.tar.gz), and
  [post-hoc gate archive](corpus3.2-run9-codex-terra56-posthoc-disqualification.tar.gz)

The recovery's compact per-cell result is
[`corpus3.2-run9-codex-terra56-posthoc-disqualification.json`](corpus3.2-run9-codex-terra56-posthoc-disqualification.json).
