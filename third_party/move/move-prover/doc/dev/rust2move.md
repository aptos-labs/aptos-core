# Verifying Safe Sequential Rust with the Move Prover

This note describes a design for extending the Move Prover so it can verify *safe, sequential*
Rust code. The idea is to translate monomorphized Rust MIR into an extended version of the
prover's stackless bytecode, and to extend the type system, the borrow/write-back model, and the
downstream pipeline phases so that this extended model verifies end-to-end through the existing
Boogie backend.

This is a design document for later use, not an implementation plan with committed milestones.
It records the motivation, the architectural assessment, and the concrete model extensions that
were worked out, with pointers into the current code base.

## Motivation

The Move Prover is a mature verification pipeline: a semantic model (`move-model`), a stackless
bytecode IR with a transformation pipeline (spec instrumentation, loop analysis, global
invariants, monomorphization), and a Boogie backend with error mapping, sharding, and solver
integration. Tools that verify Rust — Prusti (Viper), Creusot (Why3), Aeneas, Verus — had to
build comparable infrastructure from scratch.

The observation motivating this design is that safe sequential Rust and Move are semantically
much closer than they appear:

- Both enforce **exclusive mutable references**. The prover's `$Mutation` encoding — a mutable
  reference carried as a value together with its write-back address, written back when the borrow
  ends — is justified exactly by the "mutable references are single threaded" property. Rust's
  borrow checker guarantees the same property. This encoding is essentially the *prophecy
  variable* model that Creusot independently converged on, and the pure-value translation that
  Aeneas uses; the Move Prover already has a production implementation of it.
- Both have **abort/panic semantics** that map onto the same `aborts_if` machinery.
- Monomorphized MIR is structurally close to stackless bytecode: temporaries instead of an
  operand stack, explicit control flow, explicit drop points.

The gaps are precisely the places where Rust is more permissive than Move:

1. **References stored in structs and passed as type arguments** (`struct Iter<'a> { v: &'a mut Vec<T> }`,
   `Option<&mut T>`). Move prohibits both; the prover's borrow analysis and write-back placement
   rely on the prohibition.
2. **Interior mutability** (`RefCell`, and `Rc<RefCell<T>>` aliasing). Move has no aliased
   mutable state outside of global storage.
3. **Closures capturing `&mut`** (`FnMut`). The prover's function value support assumes captured
   arguments are values.

This document works out how to close these gaps. The headline results:

- The `$Mutation`/`$Location` core model in the Boogie prelude needs **no changes**. Storing a
  mutation inside a struct value is semantically inert; what must be extended is the borrow-edge
  vocabulary and the *placement* of write-backs.
- Write-back placement — the hardest part — should be **imported from rustc's borrow checker**
  (NLL/Polonius facts) rather than re-derived by extending `borrow_analysis`. The translator
  emits explicit write-backs at region-end points.
- `RefCell` maps naturally onto the prover's **global resource memories**, indexed by allocation
  id instead of account address, reusing `$Memory`, `modifies`, and abort machinery.

## Architecture Overview

```
Rust source
    ↓
rustc (type check, borrow check, monomorphization)
    ↓
Monomorphized MIR + NLL/Polonius borrow facts
    ↓
[NEW] MIR translator
    ├── builds GlobalEnv / FunctionData directly (no Move front end)
    ├── emits extended stackless bytecode
    └── emits WriteBack / IsParent at NLL region ends
    ↓
Bytecode pipeline (existing processors, extended; borrow analysis
    and memory instrumentation become validators for Rust-sourced code)
    ↓
Boogie backend (extended type/name mangling, new borrow edge arms)
    ↓
Boogie / Z3
```

Key point about the front end: the "no references in structs / no reference type arguments"
rules are enforced by the Move *compiler*, not by `move-model` itself. `Type::Reference`
composes freely in `move-model/src/ty.rs`, and `GlobalEnv`/`FunctionData` can be constructed
programmatically. A MIR front end builds the model directly and never runs the Move type
checker, so lifting the restrictions is a matter of downstream support, not of fighting the
front end.

