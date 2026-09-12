# Corpus 3.2: Opus 5, Sol 5.6, and Terra 5.6

This report compares the latest full corpus-v3.2 rounds for Claude Opus 5, GPT-5.6 Sol, and GPT-5.6 Terra. Each round contains the same 20 tasks, three inference arms, and four replicas: 240 scheduled cells. All used `high` effort and acceptance-only generation. Ordinary mutants were withheld until generation ended, and a surviving mutant counted as failure without a refutation-feedback retry.

Terra is now represented by run 10, a complete rerun whose request-level OpenTelemetry removes the prior pricing interval and whose schedule binds the same disqualification gate used by Opus and Sol.

## Result at a glance

| model round | strict / scheduled | disqualified | unmeasured | canonical generation cost | mean / cell | cost / strict | mean wall |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| Opus 5 Foundry | 230/240 (95.83%) | 6 | 4 | $115.6532 | $0.48189 | $0.50284 | 123.8 s |
| GPT-5.6 Sol | 231/240 (96.25%) | 9 | 0 | $118.5750 | $0.49406 | $0.51331 | 140.6 s |
| GPT-5.6 Terra exact | 231/240 (96.25%) | 8 | 1 | $50.4457 | $0.21019 | $0.21838 | 109.7 s |

The observed strict rates differ by only 0.42 percentage points. Terra and Sol tie at 231 strict successes; Opus is one behind. The data do not establish a model-quality ordering because failures cluster in QP-part-025 and OV-order-006, four replicas provide limited power, and the rounds used different providers and harness commits.

Terra costs 56.4% less than Opus and 57.5% less than Sol on canonical task-solving requests. Its cost per strict success is 56.6% below Opus and 57.5% below Sol. Terra also recorded $3.2244 of startup warmups, making all retained completed requests $53.6701; request-level telemetry for warmups was unavailable in the older rounds, so the canonical total is the consistent comparison.

## Protocol and comparability

- Opus used Claude Agent SDK on Microsoft Foundry with explicit `claude-opus-5` and a 1M context. Sol and Terra used Codex CLI 0.153.2.
- The same 20 task identities and the same ordinary and scoring mutant content were used. Terra run 10 bound both manifest digests before generation and passed a 240-cell audit.
- The rounds used different source/harness commits. Opus also used a different round-local Move Flow binary; Sol and the earlier Terra round shared another binary, while Terra run 10 used the newest branch build.
- Costs are API-equivalent list-price estimates. Codex subscription traffic had no observed marginal token charge; Foundry contract or invoice adjustments are outside this analysis.
- Controller wall time combines provider latency, Move Flow calls, and prover work. It is not a provider-latency or geographic-network measurement.

## Quality by model and arm

`U` denotes an inconclusive or infrastructure-unmeasured quality verdict.

| model | arm | strict | DQ | U | canonical cost | mean / cell | cost / strict |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Opus 5 | agent-only | 75 | 3 | 2 | $37.3875 | $0.46734 | $0.49850 |
| Opus 5 | flexible | 77 | 2 | 1 | $38.6501 | $0.48313 | $0.50195 |
| Opus 5 | guided | 78 | 1 | 1 | $39.6157 | $0.49520 | $0.50789 |
| Sol 5.6 | agent-only | 75 | 5 | 0 | $36.1884 | $0.45236 | $0.48251 |
| Sol 5.6 | flexible | 78 | 2 | 0 | $43.8193 | $0.54774 | $0.56179 |
| Sol 5.6 | guided | 78 | 2 | 0 | $38.5672 | $0.48209 | $0.49445 |
| Terra 5.6 | agent-only | 74 | 5 | 1 | $18.8433 | $0.23554 | $0.25464 |
| Terra 5.6 | flexible | 78 | 2 | 0 | $16.5974 | $0.20747 | $0.21279 |
| Terra 5.6 | guided | 79 | 1 | 0 | $15.0051 | $0.18756 | $0.18994 |

Hybrid guided is Terra’s strongest observed arm on both axes. It made 962 canonical requests, compared with 1,128 for flexible and 1,147 for agent-only. For Sol, guided matched flexible quality at 12.0% lower cost. Opus arms were within 6.0% of one another, with guided highest in both strict successes and spend.

## Quality by task

Each entry is `strict / disqualified / unmeasured` over 12 cells.

