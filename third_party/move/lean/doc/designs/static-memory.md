# Static global memory in the denotation

The denotation's global memory is one typed memory per resource type,
related to runtime globals once in the agreement. Open work:
[`roadmap.md`](../plan/roadmap.md).

## Why

[`denotation.md`](denotation.md), principle 1, requires that a goal
mention the typed family store for globals. In a static memory, typing
holds by construction: a slot of a resource type holds a value of that
type, nothing decodes, and nothing about memory typing is assumed or
proved per function. As a hypothesis, memory typing would have to be
restated after every call and every goal over storage would carry
encodings and a representation existential.

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
C>` would share a memory the runtime keeps apart. `ResourceType` also
distinguishes ordinary values from unbounded native collections, which the
allocation history of table handles uses.

**Memory.** One partial map per resource type, as the Move Prover's
memories, typed at the runtime family's carriers:

```lean
def Memory : Type := (r : ResourceType) → StorageKey → Option r.carrier
```

Every frame of a verification shares this one memory, so its type never
changes across a call.

**State.** The denotation's state is the memory: `Comp α := Spec Memory
Failure α`. Loan bookkeeping occurs only inside the prophetic meaning of a
call, under its existentially quantified runtime start, and leaves the
denotation's state.

**Global operations** at resource type `r` and key `k`: read `mem r k`,
aborting where absent; exists `(mem r k).isSome`; publish aborts where
present, else sets the slot; take aborts where absent, else erases it and
yields the value; a mutable borrow yields `(value, prophecy)` and sets the
slot to the prophecy. No operation decodes, so storage contributes no
`undefined` outcome.

**Families.** The carriers of a frame's types (`Carriers`) are split from
the frame (`Skolems extends Carriers`), which also says what its types are
in the memory's terms: `resolve : NTy → NTy` replaces each type parameter
by its resolution, and `toRuntime`/`ofRuntime` carry a frame value to a
memory value and back, inverse to each other and agreeing on encodings. A
frame's resource type `τ` with arguments `Ts` is the memory `⟨resolve τ,
Ts.map resolve⟩` (`Skolems.resource`). At the runtime frame, the frame of
every function without type parameters, `resolve` and the transports are
identities by definition, so these goals carry no transport. A generic
call's frame (`Skolems.instantiate θ outer`) resolves through its caller's
and its transports compose the caller's with the argument transports.

**Generics.** Monomorphization, as the Move Prover's (D3). A generic
function is proved once over every frame, the skolem instance: its
resource types index the memory at `resolve τ`, which reduces to
`R<resolve (.param i)>`, a fresh type, and to `R` itself for a resource
without parameters. `R<#0>` and `R<u64>` are distinct memories unless the
frame makes them one; the storage-aliasing instances are the cases where
it does. A call with type arguments uses the callee at the frame they
induce, its theorem instantiated by unification: at a concrete call the
callee's contract reads `⟨R<u64>⟩`.

**Calls.** A callee's run (`propheticRun`) runs the big-step semantics
from every admissible runtime start whose stores encode the caller's
memory and yields the memory its exit stores encode (`StorageEncodes`,
with the allocation history of table handles). Each runtime key holds the
encoding of its closed resource type's slot; open generic entries in the
type table are templates, which frame coherence relates to their
instantiated resources. The prophetic meaning (`propheticMeaning`) is the
run where the frame is coherent with the runtime type instantiation
(`Coherent`) and has no outcome where they disagree; a callee without
type parameters is coherent at every frame. Invoking a known generic
closure is its target's prophetic meaning once the target frame's
coherence is decided by evaluation over the unit (`coherentCheck`).

**Canonical global maps.** A runtime global map keeps its entries in key
order, so a memory has one runtime encoding (`Encodes.unique`) and runs
from the starts at one memory differ only in their loans, which the
behavioral predicates rely on.

**Tables.** A table owner's type has a hidden last field holding its
entries ([`intrinsic-maps.md`](intrinsic-maps.md), "Owned Table values"),
so a memory holds tables as values, as the runtime does. An escaping
reference leaves a heap hole; at a successful call boundary
`StorageEncodesReturned` resolves those holes with the returned row, so
`Contract.frame` takes the result as `ensures` does. Frames remain
unconditional even when abort conditions excuse `ensures`.

## Contracts