Coming in monomorphized also sidesteps the trait system: `mono_analysis` receives pre-concrete
instantiations, and `TypeParameter` handling is largely bypassed. Static dispatch disappears;
`dyn Trait` and closures go through the existing function value infrastructure (see below).

## The Verification Fragment

The target fragment is safe, sequential Rust, with the following boundaries:

- **No `unsafe`**, no raw pointer dereference. The model's soundness *relies* on rustc's
  aliasing guarantees.
- **No concurrency** (no `Send`/`Sync` reasoning, no atomics, no threads).
- **No pointer identity observation**: `ptr::eq` is excluded. Reference equality in the model is
  value equality of the pointee (consistent with `PartialEq`-through-deref); `ptr::eq` would
  distinguish states the encoding identifies.
- Interior mutability initially limited to `RefCell` (and `Cell` as a degenerate case), accessed
  through `Rc`/`Box`/direct ownership. `Rc` reference counts are not modeled (irrelevant to
  functional correctness in a sequential setting); `Drop` *effects* are modeled via MIR drop
  glue, which MIR makes explicit as terminators.
- Integer semantics must be pinned: verify against the *panicking* (debug) overflow semantics.
  Signed integers need new `$IsValid` ranges and overflow instrumentation; wrapping ops map to
  modular arithmetic.

## Recap: the Current Mutation Model

The Boogie prelude (`boogie-backend/src/prelude/prelude.bpl`) models a mutable reference as

```
datatype $Location { $Global(a: int), $Local(i: int), $Param(i: int), $Uninitialized() }
datatype $Mutation<T> { $Mutation(l: $Location, p: Vec int, v: T) }
```

The reference carries its current value `v` and its own write-back address: a root location `l`
plus an edge path `p` (field offsets and dynamic indices). Mutations update `v` in place
(`$UpdateMutation`); when the borrow ends, a `WriteBack(node, edge)` instruction folds `v` back
into the parent along the edge (`bytecode_translator.rs`, `translate_write_back`).

The model splits write-back into two halves:

- **Dynamic addressing.** `(l, p)` is carried at runtime (in the model). When the parent of a
  reference is statically ambiguous (e.g. a borrow under a conditional), `IsParent` instructions
  compare `(l, p)` dynamically (`$IsSameMutation`, `$IsParentMutation`,
  `$IsParentMutationHyper`) and guard conditional write-backs. This machinery is general.
- **Static placement.** `borrow_analysis.rs` computes a per-program-point borrow graph
  (`BorrowNode`, `BorrowEdge`) and `memory_instrumentation.rs` inserts the `WriteBack` /
  `IsParent` instructions where borrows end. This is what depends on Move's restriction that
  references never escape into values: the graph's shape is statically known.

The current vocabulary (`move-model/bytecode/src/stackless_bytecode.rs`):

```rust
pub enum BorrowNode {
    GlobalRoot(QualifiedInstId<StructId>),
    LocalRoot(TempIndex),
    Reference(TempIndex),
    ReturnPlaceholder(usize),
}

pub enum BorrowEdge {
    Direct,
    Field(QualifiedInstId<StructId>, Option<Vec<Symbol>>, usize),
    Index(IndexEdgeKind),
    Invoke,                  // borrow via a function value, unknown structure
    Hyper(Vec<BorrowEdge>),  // composed sequence
}
```

At call sites, `&mut` arguments get implicit returns — `f(x)` becomes `x := f(x)` — so the
callee's updates to the mutation's value flow back to the caller, which performs the final
write-back. Inside the callee, such a parameter has location `$Param(i)`: the callee never
writes it back to a root; the caller does.

## The Key Insight: Storing a Mutation Is Inert

A struct with a `&mut T` field is representable today: Boogie datatypes are polymorphic, and
`boogie_helpers.rs` already renders `Reference(_, bt)` as `$Mutation (T)`. The stored
`$Mutation` carries its `(l, p)` unchanged. Therefore:

