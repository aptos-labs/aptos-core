# Performance notes

Status: 2026-09-30. A working log for whoever next works on the stack's
performance: verification (roadmap §1, **Cost**), and the elaboration,
kernel certificates, interface rendering, and source scanning around it.
It covers how to measure, what was changed and why, what was tried and
rejected, and where the time still goes. The design rules for
verification are in [`denotation.md`](denotation.md) ("Principles that
yield performance" and the closer notes). Update the numbers in the
roadmap, not here.

## Where the time went (2026-09-27 round)

The work happened on branch `wrwg/leaner-check-speed` (commits `c14bdd0e`
through `be7b87ea`). The Check fixtures below are the 76 files under
`leaner-e2e-tests/LeanerE2ETests/Check`. Times are summed per file, with
four files running at once on four cores.

| Step | Check suite total | Kernel (type checking) |
|---|---|---|
| Start | 821s | 274s |
| Bounds asserted once per leaf | | |
| Incremental normalization (closer −19%) | | |
| Denotation rewritten by its equations (kernel −14%, closer −20%) | | |
| Preparation certificates without array indexing (kernel −19%) | 660s | 191s |
| Erasure analysis read through arena indexes | 621s | |

The table leaves a cell empty where that step was not measured on its own.
`ReturnedMutRefs`, the slowest fixture, took 98s before the last two steps
and 86s after them. Its kernel time went from 68s to 47s to 31s.

## How to measure

Concrete modular arithmetic (2026-10-07): nested bitvector specification helpers
can leave repeated remainder operations in a leaf even when a precondition fixes
every input. Sending that expression directly to omega introduces unnecessary
quotient/remainder constraints. `leaner_denote_ground_arithmetic` first rewrites
the target using only Int/Nat equalities to literals, then uses the existing
closed-goal decision tactic. It does not rewrite the full context or admit
symbolic equalities as rules. The source and cost evidence is recorded in the
roadmap/handoff; `ArithmeticContext` includes false-claim and unresolved-input
controls as well as a low-budget nested-modular regression.

Heartbeats do not show the largest cost: the kernel checks without
heartbeats, and a kernel stall shows up as time inside whichever `simp`
first waits for a certificate. Use all four instruments below.

1. **Wall time and exactness per fixture.** Run every Check file from
   `leaner-e2e-tests`. Use the driver caps
   (`-DmaxHeartbeats=180000 -Dweak.leaner.verifyHeartbeats=180000`), run
   four files at once with `xargs -P 4`, and compare each output with its
   `.exp` file; a missing `.exp` means the output must be empty. Any
   difference in output is a regression, even when a change is meant only
   to be faster.
2. **Closer heartbeats by stage.** Add `-Dweak.leaner.denoteDebug=true`.
   Every target then prints a line of the form
   `closer: <N>k … bind ×k Nk; exit …; leaf …`. Sum each stage across the
   suite and compare two runs stage by stage.
3. **Time by category.** Add `-Dprofiler=true -Dprofiler.threshold=100000`
   to get the per-file totals:
   - `type checking` is the kernel;
   - `interpretation` is the closer and elaboration;
   - `simp` is simp.

   `-Dtrace.profiler=true` adds per-declaration `[Kernel]` times.
   Certificates the first verify has to wait for show up as the `addDecl`
   time of an auxiliary such as `_simp_2`.
4. **Kernel unfolds of one certificate.** Paste this command into a copy
   of a fixture (below `import LeanerE2ETests.CheckSupport`). Then add
   `#kdiag <theorem>` before the fixture's final `end` for each
   certificate, for example
   `«0x42».returned_mut_refs.borrow_first_after_break.compiled_eq`:

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

   A high count of `List.get`, `List.rec`, or `List.length._f` means array
   indexing inside a kernel-evaluated computation (see below). Do not
   compute a checksum over a big structure with `Array.foldl` in the
   kernel; one such attempt ran out of memory at 13.6 GB.

5. **One target on its own.** `scripts/isolate-target.py <file> <module>
   <target> -o <out> [--option name=value]` keeps one target of a module
   file verified: every other specification block of the module gets
   `pragma verify = false`, the modules after it are dropped, and the
   options (`leaner.certifyDebug=true` for residual goals,
   `leaner.denoteDebug=true` for the closer's steps, `leaner.stageLog=…`)
   are set after the import. A package's module extract then runs one
   target in seconds rather than the whole module.
6. **A leaf's closer steps.** Under `leaner.denoteDebug`, every leaf prints
   its steps with their heartbeats (`cheap: 95k, failed: …`). Sum them by
   step over a run to find the step that dominates (a single `cheap` call
   of 1.7G heartbeats once pointed at the bound collector). To see the
   state a step leaves, print the goal and its hypotheses under the same
   option at the end of that step; a residual printed by `certifyDebug` is
   the leaf's state before its deciders, not after. The goals `denoteDebug`
   prints cost heartbeats themselves; `-Dleaner.denoteProfile=true` instead
   prints, per target, the closer's stages and, per step label, its runs and
   the heartbeats of its succeeding and of its failing runs, with no goal
   printed. The general pipeline (`pipeline`, `range pipeline`) reports its
   stages too, as `pipe-simp`, `pipe-omega`, `pipe-ranges`, `pipe-round`,
   `pipe-bounds-omega`, `pipe-instance`, `pipe-split-write`,
   `pipe-saturate`, `pipe-split`, `pipe-bv`, `pipe-grind` and the like, so a
   leaf it closes is attributed to the stage that pays (2026-10-08: the
   `bitwise_features::contains` leaves were `pipe-saturate` and `pipe-bv`).
7. **The benchmark problems.** `scripts/leaner-bench.py run [--only …]`
   builds `leaner-bench`, measures the standard problems natively, by
   phase, records the run in the local history, and prints each problem's
   change in wall time and heartbeats against the previous local run of the
   branch (`verification-benchmarks.md`, "Local use").

`perf record` on `lean` works if a statically linked Lean binary exposes
the kernel symbols. It showed the same hot spots as the instruments above,
so they are enough in practice.

Before committing a change, run everything that `CLAUDE.md` lists:
- the Check suite, with exact output;
- `leaner-ir` `lake test`, including the `DenotePerformance` and
  `CompositionPerformance` gates, re-recorded only for a deliberate cost
  change and with the diff reviewed;
- the stdlib verify baseline (`LEANER_E2E_SUITE=verify`);
- `leaner-move` and `leaner-rust` `lake test`.

Run nothing else in parallel with a timing comparison. Never rebuild
`leaner-ir` while a suite run is reading its `.olean` files.

`UB=1 lake test` accepts every output, a new failure included, and the
suite still passes. After such a run, `scripts/exp-error-delta.sh` lists
the baselines whose count of error lines differs from `HEAD` and the new
ones; any it lists that the change did not mean to touch is a regression.

## What was learned

- **An array read in a kernel-evaluated computation costs a list walk.**
  In the kernel, an `Array` is a structure over a `List`:
  - `a[i]?` walks `i` cells;
  - `a.size` walks the whole list;
  - `Array.foldl`, `find?`, and `contains` over an index range are
    quadratic.

  The compile certificates (`compiled_eq`, the compile views) and the
  frame certificates are evaluated by the kernel, so they must:
  - iterate `toList`;
  - read arenas through an `IndexedArena`, a balanced tree built in one
    pass by `IndexedArena.build`;
  - count sizes once.

  The native path keeps its arrays; an `_eq` lemma connects the indexed
  form to the native one (`sharedReferenceErasureChunksIndexed_eq`,
  `IndexedArena.get?_ofArray`).
- **Unfolding a big recursive definition leaves the kernel to redo it.**
  `simp` unfolding `Term.denote` records no proof steps, so the kernel
  re-reduces its 61-way matcher, some 27k nodes. Rewriting with the
  equation lemmas (`lir_denote_equations`) makes each step an instance the
  kernel only checks. The `call` equation has an index
  (`ResultShape.bodyType`) that does not match at reducible transparency,
  so plain unfolding stays as the fallback.
- **In `simp`, the cost is per node, not per rule.** Each lookup reduces
  the reducible carriers in implicit arguments (`NTy.carrier`, `HEnv`) by
  smart unfolding; this is about half of every normalization. The closer
  therefore normalizes only what a step creates (`normalizeAround` with
  known-normal subterms) instead of renormalizing the goal.
- **Repeated leaf work.** Saturation used to reassert bounds that an
  ancestor goal had already asserted. It now records them by goal content,
  compared without regard to hypothesis order.
- **A value read twice doubles the goal.** A computed value substituted
  into the continuation is copied at every read of its local, so a chain
  of updates reading a local twice (`math128::sqrt`'s Newton steps) grows
  the leaves exponentially: 1.6M, 6.7M, 32M heartbeats for one, two,
  three steps. The checked arithmetic and shift rules name their result
  instead, as the Prover's temporaries: the continuation receives a
  variable with `result.val = value` and the value's fit. Casts, which
  only copy a value, stay unnamed, so a cast operand and the specification
  meet at one atom. Array positions are compared by value: after a
  leaf's normalization a position reading a name is replaced by its
  definition (`leaner_denote_unname_positions`), so a write at a named
  position meets a read at the computed one. A residual leaf handed to an
  authored proof has the names replaced again (`leaner_denote_unname`). Cost on the benchmark:
  +1.7% in total, +4–7% on functions of one operation; `math128::sqrt`
  from over 1.5B heartbeats to 5 s.
- **Rewriting with hypotheses must terminate.** `leaner_simp_all` admits
  an equation as a rewrite rule only if its right side reaches no left side
  of an admitted equation that leads back to its own (ground rules without
  such a cycle terminate); a floor's `result = self >> 64` with a branch's
  `self = result << 64 % 2^128` otherwise rewrote without end.
- **Flow binds associate.** A flow bound after a flow copied the goal's
  continuation into the inner bind's arms; `wp_flowBind_flowBind` moves
  the inner action to the head first. Nested generic inlining
  (`string_utils::format4`) from over 170 s to 10 s.

## Tried and rejected

| Experiment | Result |
|---|---|
| `dsimp := false` in the normalizer | 8 fixtures stopped verifying |
| `index := false` for the normalization simp set | slower |
| Precompiling `LeanerIR` (`precompileModules`) | no gain; cross-library dynlib load error |
| Equation lemmas without the unfolding fallback | `break_with_returned_pair` fails (the `call` index mismatch above) |
| A per-view table of native types in `CompileNamespace`, so `compiled_eq` would stop re-deriving types | 1% fewer unfolds in `compiled_eq`, same time; the Check suite went from 621s to 633s, because the view certificate derives every type, including unused ones |

## Where the cost is now

For the three slowest fixtures (kernel / closer and elaboration / simp):

| Fixture | Kernel | Closer and elaboration | Simp |
|---|---|---|---|
| `ReturnedMutRefs` | 31s | 40s | 11s |
| `VectorOperations` | 16s | 12s | 1s |
| `OrderedMap` | 11s | 15s | 12s |

Leads, most promising first:

1. **`compiled_eq`**, up to 1s per function, for example
   `borrow_first_after_break`: about 80k unfolds, dominated by `List.rec`,
   and about 4.4k `List.get` with 2.7k `instDecidableEqList` and
   `List.length`. Some of the remaining lookups are still array reads:
   - `unit.namespaces[…]?` and `calleeNs.functions[…]?` for calls and
     constants;
   - `targetNs.structs[…]?` and the names table in `ntyOfFuel`;
   - `loanFacts[…]?`;
   - `declaration.locals`.

   The list equality probably comes from comparing the evaluated function
   with its literal, or from the `List.Nodup` decisions of enum types.
   Split the certificate's cost into evaluation and comparison before
   changing anything.
2. **Carrier unfolding in `simp`.** Carriers whose instances are stated at
   the row type would avoid smart unfolding, which is a deeper change to
   `Types.lean`.
3. **Leaves that no decider closes.** Each saturation round over a large
   context costs 8–18M heartbeats.

## Resume here (2026-09-28, stdlib budgets)

Quantifier automation of roadmap §1 landed for the standard library
(`longest_set_sequence_starting_at`, `shift_left_for_verification_only`,
`is_enabled`, `on_new_epoch`, `change_feature_flags_for_next_epoch`,
`apply_diff` verify); what remains is cost. Three targets verify only above
the 180M budget: `shift_left_for_verification_only` 209M,
`change_feature_flags_internal` 318M, `apply_diff` 264M. Their profiles
(`-Dweak.leaner.denoteDebug=true`; every closer step prints its heartbeats,
and a `leaner_denote_timed` label prints even when the step fails) are
dominated by leaves (100–150M: `prepare` 1.5–2M each, of which the two
normalization passes are most), then the loop steps (40–57M) and binds.
Leads, in order:
- the loop-step normalization (`normalize 1`, 5–9M per loop) normalizes the
  invariant instantiated at the entry row; the projections are reduced
  first now, the remaining cost is the row's types (`HEnv` of a 24-slot
  row derived at each `Prod` projection);
- `prepare`'s renormalization runs whether the substitutions changed
  anything or not;
- `written` (the deciders over a prepared leaf) costs 0.5–0.7M when it
  succeeds, mostly `leaner_denote_lookups_after_writes` and the split.

Lessons that cost days: a `perm` lemma (`bitwiseAnd_comm`) in a
normalization set makes simp order operands with `acLt`, which reduces the
operands and hung on `features`' contract terms — removed; a conditional
lemma whose side condition is arithmetic (`eraseIdxIfInBounds_of_lt`,
`insertIdxIfInBounds_of_le`) must not be in a set simp discharges by
recursive simp — they are applied with `disch := omega` at leaves; a
discharger spelled `assumption` compares at default transparency and
unfolds the arithmetic of every hypothesis (`leaner_denote_assumption`
instead). To find a hang: the closer's debug prints go to stdout and the
messages to the message log, so their order in one file is not preserved;
`=== closer start/end` bracket each target's stdout, an unfinished bracket
is the hanging target, and its last `→ head` line names the stage.

`acl` verifies (2026-09-28; the whole module in 16s). Its costly leaves
are `remove`'s invariant after the removal (49M) and its `ensures` (17M):
the removal splits the two quantified positions against the removed one,
and the branches where the positions shift are decided by instantiating the
injectivity invariant at every read position (`leaner_denote_instantiate_positions`,
20–30 instances a leaf). Asserting the instances costs little (omega on the
bounds, 1.5M for 30); rewriting them by the context is 10M (a third of a
million each, the hypotheses as rewrite rules over the encoded-element
forms). Leads: instantiate at the hypothesis's own lookup patterns rather
than every position; skip instances at a repeated position; rewrite the
instances with the lookup and equation facts alone.

Bugs found on the way, each of which had silently disabled a decider, and
their rule: a hypothesis name inside a tactic quotation is hygienic
(`rw [..] at search` named nothing; splice `$(mkIdent \`search)`), and an
inaccessible name (`a✝`) cannot be spelled in a `simp ... at` location at
all — rewriting at hypotheses goes through `simpAt` by `FVarId`; a nested
`(← act)` inside `goal.withContext (f (← act))` runs `act` outside the
context (do-notation lifts it), so `inferType` there saw a foreign context
— write `goal.withContext do let x ← act; f x`; after `goal.assert`, the
old goal is assigned and `getMainGoal` skips it ("no goals") — `setGoals`
the new one; `apply` at reducible transparency does not see `¬p` as
`p → False`, so a refutation by instance unfolds the negated conclusion;
`replaceMainGoal []` with an emptied goal list throws, and a `try` around
the whole decider hid it; `Array.getElem?_push_lt` rewrites to `some a[i]`,
off the `a[i]?` normal form (`getElem?_push_of_lt`); `generalize` over a
term a dependent proof mentions fails ("motive is not type correct"), so
the search split asserts the disjunction `= none ∨ ∃ i, = some i` and
rewrites by the equation instead.

The canonical membership form reaches authored proofs: a fixture lemma
concluding `¬∃ a, …` (OrderedMap's absence lemmas) is restated as
`∀ a, 0 ≤ a → a < size → ¬…`, the form the leaves now have. A loop
iteration's recursive hypothesis binds the value a slot holds
(`env.fst = some x →`), which `apply` leaves as a data goal; the loop stage
closes such equations by reflexivity right after the `apply`, so no leaf
ever sees a data goal (a data goal handed to the deciders had been closed
by `assumption` picking any term of the type). An existential's witness is
also taken from the positions the context's lookup facts read, with the
premises decided by the context, which closes `has_key`-style clauses
without the fixture's `present` lemmas; the pipeline tries the witness
after its instances.

**Open lead (2026-09-28, after the acl round): the features targets cost
2–3× what they did before it.** `apply_diff` was 264M and is 714M;
`change_feature_flags_internal` 318M → 391M; `shift_left_for_verification_only`
209M → 284M. Measured on `apply_diff` (`featuresB`: the module with every
other spec `pragma verify = false`, `set_option leaner.verifyHeartbeats
800000`, debug on): loops ×15 197M → 115M after the stage cache below,
leaves ×82 230M, exits ×208 86M, binds ×395 71M. Ruled out by A/B: the
membership translation of vector quantifiers (index form: same cost), the
normal-set additions of the round (removed: same cost), the committed
closer (does not run the current fixtures). Not yet bisected: the leaf
deciders added in the round only run at leaves, so the stage growth
(exits, binds, loop normalizations) is in the contexts themselves, which
the round's stage changes (data invariants as loop conjuncts, entry-local
bindings for `old`, anchors, `FunctionStart` facts of 12k nodes per inlined
callee) enlarged. Two general fixes landed from this analysis:
- the leaf normalization's discharger (`contextDischarge`) skips conditions
  about variables simp introduced under a binder, decides nonnegativity of
  certified unsigned values structurally, answers a condition asked twice
  from a cache, and runs omega over the hypotheses sharing a variable with
  the condition, not the whole context (37M → 4M per leaf normalization on
  `apply_diff`; `Array.length_toList`, `Array.toArray_toList`,
  `List.toList_toArray` and `boundedVector_decode?_map_encode` remove the
  conditions it was asked most);
- the loop step's `simp at *` passes skip the hypotheses a stage already
  left normal (`leaner_denote_normalize_context_stage`, its own cache):
  loops 197M → 115M.
The `closer` summary now names its target (`closer <decl>: …`), and
`=== closer start <decl>` brackets its stdout.

Next: roadmap §3's "Verification in the Move CLI".

Method: build a scratch LeanerLang file from the rendered module
(`leaner-e2e-tests/LeanerE2ETests/MoveToLeanerLang/MoveStdlib/sources/*.exp.lean`,
or the modules of the generated file `leaner-move verify <package>
--output` writes, which links the used modules), and run it with `lake env
lean` from `leaner-e2e-tests`; `pragma verify = false` on the other
functions narrows it to one target. `verify f by all_goals (trace_state;
sorry)` shows the prepared leaves; `set_option leaner.verifyHeartbeats` in
the file raises the budget. Validate every step as in "How to measure",
including the stdlib baseline (`UB=1`, diff reviewed).

## aptos-stdlib by module (2026-09-28, evening)

The 40-minute `prove --lean` runs of `aptos-stdlib` were not closer work:
the whole package's targets cost about 509M heartbeats, twenty seconds at
the 25M heartbeats a second the elaborator sustains (`DenotePerformance`).
The time went to `frameInstantiation_i` certificates proved by a
Meta-level `with_unfolding_all rfl` over the unit, which diverged (max
recursion depth, or an `isDefEq` timeout at the default 200M) once a
trusted callee's module carried a generic cross-module call; and because
the certificate theorems ran asynchronously, the failure surfaced as
`sorryAx` in the target and an error at the module header, after the target
had burned its time. Both are gone: the certificate is a kernel `Eq.refl`
of the elaborator's value, and every generated theorem is synchronous
(`denotation.md`).

Measured module by module, each module's extract elaborated alone (its
dependencies as non-targets), with the 180M budget of the time:

| Module | Time | Errors |
|---|---|---|
| `ristretto255`, `bls12381`, `ristretto255_pedersen`, `ristretto255_elgamal` | >300s (capped) | — |
| `ed25519`, `ristretto255_bulletproofs` | 134–148s (capped) | — |
| `smart_table`, `smart_vector`, `type_info` | ~90s | 0 (`type_info`) |
| `any`, `copyable_any`, `string_utils`, `secp256k1` | 63–78s | 4–6 |
| `aptos_hash`, `reflect`, `debug`, `pool_u64`, `math_fixed*`, `crypto_algebra` | 20–30s | 0–6 |
| everything else | 1–17s | mostly 0 |

The crypto modules print nothing before the cap: their module command
(elaboration and every target) is one Lean command, so the profiler
reports only when it ends, and a command's stdout is captured into its
messages, so a marker must be written to a file to be seen from a killed
run. Probed target by target (`ristretto255`, module otherwise
`verify = false`): the module elaborates in 17s, `double_scalar_mul`
verifies in 25s, `scalar_is_zero` fails in 25s (below), and
`multi_scalar_mul` stalls at 12 GB before its typed theorem: in its three
`frameInstantiation_i.decided` certificates, where the kernel evaluates
`invocationTypeInstantiation` — a fold over **every** type of the callee's
namespace that, for each compound type, searches the whole types table by
structural equality (`findIdx?` with the derived `BEq Ty`). Quadratic in the
namespace's types with expensive comparisons, replayed by the kernel per
certificate; small units pay milliseconds, `ristretto255`'s minutes. The
Meta-level `rfl` that preceded the kernel certificate diverged on the same
computation. Measured precisely (2026-09-29): the kernel decision itself, whose check ran
asynchronously behind the synchronous certificate theorem that waits for
it; the kernel cost had two parts. The array-access part is gone: the
certificate is now stated through the callee namespace's view
(`frameInstantiationIn`, `frameInstantiation_of_view`), reading the types
through the view's `IndexedArena` and iterating them as a list
(`invocationTypeInstantiationIn`, tied to the runtime's
`invocationTypeInstantiation` by `_eq` lemmas). That turned the stall into
**25–30 s per certificate** on `ristretto255` (`multi_scalar_mul`: three
certificates, 80 s; its closer 1 s), which is the second part and intrinsic
to the definition: `instantiatePlaceFieldType?` locates every instantiated
compound type by a structural `findIdx?` over the whole table, about N²
`Ty` comparisons for N ≈ 350 types. **Closed 2026-09-29** by the witness
certificate (`InstantiationCertificate.lean`, `denotation.md`): a key map
over the types' fingerprints decided once per namespace, and per
certificate a witness per type checked with logarithmic reads. On
`ristretto255::multi_scalar_mul` (three certificates) the verify item
costs 17 s in all, closer included, where the certificates alone cost 80 s
before. The route not taken, a `lookup_frameInstantiation` lemma so the
closer evaluates only the lookups a callee's contract makes, remains
available if the linear checker ever matters.

A failing target burns its whole budget, so a module of many unproved
specifications costs its target count times the budget: the reason the
budget stays "about a minute" and a failure asks for a proof
(`source-verification.md`, Proofs) rather than more time.

`scalar_is_zero` showed a second, independent gap, closed 2026-09-29: a
specification function compares a vector through its encoding (`map encode
xs = raw`), the body through the typed vector. The normalizer now decides
an encoded vector against another encoding by injectivity
(`map_encode_eq_map_encode_iff`), against a runtime literal by decoding the
literal element by element under the codec's tightness
(`map_specInt_encode_eq_toArray_iff` and its siblings, concluding at the
values, `v.values = decoded`), and a quantifier over a short literal range
as its instances (`expandLiteralRange`, a simproc that requires literal
bounds; the conditional lemma tried first fired on symbolic bounds that a
hypothesis discharged and looped). A goal about vectors is then decided at
their values (`SpecVector.eq_iff_values`, applied to the goal only in the
saturation round): the deciders read vectors as arrays, while a hypothesis
`v = ⟨…⟩` keeps the vector for substitution — as a simp lemma the same
bridge broke `acl`, `bit_vector` and `option`, and in the other direction
(values to vector) it undid the closer's own ext step and, under an
instantiated family, looped with `instantiatedEqAsEncoding`. Fixture
`Check/Specifications/EncodedVectors`. With it `ristretto255` verifies all
targets but `multi_scalar_mul` in 95 s.

`multi_scalar_mul` was a third gap, closed 2026-09-29: its native callee
is generic (`multi_scalar_mul_internal<P, S>`) and its contract applies the
opaque `spec_multi_scalar_mul_internal` under the callee's type parameters.
`Contract.opaqueSpec` keyed a call by its qualified name with the type
arguments rendered as text — `<ValueRep.parameter 0, …>` in the callee's
contract, `<twin RistrettoPoint, …>` in the caller's clause — which no
instantiation could substitute. The key is now the name alone, and the
type arguments are a list of native types under the contract's family:
`Skolems.type index` is what parameter `index` stands for (itself at the
public family, the caller's argument at an instantiated one,
`Skolems.type_instantiate`), and every type argument is quoted as
`τ.substWith Θ.type`, so the callee's `(param 0).substWith Θ.type` and the
caller's `(struct …).substWith runtime.type` both normalize to the caller's
type. Fixture `Check/Calls/OpaqueGeneric`.

## aptos-stdlib by module (2026-09-29, evening)

Each module's extract elaborated alone, sequentially, on an idle machine,
after the witness certificates and the vector normalizations:

| Module | Time | Verification failures |
|---|---|---|
| `ristretto255_bulletproofs`, `ristretto255_pedersen`, `ristretto255_elgamal`, `ristretto255` | 124–168s | `multi_scalar_mul` (since fixed; the module alone now verifies in 95s) |
| `smart_vector`, `smart_table` | 128–142s | 0 (table intrinsics unsupported) |
| `ed25519` | 57s | 2 (`sha3_256` has no contract) |
| `any`, `copyable_any` | 40–44s | 2 each (`from_bcs`) |
| `bls12381`, `debug`, `pool_u64`, `aptos_hash`, `secp256k1`, `math_fixed*`, `crypto_algebra` | 15–23s | 0–1 |
| everything else | ≤ 15s | mostly 0 |

The sum over all 50 extracts is 1212s; no module exceeds three minutes.
The whole-package run then still exceeded 30 minutes, which the extracts
could not explain: measured with markers written to a file at each module
(`run_cmd` lines between the modules, since a command's messages arrive
only at its end), every module command cost time superlinear in the size of
the file after it — 0.3s in a 10 KB file, 13s at 344 KB, 40s at 548 KB —
and the same module followed by 566 KB of line comments cost 89s, while a
plain `def` in that position cost seconds. The cost was the comment
scanner of the namespace command (`LeanerLang/Comments.lean`): it rebuilt
prefix strings of the whole file per comment and re-encoded the whole
source per trailing comment, quadratic in the file, twice per module. It is
now one linear pass over byte positions; the module commands of the package
file elaborate in under a second each (`leaner.stageLog`, below).

### Stage log

`set_option leaner.stageLog "<file>"` makes the verifier append a
timestamped line to that file at every stage of a module (unit, interfaces,
validated, linked) and of a target (semantics; compiled and contracts;
callees and certificates; theorem start and done). It is the way to time a
run that cannot be waited for: a `lean` process prints its messages when a
command ends, and a killed run prints nothing. Kill such a run by its
`lean` process: `timeout` around `lake env lean` kills `lake`, not the
`lean` it spawned (`lake env sh -c 'timeout -s KILL 300 lean file'` does).

With it, the package run's time is in targets: `math128::sqrt` 53s (over
budget in normalization — its `verify … by skip` times out at `whnf` too,
so the cost precedes the decision), `ristretto255::scalar_invert` 50s, the
certificate stage of the ristretto255 natives' callers 5–11s each, and
everything else under 5s.


