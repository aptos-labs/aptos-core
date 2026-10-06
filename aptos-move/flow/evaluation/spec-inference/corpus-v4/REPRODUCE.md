# Reproducing the corpus-v4 results

This guide is for readers who want to check what corpus-v4 asserts, re-score a
published round, or run the experiment again. [`README.md`](README.md) describes
the corpus itself; [`../README.md`](../README.md) is the runbook of the
evaluation harness and [`../DESIGN.md`](../DESIGN.md) its design.

## What reproduces, and how far

| claim | how | reproduces |
|---|---|---|
| the package is the public code it claims to be | `build.py --verify` | exactly: digests of every generated file |
| every reference contract proves, is not vacuous, and kills every mutant | `verify.py` | exactly, up to solver nondeterminism near the time limit |
| every task passes screening; which ones WP alone cannot solve | `harness.screen_v3` | verdicts exactly; timings and tool digests are machine-specific |
| the round's selection | `select_round.py` | exactly |
| a published round's scores, from the contracts its sessions produced | `harness.score_round` on the round archive | exactly, given the same `move-flow` and solvers |
| the sessions themselves | rerun the experiment | statistically only |

Model sessions are not deterministic, and the served model changes over time, so
a rerun reproduces a distribution of outcomes and costs, not a transcript. What a
published round makes exactly checkable is everything after the sessions: each
cell's final package, its verification, and its mutation score.

Only the corpus is pinned independently of your checkout: `build.py` reads its
sources from the aptos-core commit in the manifest, whatever is checked out.
The apparatus -- the harness, the prover and `move-flow`, the skills and
prompts -- is the checked-out code. A round records the commit it was scheduled
from as `source_commit`, with digests of everything it ran, so reproducing a
round means checking out that commit; `--flow-source-commit` only labels a
rendered plugin with it. At a later commit the corpus is identical but the
apparatus may not be, and a run there is a new round rather than a replication.

## Working with a coding agent

The harness is a Python package with many entry points and a strict set of
apparatus checks; the most reliable way to run the steps below is to let a
coding agent drive them. Start it in `aptos-move/flow/evaluation/spec-inference`:
that directory's [`CLAUDE.md`](../CLAUDE.md) carries the working rules (never
mix rounds, never edit a finished round, never pick tasks by results), and the
agent reads it together with this file, the runbook and the design.

```text
cd aptos-move/flow/evaluation/spec-inference
claude          # Claude Code reads CLAUDE.md here; point other agents at it explicitly
```

Prompts that work well:

- *Verify corpus-v4 following corpus-v4/REPRODUCE.md: rebuild the package, re-prove
  every reference, re-validate both mutant sets, re-screen, and report any
  difference from the committed records.*
- *Re-score the round in `<archive>` against corpus-v4's mutant sets and compare
  with its published summary, per task and per arm.*
- *Run a new corpus-v4 round with terra56 (or sol61), four replicates,
  following corpus-v4/REPRODUCE.md; stop and report after preflight before
  launching.*

Asking the agent to stop after preflight is worth it: launching spends model
budget, and preflight is where a missing tool or credential shows up.

## Prerequisites

- Linux on `x86_64` or `aarch64`, Python 3.11 or newer, a C compiler, `bwrap`
  (bubblewrap), `git`, and the GitHub CLI `gh`.
- An aptos-core clone containing two commits: the corpus pin,
  `provenance.aptos_core.commit` in [`manifest.json`](manifest.json), from which
  the package is generated, and the apparatus commit a round was scheduled
  against, `source_commit` in its schedule manifest. Check out the apparatus
  commit. A round scheduled before its apparatus landed names a commit of an
  aptos-core pull request; fetch it with
  `git fetch https://github.com/aptos-labs/aptos-core pull/<n>/head`.
- Boogie and Z3 as installed by `./scripts/dev_setup.sh -y`, exported as
  `BOOGIE_EXE` and `Z3_EXE`. A round's `preflight.json` records the versions and
  digests it ran with.
- `move-flow` built from the checked-out commit and first on `PATH`:

  ```text
  cargo build -p aptos-move-flow --profile ci
  export PATH=$PWD/target/ci:$PATH
  ```

All commands below run from `aptos-move/flow/evaluation/spec-inference`.

## 1. Verify the corpus

```text
python3 corpus-v4/verify.py
git status --short corpus-v4
```

`verify.py` regenerates the package from the pinned commit and checks every
digest, assembles the reference packages, checks every task's preparation
patch, and re-validates both mutant sets of every ready task, against the tree
that task starts from, with the harness's own validator. That validator proves the
reference, refuses a vacuous one, confirms the reference carries the corpus
implementation unchanged, and runs every mutant against it. It rewrites the
committed mutant records in place, so an empty `git status` means they were
reproduced.

Screening and the round selection re-derive the same way:

```text
python3 -m harness.screen_v3 --manifest corpus-v4/manifest.json \
  --experiment-config config/default.json --corpus-config config/corpus.json \
  --results-dir corpus-v4/screening --output corpus-v4/screening/summary.json --all-ready
python3 corpus-v4/select_round.py --size 26 --max-guessable 3 --keep-redundant --write
git diff corpus-v4/screening corpus-v4/metadata corpus-v4/manifest.json
```

