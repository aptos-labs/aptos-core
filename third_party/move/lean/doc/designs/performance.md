# Performance

How verification cost is measured, what was learned, the open levers with
their measurements, the measurement debts, and the experiments rejected.
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
   ([`verification-benchmarks.md`](../status/verification-benchmarks.md)).

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

### Modules split for edits

Lake rebuilds every importer of a changed module and compiles the C of the
changed module only. The solver, the contract generator and `verify` are
split into parts (`CLAUDE.md` lists them) so that an edit recompiles one
part's C: `Solve.Decide` and `Solve.Normal` do not import each other, and
`Verify.Contracts` does not import the solver. The split keeps every simp
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
rows, walked once, and each arrival proves an entry relation. Arrivals share
only where they agree in constructor shape and points; numerals and strings
stay parameters. A single arrival takes the tree's route. What the tree walk
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

A hand-written walk over `Expr` must keep a visited set and check heartbeats:
terms are DAGs, and a pure walk neither ends on a heavily shared term nor stops
at the budget. Lean's `kabstract` has the same gap (`kabstractShared`).

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
  its 61-way matcher. The equation lemmas (`vir_denote_equations`) make
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
   exit failed `EnumResources`), and the goal before a bind is not
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
  large context costs 8–18M (P3 of
  [`proof-strategy.md`](proof-strategy.md)); up to 280M on the benchmark.
- **Ground evaluation of leaves whose inputs are literals**, instead of
  the decider cascade: about 181M on the benchmark.
- **Carrier unfolding in `simp`**: instances stated at the row type would
  avoid the smart unfolding of `NTy.carrier`; a deeper change to
  `Types.lean`.

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

## Tried and rejected

| Experiment | Result |
|---|---|
| `dsimp := false` in the normalizer | 8 fixtures stopped verifying |
| `index := false` for the normalization simp set | slower |
| Precompiling `VerexIR` | no gain; cross-library dynlib load error |
| Equation lemmas without the unfolding fallback | `break_with_returned_pair` fails (the `call` index) |
| A per-view table of native types in `CompileNamespace` | 1% fewer unfolds, same time |
| Behavior dispatch: sibling cache, deriving before the split, a 500k bare `simp`, skipping derivation for manual proofs, a 1.5M sub-budget | 0.18M saved; slower; times out; breaks higher-order callers; 2M kept |
| Batching forall introductions in `Solve.splitOnce` | 79 → 12 splits, 28.44M → 28.38M |
| `StateOf` / `asBool` irreducible; an arithmetic-only discharger; an unconditional prefix formula for `count_even_concrete` | no effect |
| Preparing a call's facts where they are introduced (three variants) | none paid on 38 targets |
| Continuation folding at calls; stops and auxiliary theorems for shared preparation | broke authored scripts; rejected |
| Introduction preparation (A2) | needs a normal form commuting with substitution; the bounded-omega fix kept |
| Pruning dead paths at the split | contradictory leaves 3% (BigOrderedMap 21/635, OrderedMap 11/438); 0 of 4 and 2 of 99 branches refutable; `contradiction` exceeds the recursion depth |
| Proof stubbing for an express build | a tenth saved; undone |
| A lower automatic leaf cap (75M) | affects only attempts that reach it |
| Partitioning arrivals by numerals as well as constructors | value-only differences never share |
| Keying arrival classes on provenance and clauses | classes vanish (`set_frozen_flag` 234M → 358M); the union of clauses kept |
| RowFacts: a per-arrival disjunction of constructor facts | no target used it once arrivals partition by shape |
| Substituting without `cases`, opening entry relations on demand | no gain; the cost was premises reaching leaves, not exposure |
| Cheap closers before exposure | no effect once premises close at the call |
| Pinning the whole fold context | frame equations reach leaves (coin `get_paired_*` fail) |
| Refuting an arrival from new facts only | prunes that need old facts are lost |
| Deferring the fold to the second arrival | pins must hold while the branch is walked |
| Replacing the whole-context substitution at a single arrival by pin equations | coin +480M; it does real work |
| A process-wide normal-form map, or waiting on another target's summary | outcomes depend on scheduling; the wait deadlocks |
| Scoping split and substitution to a memory write's new hypotheses | the cost was a non-terminating injection loop, fixed at its cause |
| `Memory.set` spine as skeleton | no effect on `bump_resource` |

### Follow-up measurement after the compiler restack

On the whole `coin`, `primary_fungible_store`, and `randomness` modules at
`--heartbeats 300`, ignoring source-location markers when matching stated
facts and filtering variant readers before definitional abstraction retain
51 verified / 25 budget / 4 failed. The 51 verified targets fall from
3,555,636,546 to 3,531,735,616 raw heartbeats (0.67%). Mint/burn copy-ref
fall from 177.47M/179.00M to 175.14M/175.93M. This is a small saving, not a
resolution of the extra shared-mode leaves or the completeness gap.

The reader filter first checks for an exact occurrence of the tested term;
variant observations and constructor equations retain the transparent
fallback for terms spelled at another family. Marker-aware assumption
matching alone saves about 0.1% on this sample and closes the shared-call
abort test before entering the leaf pipeline.

Rejected on the same module sample: an assumption retry after substitution,
early pin substitution retaining the later whole-context substitution, and
literal simplification before the speculative passes gave negligible gains.
Moving the full cheap decider before those passes retained coverage but
increased verified-target heartbeats by 0.77%. The original single-arrival
proposal therefore remains unresolved; do not infer the predicted 50M saving
from its earlier leaf profile.

### Staking-contract aborts and invariant preservation

`new_staking_contracts_holder` exposed two completeness problems. Its module
sets strict abort checking, but the function sets partial checking with no
abort clauses. Strictness supplies an implicit `aborts_if false`; partial
mode makes it sufficient only. Treating the empty list as an unconditional
no-abort requirement produced false obligations at event-handle calls.
The contract translation now follows the Move Prover's partial semantics,
while retaining explicit abort-code checks and native/map models.

After correcting that, the final memory invariant still failed: nine event
handle calls give nine preservation implications, but `storedInvariantsProof?`
stopped after eight links. It now indexes context facts and searches their
implications with cycle detection and successful-proof memoization, matching
whole invariant propositions rather than mixing predicates at a memory.

Whole-module `staking_contract` runs at `--heartbeats 300`, with shared
continuations enabled, improve from 7 verified / 11 budget / 2 failed to
9 verified / 10 budget / 1 failed. The holder verifies at 240.22M heartbeats
(previously budget-stopped at 347.32M recorded work); `update_distribution_pool`
verifies at 42.81M (previously failed at 275.96M). The seven targets already
verified retain essentially identical cost. The remaining ten budget stops
and one failed target are unresolved.

## What cannot change

Every step is a kernel-checked term, so rewriting costs in proportion to
the terms touched; the constant is hidden only by not rewriting (smaller
goals, evaluation, verified checkers run natively at a trust cost). Lean's
tactics learn no clauses, so a VC's disjunctions are enumerated. A
presented leaf is a normalized leaf: authored proofs written against
leaves pay their normalization. At 32M heartbeats per second, one million
heartbeats is 30 ms, in which Z3 decides a whole VC; the floor per target
is fine, the slope with leaf hardness is set by the passes.