## Kernel calibration on a 351-type table (2026-09-29)

Measured as kernel checks (`decide +kernel` on a `Bool` computation) over
a synthetic 351-type table published as an `Array.mk` literal, import time
(0.75 s) subtracted:

| computation | time |
|---|---|
| one scan of a 351-element `List Nat` with `Nat.beq` | < 10 ms |
| 51 such scans of ~325 elements | 0.4 s (25 µs per element visited) |
| 351 pairs of `List` index reads (`keys[k]!`) | 1.3 s (11 µs per element walked) |
| 351 `Array` index reads with a `Ty.beq` each | 2 s |
| first-match table by scans and index reads | 10–13 s |
| 351 reads through `IndexedArena.get?` | < 100 ms |
| 351 `Ty.beq` on erased types read through the index | < 100 ms |
| 351 fingerprints of erased types read through the index | < 100 ms |

So the kernel's cost is not in the comparisons but in the *walks*: every
element a list or `Array` literal is traversed past costs 10–25 µs (the
structural recursion's unfolding, not the element), so an indexed read of
a 351-entry table costs about 2 ms and any checker that indexes per entry
is quadratic in practice. A balanced index (`IndexedArena`, `KeyMap`)
makes every read logarithmic, and a type comparison or a fingerprint is
well under 0.3 ms once its operands are at hand. The earlier reading of
these numbers — a structural comparison at 0.5 ms, so the first-match
table's N²/2 comparisons as the floor — measured the index walks around
the comparisons, not the comparisons.

The certificate therefore reads everything through indexes: the types
through the view's `IndexedArena`, the witnesses through an index certified
against their array by `ofArray … = tree` (`Eq.refl`), the search through
the key map; the `foldWitnesses` fold and the depth bound iterate the
witnesses once. Publishing a table as an `Array.mk` list literal rather
than `#[…]` changes nothing: `List.toArray` is reducible.


## Modules, generic frames, and shared terms (2026-09-30)

Measured on `ordered_map` (a 1,400-line module with 16 verified tests)
and a one-function client module that imports it.

**Dependency interfaces.** A module is lowered against the interfaces of
the modules it uses, each rendered to source and parsed back
(`Print.interfaceNamespace`). The client did not finish in 5 minutes: the
interface of `ordered_map` took 63.5 s to render. Rendering only
signatures cut that to 9 s; the remainder was the import spelling, which
walked the whole namespace (`referencedNames`) for every name printed.
`PrintNamespace` decides the spelling once per rendered namespace: the
interface now renders in 6 ms and parses in 68 ms, and the client runs in
21 s.

**Generic frames.** Every generic call of a target was certified anew
(`frameInstantiation_i`: witnesses over all of the callee namespace's
types), 5–8 s of kernel time per target on `ordered_map`'s 400 types, and
the same `<u64, u64>` instantiation for every test. The certificate is now
per unit and instantiation (`semantics.frame_<namespace>_<type ids>`), and
a target derives its frame by one rewrite. Keyed by the type arguments'
source locations as well, it was shared only by calls at one site, and
every `ordered_map` test still paid 9 s. A frame reads which types its
arguments name, not where they occur (`frameInstantiation_congr`), so it
is stated once at location 0 and every call naming the same types shares
it: 0.3 s per target after the first.

**Lifetimes in the frame check.** A Move namespace has one reference type
and one lifetime per reference occurrence: `ordered_map`'s table holds 642
types, 585 of them references, and 568 lifetimes. The checker read each
reference's lifetime through the namespace's lifetime array, a list walk
in the kernel (160k `List.get` steps per frame). It reads the lifetimes'
kinds through an index published once per namespace (`lifetimeKinds_<i>`):
one frame check went from 18 s to 2.6 s.

**Variant splits at leaves.** `leaner_denote_subst_vars` splits a leaf on
the enum locals it tests and renormalized the whole context of every case:
in `iter_walk_mut`, 8 cases at ~3.6M heartbeats each, ~29M per split and
417M of the target's 1.13G. Only the hypotheses the cases reintroduce
mention the split locals; only they and the target are normalized now
(splits 185M, `iter_walk_mut` 33 s → 28 s).

**Shared terms.** The bound-site collectors of `assertBounds` walked terms
as trees. A vector rotated three times in place is a term whose parts are
shared, exponential as a tree: one leaf of `rotate_slice_values`
(`Check/Vectors/VectorOperations`) collected 242,003 sites and ran out of
its budget. Walked once per shared subterm (`sitesWhere`), the whole
fixture verifies in 9 s.

**Reading a profile.** Under `lake env lean`, LeanerLang's elaborator
runs in the interpreter (7.8 s of the client's 21 s); the `leaner-move`
executable runs it natively. A synchronous `addDecl` waits for the kernel
checks queued before it, so a small theorem can report the queue's time:
a two-field decoder lemma showed 6.5 s. `-Dtrace.profiler=true
-Dtrace.profiler.threshold=300` names the declarations the kernel actually
spends on; `#kdiag` (above) rechecks one alone (the decoder lemma: 4 ms).
Under `leaner.denoteDebug`, `leaner_denote_timed` renders a failing step's
message, which holds the whole goal: ~3.6M heartbeats per failure, outside
the closer's stage totals. Wrap a step that fails often only to count it.

Where the time still goes (`ordered_map` 82 s, with the client 113 s):

- About 25 s once per module, before its first target: the unit
  preparation certificates replay loan-death marking and shared-reference
  erasure over every body of the unit, not only the ones the targets reach
  (`erasureIndexedChunks_eq` 5.5 s, `marked_eq` 2.3 s,
  `erasureIndexes_eq` 1.1 s, `expressionArenasIndexed_eq` 0.6 s,
  `compileView` 1 s), then the key map and the first frame (~3 s). Every
  importer re-checks them for the linked unit. Removed since: the
  semantics reads the validated unit (below).
- `iter_walk_mut` 28 s: two leaves of the loop step fall through to
  `leaner_denote_pipeline`, 134M and 150M heartbeats. Timed by stage over
  the four leaves that reach it: `leaner_denote_saturate` 109M, the
  saturation rounds 91M, the final `leaner_denote_bounds; omega` 71M,
  `leaner_simp_all` after the split 42M. The goal reads the map after
  `iter_borrow_mut` wrote 7 at position `count`, at a position `i` below
  `count + 1`, and follows from the loop invariant at `i < count` and the
  write at `i = count`; no step reads through the write and instantiates
  the invariant, so saturation does it generically. Next: a positional
  split of a written map in the map-positions pass (position equal to the
  written one or not, the other case read in the map before the write), as
  `leaner_denote_split_write` does for arrays, leaving the invariant's
  instance to `leaner_denote_instance`.
- The map-positions pass (`leaner_denote_map_positions`, a contextual simp
  with the context's facts as rules, and its generated facts) costs 2–20M
  heartbeats per leaf where keys are read (110M in `iter_walk_mut`).
- The other tests take at most 7 s each.


## No preparation to certify (2026-09-30)

The semantics ran on a prepared unit: loan-death marking and
shared-reference erasure applied to the validated unit. Every module
quoted the prepared unit as a second literal and proved it equal to the
preparation by replaying both passes in the kernel (`marked_eq`, the
`erasure*_eq` chain, `semantics_eq`), over every body of the unit and
again in every importer. The first target of a module waited for it in
its "semantics" stage.

The plan was to prove once, generically, that compiling the prepared unit
is compiling the validated unit with the preparation applied locally.
Both passes turned out to be local decisions, so the semantics now takes
them in place and reads the validated unit itself; there is nothing left
to relate:

- **Loan deaths at their anchors.** Marking replaced an anchor by a block
  of an `endLoan` marker and the anchor moved to the end of the arena, so
  prepared indexes differed from validated ones. The big-step relation
  now ends the loans the certificates record before a node, evaluates it
  (`EvalNodeWith`; a loop repeats its node), and ends those recorded after
  it once it produced a value (`EvalExprWith`). The interpreter and
  compilation do the same; `endLoan` left the syntax.
- **Shared references in place.** A dereference or freeze of a shared
  operand reads it, decided by the operand's recorded type. A place
  dereferencing a shared reference is its base; that decision needs the
  owning function's local types, so validation records the places in the
  function's borrow certificate (`sharedDereferences`).

`prepareExecution` wraps the validated unit, which indexes the executable
unit's type, so the certificates are stated at the `unit` literal. The compile view of a namespace
carries its death and shared-dereference indexes.

Measured on `ordered_map` and its one-function client (same machine,
sequential runs): the file 124 s to 95 s; the first target's "semantics"
stage 14.3 s to 0.1 s in each module; the `ordered_map` targets 87 s to
72 s, the client's 33 s to 17 s.

A node now costs two units of interpreter fuel, its deaths and its
evaluation.


## `grind` over prepared leaves (2026-10-01)

Omega reads a product or quotient of variables as an atom, so it misses
what congruence gives: `pool_u64::balance` had `result.val = lookup.asInt`
and `MAX < result.val * c / s` in the context of the goal
`MAX < lookup.asInt * c / s`, and its three undecided leaves cost 870M of
the target's 931M heartbeats. Core `grind` (congruence closure with linear
integer arithmetic) is now the last of the prepared deciders
(`leaner_denote_decide_written`): it closes each of those leaves in about
3M, and `balance` verifies in 137M. On a raw leaf, before `prepare`, it
closed none of them at 2–4M each, so it runs only on the prepared form.

`aptos-stdlib` as one package run (`leaner-move verify`, its dependencies
read whole), with and without it, concurrently on one machine: the
verification phase 173 s to 127 s, one target more verified (`balance`),
the same failures otherwise; certification (~215 s) is unchanged. The
`DenotePerformance` gate and the Check fixtures do not change, their leaves
being closed before it; the `ordered_map` file takes 202 s instead of
187 s, from leaves where it fails before the pipeline closes them.

`grind` is also the last decider of a case the write split leaves
(`leaner_denote_decide_split`), and the map laws are its lemmas, as the
SMT solver's map axioms are the Prover's: `hasKey`, `valueAt`, and `size`
over `update`, `remove`, and `empty` (`grind =`), validity preserved by
them, and a present key's rank below the size (`Maps.lean`). With them
`pool_u64::add_shares` verifies (19 s): after `push_back` of a shareholder
the map lacks, the split cases of the distinctness and membership
invariants follow by congruence from the invariants' instances. The
package run then reaches `buy_in`, which exceeds its budget in `whnf`.

A failing `grind` cost up to 91M heartbeats per attempt, and the write
split tries it in every case it leaves: `deduct_shares` spent 1.1G in
them. `grind` now runs within the 20M of an attempt
(`leaner_denote_grind`), after clearing the callees' contracts, which the
call rule has consumed and which `grind` would instantiate as theorems, and
the continuations bound in the context. Its cost on a large leaf is its
preprocessing (17M on a `deduct_shares` leaf, the same under every search
setting). An E-matching lemma needs a term to match: `rank_nonneg`'s
conclusion normalizes to a pattern that never occurs, so it is keyed on
`rank map key` (`grind_pattern`), which decides `deduct_shares`' abort
clauses; the codecs' scalar readings (`asInt`, `asBool`, `asString` of an
encoded scalar) are lemmas too. `ordered_map` takes 189 s again and
`test_verify_iter_collect_symbolic` verifies; the package run takes 318 s
(verification 142 s).

A leaf without a quantified hypothesis or a written vector is prepared
too when it observes a map, and decided by `grind` (`leaner_denote_observes_map`);
any other still goes to the pipeline, as `prepare` and a failing `grind`
before it cost the storage targets of the gate up to 26%. With the law that
an ordered removal shifts the keys after the removed one
(`keyAt_remove_ordered`) and the positions of a valid map holding distinct
keys (`keyAt_eq_keyAt_iff`, keyed on two `keyAt` terms), `ordered_map`'s
`ground_enum_123` (whose second leaf exhausted the budget in the pipeline)
and `test_verify_remove_shift_symbolic` verify; the file has 4 failing
targets of 7 (`new_from` tests, `drain`, which now exhausts its budget, and
`iter_sum`, which needs a recursive specification function unfolded) and
takes 241 s.

`grind`'s cost on a map leaf is its internalization of the terms, the same
under every search setting: an ordered map's discipline carries the
structural order's rank tables (`valueRanks` of a literal) in every map
term. Abstracted to variables before `grind` (no lemma reads inside one),
they cost 55–60% less on `drain`'s leaves (44M to 18M, 171M to 76M, 95M to
41M); abstracting the aggregate encoders as well gains nothing. Named
constants in the contracts instead of literals would not do: the compiled
bodies carry the same literal, and the closer relies on the two being the
same term. With `hasKey_keyAt` and `rank_keyAt` as lemmas, `grind` closes
every residual leaf of `drain` in an authored proof (`all_goals grind`);
the automatic run still exceeds its budget, one leaf needing more than an
attempt's 20M (100M did not suffice) before the pipeline spends 1.38G on
it. The `ordered_map` file takes 220 s.

