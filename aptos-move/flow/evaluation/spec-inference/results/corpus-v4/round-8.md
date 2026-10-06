# Corpus-v4 round 8

Round 8 runs the corpus-v4 protocol of
[`../../corpus-v4/REPRODUCE.md`](../../corpus-v4/REPRODUCE.md) once per model of
the GPT-5.6 generation, each 26 tasks × 3 arms × 4 replicates through Codex CLI
0.153.2: `corpus4-run8-codex-terra56-high` (`gpt-5.6-terra`, the cheaper model)
and `corpus4-run8-codex-sol56-high` (`gpt-5.6-sol`, the frontier model). They
were scheduled from aptos-core commits
`8dbe31181a1dea6fb489d831e568f9a21ba3bbd5` and
`2f5d6160a083b130f785a3bfa1e6a23bdca38833`, both on pull request #20682; the
harness is identical in the two, which differ only in documentation and the
analysis price table.

## Summary

Each round's aggregate archive is in this directory
(`corpus4-run8-codex-terra56-high.tar.gz`, `corpus4-run8-codex-sol56-high.tar.gz`).
The final report each agent wrote at the end of its session, with the cell's
target, tactic and outcome, is in `corpus4-run8-codex-terra56-high-reports.md`
and `corpus4-run8-codex-sol56-high-reports.md` (and as JSON lines in the
`.jsonl` files beside them), extracted by `analysis.agent_reports`.

The tables below are computed from the two archives alone:

```text
python3 -m analysis.codex_model_comparison \
  results/corpus-v4/corpus4-run8-codex-terra56-high.tar.gz \
  results/corpus-v4/corpus4-run8-codex-sol56-high.tar.gz
```

The tactics are compared within a model by the preregistered contrasts C1–C3:
equal-weight means over tasks of within-task differences in `(task, replicate)`
blocks, with 95% task-bootstrap intervals and blocked randomization tests,
Holm-adjusted across C1 and C2. The models are compared within a tactic by
pairing the two rounds' cells by `(task, replicate)`; this comparison is
exploratory. Strict success is in percentage points, cost in dollars per cell.

### Tactics within each model

| model | arm | strict | unmeasured | disqualified | mean cost / cell | vs agent-only | mean wall |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: |
| `gpt-5.6-terra` | agent_only | 97/102 | 2 | 5 | $0.202 |  | 152.9 s |
| `gpt-5.6-terra` | hybrid_flexible | 98/104 | 0 | 6 | $0.181 | -10.7% | 143.1 s |
| `gpt-5.6-terra` | hybrid_guided | 97/103 | 1 | 6 | $0.161 | -20.6% | 148.0 s |
| `gpt-5.6-sol` | agent_only | 103/104 | 0 | 1 | $0.353 |  | 120.7 s |
| `gpt-5.6-sol` | hybrid_flexible | 98/100 | 4 | 2 | $0.306 | -13.4% | 114.9 s |
| `gpt-5.6-sol` | hybrid_guided | 98/103 | 1 | 5 | $0.304 | -13.8% | 125.8 s |

| model | contrast | strict success | Holm p | cost / cell |
| --- | --- | ---: | ---: | ---: |
| `gpt-5.6-terra` | C1: hybrid_flexible minus agent_only | -1.3 pp [-5.1, +1.9] | 1.00 | -0.022 [-0.054, +0.010] |
| `gpt-5.6-terra` | C2: hybrid_guided minus hybrid_flexible | +0.0 pp [-3.8, +4.8] | 1.00 | -0.020 [-0.041, +0.007] |
| `gpt-5.6-terra` | C3: hybrid_guided minus agent_only | -0.3 pp [-6.7, +5.8] | – | -0.042 [-0.083, +0.003] |
| `gpt-5.6-sol` | C1: hybrid_flexible minus agent_only | -1.0 pp [-2.9, +0.0] | 1.00 | -0.047 [-0.094, -0.004] |
| `gpt-5.6-sol` | C2: hybrid_guided minus hybrid_flexible | -1.9 pp [-5.8, +0.0] | 1.00 | -0.001 [-0.040, +0.046] |
| `gpt-5.6-sol` | C3: hybrid_guided minus agent_only | -3.8 pp [-10.6, +0.0] | – | -0.049 [-0.124, +0.031] |

