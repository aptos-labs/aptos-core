# Corpus 3.2 run 11: GPT-5.6 Sol with exact request pricing

This rerun covers all 20 corpus-v3.2 tasks, three inference arms, and four replicas: 240 scheduled cells. It used `gpt-5.6-sol` through Codex CLI at `high` effort, generation concurrency 3, and no refutation-feedback retry. The ordinary mutant set was withheld until generation ended; a surviving gate mutant therefore counts as failure without a model retry. Infrastructure failures could receive one clean retry.

OpenTelemetry captured every completed underlying model request, including input, cached input, visible output, and reasoning output. All 2,815 canonical requests remained below the 272K request-level threshold. The retained round therefore has exact point estimates rather than an accounting range.

## Result at a glance

| arm | strict | disqualified | unmeasured | exact canonical cost | mean / scheduled cell | cost / strict success | mean wall |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| agent-only | 74/80 (92.50%) | 4 | 2 | $26.646158 | $0.333077 | $0.360083 | 119.0 s |
| hybrid flexible | 77/80 (96.25%) | 1 | 2 | $28.965154 | $0.362064 | $0.376171 | 120.4 s |
| hybrid guided | 77/80 (96.25%) | 3 | 0 | $27.441911 | $0.343024 | $0.356388 | 125.0 s |
| **overall** | **228/240 (95.00%)** | **8** | **4** | **$83.053223** | **$0.346055** | **$0.364269** | **121.5 s** |

Guided and flexible each produced 77 strict successes. Guided cost 5.3% less, so guided is the stronger observed hybrid cost/quality point. Agent-only cost less overall but produced three fewer strict successes. These differences remain descriptive with four replicas and failures concentrated in two tasks.

## Exact cost accounting

Canonical cost covers completed model requests attributable to task-solving controller turns. Startup warmups are measured separately because they perform no task work. Input tokens include the cached subset, so fresh input is `input - cached`; visible and reasoning output are billed once each.

| level | cells / requests | fresh input | cached input | visible output | reasoning output | exact cost |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| agent-only | 80 / 882 | 3,024,249 | 16,950,656 | 257,816 | 130,629 | $26.646158 |
| hybrid flexible | 80 / 992 | 3,180,631 | 20,215,424 | 284,684 | 123,139 | $28.965154 |
| hybrid guided | 80 / 941 | 2,929,397 | 18,132,608 | 291,086 | 132,478 | $27.441911 |
| **canonical total** | **240 / 2,815** | **9,134,277** | **55,298,688** | **833,586** | **386,246** | **$83.053223** |

