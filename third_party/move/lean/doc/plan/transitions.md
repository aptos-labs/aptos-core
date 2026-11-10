# Transitions

How the designs in [`doc/designs/`](../designs/) came to be what they are. One
section per transition: the state the system was in, the alternative that was
rejected and why, the measurement that decided it, and what the transition left
open.

This file is the only place in the documentation that carries that narrative.
The design documents describe the current state and nothing else; the other
files in `doc/plan/` state open problems. A line reference inside a section
below belongs to the revision the section describes and is not expected to
resolve against the current source; names are kept because they are what the
narrative is about.

## Data invariants as carriers (2026-11)

A data invariant is part of the type of a struct or enum value. Before this
transition the verifier instead carried the data invariants of *stored* values
as a predicate over global memory, `DataInvariants`, assumed at the entry of any
function that reached a carrier of an invariant, owed at its exit, re-owed after
every write, and proved again at every join arrival. The design that replaced it
makes the invariant part of the value's carrier type, so that a read owes
nothing and assumes nothing and the obligation falls only where a value is made
or a mutable loan over one ends.

### The layer that was replaced

The survey below records the implementation the transition started from. Its
line references and its descriptions of the representation are historical.

- `Resources.lean:149`: `DataInvariants` quantified over all resources;
  `Contract.lean:3173-3177, 3253-3256, 3321-3323` assumed it at entry for a
  function reaching a carrier, owed it at exit (`keepsStoredInvariants`,
  `Contract.lean:2717`) and as an implication for any other function reaching
  memory.
- `Decide.lean:4329` (`verex_data_invariants`) walked the `Memory.set` chain and
  owed each written value its invariant again. It ran at leaves
  (`Solve.lean:2958`, `storedInvariantsStep`), at each join arrival
  (`Solve.lean:1036`, `proveStoredInvariants`, `invariantProofs` in
  `SharedContinuations.lean:148`) and in `exposeNormalEntries`
  (`Solve.lean:207`). `storedInvariantsProof?` (`Decide.lean:4030`) searched the
  context for a stated or implied `DataInvariants`; `verex_stored_reads`
  (`Decide.lean:4389`) derived a read value's invariant from it.
- `storedInvariantTerm` (`Contract.lean:1830`) translated each `key`
  declaration's invariant once per unit, over the executable where a clause read
  behavior (`storedInvariantReadsUnit`, `Contract/Values.lean:1882`).
- Loop frames (`Verify/Contracts.lean:2264-2270`) carried the `DataInvariants`
  implication when the body could touch the store, and no namespace invariant.
- `Contract.lean:1741` (`owesMemoryInvariants`) had a data-invariant branch: a
  function writing a `key` resource with an invariant owed "invariants of
  memory". That branch also selected a spec-less function mutating such a
  resource through a global borrow for verification
  (`Verify.lean:777-780`), because `handlesDataInvariants`
  (`Verify.lean:523`) saw references only in parameters and results, not in
  locals (`carriesInvariant`, `Contract.lean:968`, was false of a reference
  type).
- `Compile.lean:547-554`: at a loan's death, `writesGlobal` returned
  `memoryWritten` before `lender?` was consulted, so a mutation through
  `borrow_global_mut` produced no `mutationEnd` at all; `checks` yielded one
  `Option Term` per loan, and resolution preceded the checks.
- `Verify/Contracts.lean:2710-2760` (`mutationConditions`) keyed a `mutationEnd`
  obligation on `lender?`, which was `none` for a global borrow. The slot for
  one is the loan's holder (`CheckedLoanFact.holders`, `Validated.lean:221`), a
  local of type `.ref referent`, which `translateLoopInvariants`'
  `dataInvariantSlots` already read as the referent
  (`Verify/Contracts.lean:2199, 2749`).
- `Term.lean:1692-1700, 1813`: `mutationEndAt`, `memoryWrittenAt` and
  `constructedAt` were `Spec.pure`, and their obligations were cut rules
  (`wp_mutationEndAt`, `wp_constructedAt`, `Term.lean:2795, 2844`) whose
  predicate the solver supplied from the conditions tables
  (`Verify/Contracts.lean:2710-2830`; `Solve.lean:1877-1881`,
  `Solve/Invoke.lean:159-227`).
