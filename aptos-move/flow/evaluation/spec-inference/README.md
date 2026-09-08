# Move Specification-Inference Evaluation

A reproducible framework for evaluating Move-prover specification inference. It
compares three workflows — unaided inference, a prescribed WP workflow, and a free one —
on the same Move tasks, the same model, and the same configuration, and scores
the resulting specifications both by whether they verify and by whether they
reject wrong code.

This file is the runbook. It says how to run things, not how they work.

## Where to read what

| document | covers |
|---|---|
| [`DESIGN.md`](DESIGN.md) | the design: experimental arms and contrasts, what a task is, where the corpus comes from, how a round executes, how a result is scored, the analysis plan, and the validity and contamination arguments |
| [`CLAUDE.md`](CLAUDE.md) | working rules for editing this tree — dependency-contract methodology, loop/`sathard` methodology, the safe working sequence |
| [`corpus-v3.2/README.md`](corpus-v3.2/README.md) | the benchmark corpus: targets, selection, reference specifications, mutants, and run readiness |
| [`corpus-v1.2/README.md`](corpus-v1.2/README.md) | the retained framework corpus and the pipeline that built it |
| [`sandbox/README.md`](sandbox/README.md) | the sandbox: threat model, the two confinement layers, why the solver is proxied, and the explicit non-goals |
| [`analysis/README.md`](analysis/README.md) | one-off round analysis, deliberately outside `harness/` |

## Layout

- `harness/` — the apparatus: corpus preparation, screening, scheduling,
  the controller and its arm-blind follow-up policy, judging, mutation scoring,
  and round analysis. Hashed into `controller_harness_sha256`, so editing it
  mid-round fails the apparatus-identity check.
- `prompts/` — `initial.txt` opens every session and `followups.json` holds the
  follow-up policy. Hashed into `controller_prompts_sha256`. It carries only
  what the skill cannot know; every normative instruction lives in the skill.
- `sandbox/` — the bubblewrap + Landlock launcher, its Boogie proxy, and the
  credential wrapper.
- `config/` — `default.json` (execution) and `corpus.json` (selection).
- `schemas/` — published artifact contracts.
- `corpus-v3.2/` — the benchmark corpus. `corpus-v1.2/` — retained infrastructure.
- `tests/` — dependency-free fixtures; `analysis/` — round analysis.
- `evaluation-artifacts/` — generated round material, gitignored.

## Environment

The base package has no third-party dependencies; install the optional SDK only
for real model runs. It is pinned to `0.2.139`.

```text
python3 -m venv .venv
.venv/bin/pip install -e '.[claude]'
cc -O2 -Wall -Wextra -Werror sandbox/landlock_exec.c -o sandbox/landlock-exec
```

Credentialed commands go through `sandbox/with-glm-env.sh`, which reads
`ZAI_API_KEY` from `~/.config/ai-keys.env`, maps it to the bearer token the
Anthropic-compatible endpoint wants, and forwards only `ANTHROPIC_AUTH_TOKEN`.
It never prints the key.

```text
sandbox/with-glm-env.sh .venv/bin/python -m harness.pilot_preflight ...
```

## Interrupted rounds

`harness.pilot_run --resume` retains terminal outcomes, archives an existing
launch report, and records which historical infrastructure failures are
acknowledged before dispatching unfinished cells. It still runs the full
apparatus preflight: changed harness code, binaries, prompts, or task inputs
require a separately recorded round, not an in-place continuation.

The outage gate requires failures on two distinct tasks. Repeated failures on
one task remain invalid observations but do not alone stop unrelated work.
New failures still count after a resume. A process lock prevents simultaneous
dispatchers from spending the same schedule twice.

Interrupted sandbox sessions are redacted and archived under the round's
`interrupted-runs/`, outside published `runs/`. Their raw telemetry and partial
SDK metrics remain available for cost accounting; missing terminal usage is
not zero usage. The launch report distinguishes queued aborts (`started: false`)
from in-flight aborts (`started: true`). Neither is a completed evaluation.

