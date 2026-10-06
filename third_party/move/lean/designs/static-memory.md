# Static global memory in the denotation

Status: decided 2026-10-02 (user); S1–S5 implemented 2026-10-02
([Status](#status)).

## Why

[`denotation.md`](denotation.md), principle 1, requires that a goal mention
"the typed family store for globals". The implementation does not: the
denotation is `Spec RuntimeState`, its global memory is the runtime
`GlobalMap` of untyped `RuntimeValue`s, and a global read decodes the
value it finds (`decodeOr`), with an `undefined` outcome where the value
does not decode. Contracts recover a typed view by assuming, in `requires`,
one typed map per storable family tied to the runtime map by
`FamilyRepresentation`. The consequences:

- Memory typing is a hypothesis. Nothing restates it after a call, so a
  caller cannot meet the next callee's precondition (two storage calls in
  a row do not verify), and stating it would take a proof per function.
- Every goal over storage carries encodings, decodings, the representation
  existential, and the closer's routing between them (`FamilyRepresentation`
  unfolding).

In a static memory, typing holds by construction: a slot of a resource type
holds a value of that type, there is no decoding, and there is nothing to
assume or prove about it.

## Model

**Resource types.** A resource type is the stored value's native type with
the declaration's type arguments, phantom ones included:

```lean
structure ResourceType where
  type : NTy
  arguments : NRow
```

The native type alone does not suffice: `NTy.struct` carries the field row,
not the type arguments, so `Coin<A>` and `Coin<B>` of `struct Coin<phantom
C>` would share a memory the runtime keeps apart.

**Memory.** One partial map per resource type, as the Move Prover's
memories, typed at the runtime family's carriers (`Carriers.runtime`, where
a type parameter is a loan-free runtime value):

```lean
def Memory : Type := (r : ResourceType) → StorageKey → Option r.carrier
```

Every frame of a verification shares this one memory, so its type never
changes across a call.

**State.** The denotation's state is the memory: `Comp α := Spec Memory
Failure α`. Loan bookkeeping (`globalLoans`, `nextLoan`, `pending`) occurs
only inside the prophetic meaning of a call, under its existentially
quantified runtime start (`Admissible`, `exportsAfter`), and leaves the
denotation's state; `RuntimeState.withLoansOf` goes away.

**Global operations.** At resource type `r` and key `k`:

- read: `mem r k`, aborting where absent;
- exists: `(mem r k).isSome`;
- publish: aborts where present, else sets `mem r k`;
- take: aborts where absent, else erases it and yields the value;
- mutable borrow: yields `(value, prophecy)` and sets the slot to the
  prophecy, as the denotation does today with the encoded prophecy.

No operation decodes, so storage contributes no `undefined` outcome.

**Families.** The carriers of a frame's types (`Carriers`: carrier, codec,
equality per type parameter) are split from the frame (`Skolems extends
Carriers`), which also says what its types are in the memory's terms:

- `resolve : NTy → NTy`, a type of the frame as a runtime-family type, which
  replaces each type parameter by its resolution (`resolve_eq`);
- `toRuntime`/`ofRuntime`, a frame value as a memory value and back, inverse
  to each other and agreeing on encodings.

A frame's resource type `τ` with arguments `Ts` is the memory
`⟨resolve τ, Ts.map resolve⟩` (`Skolems.resource`), and its values cross by
`toRuntime`/`ofRuntime`. At the runtime frame, the frame of every function
without type parameters, `resolve` is the identity and the transports are
identities by definition, so these goals carry no transport. A generic
call's frame (`Skolems.instantiate θ outer`) resolves through its caller's:
`resolve τ = outer.resolve (τ.subst θ)`, its transports compose the
caller's with the argument transports (`toSkolem`/`ofSkolem`).

**Generics.** Monomorphization, as decided for D3 (the Move Prover's). A
generic function is proved once over every frame, the skolem instance: its
resource types index the memory at `resolve τ`, which reduces to
`R<resolve (.param i)>`, a fresh type, and to `R` itself for a resource
without parameters. `R<#0>` and `R<u64>` are distinct memories unless the
frame makes them one; the storage-aliasing instances are the cases where it
does. A call with type arguments uses the callee at the frame they induce,
its theorem instantiated by unification: at a concrete call the frame
resolves to the call's types, so the callee's contract reads `⟨R<u64>⟩`.
Storage no longer keys through the runtime type instantiation
(`Family.instantiate`); `Meanings.typeInstantiation` remains for the
runtime side of calls and closures.

