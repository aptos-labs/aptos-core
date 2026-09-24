# Corpus 3.2: Sol 5.6 versus Terra 5.6, with and without QP

This report compares the exact-telemetry Sol run 11 and Terra run 10 over the same corpus 3.2 cells. Main costs are canonical model costs; startup warmups are excluded. “Observed” is the cost actually recorded. “Retry-adjusted” implements the proposed failure penalty by adding each disqualified cell’s observed cost once, so a failed cell contributes **2× its cost through failure**. Unmeasured cells remain charged once because they are not semantic refutation failures.

## Short reading

Across all 240 cells, Terra cost $50.4457 observed versus Sol’s $83.0532, a 39.3% reduction. The retry adjustment barely changes that comparison: $52.4254 versus $86.2599, also about 39.2% less.

QP changes the arm comparison more than the model comparison. Sol guided has three costly QP disqualifications and flexible has one, so the full-corpus retry penalty makes both hybrids more expensive than agent-only. Once QP is removed, Sol guided becomes 1.2% cheaper than agent-only under the penalty. Terra’s hybrids remain cheaper than agent-only in every view; most Terra disqualification cost is in QP, and the only failures left after removing QP are two agent-only OV cells.

These are descriptive results from four replicas per task and arm. The retry adjustment is a planning proxy, not observed spend: a real replacement attempt could cost more or less than the failed attempt.

## Overall model comparison

| scope | model | cells | strict / DQ / U | observed total | observed mean | observed cost / strict | retry-adjusted total | adjusted mean | adjusted cost / strict |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| All 20 tasks | Sol 5.6 | 240 | 228/8/4 | $83.0532 | $0.34606 | $0.36427 | $86.2599 | $0.35942 | $0.37833 |
| All 20 tasks | Terra 5.6 | 240 | 231/8/1 | $50.4457 | $0.21019 | $0.21838 | $52.4254 | $0.21844 | $0.22695 |
| Without QP (19 tasks) | Sol 5.6 | 228 | 221/4/3 | $74.9131 | $0.32857 | $0.33897 | $75.6611 | $0.33185 | $0.34236 |
| Without QP (19 tasks) | Terra 5.6 | 228 | 225/2/1 | $46.4466 | $0.20371 | $0.20643 | $46.6848 | $0.20476 | $0.20749 |

| scope | Terra versus Sol, observed | Terra versus Sol, retry-adjusted | QP share of observed model cost |
| --- | ---: | ---: | ---: |
| All 20 tasks | -39.3% | -39.2% | Sol 9.8%; Terra 7.9% |
| Without QP (19 tasks) | -38.0% | -38.3% | excluded |

Negative percentages mean Terra cost less than Sol.

## Overall arm comparison

| scope | model | arm | cells | strict / DQ / U | observed total | observed mean | vs agent-only | DQ cost added | retry-adjusted total | adjusted mean | vs agent-only |
| --- | --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| All 20 tasks | Sol 5.6 | agent-only | 80 | 74/4/2 | $26.6462 | $0.33308 | baseline | $0.7479 | $27.3941 | $0.34243 | baseline |
| All 20 tasks | Sol 5.6 | flexible | 80 | 77/1/2 | $28.9652 | $0.36206 | +8.7% | $1.0265 | $29.9917 | $0.37490 | +9.5% |
| All 20 tasks | Sol 5.6 | guided | 80 | 77/3/0 | $27.4419 | $0.34302 | +3.0% | $1.4322 | $28.8742 | $0.36093 | +5.4% |
| All 20 tasks | Terra 5.6 | agent-only | 80 | 74/5/1 | $18.8433 | $0.23554 | baseline | $1.1324 | $19.9757 | $0.24970 | baseline |
| All 20 tasks | Terra 5.6 | flexible | 80 | 78/2/0 | $16.5973 | $0.20747 | -11.9% | $0.5813 | $17.1786 | $0.21473 | -14.0% |
| All 20 tasks | Terra 5.6 | guided | 80 | 79/1/0 | $15.0051 | $0.18756 | -20.4% | $0.2660 | $15.2710 | $0.19089 | -23.6% |
| Without QP (19 tasks) | Sol 5.6 | agent-only | 76 | 71/4/1 | $24.1728 | $0.31806 | baseline | $0.7479 | $24.9207 | $0.32790 | baseline |
| Without QP (19 tasks) | Sol 5.6 | flexible | 76 | 74/0/2 | $26.1312 | $0.34383 | +8.1% | $0.0000 | $26.1312 | $0.34383 | +4.9% |
| Without QP (19 tasks) | Sol 5.6 | guided | 76 | 76/0/0 | $24.6092 | $0.32381 | +1.8% | $0.0000 | $24.6092 | $0.32381 | -1.2% |
| Without QP (19 tasks) | Terra 5.6 | agent-only | 76 | 73/2/1 | $17.6877 | $0.23273 | baseline | $0.2381 | $17.9258 | $0.23587 | baseline |
| Without QP (19 tasks) | Terra 5.6 | flexible | 76 | 76/0/0 | $15.1829 | $0.19978 | -14.2% | $0.0000 | $15.1829 | $0.19978 | -15.3% |
| Without QP (19 tasks) | Terra 5.6 | guided | 76 | 76/0/0 | $13.5760 | $0.17863 | -23.2% | $0.0000 | $13.5760 | $0.17863 | -24.3% |