## Select GLM or Opus

Select the model before screening and scheduling. `--model glm` selects
GLM 5.3 through Z.ai; `--model opus` selects `claude-opus-5` through Anthropic
with Claude subscription authentication. The selector writes a new config,
preserves budgets and source provenance, and refuses to overwrite a config:

```text
.venv/bin/python -m harness.model_profile select --model opus \
  --config evaluation-artifacts/corpus3.2-run1/config.json \
  --output evaluation-artifacts/corpus3.2-run1-opus/config.json
```

For Opus, run `claude setup-token` locally and set `CLAUDE_CODE_OAUTH_TOKEN`
to its token, or save just the token in a private file outside the repository
and set `MOVE_INFERENCE_CLAUDE_TOKEN_FILE` to that path. The sandbox passes
subscription OAuth through and redacts it from artifacts. An API key is not
used as a fallback for the subscription profile.

This launcher chooses credentials from the config for either model:

```text
.venv/bin/python -m harness.model_profile exec --config ROUND/config.json -- --preflight
.venv/bin/python -m harness.model_profile exec --config ROUND/config.json -- \
  .venv/bin/python -m harness.pilot_preflight --config ROUND/config.json \
  --schedule-dir ROUND/schedule --sandbox-wrapper scripts/pilot-sandbox \
  --output ROUND/preflight.json
```

Use the launcher around `harness.pilot_run` as well. Choose a new round ID,
refresh screening with the new config, and schedule again: a model change
changes the config digest. A harness change also invalidates previously
scheduled harness digests. The credential launcher's `--preflight` checks local
credentials and the CLI version; it does not make a model request or verify
account quota. Use the long-lived setup token for unattended runs rather than
an ordinary short-lived login access token.

## Run a round

Every round gets a new round ID. Skills, prompts, tools, models, and limits may
be improved between rounds; a change never rewrites a finished round.

**1. Check the corpus rebuilds to the bytes that were screened.**

```text
python3 corpus-v3.2/build.py --verify
```

**2. Render one plugin per arm, into the round directory.** Rendering per round
is what lets skills improve without a round mixing two versions. Do not put a
shared telemetry path in a plugin; the controller injects a run-local one.

```text
for arm in agent-only hybrid-guided hybrid-flexible; do
  move-flow plugin ROUND/plugins/acceptance/$arm \
    --inference-tactic $arm --evaluation-mode \
    --feedback-level acceptance --max-verification-timeout 20 \
    --flow-source-commit COMMIT
done
```

`plugins.json` maps `{feedback_level: {arm: path}}`; a flat `{arm: path}` map is
read as a single `acceptance` level. Its paths resolve **relative to
`plugins.json` itself**, so with the layout above the entries are
`plugins/acceptance/<arm>` -- a repository-relative path produces a doubled one
and fails when the scheduler reads the plugin manifest. The map keys use the
schedule names `agent_only`, `hybrid_guided`, and `hybrid_flexible`; the plugin
directory names may retain hyphens. Every plugin must declare the level its
cell was scheduled with, and the round's `config.json` is a copy of
`config/default.json` with the round's `source_commit`.

**3. Schedule.** Samples whose `screening_status` is not `ready` are dropped;
naming one explicitly is an error, not an override.

`--source-commit` must be a commit that will still exist when someone reads the
results. aptos-core squash-merges onto a linear `main`: landing rewrites the
message and the parent, so a branch tip becomes a different SHA and is
unreachable once its branch is deleted -- and a one-commit branch is no
exception. The scheduler records `source_commit_provenance` in the manifest and
warns when the commit is on no mainline branch. A pilot may schedule against an
unlanded commit; **a round whose report is checked in must be scheduled against
the landed commit**, as the corpus itself is (`provenance.aptos_core.commit`).