## Benchmark regressions, attributed (2026-10-04)

Against the local run of 2026-10-03 the benchmark's verified problems cost
4.9% more heartbeats, concentrated on a few targets. Snapshots of four
intermediate commits, built and run on the same problems, attributed them;
the closer's step profile (`leaner.denoteDebug`, through `LEANER_OPTIONS`
for a package problem) located the cost within a target.

- Division facts (`757379d637`): at every quotient or remainder with a
  non-literal divisor the leaf got the division algorithm with its
  remainder bound as an implication (`0 < d → r < d`). `omega` splits on
  the implication, and the identity adds atoms; division-heavy targets
  cost 45–136% more (`fixed_point32::create_from_rational` 34M→52M,
  `pool_u64::balance` 142M→333M). Now the identity is asserted only where
  the leaf reads the remainder or a product with the divisor or the
  quotient (it constrains nothing else linearly), the remainder's bound
  outright from a hypothesis making the divisor positive (`¬ d = 0` with
  the certified `0 ≤ d`), and as the implication only where the remainder
  is read. The targets are back within 2% of the base.
- Element bounds (`1532c6533e`): the bounds of the unsigned elements a
  goal's comparisons read were extended from the comparison's operands to
  every read nested anywhere in them, also under bitwise operations and
  conversions, which `omega` reads as atoms; `features::set` cost 93M→123M
  with no fixture needing it. Now a comparison's reads are collected
  through its linear operators only.
- Proof blocks (`b1e86e5d20`): a Move `proof { split c; }` is a case split
  over the rest of the body, so `fixed_point32::ceil` verifies twice
  (8→16 leaves, 22M→35M), although its automatic proof needs no hint. Open:
  whether a proof block's steps should be applied only when the automatic
  proof fails.
- The typed-carrier series (C1–C3c) costs 2–4% on most targets, 10% on
  `capability` (phantom type arguments on `Cap<F>`), as the regenerated
  cost gate records.

## Recursion depth at closure facts (2026-10-04)

Six functions of `behavioral_predicates_examples.move` failed with
`maximum recursion depth has been reached` (needing 600–1024 against the
default 512) where they had verified or timed out before, in `obtain` and
`split`, on goals only 31 deep. Instrumenting the closer's stages with
`tryCatchRuntimeEx` found the stages; `trace.Meta.isDefEq` and
`trace.Meta.whnf` on the failing step found the mechanism: a closure node
carried its facts (`closureRows? unit h m = some rows`, `closureShared`,
`closureFaithful`) as `Eq.refl` behind a type hint (`mkExpectedTypeHint`),
left to the kernel's evaluation. A normalization that strips the hint
leaves an `Eq.refl` whose inferred type is `some rows = some rows`; when
the closer then compares the stripped copy with the original, proof
irrelevance compares their types, `closureRows? unit h m =?= some rows`,
and `whnf` evaluates the unit computation in the elaborator, indexing the
unit's function array by unary list steps — depth proportional to the
target's index in the module. Marking the computations `irreducible` does
not help: the types differ, so delta is the only way to agree. Such facts
are now auxiliary theorems (`mkAuxTheorem`, kernel-checked once, cached by
statement): constants nothing rewrites, whose comparisons are syntactic.
Same shape as `compiled_eq` and the pointer-width fact.

## Failed benchmark targets (2026-10-05)

A fresh full run on `wrwg/lean4` verified 23/32 problems. Two map problems
crashed before verification: the current `simple_map` source binds
`map_spec_insertion_key_at` and `map_spec_insertion_rank`, outside the old
62-role registry. They now use the existing position operations without
selecting the ordered discipline (64 roles). `InsertionMap.lean` checks
descending keys, ranks, and the implicit sequence-map validity obligation.

`ordered_map::test_verify_drain_symbolic` now has a proof companion using
`all_goals grind` on its prepared obligations. The isolated native run
verifies at 439,463,639 heartbeats versus the original automatic timeout
at 1,500,266,913 (71% less). No specification or budget was changed. The
same simple proof did not fix `ground_enum_123`, so it was not retained.
The full-module comparison confirms `drain` at 438,622,463 heartbeats,
with `ordered_map` falling from 13,374,095,060 to 12,336,281,295 (7.8%
less), and 16 failing targets instead of 17. Wall time fell from 606 s to
429 s, but other unchanged problems were also faster, so do not attribute
all of that wall-time change to the fix.

The isolation script previously left assertion-only functions running,
because it disabled only existing spec blocks. It now adds disabled specs
for those functions and omits unrelated authored proofs, including quoted
identifiers; a Python regression covers these cases. Check the result's
`targets` array when attributing a run. The invocation projection scanner
now visits shared expression subterms once across the goal and context,
using the existing order-preserving `sitesWhere` traversal. Both existing
performance gates pass without baseline changes.

Remaining failures include `behavior::add_two` (nested invocation facts),
`capability` (storage/abort implications), `pool_u64` (quantified map
invariants and a costly `buy_in`), and further ordered-map targets.
`type_info` needs concrete reflection semantics, while `ristretto255`
leaves native-specification equalities. The higher-order paper examples
still stop at frontend gaps: `amm` refers to a contract let in a body proof;
`calculator` carries a valued condition property that the printer treats
as a Boolean flag. These are distinct from proof-search timeouts.

The final full run verifies **24/32** problems (was 23/32), with no crashes:
`simple_map` verifies and `pool_u64` reaches its remaining proof failures.
The 23 problems verified in both runs have essentially unchanged aggregate
heartbeats (4.79G). Their wall time changed from 222 s to 193 s, illustrating
why the map heartbeat comparison is the attributable saving. All four Lake
builds and suites pass, including 113 Check fixtures, both performance gates,
and the new Python isolation regression. No expected-output baselines were
changed in this benchmark increment. `local_benchmark.html` is refreshed.


### Follow-up: benchmark sample proofs (2026-10-05)

The higher-order examples now reach verification. Valued clause attributes,
contract lets used in proof steps, grouping of post-only labels, lexical lets
around free label definitions, and captured function literals are supported.
The lexical-label regression also calls an opaque specification: it does not
rely on callee program points.

The AMM companion proves `constant_product`, `constant_product_with_fee`, and
`constant_product_with_fee_non_compliant`. The proofs establish division bounds,
fee-adjusted input bounds, overflow safety, and the reserve-product inequality.
The non-compliant pricing function's own contract is valid; storing it in a
pool remains an intentional invariant violation. Other pool obligations remain.

The `ristretto255` companion proves the three previously failing option-wrapped
scalar results by normalizing option payloads and using byte-vector codec
tightness. These are representation proofs over the existing native contracts.

The `capability` companion proves `acquire` and `acquire_linear`. A proved
normalization lemma reduces the partially applied address transport under vector
mapping, connecting search results to the quantified membership contract.
`delegate` and `revoke` still leave obligations; unsuccessful proof attempts for
them were removed.

Validation for these frontend and IR changes passed all four package suites,
including 114 Check fixtures and both performance gates without baseline changes.
The Move-to-Leaner state-label baseline was updated through its owning driver
for three parenthesization changes, then the full end-to-end suite passed.
Both Python isolation tests pass.

A proposed relaxation of `abortCases?` was tested against nested `result_of`
calls and discarded: it expanded proof search to the budget without fixing
`behavior::add_two`. The narrower existing suppression rule remains in place.

The completed follow-up full run verifies **25/32** problems (was 24/32),
with no crashes. `ristretto255` drops from 483M to 451M heartbeats (6.6%);
its reduced and uniform scalar constructors drop by about 49%, and inversion
by 19%. The two capability acquisition targets drop by about 21% each;
the module drops from 467M to 442M but still fails delegation/revocation.
The 24 problems verified in both runs use essentially unchanged aggregate
heartbeats (5.00G); wall time changes from 203.0 s to 198.5 s. AMM and
calculator now spend time on actual proof obligations rather than stopping
at frontend errors, so their increased costs are not comparable proof-search
regressions. All three authored AMM pricing proofs pass in the full run.
`local_benchmark.html` contains the completed report.


### Capability delegation and revocation (2026-10-05)

Both remaining targets now verify, and a native whole-module verification run
passes. The residual storage facts retained `Memory.set` reads across distinct
resource declarations: resolving generic types kept their handles distinct,
but the simplifier did not establish that premise of `Memory.set_other`.
The companion proves a direct read-after-write lemma using the existing
`handle?_resolve_struct` theorem, then discharges concrete handle inequalities.
`delegate` additionally normalizes `ofRuntime (toRuntime value)`. Normalizing
parameter transport for the generic vector helpers is needed by both targets;
without it, the integrated proofs still leave a leaf. No Move code, contract,
verification budget, or shared verifier implementation changed in this step.

