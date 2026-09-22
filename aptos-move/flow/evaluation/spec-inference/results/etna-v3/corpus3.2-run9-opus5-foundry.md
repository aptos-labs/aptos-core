# Corpus 3.2 run 9: Claude Opus 5 on Foundry

This round evaluated the full selected corpus-v3.2 set with explicit
`claude-opus-5` through Microsoft Foundry, the documented 1M context window,
and `high` effort. The design was 20 tasks, three arms, and four replicas: 240
cells. Generation used acceptance feedback only. The ordinary mutant set was
withheld as a post-run disqualification gate, so a surviving mutant was a
final failure without an in-session refutation retry.

## Results

All 240 generation cells reached operational success. Post-run scoring yielded
230 strict successes, six conclusive disqualifications, and four cells that
remained unmeasured after their permitted infrastructure retry.

| arm | strict success | disqualified | infrastructure-unmeasured | list-price cost | mean cost per finalized cell | mean wall time |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| `agent_only` | 75/80 (93.75%) | 3 | 2 | $37.3875 | $0.46734 | 122.8 s |
| `hybrid_flexible` | 77/80 (96.25%) | 2 | 1 | $38.6501 | $0.48313 | 122.2 s |
| `hybrid_guided` | 78/80 (97.50%) | 1 | 1 | $39.6157 | $0.49520 | 126.4 s |
| **overall** | **230/240 (95.83%)** | **6** | **4** | **$115.6532** | **$0.48189** | **123.8 s** |

The strict-success percentages use all scheduled cells as their denominator.
The four unmeasured cells are infrastructure missingness rather than
experimental failures. The table is descriptive; with four replicas it does
not establish a statistically reliable ranking among the arms.

## Cost breakdown

The round cost **$115.65322** in total, or **$0.48189 per finalized cell**.
This is generation cost: post-run mutation scoring and infrastructure recovery
made no model calls. Each arm has exactly 80 finalized cells, so the arm means
use the same denominator.

### Token category

| billed component | tokens | list rate | cost | share of round cost |
| --- | ---: | ---: | ---: | ---: |
| Fresh input | 4,710 | $5/M | $0.023550 | 0.02% |
| Cache reads | 66,056,581 | $0.50/M | $33.028290 | 28.56% |
| Five-minute cache creation | 6,391,781 | $6.25/M | $39.948631 | 34.54% |
| One-hour cache creation | 0 | $10/M | $0.000000 | 0.00% |
| Output | 1,706,110 | $25/M | $42.652750 | 36.88% |
| **Total** |  |  | **$115.653222** | **100.00%** |

Output was the largest component, but it represented only 36.88% of the bill.
Cache creation and reads together accounted for 63.10%. The very small fresh
input count reflects the provider receipt categories: reusable prompt content
is reported as cache creation or cache reads.

### Arm and billing component

| arm | fresh input | cache reads | cache creation | output | total | share | difference from `agent_only` |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| `agent_only` | $0.0073 | $10.0781 | $13.2630 | $14.0391 | $37.3875 | 32.33% | baseline |
| `hybrid_flexible` | $0.0078 | $11.1051 | $13.5072 | $14.0298 | $38.6501 | 33.42% | +3.38% |
| `hybrid_guided` | $0.0084 | $11.8451 | $13.1785 | $14.5838 | $39.6157 | 34.25% | +5.96% |

The pooled arm means are close relative to cell-to-cell variation. Per-cell
cost distributions show the tail and variance hidden by those means. The p75
and p95 values use nearest rank; CV is population standard deviation divided
by the mean.

| group | mean | median | p75 | p95 | maximum | CV |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Overall | $0.48189 | $0.40677 | $0.58373 | $0.93862 | $2.34313 | 0.57 |
| `agent_only` | $0.46734 | $0.40842 | $0.53971 | $0.86327 | $2.34313 | 0.63 |
| `hybrid_flexible` | $0.48313 | $0.43877 | $0.60128 | $0.90743 | $1.28781 | 0.50 |
| `hybrid_guided` | $0.49520 | $0.39624 | $0.58269 | $1.26134 | $1.42550 | 0.58 |

### Scoring outcome

Model cost was incurred before the withheld scoring outcome was known. This
breakdown explains the all-finalized-cell denominator; it does not attribute
cost to the scoring process.