The “DQ cost added” column is the complete penalty increment. It is zero when a slice has no refutation failure.

## Per-sample comparison

Each task has 12 cells: three arms × four replicas. Dollar figures are totals across those 12 cells; the parenthesized figure is the mean per cell. The QP row is the only row removed in the without-QP view.

| sample | Sol strict / DQ / U | Sol observed (mean) | Sol adjusted (mean) | Terra strict / DQ / U | Terra observed (mean) | Terra adjusted (mean) | Terra vs Sol observed | Terra vs Sol adjusted |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| `BA-base-012` | 12/0/0 | $7.3817 ($0.6151) | $7.3817 ($0.6151) | 12/0/0 | $8.9016 ($0.7418) | $8.9016 ($0.7418) | +20.6% | +20.6% |
| `BK-bucket-016` | 12/0/0 | $4.1742 ($0.3478) | $4.1742 ($0.3478) | 12/0/0 | $1.8640 ($0.1553) | $1.8640 ($0.1553) | -55.3% | -55.3% |
| `LP-price-021` | 12/0/0 | $3.2385 ($0.2699) | $3.2385 ($0.2699) | 12/0/0 | $1.6285 ($0.1357) | $1.6285 ($0.1357) | -49.7% | -49.7% |
| `MD-median-015` | 12/0/0 | $2.1514 ($0.1793) | $2.1514 ($0.1793) | 12/0/0 | $1.2231 ($0.1019) | $1.2231 ($0.1019) | -43.1% | -43.1% |
| `MM-min-013` | 12/0/0 | $3.4971 ($0.2914) | $3.4971 ($0.2914) | 12/0/0 | $2.0019 ($0.1668) | $2.0019 ($0.1668) | -42.8% | -42.8% |
| `OV-order-006` | 8/4/0 | $2.4392 ($0.2033) | $3.1871 ($0.2656) | 10/2/0 | $1.1362 ($0.0947) | $1.3743 ($0.1145) | -53.4% | -56.9% |
| `PM-curve-027` | 12/0/0 | $2.2800 ($0.1900) | $2.2800 ($0.1900) | 12/0/0 | $1.2681 ($0.1057) | $1.2681 ($0.1057) | -44.4% | -44.4% |
| `QP-part-025` **(QP)** | 7/4/1 | $8.1401 ($0.6783) | $10.5989 ($0.8832) | 6/6/0 | $3.9991 ($0.3333) | $5.7407 ($0.4784) | -50.9% | -45.8% |
| `SM-select-022` | 10/0/2 | $6.8404 ($0.5700) | $6.8404 ($0.5700) | 11/0/1 | $4.4382 ($0.3699) | $4.4382 ($0.3699) | -35.1% | -35.1% |
| `TL-lev-020` | 12/0/0 | $4.2093 ($0.3508) | $4.2093 ($0.3508) | 12/0/0 | $2.5233 ($0.2103) | $2.5233 ($0.2103) | -40.1% | -40.1% |
| `TR-cancel-026` | 11/0/1 | $4.2436 ($0.3536) | $4.2436 ($0.3536) | 12/0/0 | $2.5274 ($0.2106) | $2.5274 ($0.2106) | -40.4% | -40.4% |
| `TR-discard-011` | 12/0/0 | $3.9491 ($0.3291) | $3.9491 ($0.3291) | 12/0/0 | $2.1015 ($0.1751) | $2.1015 ($0.1751) | -46.8% | -46.8% |
| `TR-order-010` | 12/0/0 | $3.3668 ($0.2806) | $3.3668 ($0.2806) | 12/0/0 | $2.1166 ($0.1764) | $2.1166 ($0.1764) | -37.1% | -37.1% |
| `TS-trial-019` | 12/0/0 | $3.3261 ($0.2772) | $3.3261 ($0.2772) | 12/0/0 | $1.2195 ($0.1016) | $1.2195 ($0.1016) | -63.3% | -63.3% |
| `UC-credits-008` | 12/0/0 | $4.1915 ($0.3493) | $4.1915 ($0.3493) | 12/0/0 | $2.3501 ($0.1958) | $2.3501 ($0.1958) | -43.9% | -43.9% |
| `VS-contrib-003` | 12/0/0 | $2.9271 ($0.2439) | $2.9271 ($0.2439) | 12/0/0 | $1.7184 ($0.1432) | $1.7184 ($0.1432) | -41.3% | -41.3% |
| `VS-fees-001` | 12/0/0 | $4.8959 ($0.4080) | $4.8959 ($0.4080) | 12/0/0 | $2.8326 ($0.2361) | $2.8326 ($0.2361) | -42.1% | -42.1% |
| `VS-redeem-004` | 12/0/0 | $6.5721 ($0.5477) | $6.5721 ($0.5477) | 12/0/0 | $3.8291 ($0.3191) | $3.8291 ($0.3191) | -41.7% | -41.7% |
| `VS-shares-002` | 12/0/0 | $2.6693 ($0.2224) | $2.6693 ($0.2224) | 12/0/0 | $1.4638 ($0.1220) | $1.4638 ($0.1220) | -45.2% | -45.2% |
| `WU-consume-023` | 12/0/0 | $2.5597 ($0.2133) | $2.5597 ($0.2133) | 12/0/0 | $1.3027 ($0.1086) | $1.3027 ($0.1086) | -49.1% | -49.1% |