Pricing is $4/M fresh input, $0.40/M cached input, and $20/M generated output. Above 272K input tokens per request, input rates double and output rates increase by 1.5×. No canonical or warmup request crossed that threshold. Source: [GPT-5.6 Sol model pricing](https://developers.openai.com/api/docs/models/gpt-5.6-sol).

| billed component | tokens | cost | share |
| --- | ---: | ---: | ---: |
| fresh input | 9,134,277 | $36.537108 | 44.0% |
| cached input | 55,298,688 | $22.119475 | 26.6% |
| visible output | 833,586 | $16.671720 | 20.1% |
| reasoning output | 386,246 | $7.724920 | 9.3% |
| **total** | **65,652,797 metered tokens** | **$83.053223** | **100%** |

### Startup overhead

The 240 cells issued 245 startup warmups, costing **$6.448712**. Canonical plus warmup cost is **$89.501935**. No generation cell needed an infrastructure retry; two cells needed a second controller turn after a prover timeout, and those requests are part of the canonical totals.

### Cell-cost distribution

| scope | median | p75 | p95 | maximum | coefficient of variation |
| --- | ---: | ---: | ---: | ---: | ---: |
| agent-only | $0.2836 | $0.4048 | $0.6224 | $0.7524 | 0.43 |
| hybrid flexible | $0.2939 | $0.4026 | $0.7526 | $1.1458 | 0.54 |
| hybrid guided | $0.3015 | $0.3867 | $0.7571 | $1.4005 | 0.61 |
| overall | $0.2946 | $0.4022 | $0.6912 | $1.4005 | 0.54 |

## Quality

The disqualification gate ran for every cell against the ordinary `corpus-v3.2/mutants` set. Of the 240 operationally accepted contracts, 228 passed the gate and killed every essential mutant in the disjoint scoring set. Eight were disqualified by a surviving ordinary mutant. Four prover calls remained inconclusive after a sequential infrastructure-only retry and are reported as unmeasured. No failed semantic gate received model feedback or a model retry.

| final outcome | cells | canonical cost | mean / cell |
| --- | ---: | ---: | ---: |
| strict success | 228 | $77.993902 | $0.342079 |
| disqualified | 8 | $3.206718 | $0.400840 |
| scoring inconclusive | 4 | $1.852604 | $0.463151 |

All four agent-only OV-order-006 replicas failed `OV-order-006-reorder-guards`. Four QP-part-025 cells failed `QP-part-025-lost-element`: guided replicas 1, 2, and 4, plus flexible replica 4. The four unmeasured cells were QP agent-only replica 2, SM flexible replicas 1 and 2, and TR-cancel agent-only replica 3.

Three other scores initially failed because a candidate-added loop invariant wrapped the anchored mutant location. The scorer was fixed to map the exact edit span through inserted specification text while verifying nearby implementation context; sequential infrastructure-only retries then scored all three as strict successes.

## Task-level cost and quality

Quality is `strict / disqualified / unmeasured` across 12 cells. Arm columns are mean canonical cost across four replicas.

| task | total cost | mean / cell | quality | agent-only | flexible | guided |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| `BA-base-012` | $7.3817 | $0.6151 | 12/0/0 | $0.5349 | $0.7567 | $0.5539 |
| `BK-bucket-016` | $4.1742 | $0.3478 | 12/0/0 | $0.3312 | $0.3047 | $0.4077 |
| `LP-price-021` | $3.2385 | $0.2699 | 12/0/0 | $0.2798 | $0.2725 | $0.2573 |
| `MD-median-015` | $2.1514 | $0.1793 | 12/0/0 | $0.1523 | $0.1948 | $0.1908 |
| `MM-min-013` | $3.4971 | $0.2914 | 12/0/0 | $0.3068 | $0.2794 | $0.2880 |
| `OV-order-006` | $2.4392 | $0.2033 | 8/4/0 | $0.1870 | $0.2327 | $0.1901 |
| `PM-curve-027` | $2.2800 | $0.1900 | 12/0/0 | $0.2293 | $0.1936 | $0.1471 |
| `QP-part-025` | $8.1401 | $0.6783 | 7/4/1 | $0.6184 | $0.7085 | $0.7082 |
| `SM-select-022` | $6.8404 | $0.5700 | 10/0/2 | $0.4503 | $0.4731 | $0.7867 |
| `TL-lev-020` | $4.2093 | $0.3508 | 12/0/0 | $0.2713 | $0.4321 | $0.3490 |
| `TR-cancel-026` | $4.2436 | $0.3536 | 11/0/1 | $0.3473 | $0.3598 | $0.3538 |
| `TR-discard-011` | $3.9491 | $0.3291 | 12/0/0 | $0.2988 | $0.3596 | $0.3289 |
| `TR-order-010` | $3.3668 | $0.2806 | 12/0/0 | $0.2255 | $0.3176 | $0.2986 |
| `TS-trial-019` | $3.3261 | $0.2772 | 12/0/0 | $0.3621 | $0.2336 | $0.2358 |
| `UC-credits-008` | $4.1915 | $0.3493 | 12/0/0 | $0.3911 | $0.3331 | $0.3236 |
| `VS-contrib-003` | $2.9271 | $0.2439 | 12/0/0 | $0.3075 | $0.2471 | $0.1772 |
| `VS-fees-001` | $4.8959 | $0.4080 | 12/0/0 | $0.3482 | $0.5045 | $0.3713 |
| `VS-redeem-004` | $6.5721 | $0.5477 | 12/0/0 | $0.5348 | $0.6140 | $0.4942 |
| `VS-shares-002` | $2.6693 | $0.2224 | 12/0/0 | $0.2745 | $0.2222 | $0.1706 |
| `WU-consume-023` | $2.5597 | $0.2133 | 12/0/0 | $0.2106 | $0.2016 | $0.2277 |

## Run-log observations

| diagnostic | agent-only | flexible | guided | overall |
| --- | ---: | ---: | ---: | ---: |
| candidate checks | 126 | 104 | 108 | 338 |
| first recorded check accepted | 53 | 64 | 62 | 179 |
| failed candidate checks | 45 | 24 | 26 | 95 |
| compile failures | 24 | 8 | 8 | 40 |
| prover failures | 17 | 7 | 4 | 28 |
| prover timeouts | 3 | 2 | 1 | 6 |
| forbidden weakenings | 0 | 7 | 9 | 16 |
| incomplete contracts | 0 | 0 | 3 | 3 |
| policy violations | 1 | 0 | 1 | 2 |
| WP engine calls | 0 | 140 | 152 | 292 |
| explicit prover calls | 1 | 16 | 25 | 42 |
| added specification lines | 2,213 | 1,822 | 1,848 | 5,883 |

The controller accepted 179 of 240 cells on the first recorded candidate check. The run logged 338 candidate checks, 292 WP engine calls, and 42 explicit prover calls. The two controller retries followed prover timeouts; generation still ended with 240/240 operational successes.

## Wall time

| arm | mean | median | p95 | maximum | summed cell time |
| --- | ---: | ---: | ---: | ---: | ---: |
| agent-only | 119.0 s | 84.3 s | 270.0 s | 585.3 s | 2.64 h |
| hybrid flexible | 120.4 s | 95.8 s | 287.1 s | 540.7 s | 2.68 h |
| hybrid guided | 125.0 s | 93.6 s | 295.6 s | 698.1 s | 2.78 h |
| **overall** | **121.5 s** | **91.7 s** | **295.6 s** | **698.1 s** | **8.10 h** |

Wall time combines provider latency, controller work, Move Flow calls, and prover execution. It is not a direct provider-latency measurement. Generation ran at controller concurrency 3; scoring ran sequentially to avoid prover collisions and added no model cost.

## Provenance and archived evidence

- Source commit recorded by the schedule: `283139e76fc400da577a701f2bf6a9aa72bcca62`.
- Branch: `wrwg/inf-sol-exact-cost`.
- Codex CLI: `0.153.2`; model: `gpt-5.6-sol`; effort: `high`.
- Controller concurrency: 3; infrastructure retries: 1; semantic refutation retries: 0.
- Move Flow SHA-256: `4bb73b337ad1b58bd1a97e656b3ae59da686813220efe24ba5fa0e72f737a423`.
- Codex code-mode host SHA-256: `883f2506d12f319aec6f16b3e04d73ee882a8c86270ea5644ef4be6257b069e1`.
- The archive uses the compact publication format for aggregate configuration,
  pricing, audit results, and per-cell and per-request cost tables.
- Archive SHA-256:
  `aab08250c3bf8984e4271593123d78eeb416f4cb5d67840931f20e4302155523`.
