# Performance

How verification cost is measured, the mechanisms that bound it, what was
learned, the open levers with their measurements, and the levers measured and
rejected. How the mechanisms came about is in
[`transitions.md`](../plan/transitions.md), "Verification cost and callee
summaries".
Status is in [`roadmap.md`](../plan/roadmap.md); the current route is
[`native-verification.md`](native-verification.md). Run logs and dated
measurements go to PRs and benchmark artifacts, not here. Heartbeats are
native unless stated; the reference measurements are the map modules on
`wrwg/lean6` in October 2026 (BigOrderedMap 38 targets, OrderedMap 29).

## How to measure

Heartbeats miss the largest cost: the kernel checks without heartbeats,
and a kernel stall shows as time inside whichever `simp` first waits for a
certificate. Use all of these.

1. **Exactness and wall time per fixture.** Run every Check file from
   `verex-e2e-tests` at the driver caps
   (`-DmaxHeartbeats=180000 -Dweak.verex.verifyHeartbeats=180000`), four
   at once, and diff each output against its `.exp`; any difference is a
   regression, even for a change meant only to be faster.
2. **Solver heartbeats by stage.** `-Dweak.verex.solverDebug=true` prints
   per target `solver: <N>k … bind …; exit …; leaf …` and, per leaf, each
   step with its heartbeats (`cheap: 95k, failed: …`; the pipeline stages
   `pipe-simp`, `pipe-omega`, `pipe-saturate`, `pipe-bv`, `pipe-grind`, …;
   the cheap decider's `c-assumption`, `c-ground`, `c-omega`, …). Sum by
   stage across a run and compare two runs stage by stage. The printed
   goals cost heartbeats themselves; `-Dverex.solverProfile=true` prints
   the stages and per-step run counts without goals, plus the
   continuation summary (folds, arrivals, revisits; dynamic instances,
   not CFG sites) and, under `verex.sharedContinuations`, the `joins:`,
   `automatic owners:` and `join pruning:` counters. Nested stage lines
   are inclusive: never add them to the enclosing stage.
3. **Time by category.** `-Dprofiler=true -Dprofiler.threshold=100000`:
   `type checking` is the kernel, `interpretation` the solver and
   elaboration, `simp` is simp. `-Dtrace.profiler=true` adds
   per-declaration `[Kernel]` times; a certificate the first verify waits
   for shows as the `addDecl` of an auxiliary such as `_simp_2`.
4. **Kernel unfolds of one certificate.** In a copy of a fixture, after
   `import VerexE2ETests.CheckSupport`, define `#kdiag` and apply it to the
   certificate (for example `….borrow_first_after_break.compiled_eq`):

   ```lean
   open Lean Elab Command in
   elab "#kdiag " id:ident : command => liftTermElabM do
     let some info := (← getEnv).find? id.getId | throwError "no {id}"
     let .thmInfo t := info | throwError "not a theorem"
     let tv : TheoremVal := { t with name := id.getId ++ `recheck }
     let kenv := ((← getEnv).toKernelEnv).enableDiag true
     let t0 ← IO.monoMsNow
     match Lean.Kernel.Environment.addDeclCore kenv 0 (.thmDecl tv) none with
     | .ok kenv' =>
       let counts := kenv'.getDiagnostics.unfoldCounter.toList.toArray.qsort
         (fun a b => a.2 > b.2)
       logInfo m!"{id.getId}: {(← IO.monoMsNow) - t0}ms, top {(counts.extract 0 40).toList}"
     | .error _ => logInfo m!"kernel error"
   ```

   Many `List.get`, `List.rec` or `List.length._f` unfolds mean array
   indexing inside a kernel-evaluated computation (below).
5. **One target on its own.** `scripts/isolate-target.py <file> <module>
   <target> -o <out> [--option name=value]` keeps one target verified
   (every other specification block gets `pragma verify = false`) with the
   options set after the import, so a module extract runs in seconds. Run it
   with `verex-move elaborate <out>`: the verifier is native there, and its
   heartbeats compare with the driver's, where `lake env lean` interprets it.
   The profile of a target stopped by its budget reports what it spent up to
   there; `step statement` is what it spent before its first tactic.
6. **The benchmark problems.** `scripts/verex-bench.py run [--only …]`
   measures the standard problems natively by phase and prints each
   problem's change against the previous local run
   ([`benchmark.md`](benchmark.md)).
7. **A package sample.** Compare per target on one binary, back-to-back:
   heartbeats are deterministic, wall time is not. For callee summaries
   and joins the reference sample is four framework modules:

   ```bash
   VEREX_OPTIONS=verex.solverProfile=true python3 scripts/verify-modules.py \
     <workspace>/aptos-move/framework/aptos-framework \
     --modules aptos_framework::primary_fungible_store,aptos_framework::fungible_asset,aptos_framework::coin,aptos_framework::multisig_account \
     --heartbeats 300 --jobs 8 --timeout 1200 --no-build \
     --logs <run>/logs --json <run>/raw.json --namespaces <run>/ns
   ```

   Status and heartbeats per target are in `raw.json`;
   `<run>/logs/*.stages` has `summary derived` and `summary refused` with
   their costs; `<run>/logs/*.out` has `summary applied <callee> <k>k`,
   `summary skipped <callee>: <reason>` and `stage <name> ×<n> <k>k`.
   Attribution by inlined body needs temporary instrumentation; it is not
   in the tree.

Timing comparisons need the same `verex.jobs`, the same machine load, and
nothing else running; never rebuild `verex-ir` while a suite reads its
`.olean` files. Heartbeats per second vary with load and allocation (four
runs of `upsert` at 170–181M took 6.4–9 s), so report heartbeats, never a
single wall run. Before committing, run the gates `CLAUDE.md` lists; the
`DenotePerformance` and `CompositionPerformance` baselines are re-recorded
only for a deliberate cost change, with the diff reviewed. After a `UB=1`
run, `scripts/exp-error-delta.sh` lists the baselines whose error count
changed.

## Build cost

Code generation of the verifier's metaprograms dominates a rebuild.
Splitting `solveGoals` and `verifyMember` into step functions cut the
downstream rebuild from 4m24s to 1m44s (`Solve.lean` 104 s → 28 s,
`Verify.lean` 72 s → 14 s). Profile the compiler stages before touching
proofs for build latency.

### Constructor equations of the denotation

`vir_denote_constructor_equations` specializes each constructor before
proving an evaluator equation. Lean's usual structural equation generator
first proves one symbolic `eq_def`; after adding Slice operations to the
mutual `Term.denote`/`Args.denote`/`Arms.denote` evaluator, expanding its
`below` motive exhausted the existing 800,000-heartbeat command limit.
Factoring operation bodies into helper definitions did not remove that
symbolic expansion.

With Lean 4.32.2, the comparison during the Slice integration was:

| Registration | Result at the unchanged limit | Term module wall time |
| --- | --- | --- |
| Standard structural `eq_def`, then equations | 800,000-heartbeat timeout | 53 s, failed |
| Constructor-specialized equations | Successful kernel checking | 18 s |

These wall times describe the observed build, not a performance threshold;
load and downstream compilation affect them. The command does not reset or
raise the heartbeat budget. It uses definitionally equal constructor cases
and kernel-checked theorem declarations, with explicit equality evidence so
the simplifier retains equation applications instead of implicitly reducing
the full evaluator again.

The small adapter in
[`ConstructorEquations.lean`](../../verex-ir/VerexIR/Proofs/Denote/ConstructorEquations.lean)
uses `import all Lean.Elab.PreDefinition.Eqns` for Lean's private
`mkEqnTypes` and `removeUnusedEqnHypotheses`. This is a toolchain upgrade
hazard, not a new logical assumption. An upgrade must check the adapter,
standard `eq_1` naming/discovery, and inverse equation registration, and run
both constructor-equation regression modules. They check same-module and
imported reuse, absence of a generated `eq_def`, retained rewrite evidence,
and absence of axioms in the generated equations. The ordinary command
remains available for definitions that do not need specialization.

The unchanged `DenotePerformance` gate also checks downstream effects.
Constructor-specific primitive literal/result normalization brought the
classifier from an initial 7,480,680 heartbeats to 6,625,381 against its
6,601,077 baseline (+0.37%); its proof objects fell from 35,669 to 15,576.
No existing baseline or tolerance was increased.

### Literal UTF-8 reduction

The literal validator is bounded to 64 bytes and leaves symbolic inputs
opaque. [`Utf8Performance.lean`](../../verex-ir/VerexSL/Tests/Utf8Performance.lean)
measures the full-boundary ASCII case, sixteen four-byte scalars, and an
invalid final byte after 63 valid ASCII bytes. Each proof goes through
`verex_normalize` and the kernel-checked reduction certificate. Its separate
[`Utf8Performance.exp`](../../verex-ir/VerexSL/Tests/Utf8Performance.exp)
uses the existing heartbeat and proof-object gate; it does not alter the
older denotation baselines. Kernel time remains a separate observation,
as described above, so the byte bound must be remeasured before increasing it.

### Rust differential evaluator

The generic-closure branch of `VerexRust/Tests/DenoteEval.lean` carries dependent
frame and capture certificates through the mutually recursive evaluator. Its
scoped elaboration allowance is deliberate; it does not raise source-verification
budgets or change how many inputs the differential suite executes. The before/after
module compilation measurements, heartbeat counts, source revisions and command
are recorded in [the build-cost artifact](../status/rust-denote-eval-build.json).
The measurement includes Lean's C generation, but excludes C compilation and
linking; it is not a differential-suite execution timing.

### Modules split for edits

Lake rebuilds every importer of a changed module and compiles the C of the
changed module only. The solver, the contract generator and `verify` are
split into parts so that an edit recompiles one part's C: `Solve.Decide` and
`Solve.Normal` do not import each other, and `Verify.Contracts` does not import
the solver. The last part of each keeps the original module name, so importers
are unchanged:

| Module | Parts, in import order |
|---|---|
| `VerexIR.Proofs.Denote.Solve` | `Solve.Base` (options, accounting, leaf facts, caches); `Solve.Decide` (leaf deciders) and `Solve.Normal` (the normal form's simprocs and lemmas), which do not import each other; `Solve.Invoke` (splits, behaviors, invocations); `Solve` (walker, authored tactics) |
| `VerexSL.Contract` | `Contract.Values` (native types, values, maps, owners); `Contract.Clauses` (the clause translator); `Contract` (invariants, contract assembly) |
| `VerexSL.Verify` | `Verify.Contracts` (compiled functions, contracts, loop invariants; no solver import); `Verify.Members` (targets, cycles, lemmas); `Verify` (commands) |

The split keeps every simp
lemma, macro and simproc in import order and every proof object identical;
typed heartbeats fell 4–6% from the former private names. Edit-to-executable
on an idle machine (libraries + executable):

| Edit in | Before | After |
|---|---:|---:|
| `Solve` (walker) | 164 s | 46 s |
| `Solve.Decide` / `Solve.Base` | 164 s | 88 s / 96 s |
| `Contract` (assembly) / `Contract.Clauses` | 80 s | 51 s / 50 s |

The smoke suite went from 304 s to 66 s warm: fixtures as parallel
processes, the executable built beside them, and the Move Prover tests'
verifier snapshot (1 GB digested twice) from 79 s to 12 s with `sha2`
compiled optimized in the dev profile.

## Parallel targets

Within a module, proofs elaborate as tasks, `verex.jobs` at a time; a
target waits for its specified callees, so a module's wall time is bounded
by its longest caller chain. Drivers running several processes give each
`verex.jobs` = half the hardware threads divided by the processes.
Heartbeats are counted in the proof's task and charged to the target from
there. `big_ordered_map` (38 authored targets, 7 cores): 99 s wall at one
job, 46 s at four (111 s and 129 s CPU). Every kernel check the command
requests itself waits for the kernel chain holding the forked proofs, so
the command prepares all targets first and elaborates everything else with
`Elab.async`; the audit walks dependencies through `Environment.find?`,
never the checked environment.

## Compiled namespaces

`scripts/verify-modules.py` compiles a package's namespaces to `.olean` files
first ([`relocatable-namespaces.md`](relocatable-namespaces.md)), so a module's
process elaborates only its own namespace and imports the rest. `aptos-framework`
at `--heartbeats 300`, 8 processes, one run of each: 10,235 s summed over 78 module
processes and 1,289 s wall when each elaborates what it uses, 6,476 s and 869 s
compiled, the pass being 172 s summed and 60 s wall over 155 namespaces (42 MB of
`.olean` files, the largest 3.3 MB). Rendering and lowering fall from 1,873 s to
71 s; what stays is a module's own unit, certificates and proofs, so `stake` gains
7% and a module without targets 80%. Heartbeats do not change. The `.olean` files
hold relocatable namespaces, not linked units, so their size is linear in the
package.

## Fixed points across normalizations

The walk's normalization (`normalizeAround`, inline and the release of held
continuations) records, per target, the terms it left unchanged without
reading the local context, and a later traversal returns them at once
(`FixedPoints` in `Solve.Base`). A traversal is tainted by a discharger call, a subterm taken as
normal without traversal (the `normal` arguments), and a head at which a
simproc reads the hypotheses (`contextDependent`'s shapes, `existsNamedValue`,
`givenClosureFrame`, `invokeGenericClosure`); the taint is tracked by a frame
per term at simp's recursion depth, and the terms simp's own cache returns
without a traversal are checked against the tainted ones of the call. Terms
with metavariables and below depth 3 are not recorded. Leaf preparation was
measured with its own stamp and gained nothing.

On the isolated `primary_fungible_store::deposit` (budget-stopped, so per
run): bind −29%, split −12%, inline −19%, exit −34%, release −11%, 31% of
lookups hit. `multisig_account::can_execute`: bind −51%, exit −52%, inline
−36%, release −37%. Inside `normalizeAround`, `instantiateMVars` and
`applySimpResultToTarget` cost 0.05% of `Simp.main`, and only 1–2% of the
misses are terms seen before and tainted: what is left is the traversal of
terms the steps create. `scripts/verify-modules.py` at `--heartbeats 300`,
targets with the same outcome that do not reach the budget: `big_ordered_map`
−2.8% (43 targets), `multisig_account` −6.0% (19), `primary_fungible_store`
−18.8% (5); `create_primary_store` and `ensure_primary_store_exists` verify
within the budget, two budget-stopped targets of `multisig_account` now fail
before it. `DenotePerformance` run natively (`verex-move elaborate`): no
target above +0.1%, `record` −7.8%, `return_in_loop` −6.6%, `at_most` −5.8%.
Interpreted (`lake env lean`), where the wrapper's own code is counted,
`return_in_loop` is +5% and `increment` +2%. Hashing the leaves of the smoke
fixtures, the cache changes no leaf goal.

## Shared continuations

The walk is a DAG: a join's continuation is a closed constant over entry
rows, walked once, and each arrival proves an entry relation. All arrivals
waiting at the same points share; a position where they differ is abstracted
and the relation states their disjunction. A single arrival takes the tree's
route. What the tree walk
assumed, and what a join must therefore preserve:

- **Facts along a path.** Loop hypotheses and dominating facts must survive
  the seal: the fold's loop continuation is a capture, and the locals sharing
  reads are pinned on the walk path, so a branch leaves an equation instead of
  rewriting them. Pins are lifted at leaves and completed paths, which are
  never compared with an arrival.
- **Which equations are substituted.** Frame facts the walk keeps on purpose
  stay kept at arrivals; substituting one makes `EnsuresOf` relate one memory
  to itself.
- **Per-path state.** Provenance, clauses, decisions, points and owners are
  recorded per arrival, and a shared suffix owes the union of its arrivals'
  clauses. A failed shared leaf reports per arrival.
- **Goal shape.** The fold names its application with a `let`: simp otherwise
  re-enters the constant at every step (per-step cost about 3×).
- **Order.** A region resumes when no pending goal lies in it, innermost
  first, which keeps the walk depth-first.
- **Profitability.** A suffix that only exits is not folded: a sealed class
  saves only the suffix's rule steps; its leaf work stays per arrival.

A join gives a disjunction of variants. Enum parameters stay whole until
matched. Each arm states one equation per binder, with the facts common to
all arms outside the disjunction; equations are classified by syntax, and an
arm opens in one term.

A hand-written walk over `Expr` must keep a visited set and check heartbeats:
terms are DAGs, and a pure walk neither ends on a heavily shared term nor stops
at the budget. Lean's `kabstract` has the same gap (`kabstractShared`).

## Callee summaries

A contract-free callee is inlined unless a derived summary applies at the
call (`applySummaryAt` in `Solve.lean`).

- **Route lookup.** `routeCall` (`Solve.Normal`) resolves a
  `routeMeaning`/`routeGeneric` chain directly instead of rewriting through
  it.
- **Callee-own meanings.** `calleeFamilyMeanings` (`Verify.Members`): an
  inlined callee's refinement is stated under the callee's own chain and
  ghost tables, so it reads alike in every family target.
- **Derivation.** `deriveModuleSummary` and `deriveCalleeSummary` derive a
  module's summaries serially before the proofs, by `summaryWalk`, as
  `∀ args ensures aborts initial, requires → Ψ → wp (denoteWith M_c compiled
  args) ensures aborts initial`, recorded as a `DerivedSummary`. Aborting
  paths are included. A callee's `requires` is a premise, discharged at the
  call (`takesRequires`).
- **Early refusal.** A walk stops at the first leaf a summary cannot state.
  Its budget is `7 * verex.summaryHeartbeats` (`verex.summaryHeartbeats`
  alone for an equational summary), and `verex.summaryLeaves` bounds the
  paths. A callee whose form excludes a summary (a generic signature, a
  loop, a returned reference) is refused before the walk.
- **Application.** `applySummaryAt` and `applyModelSummary` compare the
  meaning and the continuations at reducible transparency, because a failed
  default unification is expensive; scalars are assigned as projections
  (`assignSummaryArguments`), because default-transparency unification
  delta-unfolds a stuck value such as `reflectedTypeInfo`. A summary is not
  applied inside a shared-fold region (`activeSummary`), under a hand-over,
  an unrolling or a point, or to a callee being summarized.

The trusted side is [`assumptions.md`](../status/assumptions.md), "Model
agreement"; the missing unit model is in
[`ghost-fields.md`](ghost-fields.md).

## What was learned

- **An array read in a kernel-evaluated computation is a list walk**:
  `a[i]?` walks `i` cells, `a.size` the whole list, folds over index ranges
  are quadratic. Certificates the kernel evaluates iterate `toList`, read
  arenas through an `IndexedArena`, and count sizes once; `_eq` lemmas
  connect the indexed forms to the native ones. One checksum over a big
  structure with `Array.foldl` in the kernel ran out of memory at 13.6 GB.
  The whole-unit typing facts read one literal view of the tables
  (`TypingView`), walk functions in lockstep, and prove in a pointer `Eq.refl`
  that every namespace holds the unit's tables: on a unit of 70 namespaces,
  under a minute where the array-reading resource loop over one table did not
  end in 840 s. `#kdiag` on the array-reading check showed three-quarters of its
  unfolds in list walks, and a cost nobody had listed: comparing two tables'
  names decides `String` equality, about 2 s per namespace there.
- **Unfolding a big recursive definition makes the kernel redo it.**
  `simp` unfolding `Term.denote` records no steps, so the kernel re-reduces
  its mutual constructor matcher. The equation lemmas (registered by
  `vir_denote_equations` or `vir_denote_constructor_equations`) make
  each step an instance; the `call` equation's index does not match at
  reducible transparency, so plain unfolding stays as the fallback.
- **In `simp` the cost is per node, not per rule.** Every lookup reduces
  the reducible carriers in implicit arguments (`NTy.carrier`, `HEnv`),
  about half of every normalization; the solver normalizes only what a
  step creates (`normalizeAround`).
- **A value read twice doubles the goal.** A computed value substituted
  into the continuation is copied at every read (`math128::sqrt`'s Newton
  steps: 1.6M, 6.7M, 32M for one, two, three). Checked arithmetic and
  shifts name their result, as the Prover's temporaries; casts stay
  unnamed; positions reading a name are unnamed after normalization
  (`verex_unname_positions`, `verex_unname`). `sqrt` from over 1.5G to 5 s
  at +1.7% on the benchmark.
- **`isDefEq` does not see that a term is shared.** Where two terms differ in
  an argument that converts only under its application, it fails on the
  argument at every enclosing application and unfolds instead. A respelling of
  families is compared by congruence, `isDefEq` only on the smallest
  application that converts, each pair once per target (`convertsAtSites`):
  on `coin::name` one hypothesis of 1.76M tree nodes, 867 distinct, differing
  at one site, from 14.7M to 1.0M; `version::on_new_epoch` from 50.4M to 22.9M.
- **Rewriting with hypotheses must terminate.** `verex_simp_all` admits an
  equation as a rule only if its right side reaches no admitted left side
  leading back to it.
- **Flow binds associate.** `wp_flowBind_flowBind` moves the inner action
  to the head instead of copying the continuation into the arms
  (`string_utils::format4` from over 170 s to 10 s).
- **Saturation records asserted bounds by goal content**, not per
  hypothesis order, so an ancestor's bounds are not reasserted.
- **Concrete modular arithmetic**: `verex_ground_arithmetic` rewrites to
  literals with Int/Nat equalities before the closed-goal decision, since
  sending remainder chains to omega adds quotient constraints.

## Open levers

Every lever was measured on encoded leaves and is to be measured again on
native ones; most act on the normalization of encoded terms.

### Remaining on shared mode and budget stops

In the order of expected gain:

1. **Extra leaves after a single arrival** (coin `get_paired_*_copy_ref`): the
   tree closes these continuations at exit, here they reach the leaf pipeline.
   Substitute released pin equations before `normalizeAround`, and retry
   `verex_assumption` after substitution. Work in progress is untested.
2. **Targets the walk exhausts with no leaf decided**: about 600 steps at
   0.2–2M with no blow-up. Release a call's continuation once per call, not per
   subgoal; cache a compiled callee's unfolded body per target.
3. **Loop stage before its context stage** (genesis `create_*_validators`).
4. **Stored invariants** simplified per read, and position instances
   re-derived per leaf (`storage_gas`).
5. **Small two-arrival joins** cost more than exits (`dkg::initialize`).
6. **Leaf fixed points across passes**: they last one pass, since context reads
   are detected by head name. An explicit read counter bumped by every simproc
   that consults the context would allow more.
7. **`move prove --lean` runs modules one after another**, unlike the Prover's
   parallel Boogie processes; `verify-modules.py` runs them in parallel.

### Preparation

1. **Prepare once per path.** Keep a path's context prepared as the walker
   extends it; prepare an older fact again only when a new equation or
   fact changes it. Bound on `upsert`: preparation 126M → 25–30M, the
   script 171M → 75M. Needs a rule set that commutes with substitution
   (operations with lemmas never unfolded); two attempts failed on the
   encoded rule set.
2. **Directed steps instead of `simp_all` fixpoints** in authored map
   preparation: definitions substituted once in order, one pass of the
   lemmas over the changed facts. One pass costs 8–29% of the fixpoint
   (`upsert` 4.5M of 55.4M). Not measured as a replacement; the scripts
   depend on `simp_all`'s results.
3. **Map-position facts on demand**, once per path, through the existing
   cache: up to 20% of OrderedMap's automatic cost (84M of 426M).
4. **Obligation splits in the leaf** for automatic targets, instead of
   substitution and normalization at every walker split: the split stage
   is 24% of OrderedMap's automatic cost.
5. **Delay logical and variant splitting**: keep a region's clauses
   together through common preparation; unmeasured. The join scheduler
   (`verex.sharedContinuations`, on by default) already shares entry
   decomposition, cases, certificate bounds and position facts.
6. **Fact dependencies of normalized subterms.** Reusing a normal part of
   the goal is sound only where no fact added since its normalization
   reads what it reads: facts known on an arrival path simplify parts of
   a folded continuation that were normal without them (an attempt at
   exit broke a fixture), and the goal before a bind is not
   normal. Record per normalized subterm above a size threshold the
   locals it reads and the facts its rewrites and discharges read (from
   simp's post-step and the context discharger); keep it while no newer
   fact mentions either, the connected-facts key of the hypothesis cache
   applied per subterm, checked through `normalizeAround`'s stop
   predicate with a generation counter. It would unlock exit (about 165M
   on the benchmark), loop steps (about 60M) and reuse at bind. A stop
   that is too early weakens the normal form, so it costs completeness,
   not soundness. Incremental normalization after a split, where the new
   case fact's locals are known, is the special case already in place.
7. **Indexed contexts**: per-pass indexes, simultaneous substitution,
   per-pass discharger caches, a memo of canonical hypotheses; 1–8% of
   preparation today, linear as contexts grow.
8. **Formula interning and theory-state reuse** only if a measurement
   shows the need after the above.

### Calls

- **Specialized contracts** as declarations per callee and instantiation
  (`native-verification.md`): the call stage is 5–12% of automatic and
  20–28% of authored targets; a declaration costs 0.3–3M. Open: declared
  once per module, or cached proofs as the contract normal forms
  (`verex.contractLemmas`, on by default: typed −3.3%, authored +0.3%) are.
- **The call's subgoals carry the continuations** (190k–480k nodes against
  2k–25k of contract): the call rules apply them to the callee's fresh
  result, error and final state, so a simp over a subgoal walks the whole
  continuation. The solver holds each continuation abstracted over its reads
  of its parameters on both call paths (`wp_call_normal` is the default), so
  the contract's normalization walks the contract only, and names the call's
  premises after the callee (`<callee>_ensures`, `_frame`, `_returns`). The
  release introduces the goal's leading binders, puts the held values back,
  normalizes along the paths to non-variable arguments, the body's structure
  outside its lambdas and the premises a rule put over the body, then
  reverts the binders and normalizes only the logical connectives. Everything
  else is taken as normal: a continuation whose nested lambdas the walk built
  and never normalized would pass unnormalized; none measured does. On
  `multisig_account::can_execute`: call stage 384M → 186M (12 calls),
  `call-contract` 353M → 20M, release 135M, 27 leaves instead of 17.
  The release is now the largest call step.
- **Callee summaries** (above): the open items, in order, are in
  [`roadmap.md`](../plan/roadmap.md), "Cost".

### Splits

- **A conditional under `wp` split by `split`**: on `stake::leave_validator_set`
  one `wp` of `ite` costs 420M of the 590M split stage: `verex_split` fails
  after 197M (97M of it the context normalization before the `split`
  tactic, which abstracts the conditional over the whole goal and fails
  here), and `verex_prepare_leaf` then prepares the goal as a leaf for
  222M. The other 150 splits cost 0.6M together plus 84M of normalization
  after them. The dependent conditional already splits by its own rule
  (`wp_dite_rule`); splitting the plain one by `wp_ite` the same way, at the
  goal's own instance, takes the split stage from 590M to 277M, with 14
  leaves reached instead of 3, and the target still exceeds 1G. Measured on
  the isolated target with `verex.solverProfile`; the branch hypothesis is
  then named `h`, as
  after a dependent split, where `split` named it `h✝`.

### Leaf engine

- **One `grind` per native leaf**, seeded with the map, vector and
  arithmetic lemmas, the cascade as fallback. On encoded leaves `grind`
  closed 21 of 23 prepared leaves of `iter_walk_mut` at 10–130M, the cost
  being internalization (17M on a 100k-node leaf); a 7k-node native leaf
  should internalize in about 1M. No clause learning: a cap bounds cost,
  not outcome.
- **Fail fast where no decider applies**: each saturation round over a
  large context costs 8–18M
  ([`proof-strategy.md`](proof-strategy.md), "One leaf cap"); up to 280M on the benchmark.
- **Ground evaluation of leaves whose inputs are literals**, instead of
  the decider cascade: about 181M on the benchmark.
- **Carrier unfolding in `simp`**: instances stated at the row type would
  avoid the smart unfolding of `NTy.carrier`; a deeper change to
  `Types.lean`.

For an existential containing an array lookup, witness search first tries
positions whose elements a hypothesis states (`statedReads`, read by
`verex_lookup_read`). This reaches the available read witness before unrelated
integers and bounds can exhaust the search budget. Other existential shapes
retain the late read-position fallback. The reads are computed once per
context: a search asks for them per premise, and the scan walks every
hypothesis. `LiteralOptionalReads` checks the ordering with a valid read and
many unrelated certified integers.

### Fixed costs

- **`compiled_eq`**, up to 1 s per function: 80k unfolds dominated by
  `List.rec`, remaining array reads in `unit.namespaces[…]?`,
  `calleeNs.functions[…]?`, `targetNs.structs[…]?`, `loanFacts[…]?`,
  `declaration.locals`. Split the cost into evaluation and comparison
  first.
- **Certificates across importers** (R3 of
  [`relocatable-namespaces.md`](relocatable-namespaces.md)): 10–45% of a
  problem's run. Native evaluation of `compileFunction`
  (`Lean.ofReduceBool`) was measured slower than the kernel and is not
  pursued; certificates stay kernel-checked.
- **Per process**, 18–27% of wall time is export, load and certificates
  (2–4 s); bounded by `scripts/verify-modules.py` and `verex.jobs`. Elaborating
  what a module uses is not among them: it is compiled once per package (see
  "Compiled namespaces").
- **A module's own unit**, once per process: quoting it and defining its instance
  on the first target, its unit facts, and the audit after the proofs (for
  `stake`, 14 s, 12 s and 15 s of its 50 s of certification).
- **Wall time beside heartbeats** in the stage logs, so kernel checks show.

### Authored proof interface

The directed part of an authored proof is a few percent of its cost; the
rest produces and normalizes its leaves (preparation repeated at every
obligation of a path, a callee's contract normalized at every call).
Sharing preparation gave −30% automatic and 0% authored. Facts at program
points (`proof { assert …; apply lemma … }`, [`lemmas.md`](lemmas.md))
instead of scripts over presented leaves remove that where one fact
suffices; unmeasured on the corpus, and to be measured before investing in
per-leaf preparation sharing.

## Levers measured and rejected

Each lever below was measured and did not pay; do not retry one without a
reason the verdict does not cover. The measurements are in
[`transitions.md`](../plan/transitions.md), "Levers tried and rejected, with
their measurements".

| Lever | Verdict |
|---|---|
| `dsimp := false` in the normalizer | 8 fixtures stop verifying |
| `index := false` for the normalization simp set | slower |
| Precompiling `VerexIR` (`precompileModules`) | no gain for the command line, whose executable already links compiled metaprograms. Both libraries at once do not build: a precompiled `VerexSL` module's shared object wants `VerexIR`'s initializers and is not linked against them. `VerexIR` alone builds and makes the *language server* load native code, a proof file in 9–12 s against 28–33 s interpreted, for 194 s of added C compilation (`Solve.Base` 46 s, `Solve.Decide` 31 s) on every build; not adopted. Measured in `source-verification.md` ("The editor") |
| Equation lemmas without the unfolding fallback | `break_with_returned_pair` fails (the `call` index) |
| A per-view table of native types in `CompileNamespace` | 1% fewer unfolds, same time |
| Behavior dispatch: sibling cache, deriving before the split, a bare `simp`, skipping derivation for manual proofs, a sub-budget | negligible, slower, times out, breaks higher-order callers, or no gain |
| Batching forall introductions in `Solve.splitOnce` | fewer splits, same cost |
| `StateOf` / `asBool` irreducible; an arithmetic-only discharger; an unconditional prefix formula for `count_even_concrete` | no effect |
| Preparing a call's facts where they are introduced (three variants) | none pays |
| Continuation folding at calls; stops and auxiliary theorems for shared preparation | breaks authored scripts |
| Introduction preparation (A2) | needs a normal form commuting with substitution; the bounded-omega fix kept |
| Pruning dead paths at the split | too few leaves are contradictory or refutable; `contradiction` exceeds the recursion depth |
| Proof stubbing for an express build | a tenth saved; not worth it |
| A lower automatic leaf cap (75M) | affects only attempts that reach it |
| Partitioning arrivals by numerals as well as constructors | value-only differences never share |
| Keying arrival classes on provenance and clauses | classes vanish and cost rises; the union of clauses kept |
| RowFacts: a per-arrival disjunction of constructor facts | unused once arrivals partition by shape |
| Substituting without `cases`, opening entry relations on demand | no gain; the cost is premises reaching leaves, not exposure |
| Cheap closers before exposure | no effect once premises close at the call |
| Pinning the whole fold context | frame equations reach leaves (coin `get_paired_*` fail) |
| Refuting an arrival from new facts only | loses prunes that need old facts |
| Deferring the fold to the second arrival | pins must hold while the branch is walked |
| Replacing the whole-context substitution at a single arrival by pin equations | much more expensive; the substitution does real work |
| A process-wide normal-form map, or waiting on another target's summary | outcomes depend on scheduling; the wait deadlocks |
| Scoping split and substitution to a memory write's new hypotheses | no gain; the cost was a non-terminating injection loop, fixed at its cause |
| `Memory.set` spine as skeleton | no effect on `bump_resource` |
| Contract normal forms declared eagerly in each module's process, for every call reached (`C.callForm_i`) | the declarations cost more than they save, and verified targets fail. Rejects this variant only: on-demand or shared declarations remain open |
| Lazy release of a call's continuation | the work moves to bind and split |
| One model path for ghost and plain modules | not a speed lever |
| One memory cell under two keys (`coin_address<T>()` against `type_of<T>().account_address`) | real but rare |
| An assumption retry after substitution; early pin substitution; literal simplification before the speculative passes | negligible |
| The full cheap decider before the speculative passes | keeps coverage, costs more |

## What cannot change

Every step is a kernel-checked term, so rewriting costs in proportion to
the terms touched; the constant is hidden only by not rewriting (smaller
goals, evaluation, verified checkers run natively at a trust cost). Lean's
tactics learn no clauses, so a VC's disjunctions are enumerated. A
presented leaf is a normalized leaf: authored proofs written against
leaves pay their normalization. At 32M heartbeats per second, one million
heartbeats is 30 ms, in which Z3 decides a whole VC; the floor per target
is fine, the slope with leaf hardness is set by the passes.