The benchmark driver confirms **capability verified, zero errors** in a targeted
rerun and refreshes `local_benchmark.html`. Relative to the preceding full run:

| Measurement | Before | After | Change |
| --- | ---: | ---: | ---: |
| Module heartbeats | 441,834,760 | 307,590,516 | -30.4% |
| `delegate` heartbeats | 154,271,003 | 59,293,737 | -61.6% |
| `revoke` heartbeats | 116,344,784 | 74,068,608 | -36.3% |

Wall time increased from 17.9 s to 26.4 s despite the heartbeat reduction;
this run does not establish a wall-time improvement. This is a capability-only
rerun, not another full 32-problem run. The preceding full run remains 25/32;
capability is now additionally verified, leaving six failing samples from that
run (`type_info`, `pool_u64`, `ordered_map`, `behavior`, `amm`, `calculator`).


### Ordered-map proof search (2026-10-05)

The regular full run after the capability fix verifies **26/32** samples.
`ordered_map` still consumes 12,335,645,764 heartbeats; elapsed-time variation
is not the performance criterion. Four targets (`ground_enum_123`,
`test_verify_remove_or_none`, `test_verify_enumeration_view`, and
`test_verify_pop_rank`) each exhaust about 1.5G heartbeats. Together with
lower-bound rank/gap and mutable iteration, the seven most expensive failing
targets account for roughly 9.5G heartbeats.

Profiling `ground_enum_123` with an authored proof identified `omega` in
`leaner_denote_decide_residual`: a leaf costs roughly 9M to prepare and 508M
in that decider; the next decider consumes the remaining budget. The prepared
map-position facts contain conditional ordering relations. These automatic
attempts now use the existing 20M speculative-attempt bound, leaving the
residual for the authored proof. The individual residual deciders have timing
labels, so their costs are visible in `leaner.denoteProfile`/`denoteDebug`.
The target's overall budget is unchanged.

The companion adds proofs for the literal duplicate-key constructor, front/back
key borrowing, and `ground_enum_123`. Literal construction is simplified and
vector membership is established with concrete witnesses. No Move source or
specification changed. All four Lake suites pass, including 114 Check fixtures
and both performance gates without baseline changes.

Intermediate `stageLog` theorem reports do not establish success: the marker
is written before the verifier checks recorded errors, and rendering the
large residual diagnostics can continue afterward. The simple `grind` attempts for the
lower-bound targets were not retained; the rank attempt failed after its
intermediate stage report. Use completed process results and the native
benchmark for validation and final heartbeat comparisons.

The completed native module benchmark confirms the retained changes:

| Target / scope | Before heartbeats | After heartbeats | Change |
| --- | ---: | ---: | ---: |
| Entire `ordered_map` | 12,335,645,764 | 11,042,938,271 | -10.5% |
| `ground_enum_123` | 1,500,093,336 (timeout) | 263,678,562 | -82.4% |
| `test_verify_borrow_front_key` | 89,311,691 | 41,793,692 | -53.2% |
| `test_verify_borrow_back_key` | 75,892,790 | 42,339,481 | -44.2% |
| `test_aborts_if_new_from_1` | 10,512,861 | 8,689,423 | -17.3% |

The module remains failed; its error count falls from 75 to 63. The remaining
three 1.5G timeouts are `test_verify_remove_or_none`,
`test_verify_enumeration_view`, and `test_verify_pop_rank`. The symbolic
lower-bound rank/gap and mutable-walk targets still cost roughly 1.185G,
1.078G, and 1.252G. These dominate the remaining work. In the enumeration
caller, `leaner_denote_map_positions` itself times out during simplification;
a proved literal bulk-constructor rule alone did not fix it. Iterator-payload
and parameter-transport normalization alone did not close the lower-bound
proof either. These experiments were not retained. The benchmark driver
regenerated `local_benchmark.html` from this module rerun.

A separate native batch containing exactly the four newly fixed targets
finishes with `verified`, zero errors. This confirms their proof status
independently of the full module report, which truncates its error list.


### Lower-bound gap normalization (2026-10-05)

`test_verify_lower_bound_gap_symbolic` is a symbolic consequence of the opaque
lower-bound contract: for an absent key, a returned position has a strictly
larger key and a strictly smaller predecessor; End means all keys are smaller.
The End obligation already closed. The other two were blocked by the callee's
encoded enum-field read surviving as a separate arithmetic atom from the
caller's integer index. In the residual, omega could assign a negative value
to the encoded read while respecting nonnegativity of the actual u64 index.

Two normalization omissions caused this. `variantPayload_inl/inr` were not in
`lir_denote_norm`, and `canonicalFamilies` handled carriers and codecs but not
`NTy.encode` or `HList.encode`. A concrete row encoded under a generic callee's
instantiated family did not match the row-encoding simp rules when its value
was spelled in the caller's family. Canonicalize these encodings by definitional
replacement and register the existing payload reduction theorems. No new axioms,
Move edits, contract changes, or authored proof for this target are needed.

The native isolated run `/tmp/gap-native.json` verifies with zero errors:
**37,756,214 target heartbeats**, versus **1,077,776,124** in the preceding
whole-module run (96.5% fewer). This is target proof cost, excluding module
loading and certification. `GenericEnumPayload` is a small regression: a generic
opaque callee returns a concrete enum position and the caller derives
`index + 1 <= length` from its contract's `index < length`. It fails without
the payload normalization and passes with the fix.

Validation: all four Lake suites pass, including 115 Check fixtures and both
performance gates without baseline updates. The final isolated automatic run
also exits successfully against the built library, without local attributes
or a companion proof.

The completed whole-module rerun `/tmp/perf-ordered-gap-fixed.json` records
37,703,146 heartbeats for the gap target. The module's total falls from
11,042,938,271 to 8,378,929,045 heartbeats (24.1%); errors fall from 63 to 43.
The module still fails other targets. The benchmark driver recorded the run
and regenerated `local_benchmark.html` from its results.


### Ground remove-or-none proof (2026-10-05)

After the lower-bound fix, `test_verify_remove_or_none` was the most expensive
ordered-map target at 1,501,030,575 heartbeats (timeout). No suites were running
when the user requested stopping tests; no broad suites were launched for this
increment.

The model retained `updateAll` for the literal three-key constructor. The
following membership, deletion, and size obligations accumulated around that
expression; an authored-proof preparation alone spent about 764M heartbeats
on residuals. Unrestricted unfolding of the constructor did not solve it and
still timed out in map-position simplification.

The companion now proves the concrete constructor equation for keys 1, 2, 3
with arbitrary values, plus size and recursive membership laws for constructed
maps. Registering these proven equations for normalization makes the initial
facts computational before map-position reasoning. The remove-or-none proof
then closes by simplification of the model's deletion, membership, and size
operations. No Move code, contracts, core verifier, or heartbeat limits changed.

The independent native run `/tmp/remove-verified.json` is **verified, zero
errors**, at **124,901,888 target heartbeats** (91.7% fewer than the timeout).
The initial successful proof without early size/membership normalization cost
about 306M; the early rules reduce the map-position stage from about 242M to
71M. Validation is deliberately the focused native proof and the affected
module benchmark, following the user's benchmark-first/periodic-suite policy.

The completed module benchmark `/tmp/perf-ordered-remove-fixed.json` confirms
**123,966,728 heartbeats** for `test_verify_remove_or_none`. Total ordered-map
heartbeats fall from **8,378,929,045 to 6,900,704,697** (17.6%); errors fall
from 43 to 23. There are no new failing targets; `test_verify_upsert` also
stops failing with the computational lemmas. Six targets remain failed:
`test_aborts_if_new_from_2`, `test_verify_enumeration_view`,
`test_verify_iter_collect_symbolic`, `test_verify_next_key`,
`test_verify_pop_rank`, and `test_verify_prev_key`. The driver records the data
and regenerates `local_benchmark.html`. Wall time increased in this run;
heartbeat counts are the performance result, not a claimed wall-time speedup.

### Ordered-map verification complete and full suites (2026-10-05)

The native whole-module run `/tmp/perf-ordered-scoped.json` verifies **all 29
targets with zero errors**. Total heartbeats decrease from **6,900,704,697 to
4,125,866,249** (40.2%); verification accounts for 3,661,351,695. The normal
benchmark driver recorded the data and regenerated `local_benchmark.html`.
The remaining six failures from the preceding run are all closed.

Three automation fixes matter. First, map-position preparation substitutes
proven literal constructor equations into concrete map reads before generating
symbolic order facts. Quantified context is excluded from that substitution to
avoid expanding large hypotheses. Second, normalization canonicalizes opaque
map encodings exposed late in a proof across caller/callee families. This rule
is restricted to enumeration-backed map layouts and variable values: applying
it indiscriminately to control-flow enums disrupts iterator pattern matching.
Third, recursive loop hypotheses are matched after definitionally normalizing
abort-continuation obligation markers. Invariant premise markers remain intact
so failures retain their original source-clause diagnostics; unsuccessful
matching attempts restore the original goal.

The companion supplies a duplicate-index characterization of non-distinct keys
for `new_from_2`, computational next/previous-key proofs, and constructor/read/
removal equations for literal maps. Ground enumeration and pop-rank proofs use
explicit finite witnesses for specification membership. No Move source or
specification changed, and no axioms, admissions, or budget increases were used.
The earlier specification errors were unestablished proof obligations, not
counterexamples to those specifications.

Selected final target heartbeats:

| Target | Heartbeats | Previous result |
|---|---:|---|
| `test_aborts_if_new_from_2` | 21,350,722 | Failed |
| `test_verify_next_key` | 47,165,957 | Failed |
| `test_verify_prev_key` | 38,776,989 | Failed |
| `test_verify_enumeration_view` | 99,573,282 | 1.500G timeout |
| `test_verify_pop_rank` | 215,334,745 | 1.500G timeout |
| `test_verify_iter_collect_symbolic` | 182,138,877 | Failed |
| `test_verify_remove_or_none` | 63,673,408 | 123,966,728 |

The largest remaining costs are mutable iterator walk (943,189,244), drain
(438,953,085), and insertion in the middle (283,712,677). Walk companion
experiments with restricted `grind only` did not establish the specification;
they were discarded. Its passing automatic proof remains unchanged.

At the user's request, all four full Lake suites were run. The first end-to-end
run caught two diagnostic regressions (`LoopInvariantErrors` and `vault_errors`)
from overly broad obligation-marker normalization. Restricting it to abort
continuations fixed the diagnostics without updating baselines. After the final
module benchmark and report generation, **all four full suites passed**:
`leaner-ir`, `leaner-move`, `leaner-rust`, and `leaner-e2e-tests`, including 115
Check fixtures, source verification, MonoVM, differential tests, and both
performance gates. Logs are `/tmp/ordered-final-<package>-test.log`. This is a
complete Leaner test-matrix run, not a new aggregate run of all 32 benchmark
samples or all Move Prover registry tests.

### AMM and calculator proof failures (2026-10-05)

The native scoped run `/tmp/amm-calculator-final.json` verifies calculator
with zero errors and all six valid AMM targets. AMM's deliberately
non-compliant constructor still fails: it calls pricing that can abort when
the fee resource is absent, contradicting the pool's all-state no-abort law.
The benchmark continues to report AMM as failed; no expected-failure override
or specification weakening hides the negative example. Its automatic search
still reaches the 1.5G heartbeat limit, so fast rejection remains open.

The failures exposed missing propagation of behavior assumptions from data
invariants and missing default frames for stored function fields. Move
function fields now carry the empty modification frame, checked at packing,
mutation, return, and storage boundaries. The closer recovers it from stored
invariants before invoking a removed resource's closure. `EncodedKeepsMemory`
and its encoding equations are kernel-checked; the same declaration predicate
controls compiler checkpoints and generated contracts. Explicit field writes
(`modifies_of`) remain unsupported and are rejected rather than silently
receiving the default empty frame.

Calculator also read `old(State[addr])` inside the state selected after
`move_from`: Leaner's partial memory has no value there. Its specification
now binds the entry-state continuation before selecting that label. The
companion proof normalizes enum payloads, closure masks, and stored invariants.
No implementation behavior changes. State labels remain expressions over
memory, usable in opaque contracts at callers without program points.

AMM's companion proves bounded integer division and monotonicity of rounded
pricing. Literal-closure contract dispatch first identifies its family, so
the existing pricing theorems apply without expanding the pricing body at
each constructor. Arithmetic names retained by saved continuations keep
their defining equations; unused continuation lets are removed before
authored simplification. A bare `open` in the companion had prematurely
ended the module and hidden its helper lemmas; scoped `open … in` fixes it.

Scoped native target heartbeats:

| Target | Heartbeats | Result |
|---|---:|---|
| AMM `constant_product` | 26,758,185 | Verified |
| AMM `constant_product_with_fee` | 335,759,272 | Verified |
| AMM `constant_product_with_fee_non_compliant` | 614,769,503 | Verified under its own contract |
| AMM `create_constant_product_pool` | 82,260,048 | Verified |
| AMM `swap` | 157,495,677 | Verified |
| AMM `create_compliant_fee_pool` | 118,055,055 | Verified |
| AMM `create_noncompliant_fee_pool` | 1,500,124,400 | Rejected; search times out |
| Calculator `process` | 1,160,168,202 | Verified |

Calculator totals 1,242,895,584 heartbeats across all eight targets. This
is more than its earlier failed run (971M), so it is a correctness result,
not a claimed heartbeat reduction. AMM totals 2,865,860,388. New Check
fixtures cover behavioral invariants and stored closures, including rejection
of packing and field replacement with a memory-writing closure. The complete
benchmark refresh and broad-suite validation follow the scoped run.

The final full 32-problem run
`/tmp/leaner-benchmark-amm-calculator-complete.json` confirms **28/32 verified**,
up from 27 before this increment. Calculator totals **1,242,871,952**
heartbeats with zero errors; AMM totals **2,865,859,391**, with only its
intentional negative constructor failed. Aptos Framework `ordered_map`
retains all 29 passing targets and zero errors at **4,120,422,247**. No
previously verified problem regresses to failure. The generated HTML uses
this complete run against the `main` CI baseline (`ea4ecc43e7`, run
37250691414), excluding intermediate local runs. This report was regenerated
before starting the four full Lake suites.

Validation complete: all four package builds and full Lake suites pass
(`leaner-ir`, `leaner-move`, `leaner-rust`, `leaner-e2e-tests`), including
118 Check fixtures, source verification, MonoVM/differential checks, and both
performance gates. No existing baseline was regenerated. The new
`StoredFrameErrors.exp` was generated through the owning baseline helper and
reviewed for its four intended diagnostics. Logs are
`/tmp/amm-calculator-<package>-test.log`; aggregate exit statuses are in
`/tmp/amm-calculator-full-tests.log`.

### Suite-total investigation (2026-10-06)

The suite chart previously summed only completely verified problems
(`suite_value` in `scripts/leaner-bench.py`). Its rise did not measure a fixed workload. The
latest cached main run, 37398209356 (`b30c5bae4e`, October 6), has 23 passing
problems and 4,791,718,157 counted heartbeats. The validated local run
`/tmp/leaner-benchmark-behavior-guard.json` has 28 and 10,944,358,307.
Five newly passing problems contribute 6,178,478,596: framework ordered_map
4,145,490,348; calculator 1,072,283,339; ristretto255 448,278,510; capability
303,178,134; simple_map 209,248,265. The 23 problems passing in both runs
instead decrease from 4,791,718,157 to 4,765,879,711 (0.54%). Summing all
available measurements, including failed attempts, decreases from
19,366,783,385 to 17,108,476,376; main has two problems without heartbeat
measurements, versus none locally, so this is not a fixed successful workload.

The local comparison table uses an older merge-base run (37250691414,
`ea4ecc43e7`, October 5), whereas the chart also includes the newer main run.
The earlier diagnosis of an original-nine-target OrderedMap regression
(260M to 316M) compared against that older run, not the latest main result.
The newer main already measures all 17 OrderedMap targets. Its module total
is 1,437,697,990 versus 1,445,785,449 locally (+0.56%). There are small real
increases: add +3,871,799 and borrow +1,978,317; lower_bound_loop decreases
366,407. These do not explain the suite-total jump.

