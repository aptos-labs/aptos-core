# Corpus 3.2: Opus 5, Sol 5.6, and Terra 5.6

This report compares the latest full corpus-v3.2 rounds for Claude Opus 5, GPT-5.6 Sol, and GPT-5.6 Terra. Each round contains the same 20 tasks, three inference arms, and four replicas: 240 scheduled cells. All used `high` effort and acceptance-only generation. Ordinary mutants were withheld until generation ended, and a surviving mutant counted as failure without a refutation-feedback retry.

Sol is now represented by run 11 and Terra by run 10. Both reruns captured every underlying Codex request through OpenTelemetry and bound the ordinary disqualification mutants plus the disjoint scoring mutants before generation. Neither round had a request above the 272K long-context threshold, so both have exact cost points.

## Result at a glance

| model round | strict / scheduled | disqualified | unmeasured | canonical generation cost | mean / cell | cost / strict | mean wall |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| Opus 5 Foundry | 230/240 (95.83%) | 6 | 4 | $115.6532 | $0.48189 | $0.50284 | 123.8 s |
| GPT-5.6 Sol exact | 228/240 (95.00%) | 8 | 4 | $83.0532 | $0.34606 | $0.36427 | 121.5 s |
| GPT-5.6 Terra exact | 231/240 (96.25%) | 8 | 1 | $50.4457 | $0.21019 | $0.21838 | 109.7 s |

Terra produced the most strict successes at 231, Opus produced 230, and the new Sol sample produced 228. The spread is 1.25 percentage points. Failures remain concentrated in QP-part-025 and OV-order-006, and four replicas provide limited power, so the data do not establish a model-quality ordering.

Terra costs 56.4% less than Opus and 39.3% less than Sol on canonical task-solving requests. Sol costs 28.2% less than Opus. Terra's cost per strict success is 40.0% below Sol's.

## Protocol and comparability

- Opus used Claude Agent SDK on Microsoft Foundry with explicit `claude-opus-5` and a 1M context. Sol and Terra used Codex CLI 0.153.2.
- The same 20 task identities and the same ordinary and scoring mutant content were used. Sol run 11 and Terra run 10 bound both manifest digests before generation and passed 240-cell audits.
- The rounds used different source/harness commits. Opus used a different round-local Move Flow binary; the two exact Codex rounds used their branch builds.
- Costs are API-equivalent list-price estimates. Codex subscription traffic had no observed marginal token charge; Foundry contract or invoice adjustments are outside this analysis.
- Controller wall time combines provider latency, Move Flow calls, and prover work. It is not a provider-latency or geographic-network measurement.

## Quality by model and arm

`U` denotes an inconclusive or infrastructure-unmeasured quality verdict.

| model | arm | strict | DQ | U | canonical cost | mean / cell | cost / strict |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Opus 5 | agent-only | 75 | 3 | 2 | $37.3875 | $0.46734 | $0.49850 |
| Opus 5 | flexible | 77 | 2 | 1 | $38.6501 | $0.48313 | $0.50195 |
| Opus 5 | guided | 78 | 1 | 1 | $39.6157 | $0.49520 | $0.50789 |
| Sol 5.6 exact | agent-only | 74 | 4 | 2 | $26.6462 | $0.33308 | $0.36008 |
| Sol 5.6 exact | flexible | 77 | 1 | 2 | $28.9652 | $0.36206 | $0.37617 |
| Sol 5.6 exact | guided | 77 | 3 | 0 | $27.4419 | $0.34302 | $0.35639 |
| Terra 5.6 exact | agent-only | 74 | 5 | 1 | $18.8433 | $0.23554 | $0.25464 |
| Terra 5.6 exact | flexible | 78 | 2 | 0 | $16.5974 | $0.20747 | $0.21279 |
| Terra 5.6 exact | guided | 79 | 1 | 0 | $15.0051 | $0.18756 | $0.18994 |

Guided and flexible tie at 77 strict successes in the new Sol round. Guided costs 5.3% less than flexible and has the lowest Sol cost per strict success. Terra guided remains the strongest observed arm on both axes. Opus arms stay within 6.0% of one another, with guided highest in both strict count and spend.

## Quality by task

Each entry is `strict / disqualified / unmeasured` over 12 cells.

