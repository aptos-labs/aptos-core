# Corpus 3.2 run 9: GPT-5.6 Terra high

This round evaluated the full selected corpus-v3.2 set with explicit
`gpt-5.6-terra` through Codex CLI 0.153.2 at `high` reasoning effort. The design
was 20 tasks, three arms, and four replicas: 240 canonical cells. Generation
used acceptance feedback only. Refutation failures received no model retry;
genuine infrastructure failures were eligible for one clean retry.

## Scoring correction

The original schedule accidentally recorded `disqualification_mode: none` and
null ordinary-mutant digests. Its first archived report therefore covered only
`corpus-v3.2/mutants-scoring` and overstated strict quality as 238/240.

The omitted `corpus-v3.2/mutants` gate has now been applied to the unchanged
Terra candidates. Before scoring, every task's live manifest was required to
match the same SHA-256 independently bound by both the Opus 5 and Sol 5.6
schedules. The scorer also verified disjointness from the scoring set and
Terra's original configuration, harness, Move Flow, Boogie, and Z3 identities.
No model ran, no candidate changed, and no mutant feedback was disclosed.

The result below supersedes the incomplete quality result in the original
archive.

## Corrected quality and cost

| arm | strict success | disqualified | unmeasured | API-equivalent cost | mean / scheduled cell | cost / strict success |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| `agent_only` | 76/80 (95.00%) | 3 | 1 | $16.9452–$25.5731 | $0.21181–$0.31966 | $0.22296–$0.33649 |
| `hybrid_flexible` | 75/80 (93.75%) | 4 | 1 | $17.0387–$27.1586 | $0.21298–$0.33948 | $0.22718–$0.36211 |
| `hybrid_guided` | 75/80 (93.75%) | 3 | 2 | $13.8913–$19.9452 | $0.17364–$0.24932 | $0.18522–$0.26594 |
| **overall** | **226/240 (94.17%)** | **10** | **4** | **$47.8752–$72.6770** | **$0.19948–$0.30282** | **$0.21184–$0.32158** |

All ten conclusive failures occurred on `QP-part-025`. Each killed three of the
four ordinary mutants but let `QP-part-025-lost-element` survive:

| replica | failed arms |
| ---: | --- |
| 1 | `hybrid_flexible`, `hybrid_guided` |
| 2 | `agent_only`, `hybrid_flexible`, `hybrid_guided` |
| 3 | `agent_only`, `hybrid_flexible`, `hybrid_guided` |
| 4 | `agent_only`, `hybrid_flexible` |

The four unmeasured cells remain separate from conclusive model failures:

- `BA-base-012/r03/hybrid_flexible` remained infrastructure-invalid after its
  permitted clean generation retry.
- `SM-select-022/r02/hybrid_guided` inserted specification text where both
  scoring passes expected an unchanged implementation fragment, so mutation
  application was deterministically unavailable.
- `SM-select-022/r04/hybrid_guided` had one ordinary mutant remain
  solver-inconclusive after an exclusive clean retry.
- `TR-discard-011/r01/agent_only` inserted specification text across an
  ordinary-mutant anchor and could not be scored against that mutation.

Every measured scoring-set mutant was killed. The restored ordinary set is what
separated the ten incomplete QP contracts from the strict successes.

## Cost accounting

The gate correction adds prover work and no model usage, so generation cost is
unchanged. The total includes all 244 completed model attempts: 240 canonical
attempts and four quarantined infrastructure attempts assigned to their
canonical cells.

| arm | input tokens | cached input | billed output | reasoning subset | aggregate turns above 272K | total cost |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| `agent_only` | 25,563,482 | 22,038,272 | 457,259 | 163,250 | 37 | $16.9452–$25.5731 |
| `hybrid_flexible` | 27,521,916 | 23,976,448 | 429,377 | 136,134 | 41 | $17.0387–$27.1586 |
| `hybrid_guided` | 20,447,242 | 17,377,280 | 356,325 | 117,258 | 26 | $13.8913–$19.9452 |
| **overall** | **73,532,640** | **63,392,000** | **1,242,961** | **416,642** | **104** | **$47.8752–$72.6770** |

`input tokens` includes the cached subset. `billed output` is visible output plus
reasoning output, charged exactly once. Guided used the least input and output
and cost 18.0% less than agent-only at ordinary rates and 18.5% less than
flexible.

Official list pricing recorded on 2026-09-11 was $2/M uncached input, $0.20/M
cached input, and $12/M output. A request above 272K input tokens receives 2×
input/cache and 1.5× output rates. Codex's turn receipts aggregate underlying
requests, so the threshold cannot be assigned exactly:

- **$47.8752 lower bound:** ordinary rates for every token.
- **$72.6770 upper bound:** long-context rates for every aggregate turn above
  272K.

This is an accounting bound, not a confidence interval. The observed marginal
token charge under the Codex subscription was zero. The preliminary monitor
also omitted separately reported reasoning output; these figures include it.