The screening diff should show only `wall_seconds` and the digests of the local
tools; `passed`, `well_formed`, `reference_proved`, `wp_model_gate` and
`wp_hard` must match.

`wp_model_gate` runs WP with `--aborts-if-is-strict` over the target's module
in its reference package, with only the target's own contract removed. The
reference keeps its loop invariants and dependency contracts, so an error there
is a gap in WP's models (a native, an intrinsic, or a write WP cannot
characterize exactly), not work left to an arm. Screening fails such a task
until WP is fixed.

To read a task as the agents received it, with its reference and every mutant
as a diff, compose it into an untracked directory:

```text
python3 corpus-v4/compose.py --task TR-match-029 --output /tmp/corpus-v4-inspect
```

## 2. Verify a published round

A round is published in two archives. The aggregate archive, under
`results/corpus-v4/`, holds the analysis (`REPORT.md`, `analysis.json`,
`cells.csv`, `requests.csv`, `pricing.json`) beside the round's
`mutation-summary.json`, `audit.json`, `config.json`, `plugins.json`,
`preflight.json` and `pilot-manifest.json`. The round directory itself, with
each cell's final package, workspace diff, transcript and token telemetry, is
several hundred megabytes and is archived with the paper's artifact. Extract
it under `evaluation-artifacts/`, check out its `source_commit`, rebuild
`move-flow`, and re-score it:

```text
python3 -m harness.score_round --config evaluation-artifacts/ROUND/config.json \
  --round-dir evaluation-artifacts/ROUND \
  --mutants-root corpus-v4/mutants-scoring \
  --disqualification-mutants-root corpus-v4/mutants
python3 -m harness.pilot_audit --config evaluation-artifacts/ROUND/config.json \
  --schedule-dir evaluation-artifacts/ROUND/schedule --artifacts-dir evaluation-artifacts/ROUND/runs \
  --forbidden-path $PWD/corpus-v4/mutants --forbidden-path $PWD/corpus-v4/mutants-scoring \
  --output /tmp/audit.json
```

Scoring refuses mutant sets whose digests differ from the ones the round was
scheduled with, and the audit refuses a round whose recorded artifacts do not
match their hashes, so a successful run means the published scores follow from
the published contracts. Compare the new `mutation-summary.json` with the
archived one.

The analysis re-derives from the round directory alone:

```text
python3 -m analysis.codex_round_report --round-dir evaluation-artifacts/ROUND \
  --output /tmp/ROUND-report
```

It prices every recorded model request with the archived table in
[`analysis/pricing.json`](../analysis/pricing.json): input includes the cached
tokens, output includes the reasoning tokens and is billed once, and the
long-context rate applies only to a single request above the threshold, never
to a sum over requests. Startup warmups are priced but reported apart from the
task cost. The estimands are those of [`DESIGN.md`](../DESIGN.md) section 6: the
task is the unit, a contrast is the equal-weight mean over tasks of the
within-task block differences, intervals are 95% task-cluster percentile
bootstraps, and `C1` and `C2` are tested with a blocked randomization test,
Holm-adjusted. The seed is fixed, so `analysis.json` must match the archived
one exactly.

## 3. Rerun the experiment

This repeats the protocol of round 8, which runs once per model: Terra 5.6
(`terra56`, `gpt-5.6-terra`) and Sol 6.1 (`sol61`, `gpt-6.1-sol`), each through
the Codex CLI at `high` effort, three arms, four replicates of all 26 tasks (312
cells per model), concurrency 3, the ordinary mutant set withheld as a
disqualification gate and the held-out set used for scoring. Each task starts
from its task tree, with complete contracts for the helpers its target calls,
and WP's output is not simplified. Round 6, Terra under an earlier protocol
that asked for helper contracts and simplified WP's output, took about four and
a half hours and cost $82 at API-equivalent prices, a mean of $0.26 per cell, as
`analysis.codex_round_report` prices it; a four-task pilot of the current
protocol with Terra cost $0.16 per cell over the agent-only and hybrid-guided
arms. Prices per million tokens are $2.00 input, $0.20 cached and $12.00
output for Terra 5.6, and $2.00, $0.10 and $10.00 for Sol 6.1.

**Environment.**

```text
python3 -m venv .venv && .venv/bin/pip install -e .
cc -O2 -Wall -Wextra -Werror sandbox/landlock_exec.c -o sandbox/landlock-exec
codex login
```

Install the Codex CLI release the round's profile pins, with its code-mode
host, as the runbook's *Environment* section shows: `rust-v0.153.2` for
`terra56` and `rust-v0.160.1` for `sol61`, each into
`evaluation-artifacts/tools/codex-<version>`. The round puts that directory
first on `PATH`; the harness refuses any other version or host.