### Models within each tactic: `gpt-5.6-sol` minus `gpt-5.6-terra`

| arm | strict success | cost / cell | wall / cell |
| --- | ---: | ---: | ---: |
| agent_only | +3.8 pp [+0.0, +10.6] | +0.151 [+0.121, +0.184] | -32.3 s [-52.3, -15.4] |
| hybrid_flexible | +4.5 pp [+0.0, +11.5] | +0.125 [+0.100, +0.155] | -28.2 s [-59.0, -2.9] |
| hybrid_guided | +1.0 pp [-1.9, +3.8] | +0.144 [+0.094, +0.203] | -22.2 s [-87.4, +19.6] |

In neither model does a tactic change strict success measurably: every
strict-success contrast has Holm p = 1.00. Both hybrid tactics cost less than
agent-only in both models, by 10.7% and 20.6% for Terra 5.6 (flexible, guided)
and by 13.4% and 13.8% for Sol 5.6; only Sol's C1 cost interval excludes zero.

Sol 5.6 reaches strict success more often than Terra 5.6 in the agent-only and
hybrid flexible tactics, by 3.8 and 4.5 points with intervals reaching down to
zero, and about equally often in the hybrid guided tactic. It costs $0.13 to
$0.15 more per cell in every tactic, about 1.7 to 1.9 times as much, with
intervals that exclude zero. It finishes a cell 22 to 32 seconds sooner on
average; the interval excludes zero for agent-only and hybrid flexible.

## Terra 5.6

All 312 cells completed with operational success. Costs are API-equivalent and
priced per request by `analysis.codex_round_report`; no request reached the
long-context tier.

| arm | strict | unmeasured | disqualified | cost | mean / cell | vs agent-only | mean wall |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| agent-only | 97/102 | 2 | 5 | $21.05 | $0.202 | | 152.9 s |
| hybrid flexible | 98/104 | 0 | 6 | $18.79 | $0.181 | −10.7% | 143.1 s |
| hybrid guided | 97/103 | 1 | 6 | $16.71 | $0.161 | −20.6% | 148.0 s |

Contrasts are equal-weight means over tasks of within-task block differences,
with 95% task-bootstrap intervals; p-values are blocked randomization tests,
Holm-adjusted across C1 and C2.

| contrast | strict success | Holm p | cost / cell |
| --- | ---: | ---: | ---: |
| C1: hybrid flexible − agent-only | −1.3 pp [−5.1, +1.9] | 1.00 | −$0.022 [−0.054, +0.010] |
| C2: hybrid guided − hybrid flexible | +0.0 pp [−3.8, +4.8] | 1.00 | −$0.020 [−0.041, +0.007] |
| C3: hybrid guided − agent-only | −0.3 pp [−6.7, +5.8] | – | −$0.042 [−0.083, +0.003] |

### Operational notes

Three cells needed one infrastructure retry within the cell: in
`can_execute_with_timelock` (hybrid flexible), `select` and
`validate_not_zero_sizes` (hybrid guided) the first attempt failed because the
`move-flow` MCP server did not connect, in the last also because the model was
at capacity. Each retry completed. A failed attempt has no turn-level usage
total, so the round audit reports incomplete request-level telemetry for these
cells. The requests are nevertheless all recorded and priced: the failed attempt
in `can_execute_with_timelock` made six requests, the other two none.

Three cells have no strict-success measurement. In two `select` cells
(agent-only and hybrid guided) the disqualification gate reached no verdict
within the prover's time limit. In one `upsert_provider_jwks` agent-only cell
the verification of the unmutated final package, which scoring runs before any
mutant, failed although the judge had accepted the cell; the same verification
passed three times when repeated afterwards. The round was scored once and is
reported as scored.