> The entire existing write-back translation — including hyper edges and dynamic parent choices —
> works **verbatim** on a mutation after it is extracted from a container back into a temporary.
> What must be extended is (a) an edge vocabulary for the round trip through the container, and
> (b) the placement logic that knows when a stored borrow ends. The `$Location` and `$Mutation`
> datatypes themselves need no changes.

Reborrowing through a stored reference is modeled as **take-out / put-back**: extracting a
stored `&mut` temporarily moves the whole `$Mutation` out of its slot, and the slot is restored
when the use ends. This avoids ever having two live copies of the same `(l, p)` with divergent
values — the staleness hazard the value-carrying model must not have. Rust's exclusivity
guarantees the slot is genuinely inaccessible in between, so the take-out is invisible to the
program.

## Extension 1: References in Structs

### Type system

Allow `Type::Reference` in struct field types and type instantiations for Rust-sourced models.
Immutable references get value-ified (extending the approach of `eliminate_imm_refs.rs` to
struct fields, sound because no interior mutability lurks behind `&T` in the fragment). Only
`&mut` fields need the mutation machinery. `&mut` is not `Copy` in Rust, so mutation values are
never duplicated by the source language.

### Borrow edge extensions

```rust
pub enum BorrowEdge {
    Direct,
    Field(QualifiedInstId<StructId>, Option<Vec<Symbol>>, usize),
    Index(IndexEdgeKind),
    Invoke,
    /// NEW: step through a reference stored in a value slot. The parent is the
    /// container node; the child was obtained by dereferencing the stored &mut.
    /// Write-back re-installs the (updated) $Mutation into the slot.
    Deref,
    /// NEW: like Field, but selects a closure capture slot instead of a struct
    /// field. Resolves to the generated closure datatype's selector/updater.
    Capture(QualifiedInstId<FunId>, ClosureMask, usize),
    Hyper(Vec<BorrowEdge>),
}
```

`Deref` composes in `Hyper` exactly like `Index`: a reborrow of `*s.f` where `s` is a local gets
parent node `LocalRoot(s)` and edge `Hyper([Field(S, f), Deref])`.

`BorrowNode` stays unchanged. The temptation is to add an `Embedded(TempIndex, AccessPath)`
node for "the mutation inside slot `f` of temp `s`", but this can be avoided — and with it,
touching every downstream consumer of `BorrowNode` — by requiring the instrumenter to
*materialize* an embedded mutation into a fresh reference-typed temp before any write-back
involving it. Nodes remain "temp or global", which every phase already understands. If
precision problems arise later (containers holding several `&mut` fields dying at different
times), `Embedded` nodes are the escape hatch; start without them.

Two front-end prohibitions must be lifted for the materialization to be expressible: `Select`
on a reference-typed field (producing a `$Mutation`-typed temp), and `Pack` with
reference-typed arguments. Both are legal in the bytecode shape and only excluded by compiler
checks, so for a MIR-sourced model this is permission, not mechanism.

### Write-back semantics, case by case

**Case 1 — a reborrow through a stored ref ends (`Deref` edge, slot restore).**
For `let r = &mut *s.f; ...` with `r`'s region ending:

```
// at reborrow:  take out (slot is stale/unusable while r lives)
$t_r := $t_s->f;
// mutations through r update $t_r->v as usual
// at region end: WriteBack(LocalRoot(s), Hyper([Field(S,f), Deref]), r)
$t_s := $Update'S'_f($t_s, $t_r);
```

In `translate_write_back_update`, `Deref` becomes an arm structurally parallel to `Index`:
read aggregate `$Dereference`, update aggregate `$UpdateMutation`. When `Deref` is the last
edge of the path this simplifies to restoring the whole mutation, which is equivalent since
`(l, p)` never changed. This write-back deliberately does **not** touch the original root of
the stored borrow — that root is not this write-back's business.

**Case 2 — the stored borrow itself ends** (container dies, slot overwritten, ref dropped).
This is where the original root is finally updated, and where the inertness property pays off.
The instrumenter emits:

```
t := Select<S::f>(s)     // materialize the stored $Mutation into a temp
WriteBack(N, E, t)       // N, E recorded at pack time — the original parent
```

where `N —(E)→ ...` is the edge the borrow had *before* it was packed. Because packing
preserved `(l, p)`, the existing translation handles this unmodified — including `GlobalRoot`
targets (`$ResourceUpdate`, which is how this composes with the `RefCell` model below),
`LocalRoot`, and `Reference` parents with path-indexed updates. If the original parent is
statically ambiguous (the packed ref could root at `a` or `b`), the existing
`IsParent`/`$IsSameMutation` choice mechanism applies to the extracted temp verbatim, because
those tests only inspect `(l, p)`.

**Case 3 — transport across call boundaries.**
Generalize the implicit-return convention from "argument is a mutable reference" to "argument
**type contains** a mutable reference": any struct or closure argument with embedded `&mut` is
chained as an implicit return, and the callee's updates to embedded `v` slots flow back in the
returned value. Critically, the callee never interprets a foreign `(l, p)`: inside the callee,
embedded mutations are only updated in their `v` component or written back into *enclosing
value slots*, never to memory roots. Rust lifetimes guarantee every borrow outlives the callee
frame, so the final root write-back (case 2) always executes in a frame where the root is
native — directly, or via the existing `$Param(i)` indirection when the root itself was a
parameter. `$Location` needs no new variants.

For functions *returning* structs containing refs (e.g.
`fn pick<'a>(a: &'a mut T, b: &'a mut T) -> Iter<'a>`), the caller knows the candidate roots
from the signature's lifetimes and emits `IsParent`-guarded write-backs against each candidate
when the returned struct's borrows end — the same mechanism used today for conditionally-rooted
plain references.

### Path encoding

The runtime edge path `p` encodes edges as integers for the dynamic parent tests
(`$EdgeMatches`): field offsets are `>= 0`, `Index` is the wildcard `-1`. `Deref` needs a
distinct marker (e.g. `-2`), and `Capture` slots an offset-shifted range, so patterns remain
unambiguous.

## Extension 2: Closures Capturing `&mut`

A closure capturing `&mut x` is the struct case wearing a costume: the generated closure
datatype holds a `$Mutation` in a capture slot, addressed by the `Capture` edge instead of
`Field`. The genuinely new element is the calling convention: an `FnMut` call is a `&mut self`
invocation, so `Invoke` on a mutably-capturing closure must treat the closure temp itself as a
`&mut` argument — implicit return `c := invoke(c, args)` — so capture-slot `v` updates persist
across calls. When the closure dies, case 2 fires per capture slot: extract each captured
mutation and `WriteBack` to its pre-capture parent.

For *unknown* function values, `BorrowEdge::Invoke` already gives the sound answer —
`$HavocMutation` — and the behavioral-predicate infrastructure (`ensures_of` etc., see
`fun_values_note.md`) is the mechanism for recovering precision: havoc the closure value and
captured mutations, then assume the function value's `ensures`. No new edge kind is needed
there; what is needed are specs on the closure's behavior, a spec-surface question rather than
a borrow-model one.

## Extension 3: `RefCell` via Global Resources

Interior mutability is where aliasing genuinely enters (`Rc<RefCell<T>>`: two handles, one
cell), and the pure-value encoding cannot express it. The proposal is to model heap cells as
global resources:

- `$Memory<T>` is a `[int] → T` map with a domain predicate. A `RefCell<T>` becomes a resource
  in a per-`T` memory indexed by an **allocation id** instead of an account address —
  `$Location`'s `$Global(a: int)` does not care which it is. `mono_analysis` already generates
  one memory per concrete type, matching monomorphized Rust exactly.
- A **fresh-allocation primitive** is needed (ghost counter, or nondeterministic fresh id with
  a freshness axiom) — a new `Operation` or an intrinsic native with a spec.