**Calls.** A callee's run (`propheticRun`) runs the big-step semantics from
every admissible runtime start whose globals encode the caller's memory and
yields the memory its exit globals encode (`Encodes`: under every runtime
key, the encoding of the slot of the closed resource type its type identifier
denotes, `runtimeResourceOf`). Open generic entries in the type table are
templates, not independent runtime keys. The template lookup (`resourceOf`)
remains available to frame coherence, which relates a template to its
instantiated resource. Unnamed-memory agreement uses the same runtime-key
lookup as encoding, so it still preserves every slot an encoding cannot name.
Its prophetic meaning (`propheticMeaning`) is the
run where the frame is coherent with the runtime type instantiation
(`Coherent`: every resource type of the callee's namespace mentioning only
its own type parameters, instantiated as the runtime does, is the frame's),
and has no outcome where they disagree. A callee without type parameters is
coherent at every frame. Invoking a function value is its target's run;
invoking a known generic closure is its target's prophetic meaning once the
target frame's coherence is decided by evaluation over the unit
(`coherentCheck`).

**Canonical global maps.** A runtime global map keeps its entries in the
order of their keys' codes (`GlobalKey.rank`), so two maps with the same
lookups are equal (`GlobalMap.ext`) and a memory has one runtime encoding
(`Encodes.unique`). Runs from the starts at one memory then differ only in
their loans, which the behavioral predicates (`result_of`, `aborts_of`)
rely on.

## Contracts

- `global<R>(k)` reads the memory slot at the contract frame's resource
  type and reads as the encoding of the value it holds (the clauses'
  domain for every aggregate), `exists<R>(k)` whether the slot holds one.
  Nothing decodes. `FamilyRepresentation` and the per-family typed maps a
  `requires` quantified are gone.
- A modifies frame states, for each listed resource type, that every key
  other than its listed ones reads the same, and that every other resource
  type reads the same (a loose frame leaves those open); a caller frames
  with it by the memory's update laws.
- Data invariants of values (decided 2026-10-02), as the Move Prover
  checks them: owed where a value is constructed (`Term.constructed`) and
  where a mutation of a local whose type carries one ends, at the death of
  a loan of the local and after a write into it (`Term.mutationEnd`), both
  cuts the closer proves from the invariant's clauses; assumed of
  parameters at entry and owed of results and of the referents of mutable
  parameters at exit. A function constructing or mutating such a value is
  verified even without a specification.
- Data invariants of stored resources (decided 2026-10-02), as the Move
  Prover treats them: assumed at entry for the memory a function reaches,
  owed where a value is stored, at every write. One predicate per unit
  (`storedInvariant`) gives each `key` declaration's invariant over the
  encoding of a value, selected by the declaration a resource type
  instantiates; `MemoryInvariants storedInvariant memory` states it of
  every stored value. A function reaching a declaration that has one
  requires it at entry and ensures it at exit; any other function reaching
  memory ensures that it keeps it. A write keeps it by its law
  (`MemoryInvariants.set`) where the value written satisfies its own, and a
  read adds the value's invariant to what a proof knows.
- Module invariants, as the Move Prover evaluates them: those of every
  namespace of the unit that read memory a function reaches (through its
  body, its contract, specification functions, and callees, at any depth)
  are assumed in full at entry and owed at exit; under a strict frame they
  are owed at the keys it modifies of the declarations they read. A
  function writing memory such an invariant reads is verified even without
  a specification. A `[suspendable]` invariant is carried by the callers of
  a function that declares `delegate_invariants_to_caller`, or that a
  function declaring it or `disable_invariants_in_body` calls at any depth,
  unless the function is public or entry (`delegatesInvariants`).
  Within a body, as the Prover checks them, an invariant is owed after each
  write of memory it reads (a publication, a take, and the death of a
  mutable borrow of a resource, `Term.memoryWritten`) and, if
  `[suspendable]`, after each call of a callee that leaves it to the
  function (`Term.call` at its site, `CallChecked`); an update invariant
  compares with the memory before the write, where the operands are
  evaluated or the borrow begins, or before the call.
  `disable_invariants_in_body` defers these checks to the exit. The writes
  and calls of an inlined callee owe nothing in its caller.
- The invariants a module's functions answer to are those of the
  compilation unit, the modules of one file (from the CLI, a package), not
  only those of the modules it uses. A module's unit also links every
  registered module whose invariants read memory its functions reach, and
  every module the file registered before it whose functions write memory
  its own invariants read; those functions are verified again in its unit,
  against its invariants (`invariantRelatedModules`). Each pair of a
  function and an invariant of the file that concerns it is so checked
  whichever module comes first.
- A behavioral predicate inside a data invariant holds in every memory
  (`!aborts_of<f>(a)`: `f` aborts from no memory). A data invariant then
  reads no memory, so a write elsewhere cannot break it and owing it where
  a value is written justifies assuming it everywhere. Read at one memory,
  `Reader { f: read_counter }` with `!aborts_of<f>(@0x1)` would hold until
  a `move_from<Counter>(@0x1)` that touches no `Reader`.
- A theorem invoking a function value its proof cannot see needs the start
  globals typed. Every memory's encoding is typed once the unit's resource
  types correspond to their keys' types (`ResourcesTyped`), decided once
  per unit by evaluation (`resourcesTypedCheck`); nothing is assumed of
  memory per function or per step.

## Agreement

`compileFunction_agrees` becomes `Spec.Refines (propheticMeaning …)
(f.denote …)`: every outcome of the prophetic meaning is one of the
denotation. This is the only direction used (contract transport,
`satisfies_propheticMeaning`; inlining, `wp_propheticMeaning_of_compiled`),
it is a weaker assumption than the equivalence, and it is true at every
frame: at an incoherent one the prophetic meaning has no outcome.
`compileFunction_least_cycle` keeps its form over `Memory`. What the
axioms still assume about types is the compiler's: a coherent caller's
calls have coherent frames.

The public theorem (`satisfies_prophetic`) runs the runtime frame at the
empty instantiation, which is coherent by definition (`coherent_runtime`).
Its runtime contract requires a start whose globals encode a memory, and
the theorem takes as a hypothesis that the unit's runs keep global memory
encoding a memory (`GlobalsPreserved`), the fact `preservation`
(`Semantics/Preservation.lean`), with loan holes settled at a function's
exit and the codecs' tightness, is to establish once for every unit.
Nothing about memory typing is left to a function's proof or to its
callers. The restatement changes the trusted base and is reviewed as
such.

## What changes in the closer

- Removed: the representation hypothesis, `decode?`/`encode` facts for
  stored values, and the per-step memory typing of unseen invocations.
- Added: the memory's update laws (read after write at the same or another
  type or key), the frame's resolution and transports at the runtime and
  induced frames, and a stage that reads memory as the leaf's case splits
  fixed it (`leaner_denote_memory_reads`), before a split and at a leaf,
  so that a clause's read meets the program's.
