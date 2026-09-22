# Corpus 3.2 run 10: GPT-5.6 Terra with exact request pricing

This rerun covers all 20 corpus-v3.2 tasks, three inference arms, and four replicas: 240 scheduled cells. It used `gpt-5.6-terra` through Codex CLI at `high` effort, generation concurrency 3, and no refutation feedback retry. The ordinary mutant set was withheld until generation ended; a surviving gate mutant therefore counts as failure without a model retry. Infrastructure failures could receive one clean retry.

The run was created to remove the old Terra round’s pricing interval. OpenTelemetry captured every completed underlying model request, including input, cached input, visible output, and reasoning output. All 3,237 canonical requests remained below the 272K request-level threshold. The retained round therefore has exact point estimates rather than an accounting range.

## Result at a glance

| arm | strict | disqualified | unmeasured | exact canonical cost | mean / scheduled cell | cost / strict success | mean wall |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| agent-only | 74/80 (92.50%) | 5 | 1 | $18.843298 | $0.235541 | $0.254639 | 124.3 s |
| hybrid flexible | 78/80 (97.50%) | 2 | 0 | $16.597350 | $0.207467 | $0.212787 | 103.0 s |
| hybrid guided | 79/80 (98.75%) | 1 | 0 | $15.005093 | $0.187564 | $0.189938 | 101.9 s |
| **overall** | **231/240 (96.25%)** | **8** | **1** | **$50.445741** | **$0.210191** | **$0.218380** | **109.7 s** |

Hybrid guided is the observed cost/quality winner: it produced 79 strict successes, one more than flexible and five more than agent-only, while costing 9.6% less than flexible and 20.4% less than agent-only. With only four replicas and failures concentrated in two tasks, these differences remain descriptive.

## Exact cost accounting

Canonical cost means completed model requests attributable to task-solving controller turns. Startup warmups are measured exactly and reported separately because they perform no task work. Input tokens include the cached subset, so fresh input is `input - cached`; visible and reasoning output are billed once each.

| level | cells / requests | fresh input | cached input | visible output | reasoning output | exact cost |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| agent-only | 80 / 1,147 | 3,754,999 | 26,014,720 | 328,667 | 182,196 | $18.843298 |
| hybrid flexible | 80 / 1,128 | 3,378,687 | 23,984,640 | 284,337 | 135,917 | $16.597350 |
| hybrid guided | 80 / 962 | 3,142,238 | 19,350,784 | 268,103 | 136,102 | $15.005093 |
| **canonical total** | **240 / 3,237** | **10,275,924** | **69,350,144** | **881,107** | **454,215** | **$50.445741** |

Pricing is $2/M fresh input, $0.20/M cached input, and $12/M generated output. Above 272K input tokens per request, input rates double and output rates increase by 1.5×. No canonical or warmup request crossed that threshold.

| billed component | tokens | rate | cost | share |
| --- | ---: | ---: | ---: | ---: |
| fresh input | 10,275,924 | $2/M | $20.551848 | 40.7% |
| cached input | 69,350,144 | $0.20/M | $13.870029 | 27.5% |
| visible output | 881,107 | $12/M | $10.573284 | 21.0% |
| reasoning output | 454,215 | $12/M | $5.450580 | 10.8% |
| **total** | **80,961,390 metered tokens** | | **$50.445741** | **100%** |

### Startup and infrastructure overhead

The retained 240 cells issued 242 startup warmups containing 1,612,178 fresh input tokens and no output, costing **$3.224356**. Canonical plus warmup cost is **$53.670097**.

Two retained BA-base-012 replica-4 cells used their allowed clean infrastructure retry. Their failed first attempts account for **$1.797148** of canonical cost plus $0.026636 of warmups; those amounts are already included in the retained totals.

During an experiment with an external shared prover gate, three BA-base-012 replica-2 cells were paused. The gate deadlocked because paused controller groups owned the slots needed by their prover children. The gate was removed, all three cells were rerun cleanly, and only the clean results appear above. The excluded guided result recorded $0.351783 canonical plus $0.013318 warmup cost. Two interrupted artifacts recorded $1.436270 canonical plus $0.040654 warmup cost across completed responses. Including every completed retained and excluded response gives **$55.512122 observed work**. A request in flight at termination may have provider-side billing that cannot be recovered from a missing completion event, so this all-work number is an observed minimum; it does not create uncertainty in the retained 240-cell result.