| final scoring outcome | cells | model cost | mean per cell | share of round cost |
| --- | ---: | ---: | ---: | ---: |
| Strictly scored | 230 | $108.5762 | $0.47207 | 93.88% |
| Disqualified | 6 | $3.9009 | $0.65015 | 3.37% |
| Infrastructure-unmeasured | 4 | $3.1761 | $0.79403 | 2.75% |

### Task

Each task contains 12 cells: three arms times four replicas. Task mean cost
varied from $0.24770 to $1.03904 per cell, a 4.19× range, which is much larger
than the 1.06× range among pooled arm means.

| task | total cost | mean per cell | share | scored | disqualified | unmeasured |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| `BA-base-012` | $7.9293 | $0.66077 | 6.86% | 12 | 0 | 0 |
| `BK-bucket-016` | $3.6422 | $0.30351 | 3.15% | 12 | 0 | 0 |
| `LP-price-021` | $5.3051 | $0.44209 | 4.59% | 12 | 0 | 0 |
| `MD-median-015` | $2.9724 | $0.24770 | 2.57% | 12 | 0 | 0 |
| `MM-min-013` | $3.3961 | $0.28301 | 2.94% | 12 | 0 | 0 |
| `OV-order-006` | $3.0141 | $0.25117 | 2.61% | 12 | 0 | 0 |
| `PM-curve-027` | $3.1850 | $0.26541 | 2.75% | 12 | 0 | 0 |
| `QP-part-025` | $8.6396 | $0.71997 | 7.47% | 6 | 6 | 0 |
| `SM-select-022` | $9.2881 | $0.77401 | 8.03% | 8 | 0 | 4 |
| `TL-lev-020` | $5.5768 | $0.46473 | 4.82% | 12 | 0 | 0 |
| `TR-cancel-026` | $7.1876 | $0.59896 | 6.21% | 12 | 0 | 0 |
| `TR-discard-011` | $5.9029 | $0.49191 | 5.10% | 12 | 0 | 0 |
| `TR-order-010` | $3.7107 | $0.30923 | 3.21% | 12 | 0 | 0 |
| `TS-trial-019` | $8.1407 | $0.67839 | 7.04% | 12 | 0 | 0 |
| `UC-credits-008` | $3.7645 | $0.31370 | 3.25% | 12 | 0 | 0 |
| `VS-contrib-003` | $5.0470 | $0.42059 | 4.36% | 12 | 0 | 0 |
| `VS-fees-001` | $7.6437 | $0.63698 | 6.61% | 12 | 0 | 0 |
| `VS-redeem-004` | $12.4684 | $1.03904 | 10.78% | 12 | 0 | 0 |
| `VS-shares-002` | $5.4625 | $0.45521 | 4.72% | 12 | 0 | 0 |
| `WU-consume-023` | $3.3765 | $0.28137 | 2.92% | 12 | 0 | 0 |

### Task by arm

Values are mean cost per cell across four replicas. Bold marks the cheapest
arm within that task; spread is the most expensive mean divided by the least
expensive. The cheapest arm varied by task: `agent_only` was cheapest on nine
tasks, `hybrid_flexible` on five, and `hybrid_guided` on six.

| task | `agent_only` | `hybrid_flexible` | `hybrid_guided` | within-task spread |
| --- | ---: | ---: | ---: | ---: |
| `BA-base-012` | $0.72999 | $0.68920 | **$0.56314** | 1.30× |
| `BK-bucket-016` | **$0.26515** | $0.32249 | $0.32290 | 1.22× |
| `LP-price-021` | $0.45389 | $0.47221 | **$0.40018** | 1.18× |
| `MD-median-015` | $0.24859 | $0.25419 | **$0.24033** | 1.06× |
| `MM-min-013` | $0.25943 | **$0.22982** | $0.35978 | 1.57× |
| `OV-order-006` | $0.24732 | $0.26660 | **$0.23961** | 1.11× |
| `PM-curve-027` | $0.26819 | **$0.24432** | $0.28374 | 1.16× |
| `QP-part-025` | **$0.61857** | $0.70088 | $0.84045 | 1.36× |
| `SM-select-022` | $0.77759 | **$0.73154** | $0.81289 | 1.11× |
| `TL-lev-020` | $0.44172 | **$0.34692** | $0.60556 | 1.75× |
| `TR-cancel-026` | $0.59120 | **$0.59063** | $0.61506 | 1.04× |
| `TR-discard-011` | **$0.42369** | $0.61756 | $0.43449 | 1.46× |
| `TR-order-010` | **$0.27224** | $0.27292 | $0.38251 | 1.41× |
| `TS-trial-019` | $0.98522 | $0.69098 | **$0.35898** | 2.74× |
| `UC-credits-008` | **$0.30716** | $0.32143 | $0.31252 | 1.05× |
| `VS-contrib-003` | **$0.38445** | $0.43574 | $0.44158 | 1.15× |
| `VS-fees-001` | **$0.51073** | $0.63996 | $0.76024 | 1.49× |
| `VS-redeem-004` | **$0.87037** | $0.95863 | $1.28810 | 1.48× |
| `VS-shares-002` | $0.42857 | $0.59457 | **$0.34248** | 1.74× |
| `WU-consume-023` | **$0.26281** | $0.28194 | $0.29937 | 1.14× |