### Candidate-check rejections

Each row counts rejected calls of the candidate check over all 312 cells,
classified by the first diagnostic the check reported.

| first diagnostic | agent-only | hybrid flexible | hybrid guided |
| --- | ---: | ---: | ---: |
| compile error | 54 | 14 | 16 |
| missing `normal-result` category | 16 | 16 | 17 |
| `aborts_if` where the function does not abort | 7 | 5 | 16 |
| abort not covered | 17 | 7 | 2 |
| vacuous `ensures true` | 2 | 7 | 11 |
| loop invariant fails (induction or base case) | 11 | 8 | 9 |
| postcondition, timeout and other | 10 | 9 | 9 |
| total | 117 | 66 | 80 |

In the hybrid arms, the rejections that trace back to WP output occur on tasks
with loops. All 21 hybrid rejections of an `aborts_if` that claims an abort the
function does not perform belong to loop tasks (`select`, `new_tiers`,
`get_pending_transactions`, `cancel_at_price_level`, `range_with_step`): where
the loop invariants do not exclude states the loop never reaches, WP's abort
conditions include those states. A missing `normal-result` category on a loop
task arises when WP omits a result clause it cannot state exactly. Invariants
that fail to hold account for 28 of the 263 rejections; the more common case is
an invariant that holds but is too weak.

In the agent-only arm, 54 of the 117 rejections are compile errors in the
specifications the agent wrote, such as unbound modules, unexpected tokens and
signature mismatches.

`validate_scheme` requires a `normal-result` contract category, although its
target, `validate_scheme`, returns no value and its reference specification
states only abort conditions. Agents in all three arms submit contracts
without an `ensures` or with `ensures true`, which the check rejects as vacuous,
before restating the absence of the abort conditions as postconditions: bounds
on the length of `scheme` and a letter in its first position. Every accepted
contract for the task has this form. The task accounts for 10 of the 49
missing-category rejections and 4 of the 20 vacuous-ensures rejections; most of
the remaining vacuous-ensures rejections (16) are on `validate_enough_stake` and
`validate_tiers`, mainly in the hybrid arms.

## Sol 5.6

All 312 cells completed with operational success. Costs are API-equivalent and
priced per request by `analysis.codex_round_report`; no request reached the
long-context tier.

| arm | strict | unmeasured | disqualified | cost | mean / cell | vs agent-only | mean wall |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| agent-only | 103/104 | 0 | 1 | $36.72 | $0.353 | | 120.7 s |
| hybrid flexible | 98/100 | 4 | 2 | $31.81 | $0.306 | −13.4% | 114.9 s |
| hybrid guided | 98/103 | 1 | 5 | $31.67 | $0.305 | −13.8% | 125.8 s |

| contrast | strict success | Holm p | cost / cell |
| --- | ---: | ---: | ---: |
| C1: hybrid flexible − agent-only | −1.0 pp [−2.9, +0.0] | 1.00 | −$0.047 [−0.094, −0.004] |
| C2: hybrid guided − hybrid flexible | −1.9 pp [−5.8, +0.0] | 1.00 | −$0.001 [−0.040, +0.046] |
| C3: hybrid guided − agent-only | −3.8 pp [−10.6, +0.0] | – | −$0.049 [−0.124, +0.031] |

### Operational notes

One cell needed one infrastructure retry within the cell: in
`reinsert_order_into_bulk_order` (hybrid flexible) the first attempt failed
because the `move-flow` MCP server did not connect and the model was at
capacity. The retry completed. The failed attempt made one request, which is
recorded and priced; as for Terra, the round audit reports incomplete
request-level telemetry for the cell because the attempt has no turn-level
total.

Five cells have no strict-success measurement, all for lack of a verdict within
the prover's time limit: in the disqualification gate for `select` (hybrid
flexible and hybrid guided), `partition` and `upsert_provider_jwks` (hybrid
flexible), and in the scoring set for a second `upsert_provider_jwks` cell
(hybrid flexible).