- `NTy.carrier` (`Types.lean:1436`) gave a struct its row `HList fields` and a
  reference `referent.carrier × referent.carrier`; `SpecInt`
  (`Representation.lean:31`) was the one carrier with a certificate.
  `Spec.certified` (`Spec.lean:409`) was already defined for a data invariant's
  construction and used nowhere.
- `reachFrom` (`Contract.lean:589`) recorded `StructHandle`s without type
  arguments, so an invariant on `S<u64>` was selected for a function touching
  `S<u8>`; `memoryWriteInvariants` (`Contract.lean:1671`) already instantiated
  the obligation at the written type. Sound, an over-approximation. This part is
  unchanged, and it is one of the open problems of the global-invariant scheme.

### The alternative that was rejected

The Move Prover's scheme is to assume the invariants of memory at entry and
again after each havoc, as `WellFormed` augmented with the data invariant. That
is what a verifier without subtypes must do, and the layer above was a form of
it: an entry premise, preservation implications, and a proof per join arrival.

Carried as in-body assumptions the scheme needs either a hypothesis per function
propagated to every caller (`AssumptionsHold`, which also rejects a cycle
member) or a unit-level refinement hypothesis beside `GlobalsPreserved`; and a
read past a write of the same resource needs a case split on the key that
Boogie's solver makes for free and a Lean decision procedure does not. Every
piece of that restates as an assumption about runs what a certified carrier
gives by typing, and all of it disappears with the carrier. The obligations at a
pack and at a mutation's end are the same under both schemes; it is the reading
side that the carrier removes.

### The measurement that decided it

On `primary_fungible_store::deposit` — three lines of Move with `modifies *` —
1 660M of 2 374M elaborator heartbeats were spent by two leaves targeting
`DataInvariants` after 34 join arrivals had failed to prove it. Under the
carrier design neither the predicate nor the arrivals' proofs exist. The same
measurement showed that a heartbeat budget alone does not bound such a target,
because the heartbeat check does not fire for the time the kernel and the
deciders spend; a wall-clock limit belongs beside it in any measurement.

### What the transition did, and what it left open

The transition landed in parts, each green on its own before the next: the
obligation at a global write-back; the proof that a write owes nothing twice;
and the certified carriers themselves — the plain family, `Carriers.own`,
`NTy.deepHolds`, the satisfiability hypothesis and `CarriersNonempty`, the
certified `NTy.carrier` of structs, variants and references, the codec gated on
`deepHolds`, and the obligation at a pack, at a loan's death and at a `&mut`
hand-over. The Given frame for generic bodies came with them, since a generic
body's parameters needed a lent layer of their own. The resulting state is
[`designs/data-invariants.md`](../designs/data-invariants.md).

Two things the transition planned are not done, and are stated as open problems
in [`data-and-global-invariants.md`](data-and-global-invariants.md). The layer
above was not deleted: it no longer states anything of a declaration whose
invariant the carriers carry, but it is still the layer that owns an invariant
reading the executable unit, a precondition or memory, so all of it is still in
the tree. And the global-invariant scheme, which the same file designed, is not
implemented at all.

## The proof language (2026-10 → 2026-11)

Authored proofs were Lean tactic scripts over the leaves `verex_obligations`
presents. The transition replaced them with a structured language whose
proofs name obligations, paths and facts in the specification's vocabulary,
so that the solver's normal form stopped being the corpus's interface. The
resulting state is
[`designs/proof-language.md`](../designs/proof-language.md).

### The census that set the design

The corpus at the start was 18 proof files, 1 882 lines, 65 `verify` items
and 11 theorems (`aptos-move/framework/**/*.proof.lean`,
`move-prover/tests/sources/functional/**/*.proof.lean`, with the two paper
examples counted apart). Over its 1 539 body lines, classified by hand (a
line can fall in several classes):