### Cell-cost distribution

| scope | median | p75 | p95 | maximum | coefficient of variation |
| --- | ---: | ---: | ---: | ---: | ---: |
| agent-only | $0.1766 | $0.2579 | $0.5955 | $1.2662 | 0.75 |
| hybrid flexible | $0.1617 | $0.2302 | $0.4579 | $1.2739 | 0.91 |
| hybrid guided | $0.1662 | $0.1972 | $0.4239 | $1.2045 | 0.90 |
| overall | $0.1681 | $0.2296 | $0.5339 | $1.2739 | 0.85 |

## Quality

The disqualification gate ran for every cell against the ordinary `corpus-v3.2/mutants` set. Of the 240 operationally accepted contracts, 231 passed the gate and killed every essential mutant in the disjoint scoring set. Eight were disqualified by a surviving ordinary mutant. One gate invocation was inconclusive and is kept separate from a semantic failure. No failed gate received model feedback or a retry.

| final outcome | cells | canonical cost | mean / cell |
| --- | ---: | ---: | ---: |
| strict success | 231 | $48.209189 | $0.208698 |
| disqualified | 8 | $1.979681 | $0.247460 |
| gate inconclusive | 1 | $0.256871 | $0.256871 |

Six QP-part-025 cells failed `QP-part-025-lost-element`: agent-only replicas 1, 2, and 4; flexible replicas 2 and 3; and guided replica 4. Agent-only OV-order-006 replicas 2 and 3 failed `OV-order-006-reorder-guards`. Agent-only SM-select-022 replica 1 was unmeasured because `SM-select-022-restarts-at-origin` reached no gate verdict.

## Task-level cost and quality

Quality is `strict / disqualified / unmeasured` across 12 cells. Arm columns are mean canonical cost across four replicas.

| task | total cost | mean / cell | quality | agent-only | flexible | guided |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| `BA-base-012` | $8.9016 | $0.7418 | 12/0/0 | $0.7992 | $0.7529 | $0.6734 |
| `BK-bucket-016` | $1.8640 | $0.1553 | 12/0/0 | $0.1393 | $0.1706 | $0.1561 |
| `LP-price-021` | $1.6285 | $0.1357 | 12/0/0 | $0.1798 | $0.1240 | $0.1034 |
| `MD-median-015` | $1.2231 | $0.1019 | 12/0/0 | $0.1227 | $0.1069 | $0.0762 |
| `MM-min-013` | $2.0019 | $0.1668 | 12/0/0 | $0.1741 | $0.1558 | $0.1706 |
| `OV-order-006` | $1.1362 | $0.0947 | 10/2/0 | $0.1112 | $0.0921 | $0.0807 |
| `PM-curve-027` | $1.2681 | $0.1057 | 12/0/0 | $0.1482 | $0.0975 | $0.0714 |
| `QP-part-025` | $3.9991 | $0.3333 | 6/6/0 | $0.2889 | $0.3536 | $0.3573 |
| `SM-select-022` | $4.4382 | $0.3699 | 11/0/1 | $0.3146 | $0.4124 | $0.3826 |
| `TL-lev-020` | $2.5233 | $0.2103 | 12/0/0 | $0.1797 | $0.2272 | $0.2239 |
| `TR-cancel-026` | $2.5274 | $0.2106 | 12/0/0 | $0.2224 | $0.2184 | $0.1911 |
| `TR-discard-011` | $2.1015 | $0.1751 | 12/0/0 | $0.1591 | $0.1784 | $0.1879 |
| `TR-order-010` | $2.1166 | $0.1764 | 12/0/0 | $0.1638 | $0.1798 | $0.1856 |
| `TS-trial-019` | $1.2195 | $0.1016 | 12/0/0 | $0.1518 | $0.0797 | $0.0734 |
| `UC-credits-008` | $2.3501 | $0.1958 | 12/0/0 | $0.2415 | $0.1961 | $0.1499 |
| `VS-contrib-003` | $1.7184 | $0.1432 | 12/0/0 | $0.2384 | $0.1067 | $0.0845 |
| `VS-fees-001` | $2.8326 | $0.2361 | 12/0/0 | $0.2641 | $0.2429 | $0.2011 |
| `VS-redeem-004` | $3.8291 | $0.3191 | 12/0/0 | $0.4947 | $0.2443 | $0.2183 |
| `VS-shares-002` | $1.4638 | $0.1220 | 12/0/0 | $0.1752 | $0.1014 | $0.0893 |
| `WU-consume-023` | $1.3027 | $0.1086 | 12/0/0 | $0.1423 | $0.1086 | $0.0747 |