## Task-level cost and corrected quality

Each task has 12 scheduled cells. Cost includes any infrastructure-retry spend
assigned to the task. Quality is `strict / disqualified / unmeasured`.

| task | cost | mean / scheduled cell | quality |
| --- | ---: | ---: | ---: |
| `BA-base-012` | $6.8176–$12.2281 | $0.56813–$1.01900 | 11/0/1 |
| `BK-bucket-016` | $1.8641–$2.3471 | $0.15534–$0.19559 | 12/0/0 |
| `LP-price-021` | $1.6473–$1.8039 | $0.13728–$0.15032 | 12/0/0 |
| `MD-median-015` | $1.1616 | $0.09680 | 12/0/0 |
| `MM-min-013` | $1.9671–$2.5313 | $0.16393–$0.21094 | 12/0/0 |
| `OV-order-006` | $1.1462 | $0.09552 | 12/0/0 |
| `PM-curve-027` | $1.2620 | $0.10516 | 12/0/0 |
| `QP-part-025` | $3.4073–$5.7995 | $0.28394–$0.48329 | 2/10/0 |
| `SM-select-022` | $5.0636–$8.7448 | $0.42196–$0.72874 | 10/0/2 |
| `TL-lev-020` | $2.4511–$4.3239 | $0.20426–$0.36033 | 12/0/0 |
| `TR-cancel-026` | $2.8127–$4.8391 | $0.23439–$0.40326 | 12/0/0 |
| `TR-discard-011` | $2.0903–$3.2227 | $0.17419–$0.26856 | 11/0/1 |
| `TR-order-010` | $2.2604–$3.3583 | $0.18837–$0.27986 | 12/0/0 |
| `TS-trial-019` | $1.3273 | $0.11061 | 12/0/0 |
| `UC-credits-008` | $2.0937–$2.9686 | $0.17447–$0.24739 | 12/0/0 |
| `VS-contrib-003` | $1.5918–$2.2181 | $0.13265–$0.18484 | 12/0/0 |
| `VS-fees-001` | $2.5690–$3.8054 | $0.21408–$0.31712 | 12/0/0 |
| `VS-redeem-004` | $3.5037–$6.4082 | $0.29197–$0.53402 | 12/0/0 |
| `VS-shares-002` | $1.4349–$1.7771 | $0.11957–$0.14809 | 12/0/0 |
| `WU-consume-023` | $1.4038 | $0.11698 | 12/0/0 |

## Run-log observations

Canonical controller wall time had mean 118.4 seconds, median 98.5 seconds,
p95 270.5 seconds, and maximum 732.7 seconds. Agent-only, flexible, and guided
means were 120.8, 126.0, and 108.6 seconds respectively.

Terra made 401 candidate checks. Only 152 of 240 first checks were accepted;
the rejected checks included 60 prover failures, 49 compile failures, 21
forbidden weakenings, 15 incomplete contracts, seven policy violations, two
prover timeouts, and one infrastructure failure. The hardest refinement traces
were BA, SM, QP, and VS-redeem. One BA agent-only cell made 12 checks. Guided
reduced the mean checks from 2.03 in agent-only to 1.38, consistent with its
lower cost and wall time.

Generation began at concurrency three and was raised to five. A dispatcher
outage aborted 44 in-flight cells after 192 completions; four attempts were
classified as infrastructure-invalid, three recovered, and one remained
invalid. The shared cooperative gate limited machine-wide Boogie admission to
two process groups.

## Provenance and evidence

- Experiment source commit: `b2694df1dbe98d06fcd41ebf0b0d4f588e6a3682`.
- Original apparatus branch at archive time: `origin/wrwg/inf-terra` at
  `252b2b4152c4b0917dfe751dcdb2b5aa25c4bc46`.
- Move Flow SHA-256:
  `06eafaeabd8f2f43718763d42a2232ea37226e084005db733efc7ab6f0d10443`.
- Controller harness SHA-256:
  `3a8a6495c2a26da5849ca75a5a7e4256e4dea36484e4d22ebd1a38a682762d55`.
- Original archive:
  [`corpus3.2-run9-codex-terra56-high.tar.gz`](corpus3.2-run9-codex-terra56-high.tar.gz).
- Post-hoc recovery archive:
  [`corpus3.2-run9-codex-terra56-posthoc-disqualification.tar.gz`](corpus3.2-run9-codex-terra56-posthoc-disqualification.tar.gz),
  SHA-256 `90fc9ef8ced7cd322f1548ef20219bdf7430248e7b0420e82f79b3adbb8e652c`.
- Combined comparison:
  [`corpus3.2-opus5-sol56-terra56-comparison.md`](corpus3.2-opus5-sol56-terra56-comparison.md).

The original archive remains unchanged so its scheduling defect is auditable.
The recovery archive contains the gate runner, compact summary, all per-cell
gate results, the preserved first inconclusive score, retry events, and internal
SHA-256 checksums.