| task | Opus 5 | Sol 5.6 | Terra 5.6 exact |
| --- | ---: | ---: | ---: |
| `BA-base-012` | 12/0/0 | 12/0/0 | 12/0/0 |
| `BK-bucket-016` | 12/0/0 | 12/0/0 | 12/0/0 |
| `LP-price-021` | 12/0/0 | 12/0/0 | 12/0/0 |
| `MD-median-015` | 12/0/0 | 12/0/0 | 12/0/0 |
| `MM-min-013` | 12/0/0 | 12/0/0 | 12/0/0 |
| `OV-order-006` | 12/0/0 | 7/5/0 | 10/2/0 |
| `PM-curve-027` | 12/0/0 | 12/0/0 | 12/0/0 |
| `QP-part-025` | 6/6/0 | 8/4/0 | 6/6/0 |
| `SM-select-022` | 8/0/4 | 12/0/0 | 11/0/1 |
| `TL-lev-020` | 12/0/0 | 12/0/0 | 12/0/0 |
| `TR-cancel-026` | 12/0/0 | 12/0/0 | 12/0/0 |
| `TR-discard-011` | 12/0/0 | 12/0/0 | 12/0/0 |
| `TR-order-010` | 12/0/0 | 12/0/0 | 12/0/0 |
| `TS-trial-019` | 12/0/0 | 12/0/0 | 12/0/0 |
| `UC-credits-008` | 12/0/0 | 12/0/0 | 12/0/0 |
| `VS-contrib-003` | 12/0/0 | 12/0/0 | 12/0/0 |
| `VS-fees-001` | 12/0/0 | 12/0/0 | 12/0/0 |
| `VS-redeem-004` | 12/0/0 | 12/0/0 | 12/0/0 |
| `VS-shares-002` | 12/0/0 | 12/0/0 | 12/0/0 |
| `WU-consume-023` | 12/0/0 | 12/0/0 | 12/0/0 |

Terra’s six QP failures, Opus’s six QP failures, and four of Sol’s failures came from `QP-part-025-lost-element`. Terra and Sol also failed `OV-order-006-reorder-guards` in two and five cells respectively. Every conclusive failure occurred in the ordinary disqualification gate; every cell that reached the separate scoring set killed all essential scoring mutants.

Paired corresponding-cell counts reinforce the narrow aggregate differences. Opus and Terra were both strict in 225 cells; Opus alone succeeded in 4, Terra alone in 2, both failed in 4, and 5 cells lacked a verdict in at least one round. Sol and Terra were both strict in 226; Sol alone succeeded in 4, Terra alone in 5, both failed in 4, and one Terra cell was unmeasured.

## Cost accounting at multiple levels

### Token and billed-component level

| model | fresh / cache-write / cache-read input | visible + reasoning output | component costs | total |
| --- | --- | ---: | --- | ---: |
| Opus 5 | 4,710 fresh; 6,391,781 5m writes; 66,056,581 reads | 1,706,110 combined | fresh $0.0236; writes $39.9486; reads $33.0283; output $42.6528 | $115.6532 |
| Sol 5.6 | 9,094,972 fresh; 54,875,904 cached | 756,817 visible + 436,528 reasoning | fresh $52.5094; cached $35.1435; visible $21.1050; reasoning $9.8170 | $118.5750 |
| Terra 5.6 | 10,275,924 fresh; 69,350,144 cached | 881,107 visible + 454,215 reasoning | fresh $20.5518; cached $13.8700; visible $10.5733; reasoning $5.4506 | $50.4457 |

Opus pricing was $5/M fresh input, $0.50/M cache reads, $6.25/M 5-minute cache writes, and $25/M output. Sol was $4/M fresh, $0.40/M cached, and $20/M output. Terra was $2/M fresh, $0.20/M cached, and $12/M output. Sol and Terra apply 2× input and 1.5× output rates to requests above 272K input. Sol incurred $36.3778 of long-context premium. Terra’s 3,237 canonical requests all stayed below that threshold, so its result is an exact point estimate.

### Task level

The table reports mean canonical model cost per scheduled cell.

| task | Opus 5 | Sol 5.6 | Terra 5.6 exact |
| --- | ---: | ---: | ---: |
| `BA-base-012` | $0.6608 | $1.1652 | $0.7418 |
| `BK-bucket-016` | $0.3035 | $0.2528 | $0.1553 |
| `LP-price-021` | $0.4421 | $0.2650 | $0.1357 |
| `MD-median-015` | $0.2477 | $0.1887 | $0.1019 |
| `MM-min-013` | $0.2830 | $0.3908 | $0.1668 |
| `OV-order-006` | $0.2512 | $0.1999 | $0.0947 |
| `PM-curve-027` | $0.2654 | $0.1989 | $0.1057 |
| `QP-part-025` | $0.7200 | $1.1166 | $0.3333 |
| `SM-select-022` | $0.7740 | $0.8728 | $0.3699 |
| `TL-lev-020` | $0.4647 | $0.5219 | $0.2103 |
| `TR-cancel-026` | $0.5990 | $0.7772 | $0.2106 |
| `TR-discard-011` | $0.4919 | $0.4390 | $0.1751 |
| `TR-order-010` | $0.3092 | $0.3357 | $0.1764 |
| `TS-trial-019` | $0.6784 | $0.2322 | $0.1016 |
| `UC-credits-008` | $0.3137 | $0.4737 | $0.1958 |
| `VS-contrib-003` | $0.4206 | $0.2489 | $0.1432 |
| `VS-fees-001` | $0.6370 | $0.7879 | $0.2361 |
| `VS-redeem-004` | $1.0390 | $0.8562 | $0.3191 |
| `VS-shares-002` | $0.4552 | $0.3437 | $0.1220 |
| `WU-consume-023` | $0.2814 | $0.2140 | $0.1086 |