The content hashes are unaffected either way -- they hash the tree, not git --
so a round scheduled before landing still verifies afterwards. What a
non-durable commit costs is the ability to *fetch* the apparatus later.

```text
move-inference-pilot \
  --corpus-manifest corpus-v3.2/manifest.json \
  --mutants-root corpus-v3.2/mutants-scoring \
  --plugins ROUND/plugins.json \
  --output-dir ROUND/schedule \
  --source-commit COMMIT \
  --experiment-config ROUND/config.json \
  --replicates N \
  --round-id ROUND_ID
```

`--mutants-root` turns on strict scoring and requires a manifest per scheduled
task, so a round cannot fall back to core scoring in silence.

It takes the **held-out** set, `corpus-v3.2/mutants-scoring`. The other set,
`corpus-v3.2/mutants`, is what refutation shows the agent, and it is passed at
launch instead:

```text
move-inference-run-pilot ... --refutation-mutants-root $PWD/corpus-v3.2/mutants
```

Scoring an arm on the set it was shown would score it on what it was told, so
the controller refuses a run whose two roots resolve equal. Omit the refutation
root to run without the mechanism; omit `--mutants-root` and the round cannot
report strict success at all.

A corpus may withhold that set instead of refuting with it. corpus-v1.2 does:
schedule it with `--disqualification-mutants-root corpus-v1.2/mutants` and run
with no `--refutation-mutants-root`, so a contract gets no
second attempt at the counterexamples, and the set is applied after the round
as a gate -- a mutation that survives refutes the contract, and the run is
disqualified rather than measured:

```text
.venv/bin/python -m harness.score_round --config ROUND/config.json \
  --round-dir ROUND --mutants-root corpus-v1.2/mutants-scoring \
  --disqualification-mutants-root corpus-v1.2/mutants
```

The two readings differ only in whether the session was given a second attempt
at the mutation, so scoring refuses a gate set that repeats a mutation the run
was shown, or one the round is scored on.

A round may run a subset. Which subset is a corpus decision, made from the
corpus's own description of each task -- never from an arm's behaviour --
by `corpus-v3.2/select_round.py`, and recorded both as `round_selection` on every
manifest record and in `corpus-v3.2/metadata/selection.json`. Held-back samples
stay in the corpus for a later round. Schedule the recorded selection with:

```text
--tasks $(python3 -c "import json;print(' '.join(json.load(open('corpus-v3.2/metadata/selection.json'))['selected']))")
```

### What to measure

Success saturates. Across `pilot-qp-skill1` and `pilot-loop-001`, 24 consecutive
cells reached both operational and strict success -- every arm, every replicate,
on both a canonical target and an extracted one. A round scored only on success
would report that all three arms work and separate nothing.

Cost does not saturate. In `pilot-loop-001` the arms differ in output tokens
while being indistinguishable in outcome:

| arm | mean output tokens (n=6) | CV | vs `agent_only` |
|---|---:|---:|---:|
| `agent_only` | 35,692 | 60% | 1.00x |
| `hybrid_guided` | 27,409 | 36% | 0.77x |
| `hybrid_flexible` | 33,119 | 52% | 0.93x |

The spread matters as much as the mean: a prescribed workflow bounds the tail
(CV 36% against 60%), which is what to expect if its value is preventing
flailing rather than accelerating success. The effect is also target-specific
and reverses -- `hybrid_guided` costs 0.61x on `QP-part-025` and 1.06x on
`TR-cancel-026` -- so report per task before pooling.

Two consequences for round size. None of these gaps is significant at three
replicates: with CVs of 30-60%, detecting a 25% difference in mean tokens at
80% power needs roughly **8-10 replicates per task-arm cell**, not 3. And what
predicts cost best is not the arm but whether the contract needs auxiliary spec
functions: `QP-part-025` needs recursive `sweep`/`partitioned` helpers and costs
2-3x `TR-cancel-026`, which admits a direct quantified characterization with
none.

**4. Preflight, execute, audit.** Real sessions run only inside the sandbox.

