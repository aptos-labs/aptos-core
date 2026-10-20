# Proof strategy: predictable automation, authored proofs

P1 (abstract maps) and P2 (writable goals) are implemented. P3 (cheap shared
preparation) and P4 (map proof costs) remain open; track progress in
[`roadmap.md`](roadmap.md).

## Thesis

Automatic verification should decide easy obligations cheaply and report hard
ones for authored proofs. A person or an AI agent writes those proofs in the
companion file beside the Move source; Lean checks the resulting proof term.
The goal is predictable verification cost and readable mathematical arguments.
An external SMT solver, with or without proof reconstruction, is outside this
design.

The map use cases illustrate the distinction: after writing a value at a
present key, the key sequence and map size are unchanged, and the loop invariant
applies to earlier positions. Direct map laws state that argument. Repeated
normalization and general arithmetic or congruence search can cost much more
than checking it. [`perf-notes.md`](perf-notes.md) records the measured
comparisons and rejected alternatives.

## Consequences

1. **Automation must be cheap and predictable.** Measure work on obligations
   it decides and the cost of reporting failure. A decider should reject
   inapplicable goals promptly. Do not enlarge budgets to chase hard leaves.
2. **Authored proofs start directly.** `verify f by …` owns the target's whole
   goal; it does not run speculative automatic solving first. Its script may
   explicitly request cheap deciders or `verex_solve` where useful.
3. **Goals must be writable.** Use source-named parameters and hypotheses,
   generated specification-law names, and typed `SpecMap K V` laws. Keep
   runtime encoding transport at the shared model boundary. A preparation
   tactic should expose the mathematics the proof needs without translating
   unrelated facts.
4. **Shared work precedes splitting where possible.** Prepare a symbolic path's
   common context once and let its logical children inherit the facts. Preserve
   clause names, binder scopes, local instances and proof obligations.
5. **Performance claims compare the same work.** A module's total includes its
   automation, authored proofs and preparation. Keep supported proof outcomes
   separate from unsupported-feature diagnostics, and distinguish focused
   interpreted replays from complete native measurements.

## Milestones

- **P1. Abstract maps.** Stages A0–A3 of
  [`intrinsic-maps.md`](intrinsic-maps.md) give every intrinsic map owner an
  abstract verification value. Runtime storage and its agreement remain
  explicit; verification does not require unfolding a concrete tree layout.
- **P2. Writable goals.** `verex_prepare` names parameters, old values, callee
  results and hypotheses from their source declarations. Loop and callee clauses
  keep their provenance for `verex_on` routing. Map values can be named with
  `verex_name_maps` or lifted selectively with `verex_lift_maps only [...]`.
  Generated laws use the module's specification names, and shared typed laws
  supply the mathematical arguments. The failure message prints a companion
  proof template with one alternative per undecided clause; the default
  alternative uses the cheap `verex_decide` tactic. See
  [`source-verification.md`](source-verification.md), "Proofs", and
  [`proof-file-editing.md`](proof-file-editing.md).
- **P3. Cheap automation and shared preparation.** One **20M raw-heartbeat** cap
  covers a complete automatic leaf attempt, including call observations,
  deciders, preparation and fallback. The overall target budget also bounds
  obligation generation and the complete proof. Authored proofs bypass the
  automatic leaf cap unless they explicitly call `verex_solve`. There is no
  separate full-fallback cap. `verex.leafHeartbeats` and the per-function
  `pragma leaf_heartbeats = N` express the leaf cap in thousands of heartbeats.

  Authored proofs can use `verex_obligations preparing` with an indented tactic
  block before a completed path's proof clauses split. Logical children inherit
  its prepared context and clause names survive. `verex_group_contexts` can
  also share preparation after clause-specific routing where contexts agree up
  to local names. Neither tactic adds speculative search to authored proofs.

  Open: the automatic leaf chain should fail fast where no decider applies and
  avoid repeating common preparation. [`shared-obligations.md`](shared-obligations.md)
  plans measurement, bounded automatic preparation, delayed splitting, and
  formula/proof or theory-state sharing where the measurements justify it.
  Preserve the existing budgets and automatic cost gates.
- **P4. Map use cases at acceptable total cost.** All 38 supported BigOrderedMap
  targets and all 29 OrderedMap targets have verified native measurements,
  including authored proofs. The reference totals after shared preparation are
  **3.582G** and **1.430G** heartbeats, respectively. The requested tenfold
  BigOrderedMap reduction from approximately 19–20G remains open: the goal is
  **below roughly 1.9G** for all supported targets, including authored proofs.
  Keep the five unsupported BigOrderedMap diagnostics separate from this goal.

## Measurement and validation

Use focused targets and `scripts/smoke.sh` between edit batches. At a stable
implementation checkpoint, run the complete map measurements, benchmark and
full validation gates from [`roadmap.md`](roadmap.md). Record test-run results
in PRs or artifacts rather than appending session status here.

Do not sum nested `verex.solverDebug` stage lines: they overlap even when printed
at the same indentation. Use the solver's summary and independently measured
native target totals. Lowering a cap only affects attempts that reach it; it
cannot by itself reduce the cost of a direct authored proof.