Task identity dominates cost within each model. BA-base-012 is the most expensive Terra task at $0.7418/cell and 17.6% of its total spend. QP, SM, and VS-redeem also recur among the expensive tasks. Terra is cheaper than both other models on every task except BA versus Opus, where Terra costs 12.3% more.

### Cell and overhead level

Terra’s median canonical cell cost is $0.1681, p75 $0.2296, p95 $0.5339, and maximum $1.2739. Its retained warmups cost $3.2244. Two retained clean infrastructure retries account for $1.7971 of canonical cost and $0.0266 of warmup cost, already included. Excluded gate-incident artifacts contain $1.8420 of completed-response cost; they are documented separately and excluded from the canonical 240-cell comparison.

Opus covers 240 finalized canonical sessions. Sol’s comparison total covers its retained 240 sessions; known invalid and excluded work raises its recorded all-work minimum. These differing overhead observability boundaries matter more for operational accounting than for the large Terra-versus-other-model price gap.

## What the run logs show

| diagnostic | Opus 5 | Sol 5.6 | Terra 5.6 exact |
| --- | ---: | ---: | ---: |
| candidate checks | 301 | 334 | 435 |
| first recorded check accepted | 210 (87.5%) | 172 (71.7%) | 147 (61.3%) |
| failed candidate checks | 34 | 92 | 186 |
| compile failures | 2 | 38 | 54 |
| prover failures | 25 | 34 | 84 |
| forbidden weakenings | 6 | 14 | 19 |
| incomplete contracts | 0 | 3 | 13 |
| policy violations | 0 | 1 | 7 |
| WP engine calls | 218 | 295 | 299 |
| explicit prover calls | 47 | 43 | 88 |
| mean added specification lines | 31.6 | 25.0 | 24.0 |
| byte-distinct final trees | 235 | 183 | 182 |

Opus reached acceptance with substantially less repair. Terra performed the most candidate checks and prover calls, yet its lower rates kept its price far below the other models. Operational acceptance alone did not ensure semantic completeness: the disqualification gate exposed concentrated QP and OV omissions after generation.

## Wall time and operational events

| model | mean | median | p95 | maximum | summed cell time |
| --- | ---: | ---: | ---: | ---: | ---: |
| Opus 5 | 123.8 s | 93.8 s | 288.4 s | 691.2 s | 8.25 h |
| Sol 5.6 | 140.6 s | 119.0 s | 304.7 s | 575.4 s | 9.37 h |
| Terra 5.6 exact | 109.7 s | 75.0 s | 321.2 s | 950.4 s | 7.32 h |

Terra has the lowest mean, median, and summed controller time, but the highest maximum and slightly higher p95 than Sol. Its long tail is dominated by BA-base-012 and prover work. The earlier Terra round averaged 118.4 seconds; the new round is 7.3% faster on mean wall time. This does not isolate Berlin-versus-US latency because the runs include different contention, model paths, tool use, and prover durations.

Terra run 10 tested an external cross-container prover gate during generation. It deadlocked three BA cells because paused controller groups retained slots needed by prover children. The gate was removed, the three affected final cells were rerun cleanly, and the polluted result is excluded. The final audit accepted 240/240 cells. Sequential post-run scoring took about 50 minutes and added no model cost.

## Relation to the earlier Terra round

The earlier Terra run produced 226 strict successes, 10 disqualifications, and 4 unmeasured cells after its missing gate was repaired post hoc. Its request-aggregated receipts supported only $47.8752–$72.6770 because the 272K surcharge could not be assigned to underlying requests. Run 10 produced 231/8/1 and an exact $50.4457 canonical total. The exact value lies near the earlier lower bound, but the rounds are independent stochastic samples; quality and cost changes should not be attributed solely to telemetry or scheduling fixes.

## Evidence

- [Opus 5 report](corpus3.2-run9-opus5-foundry.md) and [archive](corpus3.2-run9-opus5-foundry.tar.gz)
- [Sol 5.6 report](corpus3.2-run8-codex-sol56.md) and [archive](corpus3.2-run8-codex-sol56.tar.gz)
- [Terra 5.6 exact report](corpus3.2-run10-codex-terra56-high-exact-cost.md) and [archive](corpus3.2-run10-codex-terra56-high-exact-cost.tar.gz)
- [Earlier corrected Terra report](corpus3.2-run9-codex-terra56-high.md), retained for historical comparison
