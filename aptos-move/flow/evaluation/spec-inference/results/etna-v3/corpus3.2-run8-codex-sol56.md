# Corpus 3.2 run 8: GPT-5.6 Sol with Codex

This round evaluated the full selected corpus-v3.2 set with `gpt-5.6-sol`,
Codex CLI, high reasoning effort, 20 tasks, three arms, and four replicas: 240
cells. Generation received acceptance feedback only. Withheld mutants were a
post-run gate, so a surviving mutant was a final failure with no refutation
repair retry.

## Cost and quality result

All 240 generation cells reached operational success. Held-out scoring produced
231 strict successes and nine final disqualifications, with no unmeasured cells.

| arm | strict success | disqualified | corrected cost | mean/cell | cost/strict success |
| --- | ---: | ---: | ---: | ---: | ---: |
| `agent-only` | 75/80 (93.75%) | 5 | $36.1884 | $0.4524 | $0.4825 |
| `hybrid-flexible` | 78/80 (97.50%) | 2 | $43.8193 | $0.5477 | $0.5618 |
| `hybrid-guided` | 78/80 (97.50%) | 2 | $38.5672 | $0.4821 | $0.4945 |
| **overall** | **231/240 (96.25%)** | **9** | **$118.5750** | **$0.4941** | **$0.5133** |

Guided and flexible each gained 3.75 percentage points of strict success over
agent-only. Corrected cost was 6.57% higher for guided and 21.09% higher for
flexible than agent-only. Guided and flexible tied on aggregate strict quality,
while flexible cost 13.62% more than guided. Descriptively, guided is therefore
the best cost/quality point among the two hybrid workflows, while agent-only is
the least expensive overall.

The extra cost per three additional strict successes versus agent-only is
$0.7929
for guided and $2.5436
for flexible. These ratios are descriptive, not causal estimates.

## Cost-integrity correction

The live monitor originally priced only Codex CLI `output_tokens`, which is
visible output. Codex telemetry records `reasoning_output_tokens` separately;
billable generated output is their sum. The controller independently records
that identity, and it reconciled for every retained cell. The corrected retained
estimate is **$118.5750**, replacing **$108.7579**: a **$9.8171
(9.03%)** increase. No token is counted twice.

Agent-only did use the most raw reasoning output (133,380
tokens), versus 126,736 guided and
115,189 flexible. It still remains
cheapest because input and long-context pricing dominate. Reasoning added
$3.2797 to agent-only,
$3.4101 to guided, and
$3.1273 to flexible;
guided's dollar increment is slightly larger because more of its reasoning fell
in requests above the long-context threshold.

## Pricing and cost breakdown

