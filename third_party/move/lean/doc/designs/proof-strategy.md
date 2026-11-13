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
invariant applies to earlier positions. Direct map lemmas state that argument;
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
   specification-lemma names, and typed `SpecMap K V` lemmas. A preparation
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
  `verex_name_maps`; the built-in map lemmas are named from the module's
  specification functions.
  The failure message prints a companion proof template with one alternative
  per undecided clause ([`source-verification.md`](source-verification.md),
  "Proofs").
- **One leaf cap.** A leaf's allowance (`verex.leafHeartbeats`, default 20M
  raw heartbeats; per function `pragma leaf_heartbeats = N`) bounds each
  attempt, the first and each case of its prepared split, excluding the
  attempt's preparation, which the target budget bounds. The target budget
  bounds obligation generation and the complete proof. A leaf that exhausts
  an allowance is logged as `exhausted`, one whose deciders all reject it as
  `undecided`. `on _ { }` gets the `decide` closer on the block's normal form;
  an obligation no `on` names gets the leaf's bounded deciders, no residual
  search, under the target budget. Other authored steps bypass the leaf cap
  unless they call `verex_solve`.
- **Facts a decider reads.** `grind` receives the hypotheses connected to the
  goal: from the goal's free variables, the facts sharing a variable with a
  reached one, to a fixpoint (`connectedFacts`, the closure the normalization's
  discharger uses). What is lost is only a contradiction among facts
  disconnected from the goal. A goal already `False` states no variable and
  keeps the whole context. A quantified hypothesis is a rewrite rule only if
  its pattern fixes its data binders and is no bare binder
  (`rewritesByPattern`): `∀ a b, h a = h b → a = b` would match every term of
  the binders' type and leave `b` to its premise's discharge. It stays a fact
  for the deciders that instantiate it.
- **Bounds follow the obligation.** Preparation states the range of every
  certified local in context and the bounds of the operation sites of the
  goal, of the path's decisions (branch conditions, arm tests, the failed
  checks and abort conditions the walker marks) and of the facts connected to
  those through shared arithmetic atoms (`obligationTerms`, omega's notion: a
  read, a projection chain, an uninterpreted application is one atom; `+`, `-`,
  literal multiples and coercions are not). A fact that is neither seeds
  nothing; a decider that brings one in, an instance or an implication's
  conclusion, states its bounds then. They are stated after the stored reads
  have settled into the normal form, over normal terms, and normalized once.
  With no atom in the goal and the decisions, the whole context is read.
- **Shared preparation in authored proofs.** `verex_obligations preparing`
  prepares a completed path's context once before its clauses split;
  `verex_group_contexts` shares preparation after clause-specific routing
  where contexts agree up to local names; `verex_proof preparing` is the
  structured interface, routing source-named alternatives to ordinary Lean
  arguments; `verex_map_simp` evaluates closed map observations directly.
  The proof language groups an `on` by the prepared path prefix, retaining
  the longest common context (including shared late premises) and keeping
  differing binders inside their clauses. Nested `on` blocks run a shared
  prefix before introducing clause binders; child premises enter the same
  normal form before shared facts are refreshed. Structured child steps
  reuse the prepared context when their additions are canonical scalar
  binders and inequalities; native values, equalities and conditional path
  facts retain context preparation. Raw Lean steps also retain it. Empty
  defaults on prepared paths use bounded leaf deciders directly.
  None adds speculative search.
- **Join scheduler.** `verex.sharedContinuations` (on by default) shares
  kernel-checked continuation summaries at joins, with entry decomposition,
  selected cases, certificate bounds and map-position facts shared among
  clause groups, debited to the leaf allowance of every clause they serve.

## Measurement

Focused targets and `scripts/smoke.sh` between edit batches; the full gates
at checkpoints. Do not sum nested `verex.solverDebug` stage lines; use the
solver's summary and native target totals. Lowering a cap only affects
attempts that reach it. Methods and levers: [`performance.md`](performance.md).