### Most expensive cells

| task | replica | arm | cost | scoring outcome |
| --- | ---: | --- | ---: | --- |
| `TS-trial-019` | 4 | `agent_only` | $2.34313 | `scored` |
| `QP-part-025` | 4 | `hybrid_guided` | $1.42550 | `scored` |
| `VS-redeem-004` | 2 | `hybrid_guided` | $1.38994 | `scored` |
| `BA-base-012` | 4 | `hybrid_flexible` | $1.28781 | `scored` |
| `VS-redeem-004` | 3 | `hybrid_guided` | $1.27108 | `scored` |
| `SM-select-022` | 3 | `hybrid_guided` | $1.26800 | `scored` |
| `VS-redeem-004` | 1 | `hybrid_guided` | $1.26134 | `scored` |
| `VS-redeem-004` | 4 | `hybrid_guided` | $1.23005 | `scored` |
| `VS-redeem-004` | 1 | `hybrid_flexible` | $1.16539 | `scored` |
| `SM-select-022` | 4 | `agent_only` | $1.10880 | `not_scorable` |

## Failures and infrastructure recovery

All six conclusive failures occurred on `QP-part-025` because
`QP-part-025-lost-element` survived the disqualification gate. In accordance
with the protocol, these cells were not retried:

- `agent_only`: replicas 1, 2, and 4
- `hybrid_flexible`: replicas 2 and 4
- `hybrid_guided`: replica 2

Six `SM-select-022` ordinary-gate scores were solver-inconclusive in the main
scoring pass. Each was rerun once, sequentially and with exclusive access to
both shared prover slots. Two passed the gate but initially had an
inconclusive strict score; their permitted strict-only retry killed all three
strict mutants. Four ordinary gates remained inconclusive and are retained as
unmeasured: `hybrid_flexible` replica 1, `agent_only` replicas 2 and 4, and
`hybrid_guided` replica 4.

## Pricing, time, and provenance

The cost is the cumulative model cost for this round, divided by all finalized
canonical cells in each arm. It includes cells later disqualified or left
unmeasured. Scoring and recovery used no model calls.

Every cell was independently repriced from its raw Foundry token counters at
the recorded Opus 5 list rates: $5/M fresh input, $0.50/M cache reads,
$6.25/M five-minute cache creation, $10/M one-hour cache creation, and $25/M
output. All 240 recomputed prices matched the SDK list-cost estimate within
$1e-9; the largest absolute difference was $2e-16. Azure contract or invoice
adjustments are not present in the telemetry.

Overall wall time had a 93.8-second median, 288.4-second p95, and 691.2-second
maximum. Cell concurrency was three except for one recorded interval at two.
The three participating containers cooperatively limited Boogie to two process
groups on a Linux btrfs-backed lock volume; the shared trace recorded no cap
violation.

The experiment source commit is
`01301524c92bf240f853b7eaa2aa265e4c526e53`. The archived Flow binary SHA-256
is `b4deb2871092b92e806f656d3e4aa503f198cfacf54e21ae3a46f5d6446cdd44`.
The apparatus and coordination provenance was pushed at
`04a9bac5e919609124740c25af009ae9552d55f8` on
`origin/wrwg/inf-opus5-foundry`.

The companion archive is
[`corpus3.2-run9-opus5-foundry.tar.gz`](corpus3.2-run9-opus5-foundry.tar.gz).
It contains an aggregate publication notice, per-cell and per-query tables,
the final mutation summary, pricing, audit data, and apparatus identities.
Per-run verdicts, diagnostics, transcripts, event streams, source trees, and
recovery evidence with source-bearing fields are excluded. The archive
includes an internal `SHA256SUMS`; its SHA-256 is
`e0f545f7a92f33cde0f7cde4d3f9a3cb607be70386747cb43308f6aee9290d04`.