## What creates the penalty

- **Sol 5.6:** 4 QP disqualifications add $2.4588; 4 OV disqualifications add $0.7479. Total penalty increment: $3.2067.
- **Terra 5.6:** 6 QP disqualifications add $1.7416; 2 OV disqualifications add $0.2381. Total penalty increment: $1.9797.

For Sol, all four OV disqualifications are agent-only, while the four QP disqualifications are three guided and one flexible. For Terra, QP accounts for six disqualifications spread across all arms, while both OV disqualifications are agent-only. All conclusive failures in both rounds came from the ordinary refutation gate; no cell that passed that gate later failed the separate scoring-mutant gate.

## Evidence and accounting notes

- [Sol 5.6 exact report](corpus3.2-run11-codex-sol56-high-exact-cost.md) and [cell archive](corpus3.2-run11-codex-sol56-high-exact-cost.tar.gz)
- [Terra 5.6 exact report](corpus3.2-run10-codex-terra56-high-exact-cost.md) and [cell archive](corpus3.2-run10-codex-terra56-high-exact-cost.tar.gz)
- [Three-model comparison](corpus3.2-opus5-sol56-terra56-comparison.md)

The calculations use `canonical_cost_usd` from each archive’s `cells.csv`. Startup warmups remain separate: $6.4487 for Sol and $3.2244 for Terra. Including warmups would add those fixed observed amounts to both the observed and retry-adjusted full-run totals; the failure penalty itself applies only to the disqualified cells’ canonical generation cost.
