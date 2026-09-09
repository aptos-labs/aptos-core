# Screening after the state-label repair

All 20 selected v1.2 samples passed treatment-blind admission using the repaired
CI-profile Flow binary. No tasks timed out or were excluded. Each authored
reference proved and passed the vacuity check. Fifteen tasks are WP-hard:
untrusted inferred clauses remain agent repair obligations, not screening
failures or claims that raw WP output verified.

- Tool fix: `bc8396284e` on `wrwg/inf-tool-fixes`.
- Restacked apparatus commit: `9c6a3253563faa0eb530e30f9b9afb75e667aee4`.
- Flow binary SHA-256:
  `5ffa0e4be5673096fcc04a35b79e2f7e1cd17c2dc9dcdc05b4ce9da3d9db8bb8`.
- Reference package SHA-256:
  `1c41a4a754554758e1632217bb867a0dc8c622072f937edf1e1ef44adaf1f116`.
- Stage threshold: 20 seconds; MBQI was not enabled.
- Raw results: `results/<task-id>.json`; aggregate: `manifest.json`.

This is retained raw screening evidence. The promoted scheduling summary is
`../summary.json`, and the corpus-root manifest now records `screened`.
Inherited package/patch paths in `manifest.json` are relative to the corpus root,
not this directory. Mutation validation is a separate readiness check; its
fresh results are recorded in `../../metadata/mutation-validation-005/`.

Current round preparation is in `evaluation-artifacts/round-v1.2-005-opus-xhigh/`:
Opus 5 with xhigh effort (GLM retains max),
180,000 output tokens, 4 replicates, concurrency 4, no package cache, and no
WP-simplification guidance. Final subscription, sandbox, executable, and schedule
preflight passed. All 117 mutations passed fresh validation. No model sessions
were launched. Screening summary configuration identities were refreshed after
the effort-profile change; the prover binary and corpus remain unchanged.
