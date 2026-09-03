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
| [`corpus-v3/README.md`](corpus-v3/README.md) | the benchmark corpus: targets, what makes them hard, mutants, what is left before the full run |
| [`corpus-v1/README.md`](corpus-v1/README.md) | the retained framework corpus and the pipeline that built it |
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
- `corpus-v3/` — the benchmark corpus. `corpus-v1/` — retained infrastructure.
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

## Run a round

Every round gets a new round ID. Skills, prompts, tools, models, and limits may
be improved between rounds; a change never rewrites a finished round.

**1. Check the corpus rebuilds to the bytes that were screened.**

```text
python3 corpus-v3/build.py --verify
```

**2. Render one plugin per arm, into the round directory.** Rendering per round
is what lets skills improve without a round mixing two versions. Do not put a
shared telemetry path in a plugin; the controller injects a run-local one.

```text
for arm in agent-only hybrid-guided hybrid-flexible; do
  move-flow plugin ROUND/plugins/acceptance/$arm \
    --inference-tactic $arm --evaluation-mode \
    --feedback-level acceptance --flow-source-commit COMMIT
done
```

`plugins.json` maps `{feedback_level: {arm: path}}`; a flat `{arm: path}` map is
read as a single `acceptance` level. Its paths resolve **relative to
`plugins.json` itself**, so with the layout above the entries are
`plugins/acceptance/<arm>` -- a repository-relative path produces a doubled one
and fails when the scheduler reads the plugin manifest. Every plugin must declare the level its
cell was scheduled with, and the round's `config.json` is a copy of
`config/default.json` with the round's `source_commit`.

**3. Schedule.** Samples whose `screening_status` is not `ready` are dropped;
naming one explicitly is an error, not an override.

```text
move-inference-pilot \
  --corpus-manifest corpus-v3/manifest.json \
  --mutants-root corpus-v3/mutants \
  --plugins ROUND/plugins.json \
  --output-dir ROUND/schedule \
  --source-commit COMMIT \
  --experiment-config ROUND/config.json \
  --replicates N \
  --round-id ROUND_ID
```

`--mutants-root` turns on strict scoring and requires a manifest per scheduled
task, so a round cannot fall back to core scoring in silence.

A round may run a subset. Which subset is a corpus decision, made from the
corpus's own description of each task -- never from an arm's behaviour --
by `corpus-v3/select_round.py`, and recorded both as `round_selection` on every
manifest record and in `corpus-v3/metadata/selection.json`. Held-back samples
stay in the corpus for a later round. Schedule the recorded selection with:

```text
--tasks $(python3 -c "import json;print(' '.join(json.load(open('corpus-v3/metadata/selection.json'))['selected']))")
```

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
  --forbidden-path /absolute/path/to/corpus-v3/mutants \
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
  --round-dir ROUND --mutants-root corpus-v3/mutants
```

## Analyse a finished round

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