`codex login` must sign in with a ChatGPT account that has Codex access to the
round's model (`gpt-5.6-terra`, `gpt-6.1-sol`): the round configuration pins the
ChatGPT endpoint, which an API-key login does not use. The launcher copies the
saved `~/.codex/auth.json` into each cell's private sandbox home and removes it
afterwards; it never reaches the artifacts. Set `MOVE_INFERENCE_CODEX_AUTH_FILE`
when the login is stored elsewhere. Usage counts against the account's Codex
limits; the dollar figures above are API-equivalent estimates.

**Round configuration and plugins.** Choose a new round id; every round gets
its own directory and is never rewritten.

```text
MODEL=terra56 CODEX=0.153.2          # or MODEL=sol61 CODEX=0.160.1
export PATH=$PWD/evaluation-artifacts/tools/codex-$CODEX:$PATH
ROUND=evaluation-artifacts/corpus4-ROUNDID-codex-$MODEL-high
COMMIT=$(git rev-parse HEAD)
python3 -m harness.model_profile select --model $MODEL --config config/default.json \
  --output $ROUND/config.json --source-commit $COMMIT
for arm in agent-only hybrid-guided hybrid-flexible; do
  move-flow plugin $ROUND/plugins/acceptance/${arm//-/_} --inference-tactic $arm \
    --evaluation-mode --feedback-level acceptance --aborts-if-is-strict \
    --no-wp-simplification --max-verification-timeout 20 --flow-source-commit $COMMIT
done
```

`--aborts-if-is-strict` makes WP report an abort characterization it cannot
make exact as an error instead of emitting `aborts_if_is_partial`, which the
acceptance check rejects anyway. `--no-wp-simplification` leaves WP's output as
it is instead of instructing the hybrid arms to simplify it, as in the V3.2
rounds.

and write `$ROUND/plugins.json`:

```json
{"acceptance": {"agent_only": "plugins/acceptance/agent_only",
                "hybrid_guided": "plugins/acceptance/hybrid_guided",
                "hybrid_flexible": "plugins/acceptance/hybrid_flexible"}}
```

**Screen under the round's configuration, then schedule.** The scheduler only
admits targets screened with the same configuration and binaries. The screen
rewrites `corpus-v4/screening`, and the next round's screen, for another model,
rewrites it again, so keep the copy the round was scheduled from.

```text
python3 -m harness.screen_v3 --manifest corpus-v4/manifest.json \
  --experiment-config $ROUND/config.json --corpus-config config/corpus.json \
  --results-dir corpus-v4/screening --output corpus-v4/screening/summary.json --all-ready
cp -r corpus-v4/screening $ROUND/screening
python3 -m harness.pilot --corpus-manifest corpus-v4/manifest.json \
  --mutants-root corpus-v4/mutants-scoring --disqualification-mutants-root corpus-v4/mutants \
  --plugins $ROUND/plugins.json --output-dir $ROUND/schedule --source-commit $COMMIT \
  --experiment-config $ROUND/config.json --replicates 4 --round-id $(basename $ROUND) \
  --tasks $(python3 -c "import json;print(' '.join(json.load(open('corpus-v4/metadata/selection.json'))['selected']))")
```

The scheduler records whether `COMMIT` is on `main` and warns when it is not.
A round meant for publication must name a commit that can be fetched later.
aptos-core squash-merges pull requests, so a commit of a pull request survives
only through `refs/pull/<n>/head`: schedule against a pushed commit of the
pull request, add later changes as new commits on top, never rebase or amend
the branch, and record the pull request with the round.

**Preflight, run, audit, score.**

```text
python3 -m harness.model_profile exec --config $ROUND/config.json -- \
  python3 -m harness.pilot_preflight --config $ROUND/config.json \
  --schedule-dir $ROUND/schedule --sandbox-wrapper scripts/pilot-sandbox --output $ROUND/preflight.json
python3 -m harness.model_profile exec --config $ROUND/config.json -- \
  python3 -m harness.pilot_run --config $ROUND/config.json --schedule-dir $ROUND/schedule \
  --artifacts-dir $ROUND/runs --sandbox-wrapper scripts/pilot-sandbox --concurrency 3 \
  --report $ROUND/launch-report.json
```

then audit, score and analyze exactly as in section 2, writing the audit to
`$ROUND/audit.json` and the analysis to `$ROUND/report`, and build the
aggregate archive:

```text
cp $ROUND/config.json $ROUND/plugins.json $ROUND/preflight.json $ROUND/audit.json \
  $ROUND/mutation-summary.json $ROUND/schedule/pilot-manifest.json $ROUND/report/
python3 -m harness.publication build --source $ROUND/report \
  --output results/corpus-v4/$(basename $ROUND).tar.gz --name $(basename $ROUND)
```

Preflight checks the sandbox, the
pinned CLI and host, `move-flow`, the solvers, credentials, the endpoint and the
schedule, and rehearses an outage without spending model budget. A run that is
interrupted resumes with `--resume`; the runbook's *Interrupted rounds* section
explains what that keeps and what it refuses.

Other models are selected the same way (`--model sol61`, `sol56`, or `opus` and
`sonnet` through Claude Code); the runbook lists their credentials.
