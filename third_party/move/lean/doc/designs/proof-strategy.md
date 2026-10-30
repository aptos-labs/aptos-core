# Proof strategy: predictable automation, authored proofs

Open work: [`roadmap.md`](../plan/roadmap.md), section 1.

## Thesis

Automatic verification decides easy obligations cheaply and reports hard
ones for authored proofs. A person or an AI agent writes those proofs in the
companion file beside the Move source; Lean checks the resulting proof term.
The goal is predictable verification cost and readable mathematical
arguments. An external SMT solver, with or without proof reconstruction, is
outside this design.

The map use cases illustrate the distinction: after writing a value at a
present key, the key sequence and map size are unchanged, and the loop
invariant applies to earlier positions. Direct map laws state that argument;
repeated normalization and general arithmetic or congruence search cost much
more than checking it ([`performance.md`](performance.md)).

## Consequences

1. **Automation is cheap and predictable.** Measure work on obligations it
   decides and the cost of reporting failure. A decider rejects inapplicable
   goals promptly. Budgets are not enlarged to chase hard leaves.
2. **Authored proofs start directly.** `verify f by …` owns the target's
   whole goal; it does not run speculative automatic solving first. Its
   script may request cheap deciders or `verex_solve` where useful.
3. **Goals are writable.** Source-named parameters and hypotheses, generated
   specification-law names, and typed `SpecMap K V` laws. A preparation
   tactic exposes the mathematics the proof needs without translating
   unrelated facts.
4. **Program joins are preserved and work is shared before splitting.** A
   shared suffix is a parameterized continuation over its joined entry
   state; its obligations are generated once, incoming states retained as
   guarded alternatives, the common context prepared before its logical
   clauses split, clause names and binder scopes preserved.
5. **Performance claims compare the same work.** A module's total includes
   its automation, authored proofs and preparation; supported proof outcomes
   stay separate from unsupported-feature diagnostics, and focused
   interpreted replays from complete native measurements.

## Mechanisms

- **Abstract maps** ([`intrinsic-maps.md`](intrinsic-maps.md)): every
  intrinsic map owner is an abstract verification value; runtime storage and
  its agreement stay explicit.
- **Writable goals.** `verex_prepare` names parameters, old values, callee
  results and hypotheses from their source declarations; loop and callee
  clauses keep their provenance for `verex_on` routing; maps are named with
  `verex_name_maps`; generated laws use the module's specification names.
  The failure message prints a companion proof template with one alternative
  per undecided clause ([`source-verification.md`](source-verification.md),
  "Proofs").
- **One leaf cap.** One cap (`verex.leafHeartbeats`, default 20M raw
  heartbeats; per function `pragma leaf_heartbeats = N`) covers a complete
  automatic leaf attempt: call observations, deciders, preparation and
  fallback. The target budget bounds obligation generation and the complete
  proof. Authored proofs bypass the leaf cap unless they call `verex_solve`.
- **Shared preparation in authored proofs.** `verex_obligations preparing`
  prepares a completed path's context once before its clauses split;
  `verex_group_contexts` shares preparation after clause-specific routing
  where contexts agree up to local names; `verex_proof preparing` is the
  structured interface, routing source-named alternatives to ordinary Lean
  arguments; `verex_map_simp` evaluates closed map observations directly.
  None adds speculative search.
- **Join scheduler.** The opt-in `verex.sharedContinuations` shares
  kernel-checked continuation summaries at joins, with entry decomposition,
  selected cases, certificate bounds and map-position facts shared among
  clause groups within a bounded preparation slice per owner.

## Measurement

Focused targets and `scripts/smoke.sh` between edit batches; the full gates
at checkpoints. Do not sum nested `verex.solverDebug` stage lines; use the
solver's summary and native target totals. Lowering a cap only affects
attempts that reach it. Methods and levers: [`performance.md`](performance.md).