API-equivalent list pricing retrieved 2026-09-11 is $4/M uncached input,
$0.40/M cached input, and $20/M generated output. For requests above 272K input
tokens, the full request is 2× input and 1.5× output. Cache writes, absent here,
would be 1.25× the uncached input rate. These are API-equivalent estimates, not
Codex subscription invoices. Sources: [GPT-5.6 Sol model pricing](https://developers.openai.com/api/docs/models/gpt-5.6-sol)
and the [Responses API reference](https://developers.openai.com/api/reference/cli/resources/responses/methods/create),
which states that generated-token limits include visible output and reasoning.

| billed component | tokens | rate | corrected cost | share |
| --- | ---: | --- | ---: | ---: |
| Fresh input | 9,094,972 | $4/M; 2× above threshold | $52.5094 | 44.28% |
| Cached input | 54,875,904 | $0.40/M; 2× above threshold | $35.1435 | 29.64% |
| Cache writes | 0 | $5/M; 2× above threshold | $0.0000 | 0.00% |
| Visible output | 818,040 | $20/M; 1.5× above threshold | $21.1050 | 17.80% |
| Reasoning output | 375,305 | $20/M; 1.5× above threshold | $9.8170 | 8.28% |

| arm | corrected | visible-only prior | correction | base-rate counterfactual | long-context turns |
| --- | ---: | ---: | ---: | ---: | ---: |
| `agent-only` | $36.1884 | $32.9087 | +$3.2797 | $27.0408 | 22/80 |
| `hybrid-flexible` | $43.8193 | $40.6920 | +$3.1273 | $28.5059 | 36/80 |
| `hybrid-guided` | $38.5672 | $35.1572 | +$3.4101 | $26.6505 | 27/80 |

Without the long-context multipliers, retained usage would cost
$82.1971. The
official premium is therefore $36.3778.
Flexible crossed 272K on 36/80 turns, guided on 27/80, and agent-only on 22/80;
that threshold incidence explains much of the arm ordering.

Known excluded-replica, invalid-attempt, and interrupted usage raises the
all-work API-equivalent minimum to
$121.6417.
Four interrupted sessions lack terminal usage, so this is a lower bound.

## Quality comparison and uncertainty

The planned contrasts use task as the independent unit. The intervals below are
10,000 task-cluster percentile bootstrap resamples; all four replicas and all
arms remain together when a task is sampled.

| contrast | strict-rate difference [95% task CI] | mean cell-cost difference [95% task CI] | paired blocks: left-only / right-only / both fail |
| --- | ---: | ---: | ---: |
| C1: `hybrid_flexible minus agent_only` | +3.75 pp [+0.00, +11.25] | $+0.0954 [$-0.0067, $+0.2013] | 4 / 1 / 1 |
| C2: `hybrid_guided minus hybrid_flexible` | +0.00 pp [-3.75, +3.75] | $-0.0657 [$-0.1244, $-0.0023] | 2 / 2 / 0 |
| C3: `hybrid_guided minus agent_only` | +3.75 pp [-3.75, +15.00] | $+0.0297 [$-0.0642, $+0.1262] | 5 / 2 / 0 |

The quality intervals include zero. With only four replicas and failures
concentrated in two tasks, this round does not establish a statistically
reliable arm ranking. It does show the observed cost ordering clearly.

Nine final failures came from two withheld mutants. No failure received a
refutation repair retry:

| task | replica | arm | surviving withheld mutant |
| --- | ---: | --- | --- |
| `OV-order-006` | 1 | `agent-only` | `OV-order-006-reorder-guards` |
| `OV-order-006` | 1 | `hybrid-flexible` | `OV-order-006-reorder-guards` |
| `OV-order-006` | 2 | `agent-only` | `OV-order-006-reorder-guards` |
| `OV-order-006` | 3 | `agent-only` | `OV-order-006-reorder-guards` |
| `OV-order-006` | 4 | `agent-only` | `OV-order-006-reorder-guards` |
| `QP-part-025` | 1 | `hybrid-flexible` | `QP-part-025-lost-element` |
| `QP-part-025` | 2 | `agent-only` | `QP-part-025-lost-element` |
| `QP-part-025` | 3 | `hybrid-guided` | `QP-part-025-lost-element` |
| `QP-part-025` | 4 | `hybrid-guided` | `QP-part-025-lost-element` |

## Per-task comparison

Each row contains 12 cells. Arm columns are mean corrected cost across four
replicas. Most tasks were 12/12 strict; the failures are concentrated as above.

| task | total cost | mean/cell | strict | DQ | AO mean | HF mean | HG mean |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| `BA-base-012` | $13.9827 | $1.1652 | 12 | 0 | $1.0248 | $1.1933 | $1.2776 |
| `BK-bucket-016` | $3.0331 | $0.2528 | 12 | 0 | $0.2243 | $0.2784 | $0.2555 |
| `LP-price-021` | $3.1802 | $0.2650 | 12 | 0 | $0.2938 | $0.2526 | $0.2486 |
| `MD-median-015` | $2.2648 | $0.1887 | 12 | 0 | $0.2205 | $0.1951 | $0.1507 |
| `MM-min-013` | $4.6900 | $0.3908 | 12 | 0 | $0.2505 | $0.5335 | $0.3884 |
| `OV-order-006` | $2.3989 | $0.1999 | 7 | 5 | $0.2055 | $0.2059 | $0.1883 |
| `PM-curve-027` | $2.3863 | $0.1989 | 12 | 0 | $0.2116 | $0.2303 | $0.1547 |
| `QP-part-025` | $13.3996 | $1.1166 | 8 | 4 | $0.7231 | $1.4281 | $1.1987 |
| `SM-select-022` | $10.4741 | $0.8728 | 12 | 0 | $0.6043 | $0.9117 | $1.1025 |
| `TL-lev-020` | $6.2632 | $0.5219 | 12 | 0 | $0.4756 | $0.6956 | $0.3946 |
| `TR-cancel-026` | $9.3265 | $0.7772 | 12 | 0 | $0.7679 | $0.6418 | $0.9219 |
| `TR-discard-011` | $5.2681 | $0.4390 | 12 | 0 | $0.3378 | $0.5910 | $0.3882 |
| `TR-order-010` | $4.0286 | $0.3357 | 12 | 0 | $0.2397 | $0.5000 | $0.2675 |
| `TS-trial-019` | $2.7865 | $0.2322 | 12 | 0 | $0.3043 | $0.2066 | $0.1857 |
| `UC-credits-008` | $5.6848 | $0.4737 | 12 | 0 | $0.3193 | $0.6551 | $0.4468 |
| `VS-contrib-003` | $2.9865 | $0.2489 | 12 | 0 | $0.3769 | $0.2129 | $0.1568 |
| `VS-fees-001` | $9.4546 | $0.7879 | 12 | 0 | $0.6991 | $0.9284 | $0.7362 |
| `VS-redeem-004` | $10.2739 | $0.8562 | 12 | 0 | $0.8771 | $0.8831 | $0.8083 |
| `VS-shares-002` | $4.1245 | $0.3437 | 12 | 0 | $0.6520 | $0.2087 | $0.1704 |
| `WU-consume-023` | $2.5680 | $0.2140 | 12 | 0 | $0.2388 | $0.2030 | $0.2002 |

## Most expensive retained cells

| task | replica | arm | cost | outcome |
| --- | ---: | --- | ---: | --- |
| `QP-part-025` | 4 | `hybrid-flexible` | $1.8331 | scored |
| `BA-base-012` | 4 | `hybrid-guided` | $1.8021 | scored |
| `BA-base-012` | 3 | `agent-only` | $1.7399 | scored |
| `QP-part-025` | 2 | `hybrid-guided` | $1.6592 | scored |
| `BA-base-012` | 2 | `hybrid-guided` | $1.5316 | scored |
| `QP-part-025` | 2 | `hybrid-flexible` | $1.4871 | scored |
| `BA-base-012` | 2 | `hybrid-flexible` | $1.4530 | scored |
| `QP-part-025` | 1 | `hybrid-guided` | $1.3448 | scored |
| `BA-base-012` | 4 | `hybrid-flexible` | $1.3044 | scored |
| `QP-part-025` | 1 | `hybrid-flexible` | $1.2890 | disqualified |

## Run conditions and recovery

- Model: `gpt-5.6-sol`, high effort, Codex CLI 0.153.2.
- All 240 retained cells reached operational success; scoring finished 231
  strict, nine disqualified, zero not-scorable, zero inconclusive.
- Generation concurrency began at three and changed to two after 137 retained
  completions. Scoring later ran up to five-way parallel; the cooperative gate
  limited Boogie to two machine-wide process groups.
- The gate ran from `2026-09-11T17:41:29.804231954Z` to
  `2026-09-11T22:14:56.146365742Z` and stopped cleanly.
- Refutation feedback/retries: zero. Infrastructure retry allowance: one. One
  session-ID collision and three scoring-apparatus failures were replaced
  without model feedback; invalid attempts remain in recovery evidence.

## Provenance and archive contents

- Experiment/source commit: `ed70ba961feffdc3aca03f41d113f0693ff3674f`.
- Installed `move-flow` 2.0.0 SHA-256:
  `06eafaeabd8f2f43718763d42a2232ea37226e084005db733efc7ab6f0d10443`.
- Active schedule SHA-256:
  `5b2a6e2cdf55c78be8d0751d331bb734022d2b991342afbd7f3eeb12651460f2`.
- Final audit: 240/240, zero issues and zero retained infrastructure-invalid cells.

The companion archive contains this report, per-cell and per-turn tables,
corrected cost and task-cluster analyses, all final mutant verdicts, schedule
manifests, audit/config/apparatus data, operator events, and compact recovery
evidence. Raw transcripts, workspaces, binaries, credentials, private source,
and solver scratch are excluded.

The archive is [`corpus3.2-run8-codex-sol56.tar.gz`](corpus3.2-run8-codex-sol56.tar.gz); SHA-256
`b2a532309fae78e9b8a7708ac146885d4a1a38dfa668c6e003a9b7de8f1be034`.