Focused reproduction against the older main source was built separately at
`/tmp/ordered-main-baseline`, with only final closer-summary logging enabled
in the scratch copy to avoid goal-printing overhead. No production verifier
source was modified. Inputs `/tmp/ordered-{main,regression}-*.lean` and logs
`/tmp/ordered-regression-{main-clean,current,parent}-*.log` show:

- empty: older closer 235k, two leaves; current 2,148k, three leaves, including
  a new 1,581k construction-invariant step. The parent already has that step.
- lower_bound_loop: older closer 35,695k versus current 54,536k, both 27
  leaves. Call processing rises 6,202k to 8,949k and residual processing
  22,668k to 27,237k. The parent is already 54,647k, so the large increase
  predates the latest fix batch; it is not extra scenario target count alone.
- borrow: older closer 24,719k with 13 leaves, parent 28,652k with 12,
  current 34,175k with 14. The smaller remaining regression includes more
  proof branches and residual processing; an exact attribution needs further
  profiling before changing automation.

The eight scenarios existed as interpreter tests in the older source. Current
verification also checks their map data invariants at construction and mutation;
`handlesDataInvariants` enrolls functions without explicit postconditions.
They add 911,560,604 target heartbeats relative to the older nine-target run.
Keep coverage totals separate from comparisons of the same measured targets.

### AMM expected rejection (2026-10-06)

The benchmark's `create_noncompliant_fee_pool` is an intentional negative.
Its captured pricing callback reads `Fee[owner]` unconditionally, whereas
the Pool invariant requires no abort in every state. A state without that
resource violates the invariant. MVP rejects the exact paper example too:
`/tmp/amm-example-mvp-negative.log`. Its registry test passes by matching
expected verification diagnostics (`/tmp/amm-mvp-baseline-check.log`), rather
than by proving the constructor valid.

The closer visits the constant-product invariant before the no-abort
invariant. General rewriting and the nominally cheap arithmetic solver
could each consume the entire target budget on this invalid universal claim.
Recursive case splitting then repeated those failed attempts. The automatic
leaf's cheap, prepared, pipeline, and case alternatives now use the existing
20M speculative budget when the goal or context mentions function values;
case branches share their enclosing attempt's budget.
Manual residual preparation and the target budget remain unchanged.

An unconditional limit regressed `acl::remove` in a regular benchmark attempt.
That experiment was stopped before publishing a full run, and the limits were
restricted to function-valued leaves. The scoped rerun restores ACL verification.
`type_info::verify_type_of` retains its four pre-existing diagnostics; comparison
with the checkpoint confirms that it was not a regression of this experiment.
Ordinary vector and reflection solvers retain their original budgets.

Once an authored clause has failed, additional leaves with the same clause
and provenance reuse that rejection. Other clauses and other origins still
receive their own diagnostics. This uses the existing failed-obligation
path; its logged errors still prevent publishing the target's verification
artifacts. Successful targets never reuse a rejection.

An isolated run importing the previously verified pricing prerequisites
finishes with the same four invariant diagnostics and final verification
failure, without a timeout. Preparation is approximately 184.972M raw
heartbeats, versus the original 1.5G timeout; five repeated leaves avoid
another 242.9M of search. Evidence: `/tmp/amm-negative-cached.log`; the bounded
solver without rejection reuse cost 427.879M (`/tmp/amm-negative-bounded-3.log`).
These are diagnostic profiles; the native benchmark measures the complete
module independently.

The benchmark records per-target outcomes and diagnostic counts. AMM's
manifest declares this constructor as an expected rejection; unrelated
errors, missing targets, timeouts, and acceptance of the negative remain
visible problems. Suite totals and main comparisons include all available
measurements, including rejected attempts. The default local report contains
the latest measurement against main, with intermediate runs retained only
for explicit history comparisons.

The fresh regular native benchmark (`/tmp/leaner-benchmark-amm-final.json`)
records the constructor at 185,036,096 heartbeats (87.7% lower), with all six
positive AMM targets still verified. AMM's total falls from 2,862,902,197 to
1,547,801,426. Results are 28 verified, one expected rejection, two unresolved
rejections, and one timeout, with no newly failed problems or missing targets.
All measured suite work is 15,822,941,895 heartbeats, versus 17,108,476,376 at
the checkpoint. The same 28 verified problems rise 0.22%, from 10,944,358,307
to 10,968,397,511; keep that small overhead separate from the rejected target's
search reduction. The script regenerates `local_benchmark.json` and HTML
before tests. The full core suite passes, including both cost gates; Behavior,
InvariantBehavior, eight Python outcome/report tests, and four native outcome
classification checks pass. Native tests confirm a timeout or unrelated error
cannot masquerade as an expected rejection, and acceptance of a declared
negative is flagged. Logs: `/tmp/amm-final-{core-tests,native-outcome-tests,
invariant-behavior-check}.log` and `/tmp/amm-function-budget-behavior-check.log`.


## Pool scalar summaries and shareholder coverage (2026-10-06)

`pool_u64::buy_in` computes shares before changing the pool totals and calling
`add_shares`. The call summaries name those scalar values with carrier-typed
results. Several facts reuse the binder name `consumed`; a rewrite selected by
name can therefore use a different fact. The explicit proof rewrites every
scalar equation by its hypothesis identity, after clearing unused computation
and callee contracts, then closes the arithmetic with `omega` or `grind -ring
only`. Expanding map definitions adds no information to that arithmetic.

`deduct_shares` ignores the Boolean from `index_of`. Both search branches
remain in the proof. The existing invariants say that the shareholder vector
covers the map, has the same cardinality, and contains no duplicates; these
jointly imply that every present map key occurs in the vector. A proved
finite-cardinality lemma rules out the missing-search branch. A second lemma
preserves coverage when the matching vector element and map key are removed,
including the intermediate write that ends the mutable borrow. All premises
are discharged from the leaf; the helper deliberately fails without the
cardinality premise. The proofs live in `LeanerIR/Proofs/Denote/MapCoverage.lean`
and the pool's adjacent `pool_u64.proof.lean` companion.

Adding these strategies to automatic search regressed `option::from_vec`,
`simple_map::add_all`, and bit-vector cost. Those automatic hooks were removed.
The companion invokes the strategies only for the two pool targets. Native
regression checks restore option/simple-map verification and bit-vector cost
(377M rather than 723M). The scoped native run
`/tmp/leaner-benchmark-pool-scoped.json` verifies all 22 pool targets, with no
errors or timeout, under the unchanged regular budget. `buy_in` costs
854,176,924 heartbeats versus its previous 1,500,719,364 timeout;
`deduct_shares` costs 763,747,941 versus its previous 763,008,081 rejection.
The newly attempted `redeem_shares` and `transfer_shares` cost 632,904,030 and
307,066,539. Pool total work rises from 3,050,102,018 to 3,392,927,076 because
these two formerly skipped proofs now run. Preparation profiles (618M buy-in,
310M deduct) exclude the authored script and subsequent theorem checking;
use the complete native target measurements for comparisons.


The subsequent fresh full native run, `/tmp/leaner-benchmark-pool-final.json`,
records 29 verified modules, one expected rejection, two existing failures
(`type_info`, `behavior`), and no timeout. All measured work is
16,166,103,625 heartbeats, versus 15,822,941,895 before the pool follow-up.
Pool verifies 22/22 targets at 3,393,032,305 total. Its final target measurements
are 854,318,280 (`buy_in`), 763,749,746 (`deduct_shares`), 632,868,911
(`redeem_shares`) and 307,052,384 (`transfer_shares`). The script regenerated
the full local JSON and HTML against main before the core suite started.

Validation at this batch boundary passes: the full `leaner-ir` core suite
(135 jobs, including both performance gates and scalar/coverage regressions),
eight Python report/outcome tests, and the focused native pool/option/
simple-map/bit-vector benchmark. Core log: `/tmp/pool-final-core-tests.log`.
No broad registry/parity work was resumed.


## Nested invocation abort alternatives (2026-10-06)

`behavior::add_two` calls opaque `apply_twice(increment, x)`. Its successful
result and no-overflow branch already verified, but the abort branch retained
`aborts_of<increment>(x) || aborts_of<increment>(result_of<increment>(x))`
inside the callee clause's `Obligation` marker. The behavior dispatcher needs
an individual abort fact; the generic disjunction splitter sees only bare
`Or`. Meanwhile, `abortCases?` suppresses an extra split when the first abort
predicate occurs anywhere in the context, including inside that disjunction.

The closer now exposes a marked disjunction whose alternatives are all
literal-closure abort predicates, then revisits each branch through the
ordinary invocation rules. On the second alternative, it splits the first
invocation's abort predicate: either that invocation overflows, or termination
supplies its successful run and result. The source marker is definitionally
its proposition; only this context fact is exposed for the split.

That alone did not close the successful-first/aborted-second branch. The
rewritten second argument remains `(packResults #[integer result.val]).asInt`.
`terminatingRun` had a local decoder for that expression, but
`dispatchBehavior` and `denotedRun` did not. The decoder now lives in
`nativeOf?`, shared by all three, so it recovers the existing certified carrier
rather than asking `omega` for fresh bounds on the packed projection.
The earlier broad relaxation of `abortCases?` remains discarded; its
suppression rule is unchanged.

Scratch validation verifies all 20 benchmark targets. A core regression proves
the opaque two-increment caller with a 50M raw target budget, and a guarded
negative still rejects the incorrect `aborts_if x + 1 > MAX_U64` clause. At
`x = MAX_U64 - 1`, the first invocation succeeds and the second aborts; that
failure must be accounted for. Logs: `/tmp/behavior-fixed-{probe3,
regressions3}.log`. The rebuilt native executable verifies 20/20 behavior targets with zero
errors. `add_two` costs 18,943,903 raw heartbeats, versus 16,654,950 for its
previous failed attempt. Module work is 125,588,922 versus 122,971,215.
The script regenerated benchmark data and HTML against main before broader
testing. Data: `/tmp/leaner-benchmark-behavior-fixed.json`.


The fresh full regular benchmark, `/tmp/leaner-benchmark-behavior-final.json`,
verifies 30/32 modules, with AMM's intended negative classified as expected
rejection and only the existing `type_info` failure unresolved. There are no
timeouts or newly failed problems. All measured work is 16,168,784,777 raw
heartbeats, versus 16,166,103,625 in the previous pool full run (+0.017%).
Behavior verifies 20/20 targets at 125,589,180 total; `add_two` costs 18,944,093.
The complete local data and HTML were regenerated against main before the
full core suite started. Native tools were rebuilt before both measurements.

Validation passes: the full `leaner-ir` core suite (135 jobs, including both
cost gates and the positive/negative nested-behavior checks), plus eight
Python benchmark outcome/report tests. Core log:
`/tmp/behavior-final-core-tests.log`. No broader registry/parity goal was resumed.

### Suspended vector normalization experiment (2026-10-06)

The official dependency export at `/tmp/bp-pure-callee-official-export` reproduces
`bp_pure_callee::remove_all_found`'s 25M timeout. A diagnostic 100M budget permits
an automatic proof at 52.221M raw heartbeats (24 leaves, 24.912M leaf cost).
Kernel-proved normalization removes redundant representability checks from swap
and remove; map/swap and map/erase identities preserve generic carrier transport,
and the search's default index is in bounds whenever the vector is nonempty.
Together these produce a 22.698M automatic proof at the original 25M budget.

This is **not installed**: focused vector checks reject valid `remove_middle`,
`swap_remove_value` and generic callers after the remove rewrite. Residuals retain
`wp (Spec.pure ...)`; a diagnostic `simp only [wp_pure]` reports an expression
that is not type-correct at instances transparency. Replacing the embedded index
proof or restoring the optional element match did not resolve those failures.
The swap-only version retains the passing generic checks but exceeds the 25M
budget for this target. No acceptance budget or specification changed.

Sources: `/tmp/registry-vector-operations-checkpoint-experiment.lean` and
`/tmp/registry-vector-generic-checkpoint-experiment.lean`. Logs:
`/tmp/registry-resume-remove-profile.log`, `/tmp/registry-remove-official-25k.log`,
`/tmp/registry-vector-existing-check.log`, `/tmp/registry-vector-generic-residual.log`,
`/tmp/registry-vector-generic-manual.log`. Resume with the simplifier/continuation
processing of the pure tuple result; require both the official 25M runner and
existing vector checks to pass before installing.

### Resumed vector normalization (2026-10-06)

The suspended experiment's generic regression had two causes: a partially
expanded tuple carrier inferred in the replacement denotation, and dependent
conditional branches whose continuation remained beneath an unprocessed proof
binder. The remove rule now states the canonical carrier explicitly. The
structural split introduces dependent branch assumptions before normalization;
when ordinary splitting fails, preparation requeues a changed computation.
This fallback runs only after the ordinary split attempt, preserving the fast
path (unconditional preparation exceeded the 25M cap again).

The official `bp_pure_callee` runner now accepts `remove_all_found` at the
unchanged 25k budget. The final production profile records 22.751M raw heartbeats versus
52.221M for the old automatic proof at a diagnostic 100k budget, a 56.4% reduction.
The existing VectorOperations check and new generic caller/interpreter check
pass. Logs: `/tmp/resume-fallback-mvp.log` and
`/tmp/resume-vector-{build,mvp-update,generic-check,existing-check}.log`.
The full four-package suites and all 437 registry baseline refreshes pass.
The full registry audit adds no failed targets; `verify_vector` reports more
specific residuals but the same failed functions. Logs:
`/tmp/vector-resumed-<package>-{build,test}.log`,
`/tmp/vector-resumed-prover-refresh.log`, and
`/tmp/vector-resumed-registry-audit.{json,log}`.

The first full benchmark exposed authored proofs tied to the old optional
swap reads and removal result. Quicksort now proves lookup/count laws for
`swapIfInBounds`; OrderedMap rewrites the removed entry by its earlier lookup.
The native follow-up verifies both (`/tmp/leaner-benchmark-vector-examples.json`):
Quicksort is 245M versus 265M at the checkpoint; OrderedMap remains about 1.44G.
The complete final benchmark `/tmp/leaner-benchmark-vector-final.json` records
30 verified problems and the expected AMM rejection, with no unexpected failures
or timeouts. Its 15,845,571,410 total heartbeats are 1.18% below the checkpoint
on the same 31 problems (16,035,112,646). The owning script regenerated the full
local JSON/HTML against main after the authored proof repairs.

Performance follow-ups: the initial full run reduces pool `deduct_shares`
from 764M to 560M and capability `revoke` from 71M to 54M, but increases
simple-map `add_all` from 159M to 183M, features `apply_diff` from 206M to
223M, and the ground VectorOperations `swap_remove_value` from 36.6M to
64.5M. Its isolated stage profile spends 29.7M in prepared `grind`.
Scratch-only normalization with `Array.getElem_swapIfInBounds` and
`List.getElem_toArray` reduces that isolated proof from 62.3M to 45.1M;
these attributes are not installed or validated across symbolic proofs.
Scratch sources/logs: `/tmp/vector-swap-remove-profile.lean`,
`/tmp/vector-swap-remove-normalized-literal.{lean,log}`.

### Registry residual budgets and vector companions (2026-10-06)

An authored proof still paid for repeated failed residual attempts before its
script ran. With a 25M target limit, each attempt could previously spend 20M.
Residual attempts now use at most 5% of the target's budget, capped at the
existing 20M allowance. The overall target budget and ordinary solver ceiling
are unchanged. A fixed small cap regressed larger authored examples and was
discarded; the proportional cap preserves their larger search allowance.

`verify_vector.proof.lean` proves the generic swap-last/erase-last equation
and instantiates it for the direct and library-call swap-remove tests. Bounded
reads normalize through `Array.map`, preserving generic carrier transport.
The model-call proof drops from 28.419M at a diagnostic higher limit to
21.313M at the official 25M limit. Both scoped ordinary baseline checks pass.
The smaller residual allowance exposes one more `count_all` invariant leaf;
its companion proves it by separating earlier indices from the current read.
Its ordinary baseline check still reports only the two existing warnings.

