# Hybrid guidance audit — 2026-09-07

Compared rendered Claude `/move-inf` skills against `b42254cca4`, with evaluation
mode, acceptance feedback, and the same 5/10-second timeout guidance.

| Rendered skill | Before (words / bytes) | After (words / bytes) | With `--no-wp-simplification` |
| --- | --- | --- | --- |
| Hybrid guided | 5,186 / 34,458 | 4,693 / 31,763 | 4,524 / 30,448 |
| Hybrid flexible | 5,119 / 33,951 | 4,656 / 31,484 | 4,491 / 30,209 |

These are measured word and byte counts, not model-token counts. The cleanup
removes about 9% of the skill's words; disabling routine simplification removes
12–13% relative to the previous skill. It does not establish a corresponding
reduction in total session input tokens.

## Findings and changes

- The hybrid workflow and WP reference independently explained exactness,
  bounded loop evidence, focused reruns, and removal of stale clauses. The
  workflow now specifies orchestration; the WP reference defines result handling
  once. Shared language and proof references remain available.
- The old instruction to repeat until every warning disappears contradicted
  the exception for partial callees. Both workflows now retain inherited
  partiality. A caller cannot recover missing abort behavior while the callee's
  contract remains partial.
- Warning-free WP output is complete and correct by construction. WP does not
  verify its output. Verification timeouts require proof repair or an equivalent
  representation; a counterexample to unchanged, warning-free output is a bug.
- Simplification was requested by workflow text, the editing reference, and the
  MCP tool description. The new flag removes the routine step and reference
  from hybrid instructions; the tool description no longer mandates it.
  Diagnostic timeout repair remains available.
- Timeout wording incorrectly treated the configured guidance as a hard cap.
  The candidate-check and verification references now allow agent overrides.
- Template `once(...)` guards already prevent repeated includes within each
  rendered document. The duplication was principally repeated prose and rules,
  not multiple copies of the same reference section.

The plugin contains both an inference skill and a subagent document. That does
not imply both full documents enter every session: the evaluation launches the
skill and disables subagent spawning. No subagent-based token saving is assumed.

Session input includes the accumulated conversation and tool responses on later
turns, including cache reads. Long generated specs, diagnostics, and repair
loops can therefore outweigh the initial prompt difference. Compare turn count,
tool-output size, and cache-read tokens before attributing the hybrid input
increase to static guidance alone.

The correctness wording is the tool's required contract, not evidence that all
implementation bugs are resolved. A separate mutation check exposed an existing
[state-label verification bug](state-label-assumptions-investigation.md); its
experimental fix is not applied or validated.

## Reproduction

Render each hybrid with `move-flow plugin DIR --inference-tactic ARM
--evaluation-mode --flow-source-commit COMMIT`, both normally and with
`--no-wp-simplification`. Render the baseline using its `cont/` tree through
`--content-dir`, then run `wc -w -c` on `skills/move-inf/SKILL.md`.
The flag is also recorded in `move-flow-manifest.json`.
