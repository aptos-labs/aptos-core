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
   options set after the import, so a module extract runs in seconds.
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

## What was learned

- **An array read in a kernel-evaluated computation is a list walk**:
  `a[i]?` walks `i` cells, `a.size` the whole list, folds over index ranges
  are quadratic. Certificates the kernel evaluates iterate `toList`, read
  arenas through an `IndexedArena`, and count sizes once; `_eq` lemmas
  connect the indexed forms to the native ones. One checksum over a big
  structure with `Array.foldl` in the kernel ran out of memory at 13.6 GB.
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
   together through common preparation; unmeasured. The opt-in scheduler
   (`verex.sharedContinuations`) already shares entry decomposition,
   cases, certificate bounds and position facts: −30% automatic
   OrderedMap, 0% authored, +7% micro-targets.
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
  (`verex.contractLemmas`, off: typed −3.3%, authored +0.3%) are.
- **The call's `post` subgoal carries the continuation** (190k–480k nodes
  against 2k–25k of contract), walked by the `call-contract` simp at every
  call. Folding it into a local definition before `wp_call` cut the stage
  to two thirds but broke the scripts that read the continuation.

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
  (2–4 s); bounded by `scripts/verify-modules.py` and `verex.jobs`.
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

## What cannot change

Every step is a kernel-checked term, so rewriting costs in proportion to
the terms touched; the constant is hidden only by not rewriting (smaller
goals, evaluation, verified checkers run natively at a trust cost). Lean's
tactics learn no clauses, so a VC's disjunctions are enumerated. A
presented leaf is a normalized leaf: authored proofs written against
leaves pay their normalization. At 32M heartbeats per second, one million
heartbeats is 30 ms, in which Z3 decides a whole VC; the floor per target
is fine, the slope with leaf hardness is set by the passes.