| Class | Lines | Share |
|---|---:|---:|
| Preparation (`verex_prepare` in six variants, 104 calls, and the rest of the preparation vocabulary) | 374 | 24% |
| Dispatch by trial (`all_goals first \| … \| skip`, `try tac`; about 210 alternative lines in `big_ordered_map` alone) | 536 | 35% |
| Solver vocabulary (the normal form's spelling, carrier spellings, specialized names, positional payload names) | 128 | 8% |
| Mathematical argument (`have`/`exact`/`rw`/`rcases`; 113 of them applying one of 25 distinct `SpecMap` lemmas) | 581 | 38% |

Ten of the 65 items contained no argument at all: 64 lines of preparation
and dispatch that proved nothing a decider could not have, given the right
context. The 12-step preparation pipeline was copied seven times in
`big_ordered_map.proof.lean` and the iterator block five times. Of the 162
`verex_*` tactics the solver defined, 32 occurred in proofs; the rest were
internals leaking through one namespace.

Two facts in those numbers decided the shape. Dispatch was a third of the
corpus and was paid for at check time, because nothing in the surface said
what an obligation was: every alternative that failed before the one that
closed was still elaborated and charged. And every cost lever was blocked by
the scripts — directed steps were "not measured as a replacement; the
scripts depend on `simp_all`'s results", and continuation folding at calls
"broke authored scripts; rejected" — because the scripts addressed goals by
position and rewrote them by Lean lemma names.

### What was rejected

Fixing the preparation cost instead of the interface: preparing a call's
facts where the walker introduces them was tried three ways and paid on none
of the authored targets ([`designs/performance.md`](../designs/performance.md),
"Levers measured and rejected"). That left the preparation cost where it was and the
scripts untouched, so the conclusion was the opposite one — make the
interface stable first, so that the preparation work can land later without
rewriting the corpus. The language therefore promised nothing about
preparation cost, only that a proof in it would check for no more heartbeats
than the script it replaced.

### The order it ran in, and the figures that gated each part

Obligation keys and path decisions came first (the clause and call of a
callee's precondition, then the sites of `if`, `match`, `abort`, aborting
operations, loops and invocations, the decisions recorded per leaf, and
selectors in `verex_on`), because the item and the points lower onto them.
Then the item and the steps; then the normal-form fixes, each its own change
gated on the automatic targets; then the program points; then the lemma
items with the arithmetic and vector tables and `ring`.

Each part was gated per item on elaborator heartbeats, language against the
script it replaced. The nine `ordered_map` items at the first gate:
`drain_symbolic` 27.29M against 28.50M, `iter_walk_mut_symbolic` 78.35M
against 81.02M, `iter_remove_shift_at_position` 19.76M against 20.08M,
`iter_borrow_mut_at_position` 12.55M against 13.40M,
`iter_borrow_mut_symbolic` 25.41M against 26.74M, `iter_add_middle_symbolic`
21.402M against 21.405M, `iter_add_append_symbolic` 25.99M against 27.11M,
`ground_enum_123` 9.02M against 9.23M — together 219.8M against 227.5M. The
ninth, `aborts_if_new_from_2`, owed a model predicate with no specification
form and became `on _ { }` once the normal form gained one.

The normal-form fixes' gate, against the scripts before them:
`keys_for_loop` 27.94M against 28.30M, `lower_bound_scan_start` 29.70M
against 29.82M, `leaf_walk_sum` 392.24M against 387.53M, `next_key_scan`
109.98M against 103.55M, `aborts_if_new_from_2` 10.11M against 12.80M
(`ordered_map`'s 6.24M against 7.02M). The two above their scripts carry the
new bounds as facts through every later step. The automatic targets kept
their outcomes, and `DenotePerformance` differed by at most 1.1%.

The points' gate, twenty-three `big_ordered_map` items: `next_key_scan`
67.16M against 111.87M, `upsert` 198.80M against 241.73M, `insert_shift`
12.20M against 15.26M, `iter_valid_after_insert` 31.80M against 49.66M,
`aborts_if_new_from_1` 8.78M against 8.89M, `pop_back_drain` 33.86M against
35.73M, `drain_loop` 71.84M against 84.57M, `next_key` 71.83M against
78.66M, `prev_key` 67.36M against 75.71M, `remove_shift` 13.09M against
14.32M, `drain_symbolic` 34.96M against 36.82M, `front_remove_drain` 41.29M
against 41.77M, `aborts_if_new_from_2` 10.73M; `iter_remove` 42.80M against
51.50M and `iter_remove_shift` 44.79M against 49.72M once a generic callee's
results were read in the caller's spelling. Two were above their scripts and
accepted as cost work rather than as the language's price:
`aborts_if_remove` 9.73M against 9.48M and `keys_sorted` 56.74M against
56.15M.

One normal-form rule was tried and reverted: reading `e ∈ v` positionally in
both normal forms (`e ∈ v ↔ ∃ i, … v[i]?.getD default = e`) left
`DenotePerformance` unchanged (+0.00%) but stopped
`Check/Specifications/SpecFunctions.lean`'s automatic `push` target
verifying. The membership spelling is therefore still two spellings with a
decider between them.

### What it left open

The open problems are in
[`proof-language.md`](proof-language.md): a proof-file `assume`, the
function-value constructs, the deciders' own theory of membership, the
literal-pinned local an authored proof cannot name, the nineteen remaining
`lean =>` escapes, `on` inside a lemma item, `pool_u64::deduct_shares`'
removal path, the gaps the framework's proofs reported, a proof-step
`match`, and the incremental preparation the stable interface was built to
make possible.

## Function values as native data (2026-10)

Two boundaries held runtime values after the rest of verification had moved
to native ones: the implementation of a function type's carrier, which was
typed runtime closures whose captures were runtime values, and the requires
table, which was `FunctionHandle → Array RuntimeValue → Memory unit → Prop`.
The transition made both native data, following the Move Prover's
representation of function values (`higher-order-paper-26`, "Representing
Function Values") and its monomorphization. The resulting state is
[`designs/higher-order-functions.md`](../designs/higher-order-functions.md),
"Function values as native data".

### Why a generated family, and not a carrier defined on the type

Every other carrier is defined by recursion on the native type. A function
type's cannot hold native captures that way: the captured row is not part of
the function type, and a capture may itself be a closure. So the carrier of
a function type is generated per link set instead — a finite inductive with
one constructor per closure site, holding that site's captures, and one
`foreign` constructor for every other closure the unit types at the type.
Typing then holds by construction, and the typing predicates a leaf used to
discharge (`FunctionTyped`, `ClosureTypedAt`, the closure rules' typing side
conditions) have no work left to do and are gone.

### The decisions behind the shape

- The carriers are a class with exactly one instance per unit constant
  (`FunctionCarriers unit`), declared where the unit is registered, rather
  than a field of the frame: a field would admit two frames of one unit at
  different function values, where memory needs one. With the class,
  `Carriers.runtime unit`, `Skolems unit`, `Memory unit` and `Comp unit`
  keep their spellings and the library and the agreement stay one
  definition.
- A capture's carrier is spelled concretely in the generated block and
  checked once by `rfl` against the base frame's carrier, because the
  family's own `NTy.carrier` is not a strictly positive occurrence.
- The library holds the family interface (`SiteFamily`) and derives the
  decoder, the laws and the instance from the generator's inductive,
  encoder and site table; a closure whose captures do not decode at its
  site's row is `foreign`, so every admitted closure decodes.
- `requiresForeign` stayed uninterpreted. Defining it as `True` would have
  made every value the link set does not enumerate invocable at every
  argument.
- Construction became a frame operation, `Skolems.construct`, irreducible
  and typed by its site, rather than a `Carriers` field, so the ground
  family keeps no function values.

The lemmas of the runtime vocabulary that the weave needed went with it:
`requiresOf_closureOf` no longer exists, and while the
`closureMeaning_closureOf*` family is still there for the agreement, it is
no longer what a leaf uses at the base frame, where the generated site
equations rewrite construction and invocation directly
(`constructSite`, `invokeSite`).

### The measurement it left behind

The one recorded cost effect is a debt, not a gain:
`StateLabelCallObservations`' `wrong_result` (a rejected false
postcondition) measured 19.15M before the carrier interface and 26.07M
after, over its 25M budget, because `grind` worked on the spelled-out
construction terms with their sharing literals and rows; abstracting
function values before `grind` brings it to 21.31M. It is recorded in
[`status/measurement-debts.md`](../status/measurement-debts.md), whose fix
is the representation items below.

### What it left open

Three items, in [`roadmap.md`](roadmap.md): the invocation equations of
closure sites with type arguments, which keep `functionOfGeneric` and its
invocation rule; captures holding a function value under a vector or a map,
which the generator rejects because such a carrier's certificate is a proof
over its elements and cannot nest the family — the decided remedy is to drop
that certificate and make the bound a leaf fact, as an integer's range is;
and the sharing marks of `NTy.function`, which are Move's calling convention
inside the type and should be derived data of the target. The Rust reading
of the same model (a closure type as a struct of its upvars, `dyn Fn*` as
the link set's closure structs plus `foreign`, function pointers as nullary
constructors per target and instantiation) is designed but not implemented;
Rust closures are a later milestone of the Rust frontend.

## Verification cost and callee summaries (2026-10 → 2026-11)

The goal was to get close to the Move Prover's speed on the framework, so
that verification can run on CI. The mechanisms this work left in place are
[`designs/performance.md`](../designs/performance.md), "Shared
continuations" and "Callee summaries"; the one-line verdicts of the levers
it rejected are that document's "Levers measured and rejected"; the open
items are [`roadmap.md`](roadmap.md), "Cost". The figures below are single
runs on the revisions named and are not baselines.

### The gap

On `primary_fungible_store`, `fungible_asset`, `coin` and
`multisig_account`:

| | Move Prover | Verex, `--heartbeats 300` |
|---|---:|---:|
| Wall | 24 s | 545 s |
| Result | all 209 jobs pass | 100 verified, 51 budget, 16 failed |
| Slowest function | about 1.1 s | stopped at 300M, about 16 s of CPU |

So Verex was at least 22× slower and did not finish. The ratio is rough: the
parallelism differs, and so do function counts and spec coverage. Raising
the budget to 3000M resolved 38 of the 51 budget targets: 6 verified (310M
to 2.4G), 19 failed, 13 still over budget. In `coin`, the failed leaves all
ran out of the 20M leaf budget, and with a 200M leaf budget none of its 8
failed targets verified. Cost in the budget-stopped attempts had no dominant
phase: `call` 16%, leaves over their leaf budget 11%, `leaf` 8%, `bind` 8%,
`inline` 7%, `loop` 7%, `split` 7%. The first architectural cause found was
the stored-invariant layer, which led to the carrier transition above.

### Callee summaries, with and without

Summaries worked in model modules and saved heartbeats inside targets, but
on a fresh run did not pay for themselves, because derivation cost more than
the targets saved. On the four modules (167 targets, `--heartbeats 300`):

| | Without | With |
|---|---:|---:|
| Verified / budget / failed | 101 / 55 / 11 | 101 / 55 / 11 |
| Heartbeats, the 101 verified | 5714M | 5230M (−8.47%) |
| Heartbeats, all targets | 24.05G | 23.54G (−0.51G) |
| Heartbeats, derivations (outside targets) | none | 1.22G |

No target changed status. The two runs were not back-to-back, so their wall
times (544 s, 578 s) are not comparable.
`primary_fungible_store::upgrade_to_concurrent` verified at intermediate
steps and was back at budget in the final state, uninvestigated.

Where the cost was, on the four modules' budget targets: 70% in the walk and
29% at leaves; by inlined body, 30% loop-free callees (the ceiling of
summaries as built), 14.1% callees with loops (`update_owner_schema` alone
1.94G), 8.5% the target's own body, 8.4% generic callees, 11.3% setup, 27.6%
the rest. Model setup was 13.4M per target against 3.0M outside model mode.
Derivation: 102 callees derived for 467M, 753M spent on refusals (85 by
form, cheap; 21 over the walk budget, 464M; 17 over the path bound, 254M; 4
reading a reflected type, 35M; 3 reaching a loop). Application: 863
applications for 1.15G, 0.42M per application with one conjunct, 0.57M with
two or three, 10.85M with four or more; `object::set_untransferable` cost
19.8M applied against about 5.2M inlined, while `error::not_found` took 212
applications for 69M. Skipped at a call site, 867 in all: 435 inside a
shared-fold region, 363 with no summary (`object::object_address` and
`address_to_object` lead), 40 needing a hypothesis the call lacks, 22 the
callee's module invariant, 7 not matching the call.

### Joins

Making joins total (every arrival shares; arms state one equation per
binder with common facts outside the disjunction) took `join exposure` from
1171M to 457M on the four modules and wall from 624 s to 545 s, with 100
verified. The verified count did not move, and `join decide` rose to 1509M.
Two questions were left: why alternatives rose (`fungible_asset::deposit`
6 to 24), and `fungible_asset::transfer`, which spent 2.8G of 3G in one
`join exposure`.

### Authored sharing (2026-11-03)

A whole-module `ordered_map` comparison, with shared continuations,
`verex.jobs=1` and the 300M target allowance, before and after shared path
preparation and nested `on`: all nine authored targets verified both times;
they totalled 245.306M → 212.394M heartbeats and 9.02 s → 8.01 s sampled
CPU (`test_verify_iter_walk_mut_symbolic` 91.774M → 66.782M; two small
regressions, `test_verify_iter_borrow_mut_at_position` and
`ground_enum_123`). Four loop-step clauses meant to share one path shared
none, because their clause-local binders made the full-context keys differ;
grouping by the prepared prefix fixed that. Three targets stayed more
expensive authored than automatic: middle insertion 18.294M against 12.882M,
append 24.268M against 22.072M, removal at a position 16.450M against
10.376M.

### After the compiler restack

On `coin`, `primary_fungible_store` and `randomness` at `--heartbeats 300`,
ignoring source-location markers when matching stated facts and filtering
variant readers before definitional abstraction kept 51 verified / 25
budget / 4 failed and lowered the verified targets' heartbeats by 0.67%.
An assumption retry after substitution, early pin substitution, and literal
simplification before the speculative passes gave negligible gains; moving
the full cheap decider before those passes kept coverage and cost +0.77%.
The single-arrival proposal (the first item of "Remaining on shared mode
and budget stops") stayed unresolved; its predicted 50M saving should not
be inferred from the earlier leaf profile.

### Staking-contract aborts and invariant preservation

`staking_contract::new_staking_contracts_holder` exposed two completeness
defects. Its module sets strict abort checking and the function partial
checking with no abort clauses; treating the empty list as an unconditional
no-abort requirement produced false obligations at event-handle calls, and
the contract translation was changed to the Move Prover's partial
semantics. Then the data invariants of the final memory failed: nine event
handle calls gave nine preservation implications, and
`storedInvariantsProof?` stopped after eight links; it was changed to index
context facts and search their implications with cycle detection and
memoization. Whole-module `staking_contract` went from 7 verified / 11
budget / 2 failed to 9 / 10 / 1.

### Levers tried and rejected, with their measurements

| Experiment | Measurement |
|---|---|
| Behavior dispatch: sibling cache, deriving before the split, a 500k bare `simp`, skipping derivation for manual proofs, a 1.5M sub-budget | 0.18M saved; slower; times out; breaks higher-order callers; 2M kept |
| Batching forall introductions in `Solve.splitOnce` | 79 → 12 splits, 28.44M → 28.38M |
| Pruning dead paths at the split | contradictory leaves 3% (BigOrderedMap 21/635, OrderedMap 11/438); 0 of 4 and 2 of 99 branches refutable; `contradiction` exceeds the recursion depth |
| Keying arrival classes on provenance and clauses | `set_frozen_flag` 234M → 358M |
| Replacing the whole-context substitution at a single arrival by pin equations | coin +480M |
| Contract normal forms declared eagerly in each module's process (`C.callForm_i`) | targets 73.20G → 72.19G against 3.81G of declarations; wall and CPU +9%; 4 verified targets fail at their leaves |
| One memory cell under two keys (`coin_address<T>()` against `type_of<T>().account_address`) | 2 of 154 budget targets |
