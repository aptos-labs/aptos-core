# Round v1.2-005 Opus: prepared, not launched

Configuration: Opus 5 (`claude-opus-5`), effort **xhigh**, 180,000 output tokens per
cell, 4 replicates, concurrency 4. All 20 selected tasks and all three arms:
240 cells in 80 randomized blocks. MCP uses `--no-package-cache`; hybrid
plugins use `--no-wp-simplification`. Intermediate verification guidance stays
5/10 seconds, with agent overrides allowed; judging stays at 40 seconds.

Readiness evidence:

- All 20 tasks passed fresh screening; references proved without vacuity.
- All 117 reviewed mutations were rejected: 60 scoring, 57 withheld
  disqualification mutations. No survivors or inconclusive results.
- All 10 final preflight checks passed, including subscription credential
  presence, pinned executable/SDK versions, sandbox isolation/prover execution,
  and all 240 scheduled cell identities in `round-v1.2-005-opus-xhigh`.
- The 386 harness tests passed (one skipped). GLM retains max effort; tests cover
  selecting Opus xhigh, restoring GLM max, and passing xhigh into the runtime.
  Tool repair validation is recorded
  in `aptos-move/flow/state-label-assumptions-investigation.md`.

Source and retention:

- Apparatus commit: `ef12d18d63c0f08278b61cb07a4f7d9d228b9def`.
- Prover repair: `bc8396284e`; no new Flow/prover edits during this preparation.
- Corpus source and Etna pin unchanged.
- Retained branch: `origin/snapshots/corpus-v1.2-005-opus` in
  `https://github.com/wrwg/aptos-core`. Its preparation commit contains this
  record and retains the apparatus commit as an ancestor. Keep it outside
  Graphite; never restack, force-push, or delete it. This is a retention policy,
  not a server-side branch-protection rule.
- The user explicitly approved this alternative to waiting for landing. The
  scheduler's `source_commit_provenance.durable = false` means not on mainline;
  it does not know about the retained snapshot. Record a future landed commit
  separately, without replacing the run's original provenance.

The generated configuration, plugins, schedule, launcher, and final preflight
are archived at `results/corpus-v1.2/round-v1.2-005-opus-xhigh-preparation.tar.gz`
(relative to the evaluation directory). Extract there to recreate
`evaluation-artifacts/round-v1.2-005-opus-xhigh/`; the snapshot also retains the corpus
and both mutant sets. Rebuild Flow with profile `ci` and recheck executable
digests before launching on another machine. No credential values or model
session artifacts are in the archive; the launcher references a local token
file, overridable with `MOVE_INFERENCE_CLAUDE_TOKEN_FILE`.

The older `round-v1.2-005-opus-preparation.tar.gz` records a max-effort
preparation that was superseded at the user's request before any run started.
Do not launch it. Screening was resumed against unchanged corpus, solver, and
Flow binary identities to refresh configuration evidence; all 117 mutation
proof results remain applicable. The effort change affected only the harness.

No model sessions have been launched. Local preflight does not test provider
quota or model availability with a paid request.