BA-base-012 dominates both cost and the runtime tail: $8.9016, or 17.6% of canonical spend. QP-part-025 is the other quality-sensitive task; half its cells failed the lost-element gate even though every candidate passed ordinary operational verification.

## What the run logs show

| diagnostic | agent-only | flexible | guided | total |
| --- | ---: | ---: | ---: | ---: |
| candidate checks | 185 | 116 | 134 | 435 |
| first check accepted | 34 | 59 | 54 | 147 |
| failed candidate checks | 102 | 31 | 53 | 186 |
| compile failures | 35 | 9 | 10 | 54 |
| prover failures | 49 | 10 | 25 | 84 |
| forbidden weakenings | 0 | 9 | 10 | 19 |
| incomplete contracts | 6 | 2 | 5 | 13 |
| policy violations | 6 | 0 | 1 | 7 |
| WP engine calls | 0 | 146 | 153 | 299 |
| explicit prover calls | 14 | 54 | 20 | 88 |
| added specification lines | 2,230 | 1,804 | 1,719 | 5,753 |

Agent-only required 185 candidate checks and accepted only 34 cells on the first recorded check. Flexible needed 116 checks and guided 134; their first-check acceptance was 59 and 54 cells. Guided made fewer model requests than either alternative (962 versus 1,128 flexible and 1,147 agent-only), explaining most of its cost advantage.

## Wall time

| scope | mean | median | p95 | maximum | summed cell time |
| --- | ---: | ---: | ---: | ---: | ---: |
| agent-only | 124.3 s | 78.4 s | 329.7 s | 804.1 s | 2.76 h |
| hybrid flexible | 103.0 s | 74.6 s | 237.3 s | 641.5 s | 2.29 h |
| hybrid guided | 101.9 s | 70.8 s | 268.3 s | 950.4 s | 2.26 h |
| overall | 109.7 s | 75.0 s | 321.2 s | 950.4 s | 7.32 h |

The clean controller results span 07:35:05–10:23:15 UTC, including clean replacement runs after the prover-gate incident. Their wall times overlap under concurrency 3. Sequential post-run mutation scoring then ran until 11:14:04 UTC and added no model cost. The 950.4-second maximum is a clean BA-base-012 guided cell; the excluded paused guided attempt is not in this table. These controller durations combine provider latency, tool calls, and prover work, so they do not isolate geography or network latency.

## Provenance and archive

- Branch: `wrwg/inf-terra-exact-cost`.
- Source commit: `823ddf1aac26a0db6833fbf96d5ea0616082d948`.
- Model/runtime: `gpt-5.6-terra`, Codex CLI `0.153.2`, `high` effort.
- Codex code-mode host SHA-256: `883f2506d12f319aec6f16b3e04d73ee882a8c86270ea5644ef4be6257b069e1`.
- Move Flow: `move-flow 2.0.0`, SHA-256 `4bb73b337ad1b58bd1a97e656b3ae59da686813220efe24ba5fa0e72f737a423`.
- Experiment config SHA-256: `399b85dc1bcce8db565c53631fe0f4716a96d657dc7dc65cb7fa7f7ca46419e3`.
- Controller harness SHA-256: `3327f404208e3ef3bdde57f1db0d78c88b40132838d79b349fda81b3559f51a8`.
- Controller prompts SHA-256: `673cae42ce32a29cc1af1b894437eea40b5adce5e26f6071074be2fd961136e5`.

The companion archive `corpus3.2-run10-codex-terra56-high-exact-cost.tar.gz`
contains only source-free aggregate reports, configuration, pricing, audit
results, and per-cell and per-request cost tables. Per-run diffs, diagnostics,
transcripts, event streams, source trees, and excluded-attempt evidence are not
published. `SHA256SUMS` authenticates the files inside the archive.

The archive SHA-256 is `3874d346acb7645b4cebb637186988035086c21a26350ecf18c979240ad70460`.