| task | Opus 5 | Sol 5.6 exact | Terra 5.6 exact |
| --- | ---: | ---: | ---: |
| `BA-base-012` | 12/0/0 | 12/0/0 | 12/0/0 |
| `BK-bucket-016` | 12/0/0 | 12/0/0 | 12/0/0 |
| `LP-price-021` | 12/0/0 | 12/0/0 | 12/0/0 |
| `MD-median-015` | 12/0/0 | 12/0/0 | 12/0/0 |
| `MM-min-013` | 12/0/0 | 12/0/0 | 12/0/0 |
| `OV-order-006` | 12/0/0 | 8/4/0 | 10/2/0 |
| `PM-curve-027` | 12/0/0 | 12/0/0 | 12/0/0 |
| `QP-part-025` | 6/6/0 | 7/4/1 | 6/6/0 |
| `SM-select-022` | 8/0/4 | 10/0/2 | 11/0/1 |
| `TL-lev-020` | 12/0/0 | 12/0/0 | 12/0/0 |
| `TR-cancel-026` | 12/0/0 | 11/0/1 | 12/0/0 |
| `TR-discard-011` | 12/0/0 | 12/0/0 | 12/0/0 |
| `TR-order-010` | 12/0/0 | 12/0/0 | 12/0/0 |
| `TS-trial-019` | 12/0/0 | 12/0/0 | 12/0/0 |
| `UC-credits-008` | 12/0/0 | 12/0/0 | 12/0/0 |
| `VS-contrib-003` | 12/0/0 | 12/0/0 | 12/0/0 |
| `VS-fees-001` | 12/0/0 | 12/0/0 | 12/0/0 |
| `VS-redeem-004` | 12/0/0 | 12/0/0 | 12/0/0 |
| `VS-shares-002` | 12/0/0 | 12/0/0 | 12/0/0 |
| `WU-consume-023` | 12/0/0 | 12/0/0 | 12/0/0 |

Every conclusive failure occurred in the ordinary disqualification gate; every cell that reached a conclusive separate scoring verdict killed all essential scoring mutants. Sol run 11 lost all four agent-only OV replicas to `OV-order-006-reorder-guards` and four QP replicas to `QP-part-025-lost-element`. Four Sol scoring calls remained prover-inconclusive after a sequential infrastructure-only retry.

Paired corresponding-cell counts also show a narrow quality spread. Opus and Terra were both strict in 225 cells; Opus alone succeeded in 4, Terra alone in 2, both conclusively failed in 4, and 5 lacked a verdict in at least one round. Sol and Terra were both strict in 223; Sol alone succeeded in 4, Terra alone in 5, both conclusively failed in 3, and 5 lacked a verdict. Opus and Sol were both strict in 222; Opus alone succeeded in 6, Sol alone in 3, both conclusively failed in 2, and 7 lacked a verdict.

## Cost accounting at multiple levels

### Token and billed-component level

| model | fresh / cache-write / cache-read input | visible + reasoning output | component costs | total |
| --- | --- | ---: | --- | ---: |
| Opus 5 | 4,710 fresh; 6,391,781 5m writes; 66,056,581 reads | 1,706,110 combined | fresh $0.0236; writes $39.9486; reads $33.0283; output $42.6528 | $115.6532 |
| Sol 5.6 exact | 9,134,277 fresh; 55,298,688 cached | 833,586 visible + 386,246 reasoning | fresh $36.5371; cached $22.1195; visible $16.6717; reasoning $7.7249 | $83.0532 |
| Terra 5.6 exact | 10,275,924 fresh; 69,350,144 cached | 881,107 visible + 454,215 reasoning | fresh $20.5518; cached $13.8700; visible $10.5733; reasoning $5.4506 | $50.4457 |

Opus pricing was $5/M fresh input, $0.50/M cache reads, $6.25/M 5-minute cache writes, and $25/M output. Sol was $4/M fresh, $0.40/M cached, and $20/M output. Terra was $2/M fresh, $0.20/M cached, and $12/M output. Sol and Terra apply 2× input and 1.5× output rates to requests above 272K input. All 2,815 Sol and all 3,237 Terra canonical requests stayed below that threshold.

### Task level

The table reports mean canonical model cost per scheduled cell.

| task | Opus 5 | Sol 5.6 exact | Terra 5.6 exact |
| --- | ---: | ---: | ---: |
| `BA-base-012` | $0.6608 | $0.6151 | $0.7418 |
| `BK-bucket-016` | $0.3035 | $0.3478 | $0.1553 |
| `LP-price-021` | $0.4421 | $0.2699 | $0.1357 |
| `MD-median-015` | $0.2477 | $0.1793 | $0.1019 |
| `MM-min-013` | $0.2830 | $0.2914 | $0.1668 |
| `OV-order-006` | $0.2512 | $0.2033 | $0.0947 |
| `PM-curve-027` | $0.2654 | $0.1900 | $0.1057 |
| `QP-part-025` | $0.7200 | $0.6783 | $0.3333 |
| `SM-select-022` | $0.7740 | $0.5700 | $0.3699 |
| `TL-lev-020` | $0.4647 | $0.3508 | $0.2103 |
| `TR-cancel-026` | $0.5990 | $0.3536 | $0.2106 |
| `TR-discard-011` | $0.4919 | $0.3291 | $0.1751 |
| `TR-order-010` | $0.3092 | $0.2806 | $0.1764 |
| `TS-trial-019` | $0.6784 | $0.2772 | $0.1016 |
| `UC-credits-008` | $0.3137 | $0.3493 | $0.1958 |
| `VS-contrib-003` | $0.4206 | $0.2439 | $0.1432 |
| `VS-fees-001` | $0.6370 | $0.4080 | $0.2361 |
| `VS-redeem-004` | $1.0390 | $0.5477 | $0.3191 |
| `VS-shares-002` | $0.4552 | $0.2224 | $0.1220 |
| `WU-consume-023` | $0.2814 | $0.2133 | $0.1086 |

