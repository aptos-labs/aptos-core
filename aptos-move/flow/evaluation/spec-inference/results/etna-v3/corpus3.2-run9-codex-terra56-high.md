# Corpus 3.2 run 9: GPT-5.6 Terra high

This round evaluated the full selected corpus-v3.2 set with explicit
`gpt-5.6-terra` through the pinned Codex runtime at `high` reasoning effort.
The design was 20 tasks, three arms, and four replicas: 240 canonical cells.
Generation used acceptance feedback only, with no refutation or scoring retry.
A separately recorded clean retry was permitted only for genuine infrastructure
failures. Held-out mutation scoring ran after generation.

## Executive comparison

The measured quality result saturated: every scoreable contract killed every
essential held-out mutant. The meaningful arm difference in this round is
therefore cost, where `hybrid_guided` was clearly cheapest. `agent_only` and
`hybrid_flexible` were nearly tied at the lower price bound, while flexible's
greater long-context exposure made its upper bound higher.

| arm | strict success / scheduled | measured mutants | missingness | API-equivalent cost | mean / scheduled cell | cost / strict success |
| --- | ---: | ---: | --- | ---: | ---: | ---: |
| `agent_only` | 80/80 (100.00%) | 240/240 killed | none | $16.9452–$25.5731 | $0.21181–$0.31966 | $0.21181–$0.31966 |
| `hybrid_flexible` | 79/80 (98.75%) | 237/237 killed | 1 infrastructure-invalid | $17.0387–$27.1586 | $0.21298–$0.33948 | $0.21568–$0.34378 |
| `hybrid_guided` | 79/80 (98.75%) | 237/237 killed | 1 not scorable | $13.8913–$19.9452 | $0.17364–$0.24932 | $0.17584–$0.25247 |
| **overall** | **238/240 (99.17%)** | **714/714 killed** | **2 cells** | **$47.8752–$72.6770** | **$0.19948–$0.30282** | **$0.20116–$0.30537** |

For comparison with point estimates from other Codex rounds, use the first
number: **$47.8752 total at ordinary list rates**. The second number is a
**$72.6770 conservative long-context ceiling**, not a second estimate or a
confidence interval. Actual model access was included in the Codex subscription,
so the observed marginal token charge was zero.

## Quality comparison

Generation produced 239 operational successes and one terminal infrastructure
invalidity. Hidden scoring produced 238 strict successes, no surviving mutant,
no disqualification, and no inconclusive mutant result. Consequently:

- All 238 scored cells had mutation adequacy 1.0.
- There is no observed contract-quality separation among the arms on the held-out
  set. Scheduled-cell percentages differ only because of missingness.
- `agent_only` is the only arm with 80 fully measured cells; that is not evidence
  that its contracts were stronger than the two hybrid arms.

The infrastructure-invalid cell was
`BA-base-012/r03/hybrid_flexible`. Four initial infrastructure-invalid attempts
received the one clean retry allowed by the clarified protocol; three recovered
and this one remained invalid. No further retry was made.

The other unmeasured cell was `SM-select-022/r02/hybrid_guided`. It was an
operational success, but the scorer could not apply
`SM-select-022-one-round-short`: the candidate inserted a specification block
into the loop condition, so the implementation fragment expected by the mutant
was no longer present unchanged. This is deterministic scoring-artifact
incompatibility, not a refutation and not a transient infrastructure failure. It
was retained as `not_scorable` and was not retried.

## Cost comparison

The total includes all 244 completed model attempts: the 240 canonical final
attempts plus four quarantined infrastructure attempts. Each quarantined attempt
is assigned back to its canonical run ID in the per-cell accounting. Dividing
only by successful or scoreable attempts would hide genuine spend.

| arm | input tokens | cached input | billed output | reasoning subset | aggregate turns above 272K | total cost |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| `agent_only` | 25,563,482 | 22,038,272 | 457,259 | 163,250 | 37 | $16.9452–$25.5731 |
| `hybrid_flexible` | 27,521,916 | 23,976,448 | 429,377 | 136,134 | 41 | $17.0387–$27.1586 |
| `hybrid_guided` | 20,447,242 | 17,377,280 | 356,325 | 117,258 | 26 | $13.8913–$19.9452 |
| **overall** | **73,532,640** | **63,392,000** | **1,242,961** | **416,642** | **104** | **$47.8752–$72.6770** |