A further companion instantiates the no-match prefix invariant for
`verify_index_of`, closing its exit and preservation goals in 22.520M, below
the unchanged 25M limit. The official scoped refresh accepts it. Logs:
`/tmp/vector-residual-{bp,verify-vector}-check.log`,
`/tmp/vector-index-mvp-update.log`,
`/tmp/verify-model-swap-remove-production.log`, and
`/tmp/verify-index-profile.log`. The complete benchmark
`/tmp/leaner-benchmark-vector-residual.json` verifies 30 problems with one
expected AMM rejection and no unexpected failures/timeouts. Total cost is
15,846,440,343 raw heartbeats (+0.0055% against the first vector run). The
script regenerated local JSON/HTML against main before broad validation.
The full 437-file registry audit adds no failed targets, and all four full
suites pass, including both cost gates and the 123 Check fixtures. Logs:
`/tmp/vector-residual-<package>-{build,test}.log`,
`/tmp/vector-residual-prover-refresh.log`, and
`/tmp/vector-residual-registry-audit.{json,log}`.

The next `verify_reverse` diagnostic still exhausts 25M while preparing
residuals, before an authored script can run (`/tmp/verify-reverse-residual.log`).
At a diagnostic 100M limit it leaves four proof goals after 37.672M, with
20.171M in residual processing and 9.259M in loop processing
(`/tmp/verify-reverse-diagnostic.log`). No higher acceptance limit is installed.
It needs that preparation cost reduced before adding its proof script.

### Opaque inline declarations (2026-10-06)

The frontend omitted `.inlineRetained` functions even when opaque calls and
behavioral predicates still named them. Retaining opaque declarations fixes
the three C12 positive-gap fixtures. Their bodies remain verification targets;
`verify = false` retains only the existing explicit trusted-contract behavior.
The negative `inc` body is rejected, and its caller cannot use an unproved
callee theorem. `behavioral_predicate_inline_fun` is clean.

The loop-sum companion proves nonnegativity of consecutive products, the
triangular-number recurrence, and a product bound for the u64 addition. Its
body costs 13.196M at the official 25M limit. The two-call caller needs the
consecutive product's evenness and costs 5.093M. All positive targets prove;
the wrong doubled-result postcondition still fails. Logs:
`/tmp/opaque-inline-*.move-check.log`, `/tmp/opaque-sum-profile.log`, and
`/tmp/opaque-twice-profile.log`. The fresh full benchmark
`/tmp/leaner-benchmark-opaque-inline.json` is complete: 30 verified and one
expected rejection, no unexpected failures/timeouts, 15,846,650,541 raw
heartbeats. The generated local report compares with main. The Move package
tests pass after that report refresh. All 437 registry baseline checks pass;
only the three intended opaque-inline files change, with no regression in
previously verified targets. Audit: `/tmp/opaque-inline-registry-audit.{json,log}`.

### Literal swap reads (2026-10-06)

The ground `swap_remove_value` proof regressed after certified vector
normalization because its bounded reads still contained `swapIfInBounds`.
Expanding that operation's read everywhere lowers the ground proof's cost,
but pushes `verify_model_swap_remove` past its 25M limit. A simplification
procedure now applies the standard array lookup theorem only to arrays backed
by explicit list constructors. Symbolic arrays keep their existing form.
`List.getElem_toArray` also exposes reads of the resulting literal vector.

The production isolated ground proof costs 45.087M raw heartbeats versus
62.296M previously (−27.6%). The generic scratch proof costs 21.365M at the
unchanged 25M limit. Both official `verify_vector` and `bp_pure_callee` ordinary
baseline checks pass. Logs: `/tmp/literal-swap-ground-production.log`,
`/tmp/literal-swap-vector-check.log`, `/tmp/literal-swap-bp-check.log`.
Native benchmark and CLI builds pass; the full benchmark
`/tmp/leaner-benchmark-literal-swap.json` is complete: 30 verified problems,
one expected AMM rejection, no unexpected failures/timeouts, and
15,825,019,985 raw heartbeats. The owning driver regenerated the full local
JSON/HTML against main before broad tests. All four full package suites pass,
including both cost gates and the 123 Check fixtures. All 437 registry baseline checks pass, with no baseline changes. Logs:
`/tmp/literal-swap-<package>-{build,test}.log`,
`/tmp/literal-swap-prover-refresh.log`, and
`/tmp/literal-swap-registry-audit.{json,log}`.

In the complete benchmark, `VectorOperations` falls from 272,547,867 to
252,424,096 heartbeats (−7.4%). Its `swap_remove_value` target falls from
64,554,891 to 47,189,204 (−26.9%). Suite cost is down 21,630,556 heartbeats (−0.14%)
against the preceding opaque-inline run.

Reverse remains an open proof-automation problem: neither broad lookup
normalization, earlier quantified instantiation, nor an explicit swap result
carrier removes its preparation timeout. Broader rewrites increased its
branch count. Tighter speculative budgets exposed extra companion obligations
without getting reverse below 25M, and were removed. The validated five-percent
per-attempt budget remains. Scratch experiments live under `/tmp/reverse-*`;
none of those experimental budget, carrier or broad-read changes is installed.


### Table snapshot contract integration (2026-10-06)

The full benchmark `/tmp/leaner-benchmark-snapshot-routing.json` retains 30
verified samples and one expected AMM rejection, with no unexpected failures
or timeouts. It measures 15,807,887,568 raw heartbeats, +450,576 (+0.0029%) from
`/tmp/leaner-benchmark-table-contents-final.json`. Framework ordered_map verifies
all 29 targets at 4,158,157,741 heartbeats; pool_u64 verifies all 22 at
3,189,643,146. Calculator verifies all eight at 1,066,759,897. The benchmark
generator refreshed `local_benchmark.json` and main-relative HTML before broad
suite validation. The local JSON is byte-identical to the measured artifact.
The new Table source proofs and six focused compatibility checks pass. All four
full suites pass, including 125 Check fixtures and both cost gates. After reviewing
and regenerating seven changed Table baselines, all 437 registry checks pass
(`/tmp/snapshot-routing-full-tests.log`, `/tmp/snapshot-routing-prover-recheck.log`).

### Table shared-read contracts (2026-10-06)

The full benchmark `/tmp/leaner-benchmark-table-reads.json` retains 30 verified
problems and the expected AMM rejection, with no unexpected failures/timeouts.
It uses 15,808,054,447 raw heartbeats, +166,879 (+0.0011%) from snapshot-routing.
Framework `ordered_map` verifies 29/29 at 4,158,162,148; `pool_u64` verifies 22/22
at 3,189,774,048; calculator verifies 8/8 at 1,066,777,655. The separate LeanerLang
`OrderedMap` sample uses 1,438,458,701. Local JSON is byte-identical to the artifact,
and the driver generated HTML relative to main CI 37250691414 (`ea4ecc43e7`).
Full suites and the registry check were started only after that report refresh.
All four suites pass, including 127 Check fixtures and both cost gates, as do all
437 registry baseline checks (`/tmp/table-reads-full-tests.log`,
`/tmp/table-reads-prover-check.log`). The full diagnostic audit is unchanged.

The newly enabled Table membership/shared-lookup contracts pass five focused
source proofs and reject a false lookup claim. The registry's `table_option`
advances from a missing native model to a 25,000-maxHeartbeats proof timeout.
Scratch normalization reduces two residual goals to the stored Option vector's
`size ≤ 1`, with no such invariant premise. Deep invariants of external Table
values and aggregate observation normalization must be supplied; a larger search
budget would not address the missing invariant. Diagnostic artifacts are recorded
in `intrinsic-maps.md` under shared Table read contracts.

### Table aggregate observation normalization (2026-10-06)

The full benchmark `/tmp/leaner-benchmark-table-projections.json` retains 30
verified problems and one expected AMM rejection, with no unexpected failures.
Its 15,807,844,181 raw heartbeats are 210,266 lower (−0.0013%) than shared reads.
Framework `ordered_map` verifies 29/29 at 4,157,996,222; `pool_u64` verifies 22/22
at 3,189,706,932; calculator verifies 8/8 at 1,066,798,157. Local JSON is identical
to the measured artifact, and the driver regenerated HTML against main CI
37250691414 (`ea4ecc43e7`) before the cost gates and scoped registry checks.

Known aggregate observations now normalize without splitting unknown snapshots.
List/array traversal normalization proves equality and length of a nested-vector
payload read through a Table contract. Six positive source proofs, two intended
rejections, the existing snapshot checks, and both cost gates pass. Six scoped
registry baselines match after the owning runner refreshed `table_option`.
It still times out at its unchanged 25000 budget. Source diagnostic probes using
the verifier's default budget expose one residual postcondition and two missing
Option invariants; they are not evidence of a fix at the registry budget.
The last complete broad-suite checkpoint remains the preceding shared-read run.

### Vector index ranges (2026-10-06)

`/tmp/leaner-benchmark-vector-ranges.json` retains all target outcomes: 30
verified problems and one expected AMM rejection, no unexpected failures or
timeouts. Total raw heartbeats are 15,808,427,132, up 582,951 (+0.0037%) from
the preceding local run. The driver refreshed identical local JSON and generated
HTML against main CI 37250691414 before both cost gates passed
(`/tmp/vector-ranges-postcheck.log`). No broad suites were rerun for this increment.

The new `VectorRanges` Check proves five positives at 25k and rejects a false
upper-endpoint existential; the snapshot-range Check also passes. The scoped
`macro_verification` baseline update and ordinary recheck pass. Its `foreach`
now reaches proof search, but even a positive-only diagnostic times out at
25k. The debug/profile probes show roughly 7.6M raw heartbeats for loop setup,
then exhaustion while normalizing the unchanged-tail invariant after a mutable
element write, before the authored tactic runs. No timed `p-clear` stage is
reported for that leaf; inspect the preceding `leaner_denote_unname` and
carrier normalization rather than assuming the arithmetic solver is responsible.
Probes: `/tmp/VectorRangesForeach{Positive,Debug,Profile}.lean` and
`/tmp/vector-ranges-foreach-{positive,debug,profile}.log`.

### Certified reads and continuation aliases (2026-10-06)

Unused saved-continuation aliases kept their entire definition chain alive
through leaf simplification. `leaner_denote_clear_computations` now recognizes
these aliases in context order and tries to clear them in reverse, retaining
every definition still needed by a hypothesis or goal. `ContinuationCleanup`
checks both removal and preservation.

A proved normalization rewrites optional encoded integer reads to a certified
integer's value, with certified zero for a missing entry. Bound collection now
also recognizes projections of computed certified integers. `CertifiedReads`
checks signed and unsigned bounds, missing entries, negative values, and an
intended rejection at 25k. Both cost gates pass without baseline changes.
The ordered-map duplicate-key companion normalizes its helper fact to this
same read representation; its isolated target passes.

The same instrumented positive-only `foreach` diagnostic decreases from
48.483M to 36.190M raw heartbeats (25.4%). These are diagnostic runs at 100k,
not acceptance-budget results. The uninstrumented 25k positive probe still
times out. Its remaining profile spends 18.359M in fourteen leaves, including
6.789M in preparation and 5.917M in written-value deciders. The counters are
nested and must not be summed as independent costs. Profile artifacts:
`/tmp/foreach-auto-diagnostic.log`, `/tmp/certified-read-foreach-profile.log`;
focused checks: `/tmp/certified-read-focused.log`.

The full native benchmark `/tmp/leaner-benchmark-certified-reads-final.json`
retains all target outcomes: 30 verified problems and one expected AMM rejection.
It measures 15,727,043,391 raw heartbeats, down 81,383,741 (0.5148%) from vector
ranges. Comparator falls 21.0% to 120,907,579, bit_vector 3.2% to 364,856,646,
and features 1.6% to 766,838,269. Framework ordered_map verifies 29/29 at
4,141,160,834; pool_u64 verifies 22/22 at 3,193,150,321 (+0.1030% locally).
The local JSON is identical to the measured artifact. HTML compares against
main CI 37250691414, and its updated roster omits the retired type_info entry.
The outcome comparison and report freshness checks pass before broad validation
starts (`/tmp/certified-read-validation.log`).

All four full suites pass, including both cost gates and 129 Check fixtures
(`/tmp/certified-read-full-tests.log`). The full registry check matches 432/437
baselines. Its one proof regression is a companion still naming a leaf that
automation now closes; `bp_pure_callee::count_all` passes again after removing
that obsolete case, against its unchanged baseline. Four reviewed diagnostic
baselines were regenerated and pass ordinary rechecks. Their audit finds no
added failed target: `folds_of_callee_ensures::count_small` newly verifies at
25k; `find_value` rejects normally rather than timing out; the remaining
changes are residual normalization or an additional clause diagnostic for an
already failing target. Logs: `/tmp/certified-read-pure-callee-recheck.log`,
`/tmp/certified-read-registry-refresh.log`, and
`/tmp/certified-read-registry-audit.json`. The full 437-file run plus five
focused ordinary rechecks cover this final state; no second full run is claimed.

### Vector element quantifiers (2026-10-06)

Normalizing the mapped runtime values of an element quantifier exposes the
native element predicate. Position instantiation now retains the element's
universal binder and dependent read equation rather than rejecting the binder.
`ElementQuantifiers` passes four positive proofs and two intended rejections at
25k; certified reads, vector ranges, Table snapshots and both cost gates also
pass (`/tmp/element-quantifiers-focused.log`). The ordinary positive-only
`foreach` probe still times out at 25k.

The full native benchmark `/tmp/leaner-benchmark-element-quantifiers.json`
preserves all target outcomes: 30 verified problems and one expected AMM
rejection. Its 15,834,183,512 raw heartbeats are +107,140,121 (+0.6813%) over the
certified-read checkpoint. The report is generated against main, omits type_info,
and the local JSON matches the measured artifact. Scoped registry checks follow
that refresh under `/tmp/element-quantifiers-after-benchmark.py`.

A scratch specialization of `List.forall_mem_map` to runtime-value predicates
retains all six new checks and also proves a generic-vector equality read at
25k. In isolated native comparisons it reduces bit_vector's shift target from
252,685,657 to 248,810,251 heartbeats and features' next-epoch target from
40,974,012 to 40,297,016 (`/tmp/element-{shift,features}-compare.log`). The
specialization is now installed, with a fifth generic source proof. All focused
checks and both cost gates pass (`/tmp/element-final-focused.log`). The scoped
registry checks exposed one real authored-proof regression in
`invariants_with_quant`: its literal-element clause now leaves an extra goal.
The companion explicitly enumerates its three positions, simplifies the reads,
and passes against the unchanged baseline (`/tmp/element-final-literal-vector-recheck.log`).
The final full benchmark `/tmp/leaner-benchmark-element-quantifiers-final.json`
measures 15,744,282,692 raw heartbeats: 89,900,820 below the wider rule, and
17,239,301 (+0.1096%) above certified reads. All target outcomes remain unchanged
(30 verified problems and one expected rejection). Framework ordered_map costs
4,145,558,609 and pool_u64 3,196,518,381. Local data and main-relative HTML were
refreshed before broad validation; the generated report excludes type_info.
All four full suites pass with both cost gates and 130 Check fixtures.
The full registry check matches 436/437 baselines; the sole difference is a
newly verified `moved_local_in_loop` at 25k. Its obsolete timeout baseline was
removed by the owning runner and an ordinary focused recheck passes. Logs:
`/tmp/element-final-full-tests.log`, `/tmp/element-final-registry-check.log`,
`/tmp/element-final-moved-local-{refresh,recheck}.log`.

Further positive-only foreach profiling (2026-10-07) does not close the 25k
gap. Quiet automatic profiling at a diagnostic 100k measures 33.353M; the
current manual decider measures 32.647M. Clearing computations before canonical
families slightly worsens the automatic result to 33.466M. Clearing before
unname reduces the manual result to 31.577M, but its ordinary 25k run still
fails. Neither scratch change was installed. These are nested diagnostic
counters, not acceptance results. Logs: `/tmp/foreach-quiet-profile.log`,
`/tmp/foreach-current-manual-profile.log`,
`/tmp/foreach-early-cleanup-profile.log`, and
`/tmp/foreach-manual-early-{profile,25k}.log`.


## Table stored-value invariants (2026-10-07)

Generated contracts now combine global-resource predicates with deep invariants
of external collection entries, retaining generic and phantom type arguments.
The closer obtains the invariant from raw-key membership at the same snapshot
memory. This avoids reconstructing a native key for an arithmetic expression.
Both Table handle layouts have codec-to-observation bridges. Native integer
round trips and vector round trips take priority over decoder expansion, so
already-certified values do not create duplicate bound checks. Snapshot identity
comparisons receive a focused closing attempt without globally unfolding unknown
snapshots or losing their selected memory.

The nine positive source proofs and two false-claim rejections in
`TableStoredInvariants.lean` pass at 25k. `StoredTableInvariantErrors` rejects a
stored Table-equality invariant whose meaning a broad physical-input shortcut
would change: only a bare vector local's length may bypass observation. The
axiom audit of eight bridge/core laws contains only standard Lean axioms.