Task identity remains a major cost driver. In Sol run 11, BA-base-012 is the most expensive task at $0.6151/cell and 8.9% of canonical spend. Terra is cheaper than Sol on every task in these samples and cheaper than Opus on every task except BA-base-012.

### Cell and overhead level

Sol's median canonical cell cost is $0.2946, p75 $0.4022, p95 $0.6912, and maximum $1.4005. Its 245 startup warmups cost $6.4487, making all retained completed requests $89.5019. Terra's corresponding canonical distribution is $0.1681 median, $0.2296 p75, $0.5339 p95, and $1.2739 maximum; its warmups cost $3.2244. Opus startup overhead was not captured at the same request boundary, so canonical cost is the consistent cross-model comparison.

## What the run logs show

| diagnostic | Opus 5 | Sol 5.6 exact | Terra 5.6 exact |
| --- | ---: | ---: | ---: |
| candidate checks | 301 | 338 | 435 |
| first recorded check accepted | 210 (87.5%) | 179 (74.6%) | 147 (61.3%) |
| failed candidate checks | 34 | 95 | 186 |
| compile failures | 2 | 40 | 54 |
| prover failures | 25 | 28 | 84 |
| prover timeouts | not separately reported | 6 | not separately reported |
| forbidden weakenings | 6 | 16 | 19 |
| incomplete contracts | 0 | 3 | 13 |
| policy violations | 0 | 2 | 7 |
| WP engine calls | 218 | 292 | 299 |
| explicit prover calls | 47 | 42 | 88 |
| mean added specification lines | 31.6 | 24.5 | 24.0 |

Opus reached acceptance with the least repair. The new Sol sample sits between Opus and Terra in candidate-check volume. Operational acceptance still did not ensure semantic completeness: each model needed the same withheld gate to expose concentrated QP or OV omissions.

Sol scoring initially had three mutant-application failures where candidate-added loop invariants wrapped the anchored implementation guard. The scorer was changed to map the exact mutant edit span through inserted specification text while checking nearby implementation context. Sequential infrastructure-only retries converted all three to strict successes. Four other sequential retries remained prover-inconclusive; no semantic mutant survivor was retried.

## Wall time and operational events

| model | mean | median | p95 | maximum | summed cell time |
| --- | ---: | ---: | ---: | ---: | ---: |
| Opus 5 | 123.8 s | 93.8 s | 288.4 s | 691.2 s | 8.25 h |
| Sol 5.6 exact | 121.5 s | 91.7 s | 295.6 s | 698.1 s | 8.10 h |
| Terra 5.6 exact | 109.7 s | 75.0 s | 321.2 s | 950.4 s | 7.32 h |

Sol's new mean wall time is 13.6% below the earlier Sol round's 140.6 seconds, but the new round remains 10.7% slower than Terra on mean. Sol generation ran at controller concurrency 3 and completed all 240 cells without an infrastructure failure. Two cells took a second controller turn after a prover timeout. Sequential post-run scoring avoided shared-prover collisions and added no model cost.

## Correction relative to the earlier Sol round

Run 8 reported $118.5750 by applying the 272K surcharge to aggregated controller-turn usage. That boundary was too coarse: a controller turn may contain many smaller underlying model requests. Run 11 observed all 2,815 canonical requests directly; the largest contained 55,857 input tokens, so the exact canonical total is $83.0532 with no long-context premium.

Run 11 is an independent stochastic sample, so its token totals cannot retroactively yield an exact run-8 bill. Run 8 remains useful as a historical quality sample, while run 11 supersedes it for current Sol cost comparison.

## Evidence

- [Opus 5 report](corpus3.2-run9-opus5-foundry.md) and [archive](corpus3.2-run9-opus5-foundry.tar.gz)
- [Sol 5.6 exact report](corpus3.2-run11-codex-sol56-high-exact-cost.md) and [archive](corpus3.2-run11-codex-sol56-high-exact-cost.tar.gz)
- [Terra 5.6 exact report](corpus3.2-run10-codex-terra56-high-exact-cost.md) and [archive](corpus3.2-run10-codex-terra56-high-exact-cost.tar.gz)
- [Earlier Sol report](corpus3.2-run8-codex-sol56.md), retained as a historical quality sample with a pricing correction notice
- [Earlier corrected Terra report](corpus3.2-run9-codex-terra56-high.md), retained for historical comparison