`input tokens` includes the cached subset. `billed output` is visible output plus
reasoning output, with reasoning charged exactly once.

### Relative arm cost

| contrast | lower-bound difference | upper-bound difference | quality result |
| --- | ---: | ---: | --- |
| C1: flexible versus agent-only | +0.6% | +6.2% | no measured mutant difference; flexible has one infrastructure-invalid cell |
| C2: guided versus flexible | −18.5% | −26.6% | no measured mutant difference; guided has one not-scorable cell |
| C3: guided versus agent-only | −18.0% | −22.0% | no measured mutant difference |

Agent-only did use the most reasoning tokens, so including reasoning raised its
estimate more than either hybrid arm: about $1.96 at the lower bound, versus
$1.63 for flexible and $1.41 for guided. Flexible nevertheless remains slightly
more expensive because it used 1.96M more input tokens than agent-only and had
41 rather than 37 aggregate turns above the long-context threshold. Guided used
both the least input and the least output.

### Long-context pricing

[Official OpenAI documentation](https://developers.openai.com/api/docs/models/gpt-5.6-terra)
listed, on 2026-09-11, $2.00/M uncached input, $0.20/M cached input,
and $12.00/M output for GPT-5.6 Terra. Prompts over 272K input tokens are
priced at 2× input/cache and 1.5× output for the full request.

Codex's retained `turn.completed` telemetry reports aggregate usage for a turn,
not the input size of each underlying model request. Because the surcharge is
decided per request, exact surcharge attribution is therefore impossible from
the available receipts:

- The lower bound applies ordinary rates to every recorded token.
- The upper bound applies the long-context multipliers to every aggregate turn
  whose summed input exceeded 272K.

The report uses the ordinary-rate amount as its comparable point estimate and
retains the ceiling so that possible Terra long-context charges are not silently
discarded. Any underlying request-level total lies between those two
calculations. There were no cache-write tokens. The preliminary live report
omitted the separately reported reasoning output; the archived calculation
corrects that error and records both the old and corrected totals in the
operator events.

### Task cost

Each task has 12 canonical cells. The total includes any infrastructure retry
spend assigned to that task; the mean divides by 12 scheduled cells.

| task | total cost | mean / scheduled cell | strict success |
| --- | ---: | ---: | ---: |
| `BA-base-012` | $6.8176–$12.2281 | $0.56813–$1.01900 | 11/12 |
| `BK-bucket-016` | $1.8641–$2.3471 | $0.15534–$0.19559 | 12/12 |
| `LP-price-021` | $1.6473–$1.8039 | $0.13728–$0.15032 | 12/12 |
| `MD-median-015` | $1.1616–$1.1616 | $0.09680–$0.09680 | 12/12 |
| `MM-min-013` | $1.9671–$2.5313 | $0.16393–$0.21094 | 12/12 |
| `OV-order-006` | $1.1462–$1.1462 | $0.09552–$0.09552 | 12/12 |
| `PM-curve-027` | $1.2620–$1.2620 | $0.10516–$0.10516 | 12/12 |
| `QP-part-025` | $3.4073–$5.7995 | $0.28394–$0.48329 | 12/12 |
| `SM-select-022` | $5.0636–$8.7448 | $0.42196–$0.72874 | 11/12 |
| `TL-lev-020` | $2.4511–$4.3239 | $0.20426–$0.36033 | 12/12 |
| `TR-cancel-026` | $2.8127–$4.8391 | $0.23439–$0.40326 | 12/12 |
| `TR-discard-011` | $2.0903–$3.2227 | $0.17419–$0.26856 | 12/12 |
| `TR-order-010` | $2.2604–$3.3583 | $0.18837–$0.27986 | 12/12 |
| `TS-trial-019` | $1.3273–$1.3273 | $0.11061–$0.11061 | 12/12 |
| `UC-credits-008` | $2.0937–$2.9686 | $0.17447–$0.24739 | 12/12 |
| `VS-contrib-003` | $1.5918–$2.2181 | $0.13265–$0.18484 | 12/12 |
| `VS-fees-001` | $2.5690–$3.8054 | $0.21408–$0.31712 | 12/12 |
| `VS-redeem-004` | $3.5037–$6.4082 | $0.29197–$0.53402 | 12/12 |
| `VS-shares-002` | $1.4349–$1.7771 | $0.11957–$0.14809 | 12/12 |
| `WU-consume-023` | $1.4038–$1.4038 | $0.11698–$0.11698 | 12/12 |

Task difficulty dominated pooled arm differences. At the lower bound the most
expensive task, `BA-base-012`, cost 5.95× the cheapest, `OV-order-006`; at the
upper bound it cost 10.67× as much. The two cells with missing quality
measurements are also in the two most expensive task families.

## Comparison with the archived Opus 5 Foundry round

The adjacent Opus 5 report used the same 20-task, three-arm, four-replica shape.
The comparison is descriptive rather than a controlled model ablation: provider,
runtime, apparatus commit, and scoring treatment differ. In particular, the
Opus round also applied the ordinary mutant set as a disqualification gate,
while this Terra round applied only the scheduled held-out scoring set.

| model round | strict / scheduled | conclusive mutant failures | unmeasured | API-equivalent total | cost / strict success |
| --- | ---: | ---: | ---: | ---: | ---: |
| Opus 5 Foundry high | 230/240 (95.83%) | 6 | 4 | $115.6532 | $0.50284 |
| GPT-5.6 Terra high | 238/240 (99.17%) | 0 | 2 | $47.8752–$72.6770 | $0.20116–$0.30537 |

Terra's API-equivalent total is 37.2–58.6% lower, and its cost per strict
success is 39.3–60.0% lower. Its scheduled strict-success count is 3.33
percentage points higher. That is not evidence that Terra wrote stronger
contracts: every measured cell in both rounds passed the held-out scoring set,
and the Opus shortfall consists of six failures from the additional gate plus
four infrastructure-unmeasured cells. The defensible cross-round conclusion is
lower Terra cost with similarly saturated held-out quality, not a quality
ranking between the models.

## Time and provenance

Canonical controller wall time had mean 118.4 seconds, median 98.5 seconds,
p95 270.5 seconds, and maximum 732.7 seconds. These are diagnostics, not the
primary efficiency measure, and overlapped under concurrent dispatch.

Generation began at concurrency three and was raised to five at
2026-09-11 20:55:36 UTC. Infrastructure recovery used concurrency four, and
hidden scoring used concurrency five. One cooperative gate in this container,
activated at 2026-09-11 17:37:46 UTC, coordinated through the shared btrfs
volume and limited machine-wide Boogie admission to two process groups.

- Experiment source commit: `b2694df1dbe98d06fcd41ebf0b0d4f588e6a3682`.
- Preserved apparatus branch: `origin/wrwg/inf-terra`, head
  `252b2b4152c4b0917dfe751dcdb2b5aa25c4bc46` at archive time.
- Flow binary SHA-256:
  `06eafaeabd8f2f43718763d42a2232ea37226e084005db733efc7ab6f0d10443`.
- Controller harness SHA-256:
  `3a8a6495c2a26da5849ca75a5a7e4256e4dea36484e4d22ebd1a38a682762d55`.
- Experiment config SHA-256:
  `101929086b6f29517e37a7cc77685180c2f2c03a224574712694c94a3b9fc471`.
- Gate binary SHA-256:
  `77bb6138b2d3b938e4890dde0ed64e7aaeaf90e5a89764923f5b0406f89d0900`.

## Archive contents

The companion archive excludes raw transcripts, full workspaces, binaries,
credentials, solver scratch files, and proprietary Etna source. It includes
the report; compact canonical-cell, attempt, and turn tables; corrected pricing
and analysis data; final mutation summary and 238 per-cell mutation verdicts;
infrastructure-recovery evidence; all 240 schedule records; execution and
apparatus identities; and an internal `SHA256SUMS`.

- Archive: `corpus3.2-run9-codex-terra56-high.tar.gz`
- Archive SHA-256:
  `d2030b624163ea31edc9b9a3e86ee6c61aeb5203a3bcc6bbc59a80be4c6f9d5d`