The native registry runner still matches `table_option`'s timeout baseline.
A diagnostic generated-source run closes its proof at approximately 25.99M raw
heartbeats, above the unchanged 25k acceptance budget; it is not a registry pass.
That diagnostic predates the final length-only restriction. Its main remaining
costs are call normalization (6.9M), stored-read facts (2.35M across four uses),
and two general leaf pipelines (2.96M). Logs are
`/tmp/table-option-Production-profile.log` and
`/tmp/table-invariant-native-registry-final.log`. Experimental broad stored-fact
simplification and broader snapshot dispatch increased cost and were not
installed. A map-specific integer encoder normalization saved about 0.63M in a
diagnostic but remained over budget and was also left out.

The full benchmark, run before broad suites, preserves every outcome:
30 verified problems and one expected AMM rejection, at **15,725,534,459** raw
heartbeats. This is −18,748,233 (−0.1191%) from the element-quantifier checkpoint.
Capability improves 283,915,114 → 237,395,590; framework ordered_map changes
4,145,558,609 → 4,157,454,627; pool_u64 changes
3,196,518,381 → 3,201,625,758. No target outcome changes. Local JSON is identical
to `/tmp/leaner-benchmark-table-invariants.json`, and the generator refreshed HTML
against main with retired type_info absent. All four full suites pass, including
both cost gates and 131 Check fixtures (`/tmp/table-invariants-full-tests.log`).
The full registry matches 435/437 baselines. Its two differences are fewer
diagnostics, not new verified targets: `different_addr_global` finishes with an
ordinary verification failure, and `verify_remove_with_unroll` still times out.
A new diagnostic profile after the length-only safety restriction reproduces
25,986k raw heartbeats for `table_option` (`/tmp/table-option-current-profile.log`).


## Snapshot equality follow-up (2026-10-07)

The focused snapshot-identity closer now uses `lir_denote_norm` and the
context equalities directly. It no longer unfolds the broader `lir_denote`
and `lir_denote_eval` rule sets or runs a separate scalar-equality pass.
The installed generated-source diagnostic falls from **25,986k to 24,867k**
raw heartbeats in `closeGoals` (`/tmp/table-option-normalization-profile.log`).
This is **not the full theorem cost**: the surrounding theorem elaboration
costs **29,139,877 raw heartbeats**, with 22,076 distinct proof objects
(`/tmp/table-option-measured-total.log`). The separate transport theorem costs
465,869 raw heartbeats.
The unchanged 25k registry limit still times out, and its ordinary baseline
check matches (`/tmp/table-option-normalization-registry.log`). Do not infer
acceptance from a closer-only profile below 25M.

The nine positive and two negative Table invariant checks still pass, as does
the identity-invariant rejection guard. Logs:
`/tmp/table-option-normalization-stored-invariants.log` and
`/tmp/table-option-normalization-identity-guard.log`. The focused `TableSnapshots` and `TableReads` source checks also pass
(`/tmp/snapshot-equality-TableSnapshots.log`,
`/tmp/snapshot-equality-TableReads.log`). The complete package-suite
checkpoint immediately precedes this two-line closer change; it has not been
rerun after it. The regular benchmark has completed (`/tmp/snapshot-equality-benchmark.log`):
30 verified plus one expected AMM rejection, **15,725,421,251** raw heartbeats,
a local decrease of 113,208 (−0.00072%). Outcomes are unchanged. Local JSON is
byte-identical to `/tmp/leaner-benchmark-snapshot-equality.json`; the generator
refreshed HTML against main with type_info absent. Both focused cost gates pass after that refresh:
`/tmp/snapshot-equality-DenotePerformance.log` and
`/tmp/snapshot-equality-CompositionPerformance.log`.

Instrumentation found only about 5k raw heartbeats each for lookup theorem
argument unification and closed registration/layout decisions, versus about
390k for normalization of each Table invariant fact. Replacing the telescope
with `mkAppM` did not help. Broad early derivation of stored facts increased
later context-normalization work (about 35.88M closer-only in the scratch
probe), so it was not installed. A map-specific encoder normalization saves
about 0.41M with the narrower snapshot closer, but still fails at 25k and was
not installed. A Lean heartbeat trace (`/tmp/table-option-theorem-profile.log`) separates
about 1.26M in the initial tactic setup/normalization from about 2.58M in
kernel checking of `typedVerified`. Tracing adds overhead: its entire theorem
is about 32.72M, versus 29.14M without tracing. Do not use the traced total as
an acceptance measurement. Targeted alias-fact normalization and early
conjunction splitting did not improve the installed implementation; those
scratch probes are also not installed. The next optimization must reduce
proof construction/checking as well as leaf search.


## Call-range eligibility (2026-10-07)

The call-result case splitter now checks for a possible small literal range
before copying and normalizing continuation hypotheses. The check preserves
metavariables and transported views, and collects bounds across expressions
so equalities connecting locals do not exclude a candidate. It only controls
whether to try an existing proof procedure; it adds no semantic assumption.
Eight `CallRangeResults` targets verify at 25k, covering opaque, generic, mutable,
strict-bound and large-literal results. A simpler scratch guard saved more but
did not retain the same conservative eligibility; its saving is not the installed
result.

Installed `table_option` whole-theorem cost is **28,686,998 raw heartbeats**,
down from 29,139,877, with unchanged 22,076 proof objects. Closer-only cost is
24,405k; transport costs 465,938. The normal registry check still matches its
25k timeout baseline. Logs: `/tmp/call-range-guard-table-total.log` and
`/tmp/call-range-guard-table-option-registry.log`.

The full regular benchmark completes before broad tests: **30 verified plus
one expected AMM rejection**, **15,624,166,021** raw heartbeats, a decrease of
101,255,230 (0.6439%). The measured artifact is
`/tmp/leaner-benchmark-call-range-guard.json`; local JSON matches and generated
HTML compares with main. All four builds and full suites pass; all 437 registry baselines match
(`/tmp/call-range-checkpoint-full-tests.log`). No diagnostic baseline refresh
was needed.

Scratch computed-index investigation: opaque bounds 10 <= result <= 11 suffice
for a two-element vector read at result - 10, but automatic verification fails
under both old and new call-range logic. Residual context introduces a separate
certified integer `named` and the equality `named.val = result.val - 10`. Range
detection sees the bounded result and index as unrelated. A descendant scan
inside `Int.toNat` alone is insufficient before scalar-equality rewriting;
it does close the prepared manual proof. No fix for this gap is installed.

A subsequent scratch scalar-equality rewrite before range detection proves
all eight computed-index targets and rejects four deliberately false
postconditions (`/tmp/call-range-computed-scalar{,-negative}.log`). Applying it
unconditionally costs 30,915,264 whole-theorem heartbeats for table_option in
the scratch environment, versus 29,800,403 without the rewrite. Restricting it
to a nonempty literal-bound candidate set costs 29,810,087. These scratch
figures include elaborator aliases/overrides and are not comparable directly
to the smaller native installed total. A dependency scan without rewriting
still fails all three computed-index callers. The guarded rewrite plus
computed-position recognition is the promising next implementation; keep the
unchanged proof budget and the false-postcondition guards.


The computed-index fix is installed after the call-range full-suite checkpoint.
The existing scalar-equality tactic was moved before the range tactic and is
invoked only with literal-bound candidates. Range position detection now looks
inside the `Int.toNat` argument for the bounded scalar. Native build, both
cost gates, 16 positive fixture targets and two guarded rejections pass at
25k. Logs: `/tmp/computed-index-{build,regression,DenotePerformance,CompositionPerformance}.log`.
Installed table_option costs 28,689,764 typed-theorem heartbeats (only +2,766),
with unchanged 22,076 objects and 465,938 transport heartbeats. Its ordinary
native registry check matches the timeout baseline. The subsequent full
benchmark is pending; the four-full-suite result above predates this increment.


## Table normalization experiments (2026-10-07)

The computed-index benchmark completes with 30 verified problems and one
expected AMM rejection at **15,646,803,818** raw heartbeats (+22,637,797,
+0.1449%). The driver refreshed main-relative HTML; local JSON matches
`/tmp/leaner-benchmark-computed-index.json`. Focused registry checks for
table_option and verify_vector match their unchanged baselines.

Further isolated Table experiments, all at a diagnostic budget:
- Normalizing an invariant schema over a fresh raw variable before instantiation
  increases the whole theorem to 30,767,376 heartbeats; not installed.
- Context-only call-range eligibility preserves all 16 positive proofs and two
  guarded rejections in scratch. It avoids transports from the future program
  that cannot supply bounds on the current call result.
- Exact lookup-equality transport plus early physical-field and vector-length
  rewriting reduces leaf pipeline work. Primitive integer-vector invariants
  are tautologies under DataInvariant.value and can be eliminated before
  traversing the stored vector expression.
- The combined scratch proof costs 27,929,337 heartbeats with 21,212 objects,
  including substantial scratch declaration/override overhead. This is not
  an acceptance measurement. The corresponding native batch is now building
  (`/tmp/table-normalization-build.log`); its real budget check is pending.
No unused-contract clearing or weakened transport eligibility is installed.


The first native normalization batch reduces table_option's whole typed theorem
to 24,531,222 heartbeats and 21,212 proof objects. The ordinary registry runner
reports only deletion of the obsolete timeout output: the target verifies at
25k. However, reversing the physical-field normal form for arbitrary snapshots
regresses generic `count` (both layouts) and phantom-typed `tagged`. No baseline
was refreshed for this intermediate state.

Restoring the original unknown-snapshot rule and adding a pre-rule only for a
known aggregate restores all TableStoredInvariants cases. Scratch table_option
also passes with both maxHeartbeats and leaner.verifyHeartbeats set to 25000
(`/tmp/table-option-aggregate-field-budget.log`), despite its recorded whole
sample being 25,479,304 raw heartbeats: that sample includes work outside the
local proof-budget scope. Use the actual unchanged-budget native check for
acceptance, not an inferred cutoff on the broader diagnostic sample. The
corrected native build is `/tmp/table-normalization-final-build.log`; checks
and the next benchmark are pending. Generic observation-field bridge lemmas
alone did not fix these regressions and are not installed.


The corrected native batch passes: table_option verifies at the unchanged
25k setting; the owning runner removes its obsolete baseline and the ordinary
recheck passes (`/tmp/table-option-fixed-registry-recheck.log`). Whole typed
cost is **25,254,204 raw heartbeats** (−3,435,560, −11.9749%), with **21,500 proof
objects** (−576), and 465,850 transport heartbeats. Closer cost is 21,019k;
leaf pipeline work falls to 920k. The aggregate-only pre-rule retains the
old generic observation form and fixes the initial three regressions.

Both cost gates, 16 call-range positives/two negatives, nine Table invariant
positives/two negatives, the Table identity guard, TableReads, TableSnapshots,
and kernel snapshot/invariant tests pass. Logs have prefix
`/tmp/table-normalization-final-`. The new lemmas' audit lists only propext and
Quot.sound. The full regular benchmark and periodic suites are the next checks;
none is claimed for this batch yet.


The first normalization benchmark reveals a guard regression: framework
`test_verify_upsert` rises 114,337,592 → 423,982,783 heartbeats,
`test_verify_remove_or_none` rises 63,126,801 → 268,615,937, and
`test_verify_iter_collect_symbolic` rises 180,692,547 → 223,094,867.
LeanerLang OrderedMap times out. The measured report records these outcomes;
the checkpoint assertion stops before the full suites.
An identical isolated upsert source costs 432,274,533 with the context-only
guard and 120,693,726 with the original guard (scratch override overhead is
included). The original conservative target-and-context guard is restored in
production. It is an eligibility heuristic, not a source of proof premises:
target transports must remain eligible so subsequent normalization can expose
facts already held in the context. The remaining Table optimization still
passes at the actual 25k setting in scratch with that guard restored
(`/tmp/table-option-restored-guard-budget.log`). Native rebuild, the ordinary
table_option check and both focused cost gates pass. The corrected regular
benchmark restores 30 verified problems plus one expected AMM rejection, with
no timeouts. Total: 15,646,929,683 raw heartbeats, +125,865 (+0.0008044%)
against the previous good computed-index run. Framework ordered_map returns
to 4,123,667,100; LeanerLang OrderedMap to 1,433,690,178.
`/tmp/leaner-benchmark-table-restored.json` matches local JSON and the generated
HTML compares against main. All four builds and full suites pass, and all 437 registry baselines match
without refresh (`/tmp/table-restored-full-tests.log`). Final Table measurement
with the restored guard: 26,045,418 typed raw heartbeats / 21,500 proof objects,
plus 465,826 transport heartbeats (`/tmp/table-restored-total.log`). The native
25k check passes; the broader measurement includes work outside that budget.


### Decoded state-label updates (2026-10-07)

The registry's two_increments timeout concealed ordinary residual failures at
200k. Non-aborting reads were destructured as `some value = memory resource key`,
which the read tactics do not consume, and decoder expansion preceded the
arithmetic facts needed to prove its range checks. Use the forward witness
lemma, expose known values without opening the codec, then discharge its range
conditions. An opaque caller additionally supplies isSome facts and successful
decoder equations: use the known decoder result before rewriting its input.
The final native target verifies at the unchanged 25k limit, and the owning
runner deletes the timeout baseline. All 30 state-label registry baselines
match. Typed cost: 18,269,904 raw heartbeats, 11,428 objects; transport 367,301.
The registered StateLabelDecoding fixture has three positive proofs, including
an opaque caller with no callee program points, and missing-resource/overflow
rejections. Both cost gates and both focused existing label fixtures pass.
The regular benchmark passes: 30 verified + one expected rejection, no timeouts,
15,647,129,560 raw heartbeats (+199,877, +0.0013% against restored-guard).
`/tmp/leaner-benchmark-state-label-decoded.json` matches local JSON; HTML is
regenerated against main with type_info absent. Full suites were last run at
the preceding restored-guard checkpoint.


### Constructive state-label witnesses (2026-10-07)

`spec_fun_old_param_labeled_with_memory::inc_under_cap_twice` used 30,912,290
raw typed heartbeats. Its three impossible branches already closed cheaply;
the actual success path spent 8.4M in witness search. The intermediate label
can use the initial memory and a counter value one greater than the input.
Trying every program point first repeatedly rejects environments before the
existing arithmetic witness builder constructs this value. Move program-point
search after the predicate/context candidates. All candidates still require
proof of the instantiated predicate; labels remain ordinary existentials and
do not require execution points at callers.

Native typed cost: 24,132,039 raw heartbeats, unchanged 15,523 objects;
transport 208,976. The original registry and new registered StateLabelWitnesses
fixture verify at 25k. The state-label registry directory passes 30/30 after
removing its old timeout baseline. Both cost gates pass, as does StateLabelDecoding;
StateLabels retains byte-identical intended-negative diagnostics. The regular
benchmark retains 30 verified + one expected rejection, no timeouts, at
15,647,293,919 raw heartbeats (+164,359, +0.00105%). Main-relative JSON/HTML
were refreshed before the broad checkpoint: all four builds/full suites and
437/437 registry baseline checks pass (handoff.md).
Scratch-only alternatives: contradictory-branch checks cost more; moving the
point search only after arithmetic synthesis gives 27.735M; the fully delayed
scratch version gives 24.670M (native production measurement above is authoritative).


### Reusing invocation facts (2026-10-07)

The closer reconstructed a chosen terminating run even when matching
EnsuresOf, ResultOf and StateOf facts were already available. It also split
on abort decisions under an encodeFor spelling despite a known decision at
the definitionally equal encode spelling. Reuse the complete three-fact run,
and recognize direct positive/negative abort decisions by reducible conversion.
No premises are assumed: these checks only skip redundant proof search.

The original aborts_if_at_state_label diagnostic now fails only its ensures
at 102.153M typed heartbeats (formerly 167.735M with multiple failed clauses).
Its native 25k timeout remains. All 30 labeled registry checks and both cost
gates pass. The regular benchmark completed with 30 verified + one expected
AMM rejection, no timeouts, at 15,644,746,679 raw heartbeats (-2,547,240).
Measured JSON and main-relative HTML are refreshed; the latest broad validation
still precedes these guards.
A scratch leaf tactic proves all clauses at 29.391M, but still times out at
25k, so it is not installed and the registry baseline is unchanged.