```text
move-inference-preflight-pilot --config ROUND/config.json \
  --schedule-dir ROUND/schedule --sandbox-wrapper scripts/pilot-sandbox \
  --output ROUND/preflight.json

move-inference-run-pilot --config ROUND/config.json \
  --schedule-dir ROUND/schedule --artifacts-dir ROUND/runs \
  --sandbox-wrapper scripts/pilot-sandbox \
  --concurrency CONCURRENCY_FIXED_BEFORE_LAUNCH \
  --report ROUND/launch-report.json

move-inference-audit-pilot --config ROUND/config.json \
  --schedule-dir ROUND/schedule --artifacts-dir ROUND/runs \
  --forbidden-path /absolute/path/to/corpus-v3.2/mutants \
  --output ROUND/audit.json
```

Preflight writes no credential values. It requires the exact SDK and Claude
Code versions, the pinned endpoint, one credential, every hash-valid run cell,
a rehearsal in which a simulated outage withholds queued cells, and a sandbox
probe. The audit fails on a missing artifact, malformed event, unexplained
model, token reconciliation error, reused session ID, changed baseline or
plugin, forbidden-path disclosure, or unclassified termination.

**5. Score, after the round.** Mutation scoring runs separately because the
agent shares the sandbox mount namespace and hidden material must never be
mounted beside it.

```text
.venv/bin/python -m harness.score_round --config ROUND/config.json \
  --round-dir ROUND --mutants-root corpus-v3.2/mutants-scoring
```

## Analyse a finished round

Each real session records telemetry before the SDK parser in
`claude-events.jsonl` (`sdk_message` events), in addition to the typed messages
used by the controller. Partial-message streaming is enabled. This retains
native usage fields, per-message IDs and usage, nested cache-write durations,
per-model usage/cost/provider/context limits, stop reasons, permission denials,
API errors, rate-limit/reset/overage events, compaction events, and future
fields whenever Claude Code emits them. HTTP headers and account billing data
that the CLI does not emit are not available here.

Text/thinking/tool-JSON/signature payload deltas are coalesced into
`sdk_stream_summary` records with counts, character lengths and first/last
receipt times; completed content remains in the transcript. Unknown delta
fields and usage updates are retained verbatim. These receipt timings are
local observations, not server-side latency measurements.

`sdk-metrics.json` preserves every result and rate-limit event and summarizes
the reported counters. It sums per-query result usage, takes the last
cumulative cost/model usage per session, and deduplicates API message IDs.
Incomplete queries are flagged; missing costs remain unknown. USD figures are
SDK API-equivalent estimates, not subscription invoices. New runs declare
`sdk_telemetry_schema: 1`, which makes audit require this artifact and check
raw-versus-typed result coverage. Credentials are redacted before logging.

```text
move-inference-mine-transcripts --runs-dir ROUND/runs \
  --output ROUND/mined.json --markdown ROUND/mined.md

move-inference-failure-taxonomy --runs-dir ROUND/runs \
  --output ROUND/taxonomy.json --markdown ROUND/taxonomy.md
```

The miner labels each run by turn use, token category, and failure kind. The
taxonomy holds those labels against the diagnostic categories proposed before
any data existed, and marks a category that never fired as either unreachable
in the corpus or reachable and never triggered — only the second is evidence
about the design. Neither report licenses dropping a category.

For cost accounting, note that the runtime mixes per-turn and
session-cumulative fields in one record; see `DESIGN.md` §5, "Measuring cost".

## Round discipline

- Tune skills, limits, transitions, tools, and corpus composition between
  rounds. Give every iteration a new round ID, hashes, parent, and change log.
- Never overwrite or silently reinterpret a completed round. Report which
  rounds informed tuning and which results are presented as final.
- Corpus changes after seeing results are allowed only as a new, explicitly
  versioned corpus and round; prior artifacts remain unchanged and reportable.
- Never use an arm's outcome to choose corpus membership.