- `RefCell`'s *dynamic* borrow flag is part of the resource state. `borrow_mut` on an already
  borrowed cell panics; panic maps to Move abort, so `aborts_if` machinery, error reporting,
  and inconsistency checking apply unchanged. Note that releasing the flag is a `Drop` of the
  `Ref`/`RefMut` guard — faithful drop-glue translation is a *prerequisite* for this extension,
  not an optional nicety.
- A `&mut` obtained from `borrow_mut` is an ordinary mutation with location
  `$Global(alloc_id)`; write-back is the existing `$ResourceUpdate` path. Such mutations can be
  written back in any frame (global memory is global), consistent with how `&mut` to global
  resources already behaves.
- `Rc` clone/drop are no-ops in the model (no count reasoning); `Rc::ptr_eq` is excluded with
  `ptr::eq`.

The cost is the known one: this reintroduces a heap. Framing returns (opaque calls must
declare footprints over the heap memories via `modifies`, or havoc them), and quantifiers over
memory maps are the classic solver performance sink. Verification of `RefCell`-heavy code will
perform like storage-heavy Move code, not like pure-value Move code. This argues for keeping
the `RefCell`-free fragment on the pure-value path rather than uniformly heap-encoding
everything — which the design above does.

## Write-Back Placement: Import, Don't Re-Derive

Extending `borrow_analysis.rs` to place write-backs for escaping references would mean: nodes
for temps whose type *contains* `&mut`, transfer functions for `Pack`/`Unpack`/`Select` that
record and replay pre-pack edges, and slot-sensitive liveness. That is a shape-limited borrow
checker — exactly the analysis rustc has already run.

For the MIR path, the translator should instead consume **NLL/Polonius facts**: per borrow,
rustc knows precisely where its region ends and what it may root at. The translator emits the
`WriteBack`/`IsParent` sequences of cases 1–3 directly at region-end points.
`BorrowAnalysisProcessor` and `MemoryInstrumentationProcessor` become validators (or no-ops)
for Rust-sourced functions. This is the single most important architectural decision in the
design: it converts the riskiest pipeline rework into translator logic, where the information
is native. (Prusti made the analogous choice for Viper.) The extended `BorrowEdge` vocabulary
and Boogie translation are needed either way; the extended *analysis* is only needed if Move
source should some day enjoy references in structs.

**Ordering soundness.** Write-backs must fire in reverse borrow order per chain. NLL regions on
the same path are nested by exclusivity (overlapping borrows of one place are parent/child),
and write-backs on disjoint paths commute; hence emitting write-backs at NLL region ends in
program order is sound, with no extra sequencing machinery.

## Downstream Phase Inventory

In roughly ascending order of effort:

- **Unaffected or trivially affected:** `reaching_def_analysis`, `livevar_analysis`,
  `loop_analysis` (loop invariants over mutation-typed locals already work; `$HavocMutation`
  exists), `clean_and_optimize` — except both liveness-adjacent passes must extend the
  don't-eliminate courtesy from reference-typed temps to "type contains a mutation" temps
  between borrow and write-back.