### Close existing call observations before deriving behavior (2026-10-07)

The call rule already supplies the typed result, abort decision, and contract.
Trying a bounded leaf proof before terminatingRuns/dispatchBehavior avoids
re-deriving those facts. Recognize both ResultOf and AbortsOf: excluding aborts
left a trivial impossible branch spending 5.4M in the ordinary cheap solver.
All 16 leaves of aborts_if_at_state_label::caller now close from existing call
observations, at 20,251,470 typed raw heartbeats / 14,523 objects (transport
396,238). The native 25k runner verifies and removes its timeout baseline.

The speculative attempt is limited to 3M raw heartbeats and restored on failure.
It adds unsigned/certified bounds but omits generated natAbs facts; ordinary
assertBounds defaults to full facts, including signed magnitudes. Normalize
single returned values with packResults_single; preserve tdiv rather than
unnecessarily converting to ediv. No contract premises are introduced.
StateLabelCallObservations covers the symbolic quotient, nested labeled result,
opaque caller, and a rejected false postcondition. Both cost gates and existing
decoding/witness fixtures pass. The regular benchmark retains 30 verified + one
expected rejection at 15,677,942,351 raw heartbeats (+33,195,672, +0.2122%).
Measured JSON and main-relative HTML refreshed before broad suites; those suites
are running (handoff.md).


### Shared Boolean observations and direct-call eligibility (2026-10-07)

The preceding full checkpoint passes all four suites, but the registry reveals
one constructor regression: amm_example::create_pool pays for the new call
fast path because its quantified PricingStrategy invariant mentions AbortsOf.
Require a direct ResultOf equality or direct positive/negative AbortsOf fact
instead. The native AMM baseline now matches without refreshing it.

intermediate_states::test_config_preserved leaves four implications: for each
Config.active outcome, both results equal the same value. A bounded attempt
finds multiple distinct conditional equalities for each polarity, prepares the
leaf to identify unchanged memories, then splits the shared Bool and simp_all.
The native 25k proof uses 8,793,754 typed raw heartbeats, 6,552 objects, transport
366,506. Both intended negative siblings retain their clause diagnostics.
StateLabelBooleanObservations covers direct conditional opaque reads and the
incorrect claim that an unconditional reader always returns the same value.
Both cost gates and all focused label fixtures pass. The benchmark preserves
30 verified + one expected rejection at 15,676,741,907 raw heartbeats; the full
registry recheck matches 437/437. The four suites passed just before this follow-up.


### Updated memories and conditional frame equations (2026-10-07)

`aliasing::different_addr_global` compares two successive writes. The stored
structure's opaque empty HList tail remains a local on one side and Unit.unit
on the other; reflexivity recognizes their definitional equality. A bounded
3M raw-heartbeat attempt prepares equalities whose two sides are Memory.set,
then simplifies, tries reflexivity, and uses congruence for residual reads.
The native target now verifies at 25k, with 19,832,667 typed raw heartbeats,
13,887 proof objects, and 216,817 transport heartbeats. The owning runner
removes the whole aliasing failure baseline.

An opaque-caller regression additionally exposed a cyclic conditional frame:
the post-memory equals writes whose final value reads that same post-memory.
The generic self-reference filter previously recognized only direct equations.
It now inspects equations under binders; rejected rewrite rules remain proof
facts. The decoded-read closer can expose presence witnesses and use encoded
value observations without expanding the frame. This branch retains the
existing attempt budget. The caller verifies at 25k, 11,577,355 typed raw
heartbeats, 8,649 objects, and 298,173 transport heartbeats. A wrong labeled
update is still rejected. Both cost gates, all five label fixtures, and the
two conditional-cycle regressions pass. The regular benchmark preserves 30
verified + one expected rejection at 15,696,214,192 raw heartbeats (+0.1242%).
Main-relative HTML is regenerated. All four full suites and all 437 registry
baseline checks subsequently pass; see handoff.md and `/tmp/aliasing-observations-*`.

## Optional integer reads and sum folds (2026-10-07)

The literal certified-read normalizer now also handles optional reads,
preserving signed values and the actual fallback. Its certificate uses
`val_getD_getElem?_map_val`; `Option.bind_fun_some` is normalized as well.
Registered `LiteralOptionalReads` covers unsigned/signed nonzero fallbacks
and a symbolic stored value. The native `vector_hofs_fold` runner now verifies
`sum_concrete`, `sum_inferred`, and `sum_scaled` at the unchanged 25k budget.
Their companion unfolds recursive specifications, rewrites the goal using
context equations, then decides finite index cases. Whole typed artifacts
cost 27,897,980 / 27,332,778 / 27,979,895 raw heartbeats respectively; those
include work outside the native verification-budget scope. No residual-search
or acceptance-budget change is installed. The owning runner removed exactly
these three obsolete timeout sections; product and even-count still time out.
Core/Move builds, both cost gates, optional-read, call-range, and labeled-memory
fixtures pass. The full benchmark preserves 30 verified + one expected rejection at
15,688,482,526 raw heartbeats (-0.0493% versus aliasing). Measured JSON and
main-relative HTML are refreshed, without type_info. All 437 registry
baselines match without refresh (`/tmp/fold-optional-checkpoint.log`, exit 0). All four full suites passed
at the preceding aliasing checkpoint; they have not been rerun for this increment.

## Product-fold range-bound follow-up (2026-10-07)

`vector_hofs_fold::product_concrete` now verifies at native 25k. The range
instance solver tries congruence (`grind only`) at the new endpoint, so an
accumulator equality also transports a multiplication bound. Previous
normalization fallbacks remain. ArithmeticContext includes a positive case
and rejects the same inference without either the current bound or the
accumulator equality. The original solver fails the positive regression.
The installed whole typed proof costs **23,960,945 raw heartbeats**, 22,596
objects, transport 282,210. Native refresh removes only the product timeout;
`count_even_concrete` is the sole remaining failure in this fold module.
Core/Move builds and the focused checks pass. The first broad registry check
exposed two regressions; restricting endpoint congruence to symbolic products
restores their original behavior. The subsequent pool checkpoint passes all four
full suites and all 437 registry checks without refresh, retaining this product
improvement (`/tmp/pool-final-checkpoint.log`).

## Conditional range instances in the even-count fold (2026-10-07)

The range solver now has a fallback for a quantified invariant whose conditional
has already been split in the goal. It specializes at the goal's integer
positions and simplifies only those new instances using known guards. Range
premises are retained. At the new endpoint, known guards also simplify the
current iteration's bound. The original range path is tried first.
`ArithmeticContext` covers both branches and rejects missing branch evidence
and missing endpoint bounds. Its focused build passes
(`/tmp/conditional-range-build.log`, 116 jobs).

This is a partial fix for `vector_hofs_fold::count_even_concrete`, not a new
registry success. On the same isolated authored proof at a diagnostic 100k
budget, closer work falls from 39.076M to 34.551M raw heartbeats; residual work
falls from 24.137M to 19.610M and normalized leaves from eight to five. The
remaining proof still exhausts that diagnostic budget. No acceptance budget
or companion is changed. `/tmp/count-even-conditional-installed.log` records
the installed result.

Two scratch experiments identify the remaining arithmetic work. Normalizing
truncating remainder over a nonnegative literal list, together with the
conditional-range prototype, proves the target in 48,803,217 typed raw
heartbeats (`/tmp/count-even-conditional.log`). This is still above native
25k and the remainder normalizer is not installed. An explicit lemma giving
the six-element prefix count as `n / 2` also proves the target, but costs
78,885,053 typed raw heartbeats (`/tmp/count-even-prefix-cases.log`), so that
companion is rejected. The full benchmark/report and cost/regression checks
for the installed range change pass
(`/tmp/conditional-range-checkpoint.log`, exit 0). The regular benchmark retains
30 verified samples plus one expected AMM rejection at 15,066,137,470 raw
heartbeats (+0.0328%); JSON and main-relative HTML are regenerated. Both cost
gates and all three focused registry baselines match without refresh.
All four full builds/suites and all 437 ordinary registry checks subsequently
pass without baseline refresh (`/tmp/conditional-range-full-tests.log`, exit 0).

With the installed endpoint handling, the scratch nonnegative-remainder
normalizer alone proves the fold in 44,958,953 typed raw heartbeats
(`/tmp/count-even-nonnegative-installed.log`). Two further experiments lose:
evaluating closed recursive applications before residual preparation costs
51,018,577 (`/tmp/count-even-ground-evaluation.log`), and extending unused
computation cleanup to `Comp` aliases costs 47,967,311
(`/tmp/count-even-clear-comp.log`). None of these scratch changes is installed.

## Pool performance attribution (2026-10-07)

The current regular measurement (`/tmp/leaner-benchmark-fold-product.work/pool_u64.json`)
verifies all 22 functions at 3,185,090,652 total raw heartbeats: verification
2,929,430,533 (92%), certification 225,676,539. The largest targets are buy_in
847,944,044; redeem_shares 629,034,382; deduct_shares 571,115,483; transfer_shares
297,478,947; add_shares 251,910,369. These five consume 88.7% of verification work.
They are symbolic production proofs, not ground-data tests. Pool's data invariants
relate the shareholders vector to the shares map: coverage, equal cardinality,
uniqueness and the shareholder limit.

A fresh read-only profile (`/tmp/pool-investigation-profile.{json,log}`) confirms
that buy_in spends 600.596M in the closer before its companion takes over,
535.908M in residual processing across 81 leaves. Failed residual-context
attempts cost 130.454M; failed cheap attempts 86.977M. The companion already
clears unused callee summaries and rewrites scalar equations, but only after
this expensive preparation. redeem_shares has 98 leaves: 245.883M preparing
45 leaves, including 167.615M renormalization, plus 138.614M successful w-omega
work. These step figures overlap with their enclosing stages and must not
be added to those stages. deduct_shares spends 281.430M in the closer before
its companion, including 228.851M residual processing across 70 leaves; its
full target is 571.115M. Attribution of the remainder to individual companion
steps needs separate instrumentation. No pool optimization is installed yet.
The next measured optimization should target buy_in's repeated failed
pre-companion search and context growth, rather than assuming its simple
arithmetic or map runtime representation inherently needs this much proof work.

## Bounded residual search follow-up (2026-10-07)

The first pool optimization reduces the absolute ceiling of speculative
residual attempts from 20M to 2M raw heartbeats, retaining the five-percent
limit. Native 25k verification therefore keeps its existing 1.25M per-attempt
allowance. The target acceptance budget and all specifications are unchanged.
The installed solver preserves ordinary tactic failure and fallback behavior.
On the same isolated buy_in rendering, typed proof work falls from 852,580,897
to 727,871,678 raw heartbeats (14.6%). Closer work falls from 613.109M to
472.295M; its failed context attempts fall from 131.629M to 21.261M.
Proof objects change from 109,439 to 109,532. Core/Move builds, ArithmeticContext
and both cost gates pass. The full benchmark and all-package/registry
results are recorded below. Focused logs:
`/tmp/pool-residual-cap-*`, `/tmp/pool-context-checkpoint.log`.

Rejected scratch experiments: clearing callee contracts early did not reduce
cost, and moving scalar cleanup into automatic residual search spent more on
failed attempts than it saved. The initial scratch budget override saved more
(698.316M) but swallowed tactic failures, so that figure is not the production
result. The ordinary solver run above is the relevant comparison.

The first full run retained every pool outcome and reduced its native total
from 3,185,090,652 to 3,099,911,251 raw heartbeats. buy_in fell from 847,944,044
to 729,680,787, but deduct_shares rose from 571,115,483 to 604,298,240.
The companion now tries map coverage first; a same-rendering interpreted
comparison verifies at 460,613,609 versus 623,020,988 heartbeats, with fewer
proof objects (88,098 versus 91,972). The first full run also exposed an AMM
companion regression: it demanded fee ≤ 10000 on early abort branches whose
abort condition follows from fee > 10000. Discharging those disjunctions with
constructor simplification and omega before deriving return-path fee bounds
restores the isolated proof at 428,297,037 raw heartbeats. No contract was
changed. The first report records the regression honestly; broad validation
did not start. The corrected complete checkpoint below is recorded in
`/tmp/pool-final-checkpoint.log` (session 14303, terminal 0).

The corrected complete benchmark preserves 30 verified + one expected AMM
rejection at **15,061,197,787 total raw heartbeats**, down 4.03% from the preceding
15,694,278,677 checkpoint. Pool falls 7.68% to 2,940,369,701; native buy_in is
729,515,623 and deduct_shares 445,113,352. Ordered_map is 4,031,232,214 and AMM
1,293,879,404. `/tmp/leaner-benchmark-pool-final.json` matches local_benchmark.json;
main-relative HTML is regenerated with the global ranking first and no type_info.
All four package builds and full suites pass; all 437 registry baselines match
without refresh (146.59s). Both cost gates pass. The subsequent redeem_shares
manual-preparation probe is rejected: it verifies but costs 1,087,346,931 versus
642,092,999 isolated typed raw heartbeats. No redeem_shares companion is installed
(`/tmp/PoolRedeem{Current,Prepared}.lean`, `/tmp/pool-redeem-*.log`).

## Literal results and unrolled paths (2026-10-07)

Checked arithmetic names its result (`∀ named, named.val = e → …`). When `e`
normalizes to a literal, the name hid the value from the loop condition, so
every unrolled branch survived to a leaf: the ground 2×2 nested loop had 67
leaves and 64.5M closer heartbeats. The `namedLiteral` simproc substitutes a
fitting literal instead (its certificate is decided once); the same loop has one
leaf and 7.7M. Full benchmark −0.48% (VectorOperations −16.2%); cost-gate targets
unchanged.

Lean's `split` under the default `backward.split` simplifies with contextual
`ite` congruence, discharging every nested condition along both branches:
2^k attempts for a chain of k `if`s. One split of the 11-level inlined
`spec_pow_raw` takes about 8 s. `backward.split false` avoids the blow-up but
does not decide branches whose conditions an earlier hypothesis fixes; neither
is installed. `math_fixed8::pow_raw` also needs `n & 1` as `n % 2`, pinning of
`n` from omega-derived bounds, and many nonlinear overflow leaves, which is not
feasible at 25k.

## Domain guards of specification functions (2026-10-08)

A specification function whose parameter has a fixed-width type is defined
inside that domain only (G15): its recursive definition is
`dite bounds body outside`. The closer decides only an unfolding's first
guard, and keeps an instance it decides. With the domain on top, it decided
the bounds and kept instances whose body's own first condition (`i.val = 0`)
the context leaves open; a function without a domain drops those. Carried
into normalization, preparation and grind, they made
`folds_of::sum_direct` cost 33.5M closer heartbeats instead of 20.8M with
`Int` parameters, and timed out five registry targets and a lemma at 25k.

`unfoldSpecsOnce` now treats a domain guard (a `dite` binding `bounds`) as
transparent: it decides the bounds, then the body's first condition, and drops
the instance if that one is open. It decides guards with the bounds their
values' types carry (`typedBound?`: `SpecInt` ranges, `SpecVector` lengths),
which `assertBounds` now shares, and it reduces bundle projections of the
instance (`(a, b, ()).snd.fst` to `b`) when it creates it. `sum_direct` costs
23.2M (the `Int` variant 20.8M with the projection reduction, 22.4M before),
and the five targets and the lemma verify again at 25k.

## Bitwise operand order (2026-10-08)

`bitwise_features::contains` compares `v & m` from the code with `m & v`
from its specification, which `omega` reads as two atoms; the pipeline closed
the leaf only through `leaner_denote_bv`, about 20M heartbeats each for two
leaves. `bitwiseAnd_comm` (with `Or`/`Xor` variants) as `lir_denote_norm`
simp lemmas, which simp applies by its ordered rewriting, decides the leaf
after the first saturation round but doubled
`features::change_feature_flags_for_next_epoch` (76.4M to 147.1M) and cost
`features::set` 21%: rejected. `assertBounds` instead states `a & b = b & a`
for a conjunction a leaf mentions in both orders. The other leaf's cost was
`Int.shiftLeft 1 k % 256` beside the specification's
`(Int.shiftLeft 1 k).tmod 256`, which the normalizer could not equate before
the shift's bounds were asserted; `shiftLeft_tmod_of_nonneg` rewrites a
truncating remainder of a nonnegative shift to the runtime's. `contains`
costs 18.3M (57.0M before), and `features` 769.7M (854.6M).