- Callee frames apply to memory reads directly.

## Milestones

1. **S1, model.** `ResourceType`, `Memory`, `Encodes`, the
   `Carriers`/`Skolems` split with frame resolution and transports, the state
   change, the global operations, the prophetic meanings over `Memory`, and
   the restated agreement. Gate: `Check/Storage` fixtures with contracts
   rewritten to the memory view.
2. **S2, contracts.** Clauses, twins over carriers, frames; remove
   `FamilyRepresentation`. Gate: all `Check` fixtures, MoveStdlib.
3. **S3, calls.** Callee contracts and frames over memory; two storage
   calls in a row (`Check/Storage/CalleeFrames` extended). Gate: the Prover
   test registry (`prover-test-problems.md`) shows no new failure.
4. **S4, generics.** Generic theorems over every frame, callers at the
   frames their type arguments induce. Gate: `Check/Generics`.
5. **S5, cleanup and cost.** Remove the storage routing; the verification
   benchmark (`verification-benchmarks.md`) does not regress.

## Status

S1–S5 are implemented (2026-10-02): every `Check` fixture matches its
baseline. Over the benchmark problems that verify before and after, the
verification spends 8% fewer heartbeats (`features` 22%, `simple_map` 9%,
`GenericStorage` 16% fewer). The Prover test registry verifies 21 functions
more; 13 functions it refused before are now attempted and fail for the
gaps below. Open:

- Prove `GlobalsPreserved` from `preservation`, so that the public theorem
  no longer takes it.
- `ResourcesTyped` is not decided for a unit with an enum resource or a
  generic resource (registry V22), so such a unit's unseen function values
  are not carried.
- Invoking a function value read from storage is the rule for an unseen
  closure (`higher-order-functions.md`, H4c), which the closer lacks, so a
  stored invariant about it is not used yet (`closures/stored_fun_values`,
  `use_reader`).

## Open questions

1. Whether clauses keep reading aggregates as encodings or read native
   carriers directly.