- **Mechanical extensions:** `stackless_bytecode.rs` (new edges, fresh-allocation op; the
  repo's exhaustive-match convention makes the compiler enumerate every affected site),
  `usage_analysis` (heap memories in read/write sets), `mono_analysis` (reference type
  arguments in instantiation sets), Boogie name mangling and `$IsValid`/`$IsEqual` instance
  generation (`$IsValid` of a stored mutation = `$IsValid` of its `v`; `$IsEqual` = value
  equality of `v`), counterexample rendering in `boogie_wrapper` for mutation-valued fields.
- **Real design work:** `spec_instrumentation.rs` — function-boundary treatment of ref-containing
  arguments (the same "type contains `&mut`" predicate as the implicit-return change; havoc-
  and-writeback at opaque call sites generalizes from `&mut` params to embedded mutations,
  expressible as `$Param(i)` locations with non-empty paths); `data_invariant_instrumentation`
  (when do invariants of a struct with a `&mut` field hold?); `global_invariant_instrumentation`
  (heap memories now trigger invariants — a performance concern to watch).
- **Replaced rather than extended:** `borrow_analysis` + `memory_instrumentation` for
  Rust-sourced code, per the previous section.
- **Backend arms:** `translate_write_back_update` gains `Deref` (read `$Dereference` / update
  `$UpdateMutation`) and `Capture` (a clone of `Field` with closure selectors); `IsParent`
  translation and `$EdgeMatches` learn the new path-element encodings.

## New Bytecodes

Few are strictly required:

- A **fresh allocation** operation for cell ids (or an intrinsic native with a spec).
- Possibly an explicit **`EndBorrow`/region-end marker** emitted by the translator, if keeping
  write-back emission in a pipeline pass (fed by translator-provided region facts) turns out
  cleaner than emitting `WriteBack` directly from the translator. Semantically it is only a
  placement anchor.
- Everything else reuses existing operations: `Select` on reference-typed fields and `Pack`
  with reference arguments are lifted prohibitions, not new instructions; take-out/put-back is
  `Select` + `WriteBack` with the new edges.

## Co-Equal Risks Outside the Borrow Model

1. **Spec surface.** Specs must arrive as `move-model` `Exp` for `spec_instrumentation` to
   inject. A Rust-side annotation language is needed (attribute macros in the style of
   Prusti/Creusot) plus a translator into the spec AST — including how `old()`, quantifiers,
   and heap-dependent specs (`global<RefCellMem<T>>(id)`) surface to the user. This is a
   significant fraction of the total work and deserves its own design note.
2. **Closures and `dyn Trait`.** Monomorphization removes static dispatch; closures and trait
   objects ride on the function-value infrastructure (`MonoInfo.fun_infos`, behavioral
   predicates), which exists and is recent enough to be extensible.
3. **Integer semantics.** Signed integers, casts, wrapping ops; new `$IsValid` ranges and
   overflow instrumentation matching the chosen (panicking) semantics; interaction with the
   `number_operation` bv analysis.
4. **`Drop` order and effects.** MIR gives drop points explicitly, but `Drop` impls with
   observable effects must be translated faithfully — and `RefCell` borrow-flag release *is*
   such a `Drop`.

## Staging

1. **Fragment 0 — Move-shaped Rust.** No refs in structs, no interior mutability. Mostly
   front-end work: MIR translator building `GlobalEnv`/`FunctionData`, integer semantics,
   drop glue, spec attribute plumbing. Mechanical backend extensions only. Produces a working
   prototype and shakes out the model-construction path.
2. **Fragment 1 — references in structs and type arguments.** `Deref`/`Capture` edges,
   take-out/put-back, implicit returns for ref-containing arguments, NLL-driven write-back
   emission. The pivotal extension.
3. **Fragment 2 — `FnMut` closures.** Capture slots + `&mut self` invoke convention; mostly
   falls out of fragments 1's machinery plus the function-value infrastructure.
4. **Fragment 3 — `RefCell`/`Rc`.** Allocation ids, borrow-flag resources, panic/abort mapping,
   footprint specs. Performance evaluation against storage-heavy Move workloads as a baseline.

## Related Work

- **Prusti** (Viper): imports rustc borrow information into a permission logic; the precedent
  for "import, don't re-derive".
- **Creusot** (Why3): prophecy-variable encoding of `&mut` — semantically the same move as
  `$Mutation`'s carried value + write-back; validation that the encoding scales to real Rust.
- **Aeneas**: translates LLBC to pure functions with backward functions for borrows; evidence
  that safe sequential Rust without interior mutability is a functional language in disguise —
  which is what Fragment 0/1 exploit.
- **Verus**: SMT-native verification with a restricted Rust subset and explicit permission
  types for interior mutability.

None of these handles `RefCell` gracefully today; the global-resource encoding, with panic
mapped to abort and `modifies` for framing, is the differentiating bet of this design. The
reuse bet is the rest of the pipeline: spec instrumentation, global invariants,
monomorphization, error mapping, and solver orchestration come for free.
