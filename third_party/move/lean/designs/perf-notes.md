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
   printed.
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