- `global<R>(k)` reads the memory slot at the contract frame's resource
  type, `exists<R>(k)` whether the slot holds one. Nothing decodes.
- A modifies frame states, for each listed resource type, that every key
  other than its listed ones reads the same, and that every other
  resource type reads the same (a loose frame leaves those open); a
  caller frames with it by the memory's update lemmas.
- **Data invariants of values**, as the Prover checks them: owed where a
  value is constructed (`Term.constructed`) and where a mutation of a
  local whose type carries one ends, at the death of a loan of the local
  and after a write into it (`Term.mutationEnd`); assumed of parameters
  at entry and owed of results and of the referents of mutable parameters
  at exit. A function constructing or mutating such a value is verified
  even without a specification.
- **Data invariants of stored resources**, as the Prover treats them:
  assumed at entry for the memory a function reaches, owed at every
  write. One predicate per unit (`storedInvariant`) gives each `key`
  declaration's invariant; `MemoryInvariants storedInvariant memory`
  states it of every stored value. A function reaching a declaration that
  has one requires it at entry and ensures it at exit; any other function
  reaching memory ensures that it keeps it. A write keeps it by its lemma
  (`MemoryInvariants.set`) where the value written satisfies its own, and a
  read adds the value's invariant to what a proof knows. The invariant of
  a stored value is stated at its resource's own type arguments
  (`StoredCase`), one statement per row of type values, as the Prover
  indexes memory by type; a writer or reader at a generic frame moves its
  view of the value there (`Skolems.reframe`), which keeps its encoding,
  so the invariant holds at every frame by construction.
- **Module invariants**, as the Prover evaluates them: those of every
  namespace of the unit that read memory a function reaches (through its
  body, contract, specification functions, and callees, at any depth) are
  assumed at entry and owed at exit; under a strict frame, at the keys it
  modifies. A function writing such memory is verified even without a
  specification. A `[suspendable]` invariant is carried by the callers of
  a function declaring `delegate_invariants_to_caller`, or calling one at
  any depth, unless the function is public or entry
  (`delegatesInvariants`). Within a body an invariant is owed after each
  write of memory it reads (`Term.memoryWritten`) and, if `[suspendable]`,
  after each call that leaves it to the function (`CallChecked`); an
  update invariant compares with the memory before the write or call.
  `disable_invariants_in_body` defers these checks to the exit. An inlined
  callee's writes and calls owe nothing in its caller.
- The invariants a module's functions answer to are those of the
  compilation unit, the modules of one file. A module's unit also links
  every registered module whose invariants read memory its functions
  reach, and every earlier module whose functions write memory its own
  invariants read, verifying those functions again in its unit
  (`invariantRelatedModules`), so each function and invariant of the file
  that concern each other are checked whichever module comes first.
- A behavioral predicate inside a data invariant holds in every memory
  (`!aborts_of<f>(a)`: `f` aborts from no memory). A data invariant then
  reads no memory, so a write elsewhere cannot break it and owing it where
  a value is written justifies assuming it everywhere.
- A theorem invoking a function value its proof cannot see needs the
  start globals typed. Every memory's encoding is typed once the unit's
  resource types correspond to their keys' types (`ResourcesTyped`),
  decided once per unit by evaluation (`resourcesTypedCheck`); nothing is
  assumed of memory per function or per step.

## Agreement

`compileFunction_agrees` is `Spec.Refines (propheticMeaning …) (f.denote
…)`: every outcome of the prophetic meaning is one of the denotation, the
only direction used (contract transport, inlining), weaker than an
equivalence, and true at every frame, since at an incoherent one the
prophetic meaning has no outcome. What the axioms assume about types is
the compiler's: a coherent caller's calls have coherent frames.

The public theorem (`satisfies_prophetic`) runs the runtime frame at the
empty instantiation, coherent by definition. It takes as a hypothesis that
the unit's runs keep global memory encoding a memory
(`GlobalsPreserved`), which `preservation`
(`Semantics/Preservation.lean`), with loan holes settled at a function's
exit and the codecs' tightness, is to establish once per unit. Nothing
about memory typing is left to a function's proof or to its callers.

## Solver

The memory's update lemmas, the frame's resolution and transports, and a
stage that reads memory as the leaf's case splits fixed it
(`verex_memory_reads`), before a split and at a leaf, so that a clause's
read meets the program's. Callee frames apply to memory reads directly.
