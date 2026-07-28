# RustMV: Embedding Safe, Sequential Rust into Move Bytecode

**Status:** Exploration / Draft
**Scope:** Move bytecode format, bytecode verifier, Move VM runtime, and a new
Rust MIR frontend

## Motivation

Move bytecode is more than the compilation target of the Move language: it is
a *verified* execution format. The bytecode verifier independently
re-establishes type, resource, and reference safety on untrusted input, and
the VM executes it deterministically under gas metering. That combination
makes it a candidate substrate for other source languages — provided their
semantics fit the verifier's discipline. This document argues that Rust is a
uniquely good fit, and that the payoff justifies the (substantial) verifier
work:

- **Developer reach.** Rust has one of the largest systems-programming
  communities, mature tooling, and a large body of existing code. Letting
  Rust programmers write on-chain logic in their own language — with their
  editors, test harnesses, and crates workflow — lowers the platform's entry
  barrier more than Move-side ergonomics work alone could.
- **Code reuse.** Pure-computation Rust (data structures, parsers, codecs,
  arithmetic-heavy business logic) frequently falls within the safe,
  sequential, deterministic subset defined below and could be ported on-chain
  with little or no modification.
- **No new trust assumptions.** The bytecode verifier re-checks everything at
  deployment, so rustc never joins the trusted computing base. Rust code
  obtains Move's on-chain guarantees the same way Move code does — by
  verification of the produced bytecode, not by trusting its producer.
- **A semantic fit no other target offers.** Compiling Rust to Wasm or native
  code discards its ownership discipline at the boundary; the host must
  re-erect sandboxing around an opaque memory blob. Move bytecode instead
  *speaks* that discipline: sequential safe Rust's aliasing rules coincide
  with Move's reference rules (developed below), so Rust's safety story
  survives compilation and remains checkable at the bytecode level.

## Overview

This document explores the idea of compiling *safe, sequential* Rust into an
extended form of Move bytecode. The compilation source would be Rust MIR (the
mid-level IR of rustc), taken after borrow checking and monomorphization.

### The source subset

"Safe, sequential" deserves a precise reading, since on-chain execution adds
a third requirement — determinism — that Rust does not impose by itself. The
intended source language is the subset of Rust that is:

- **Safe:** no `unsafe` blocks, raw pointers, `transmute`, or FFI. The
  aliasing guarantees the compilation relies on are properties of the safe
  fragment only.
- **Sequential:** no threads, no `Sync`/atomics, no `async`. Move execution
  is single-threaded per transaction; concurrency primitives have no
  counterpart.
- **Deterministic:** no floating point (`f32`/`f64` have no Move counterpart,
  and consensus-critical code must not depend on platform-sensitive numeric
  behavior), and no ambient effects — no clock, no OS randomness, no
  environment, no I/O of any kind. Practically this means `core` + `alloc`
  rather than `std`; the library port (D3) must additionally replace
  randomized data structures (`HashMap`'s default `RandomState` hasher) with
  deterministic equivalents, and address identity (casting or comparing
  pointers) is not observable.

Programs outside this subset are rejected by the frontend with an explicit
diagnostic, not compiled approximately (see C6 on honest subset definition).

The compilation target would be Move bytecode extended in two directions:

1. **References inside structs.** Move today confines references (`&T`,
   `&mut T`) to function parameters, return values, and locals; they cannot be
   stored in struct fields, vector elements, or global storage. Rust code
   pervasively stores references in structs (iterators, slices, fat pointers,
   `&dyn Trait`, borrowed views). Supporting this requires extending the
   bytecode type system and — much more significantly — the reference safety
   verifier.

2. **Interior mutability (`RefCell`, and by extension `Rc`).** Safe Rust's
   escape hatch from the exclusive-borrow discipline is dynamically checked
   borrowing. Move has no equivalent. One candidate encoding is via global
   resources; this document argues a *native dynamically-checked cell* is the
   better fit (see below for why the global-resource encoding runs into the
   verifier's per-type global borrow tracking).

The fundamental reason this is plausible at all: **sequential safe Rust's
aliasing discipline is Move's reference discipline.** Both enforce "unique
mutable xor shared immutable" references with lexically-bounded lifetimes. MIR
after NLL borrow checking satisfies exactly the invariants Move's bytecode
verifier is designed to check. The gap is not semantic mismatch but
*expressiveness*: Move's verifier enforces a strictly smaller fragment of that
discipline (no references in values), and Move lacks a handful of type-system
features Rust programs rely on.

The difficulty is unevenly distributed:

| Component | Effort share | Character |
|---|---|---|
| VM runtime (value representation, new ops) | ~10% | Mostly ready today |
| MIR frontend (rustc → extended bytecode) | ~20–30% | Conventional compiler engineering |
| Bytecode verifier (reference safety with refs-in-structs) | ~60–70% | Genuine research; requires lifetime annotations in the format |

The last row is the crux and is discussed at length below.

## Background: Where the Current System Stands

The following observations are grounded in the current codebase and shape the
design space.

### The binary format is already syntactically permissive

`TypeSignature` is a plain wrapper around `SignatureToken`
(`move-binary-format/src/file_format.rs`), and `SignatureToken::Reference` /
`MutableReference` are ordinary variants. A struct field whose type is a
reference is *representable in the file format today*. Every restriction is
enforced by the bytecode verifier, not the format:

- `move-bytecode-verifier/src/signature_v2.rs` threads an `allow_ref: bool`
  through `check_ty`. Field types are checked with `allow_ref = false`
  (see `verify_fields_of_struct`); the same flag rejects references in vector
  element types and in type arguments. Function parameter and return
  signatures are the only positions checked with `allow_ref = true`.
- `move-bytecode-verifier/src/struct_defs.rs` (the recursive-struct-definition
  checker) independently hard-codes the same assumption: encountering a
  reference field is treated as an invariant violation
  (`UNKNOWN_INVARIANT_VIOLATION_ERROR`), on the premise that `signature_v2`
  already rejected the module.

So "allow references in fields" is not a format change per se — but making it
*verifiable* is, because the format carries no lifetime information (see
Challenge 1).

### Abilities already provide the containment story

`AbilitySet::REFERENCES = Copy | Drop` — references have neither `store` nor
`key`. Ability derivation for struct fields
(`verify_fields_of_struct`) requires fields to support the abilities the
struct declares. Consequently, even with the field gate removed, **any struct
containing a reference field can never have `store` or `key`**, and therefore
can never be written to global storage, embedded in a storable struct, or
captured in a storable closure. Reference-carrying structs are automatically
confined to ephemeral, stack-lifetime values — precisely the shape of
sequential Rust locals. (`&mut` fields should additionally suppress `copy`,
mirroring Rust's `&mut T: !Copy`.)

This is a major simplification: the extension never needs to interact with
persistent storage, serialization, or the storage-side type system.

### The reference safety verifier is the hard boundary

`move-bytecode-verifier/src/reference_safety/` implements an abstract
interpretation over a borrow graph (the formalized Move borrow checker):

- The abstract domain classifies each local/stack slot as
  `Reference(RefID)` or `NonReference`. Struct *values* are opaque: the
  `Pack`/`Unpack` transfer functions assert all fields are values and emit
  `NonReference` for every unpacked field. A struct value carrying a
  reference is invisible to the analysis by construction.
- Function calls are summarized *by type shape only*, maximally
  conservatively (`AbstractState::core_call`): every returned `&mut` is
  assumed to borrow from every `&mut` argument; every returned `&` from every
  reference argument; all argument references are then released. There is no
  per-parameter "this result borrows from that parameter" information in the
  file format.
- The analysis is gas-metered (e.g. `REF_PARAM_EDGE_COST` charges for the
  all-to-all edge fan-out at calls), reflecting a real adversarial-complexity
  concern.

The type-shape-based call summary is the load-bearing limitation. If a
struct-typed argument or result can *contain* references, the summary is
either unsound (it misses references smuggled inside values) or — if made
conservative by assuming every struct value may borrow from everything — so
imprecise that essentially no interesting program verifies, because no
argument reference could ever be released across a call.

### Global borrow safety is static and per-*type*

Global storage borrow discipline is enforced by two static layers
(`acquires_list_verifier.rs` plus `Label::Global(StructDefinitionIndex)`
edges in the reference safety analysis) and has **no runtime component**.
Notably, the tracking granularity is the struct *type*, not the address:
while a `borrow_global_mut<T>` is outstanding, a second
`borrow_global_mut<T>` — at *any* address — is statically rejected, as is a
call to any function that `acquires T`.

This granularity is what breaks the "encode `RefCell` as a global resource"
idea in its naive form; see Challenge 5.

### The VM runtime is nearly ready

The value layer (`move-vm-types/src/values/values_impl.rs`) already
represents every struct, vector, and locals frame as
`Rc<RefCell<Vec<Value>>>`, and references (`ContainerRef`, `IndexedRef`) are
first-class `Value` variants stored freely in locals. Nothing in the Rust
type structure prevents a reference from sitting inside a
`Container::Struct`; only defensive invariant-violation branches
(`check_valid_for_value_vector`, the exhaustive matches in `borrow_elem`)
reject it at runtime, on the assumption the verifier made it unreachable.

Runtime aliasing enforcement is deliberately absent — the VM trusts the
static verifier — with two narrow dynamic backstops that serve as useful
precedents for this design:

- `Rc::strong_count` checks on `move_from` and whole-container `WriteRef`
  ("moving global resource with dangling reference"), turning a verifier bug
  into an invariant-violation abort rather than UB;
- the enum-variant tag staleness check on `IndexedRef`
  (`EINDEXED_REF_TAG_MISMATCH`), an always-on dynamic guard layered on top of
  the static discipline.

One caveat: there is no `catch_unwind` anywhere in the runtime, so any new
dynamic mechanism must report conflicts as `PartialVMError` aborts and must
not lean on the internal Rust `RefCell` panicking.

### Recently-landed features that close gaps

- **Signed integers**: `SignatureToken::{I8..I256}`, load/cast opcodes, and
  checked arithmetic are already in the format. Rust's integer types map
  directly. (Explicit `wrapping_*` operations still need a small library or
  a handful of ops; checked/aborting overflow is compatible with Rust's
  overflow-checks-on semantics.)
- **Enums** (Move 2): `StructFieldInformation::DeclaredVariants`,
  `PackVariant`/`UnpackVariant`/`TestVariant`. Rust `enum`s map directly.
- **Function values / closures** (Move 2.2): `SignatureToken::Function`,
  `PackClosure`/`CallClosure`. These provide the substrate for trait-object
  vtables and closure lowering — though captures are currently checked with
  `allow_ref = false`, so closures capturing references (`FnMut` closures
  over borrowed state, `&dyn Trait`) are downstream of the
  references-in-structs extension.

## Design Exploration

### D1. References in structs via lifetime-annotated bytecode

The central design decision: how does the verifier soundly check
references-in-structs on *untrusted* bytecode? The verifier cannot trust
rustc — bytecode arrives adversarially, so whatever discipline rustc's borrow
checker established at the source level must be independently re-checkable at
the bytecode level.

The proposal is the same move Rust itself makes: **lifetime parameters in
signatures, declared by the producer, checked (not inferred) by the
verifier.**

**Format extension.**

- Struct handles gain a lifetime arity (analogous to type-parameter arity).
  `StructInstantiation` signature tokens gain lifetime arguments alongside
  type arguments. `Reference`/`MutableReference` tokens in annotated
  positions carry a lifetime argument.
- Function handles gain lifetime parameters and a set of *outlives
  constraints* (`'a: 'b`) plus the binding of each parameter/return reference
  position (including positions nested inside struct types) to a lifetime
  parameter.
- Struct definitions with reference fields record which declared lifetime
  each field reference uses (fully analogous to `struct S<'a> { r: &'a T }`).

Backward compatibility is clean: existing modules have zero lifetime arity
everywhere, and the extension is gated by a file-format version bump plus a
feature flag, following the established pattern for enums and function
values.

**Verifier extension.**

The abstract domain generalizes from the binary
`Reference(RefID) | NonReference` classification to a structured value
abstraction: a value of a reference-carrying struct type is tracked as a
value *plus* one `RefID` per lifetime position of its type. The existing
borrow graph machinery extends rather than restarts:

- `Pack` collects the `RefID`s of reference-typed field operands into the
  lifetime slots of the produced struct value (the existing field-labeled
  edge mechanism, `Label::Field`, already models per-field borrow structure).
- `Unpack` redistributes the struct value's lifetime slots back to the
  unpacked field references (replacing today's blind `NonReference` push).
- Borrowing a reference-typed field through a struct reference
  (`&s.r` where `r: &'a T`) produces a reference whose borrow edges follow
  the *lifetime slot*, not the container — a genuinely new edge pattern, and
  the subtlest part of the transfer-function design.
- Calls are summarized *precisely* using the callee's declared lifetime
  signature: a returned reference (or a lifetime slot of a returned struct)
  borrows exactly from the argument positions sharing its lifetime parameter,
  subject to the declared outlives constraints. This replaces the all-to-all
  conservative summary for annotated functions and is what makes the
  extension usable in practice. Modular, signature-based checking also keeps
  the analysis per-function, exactly as today.

Because only *checking* is required, decidability and complexity stay
tractable: the abstract domain remains finite per function (bounded by
locals × lifetime positions), and joins at control-flow merge points work as
today. The existing gas metering must be extended, since lifetime slots
multiply borrow-graph size — the metering hooks
(`REF_PARAM_EDGE_COST` and friends) already exist for exactly this class of
concern.

Ability interaction, restated as verifier rules:

- A struct with any reference field cannot declare `store` or `key`
  (falls out of existing ability derivation).
- A struct with a `&mut` field cannot declare `copy`.
- Reference-carrying struct types are ineligible as global-storage types,
  vector-in-storage element types, and storable-closure capture types — all
  already implied by the `store` rule.

The second, independent bake-in of "no reference fields" in
`struct_defs.rs` needs a coordinated decision: reference-typed fields should
be *excluded* from the recursion graph (a reference does not contain its
pointee by value, so `struct S<'a> { next: &'a S<'a> }` is not a recursive
*layout* — though whether to allow it is a separate expressiveness choice).

**Runtime extension.** Small: relax the defensive branches that reject
`ContainerRef`/`IndexedRef` inside containers, define reference-field
semantics for `Pack`/`Unpack`/`ReadRef`/`WriteRef`/equality, and extend
value-depth/gas accounting. Serialization never sees these values (no
`store`). The `Rc<RefCell<...>>` substrate needs no structural change.

### D2. Interior mutability: native cell, not global resource

Two candidate encodings for `RefCell<T>`:

**(a) Global resource encoding** (the initially attractive option): each
`RefCell` becomes a resource at a fresh address (as in the object model);
`borrow`/`borrow_mut` become `borrow_global` operations plus an explicit
borrow-flag field. Problems, in increasing severity:

1. Requires `T: store` semantics, address generation, storage gas, and drop
   glue that releases the resource (MIR's explicit `Drop` terminators make
   the glue compilable, but leak-on-abort and cost remain).
2. Semantics diverge from Rust: the cell acquires storage identity and stops
   moving/copying with its owner.
3. **Fatal in the naive form:** the verifier's global borrow tracking is per
   struct *type* (`Label::Global(StructDefinitionIndex)`), so two distinct
   `RefCell<T>` instances at different addresses cannot be mutably borrowed
   simultaneously — rejecting the bread-and-butter Rust pattern of multiple
   disjoint cells alive at once. Working around this via native indirection
   abandons most of what the encoding was supposed to buy.

**(b) Native dynamically-checked cell** (proposed): a VM-native value type
`Cell<T>` with an explicit runtime borrow flag
(unborrowed / shared(n) / exclusive), and native or bytecode-level
operations `cell_borrow` / `cell_borrow_mut` / (implicit) release. A
conflicting borrow aborts with a dedicated status code — exactly `RefCell`'s
dynamic semantics, with abort in place of panic.

Why this fits the existing system well:

- The VM's value substrate is *already* `Rc<RefCell<...>>`; the mechanism is
  the implementation status quo, only surfaced and guarded.
- Precedent exists for natives returning dynamically-managed references
  (the table extension) and for narrow dynamic guards over the static
  discipline (the `IndexedRef` variant-tag check).
- The borrow flag must be an explicit field checked by the VM with
  `PartialVMError` aborts — the runtime has no `catch_unwind`, so the
  internal Rust `RefCell` panic path must never be reachable.
- Statically, `cell_borrow_mut(&Cell<T>) -> &mut T` is a mutable reference
  derived from an immutable one — a deliberate, contained hole in the static
  discipline (precisely Rust's `UnsafeCell` role), sound because the dynamic
  flag enforces exclusivity. The verifier treats the returned reference as
  borrowing from the cell reference with a fresh dynamic provenance.

The release story is the main design question: Rust ties release to the
`Ref`/`RefMut` guard's `Drop`. In Move, the returned reference's release is
static (end of borrow, known to the verifier), so the VM can be told
*where* releases happen either by (i) compiling guard drops (visible in MIR)
to explicit `cell_release` calls, or (ii) a scoped bytecode form. Option (i)
is simpler and MIR hands us the drop points for free.

`Rc<T>` — the usual companion of `RefCell` in safe sequential Rust — maps to
the same mechanism: a copyable handle to a shared native cell (reference
counting is already how the VM owns the backing store; the count is
observable only as "is this uniquely owned", which `Rc::try_unwrap`/
`get_mut` need).

### D3. The MIR frontend

MIR is the right source level, and several traditional obstacles simply do
not arise:

- **Drops are explicit** MIR terminators — no destructor-glue invention; they
  lower to explicit calls (and to `cell_release` for cell guards).
- **`panic=abort`** maps to Move `abort`; no unwinding, no cleanup edges.
- **Post-NLL borrow information** is available to emit the lifetime
  annotations of D1 — the frontend *transcribes* rustc's borrow-checking
  results into the format; the verifier re-checks them.
- **Monomorphization** happens on the rustc side; Move generics can be used
  where they fit, but full monomorphization is always a safe fallback.
- **Aliasing discipline matches** by construction (sequential, safe subset).

Direct mappings: integers (incl. signed, already landed) and `bool`;
structs/tuples; Rust `enum` → Move enum; `Vec<T>` → `vector<T>`;
`String`/`str` data → `vector<u8>`; functions and closures → Move function
values; trait objects → structs of function values (vtables), pending
reference capture.

The remaining gaps, roughly ordered by severity:

1. **Recursive types (`Box` in type cycles).** Move forbids recursive struct
   definitions and `Box<T>` has no indirection to hide behind. This is the
   largest *non-reference* type-system gap (lists, trees, ASTs). Options:
   an "indirect/boxed field" format extension (a field that is a heap
   indirection, breaking the layout recursion the checker rejects), or
   compiler-side arena/index lowering (each recursive type gets a
   `vector`-backed arena; `Box` becomes an index). The arena lowering needs
   no bytecode changes and is the natural Stage-0 answer; the format
   extension is cleaner long-term.
2. **Slices and fat pointers.** `&[T]`, `&str`, `&dyn Trait` are structs
   containing references — downstream of D1. Slices additionally want either
   a range-view reference (today's `IndexedRef` is single-index; no
   slice/range reference exists in the VM) or the library encoding
   `(vector_ref, start, len)` with bounds-checked indexing. The library
   encoding composes with D1 and avoids new reference kinds; a native range
   view is a later optimization.
3. **Standard library.** A `core`/`alloc` port onto Move primitives
   (`Vec`, `Option`, `Result`, `String`, iterators, `HashMap`/`BTreeMap`
   onto suitable Move structures). Unavoidable, sizable, conventional. The
   safe-sequential scope keeps `std` (I/O, threads, processes) out entirely.
4. **Arithmetic details.** `wrapping_*`/`overflowing_*`/`Wrapping<T>` need a
   small intrinsic library (Move arithmetic aborts on overflow; aborting is
   compatible with Rust's checked semantics, but explicit wrapping ops must
   be emulated or added).
5. **Out of scope** (excluded by the source subset defined in the Overview):
   raw pointers and `unsafe` blocks, `transmute`, threads/`Sync`/atomics,
   floating point, ambient effects and other sources of non-determinism,
   `mem::size_of`-dependent layout tricks, address-identity observation
   (pointer comparison of `&T` — Move references have no observable
   identity).

## Challenges

**C1. Sound modular lifetime checking of adversarial bytecode (the research
core).** The verifier must re-establish, from annotations it does not trust,
what rustc established by inference — against inputs crafted to exploit any
unsoundness, within metered time. The Move borrow checker's existing
formalization is the right foundation, and "check declared annotations"
(rather than infer) keeps the problem tractable, but extending the abstract
domain to structured values with lifetime slots, designing the
field-of-reference borrow-edge pattern, and proving the result sound is a
genuine research effort — likely the publishable artifact of the project.

**C2. Precision vs. metering.** Lifetime slots multiply borrow-graph size,
and call summaries become richer. The verifier is adversarially metered
today for exactly this reason; the extension needs a complexity budget
(bounds on lifetime arity, on nesting depth of reference-carrying types) and
metering rules that keep verification linear-ish per function in practice.

**C3. Two independent bake-ins of the current restriction.** The "no
reference fields" assumption lives in at least two verifier components
(`signature_v2.rs`, `struct_defs.rs`) and one runtime layer
(`values_impl.rs` defensive matches). All relaxations must be coordinated
under one feature gate; a partial relaxation is a soundness hole.

**C4. Interior mutability pokes a hole in the static story — on purpose.**
`cell_borrow_mut` derives `&mut` from `&`. The design must make the dynamic
flag the *single* authority for cell exclusivity and keep the static
verifier's guarantees intact everywhere else (in particular: values reached
through a cell must obey the same discipline once borrowed, and the borrow
flag must be released on every path — MIR drop points give us the paths, but
`abort` semantics make leaked flags unobservable since the transaction
dies).

**C5. Global-resource encoding of `RefCell` conflicts with per-type global
borrow tracking.** Recorded as the reason D2 rejects the global-resource
route for the general case (it remains viable for genuinely global
singletons, where Rust would use a `static` + `RefCell`).

**C6. Ecosystem semantics.** Rust programmers expect `Box` (recursive
types), slices, iterators borrowing collections, and `dyn Trait`. Each is
individually solvable (see D3) but the *conjunction* defines whether the
result feels like "Rust on Move" or a restricted dialect. Honest subset
definition — and compiler errors that say clearly what is outside it — will
matter as much as the verifier work.

**C7. No runtime safety net.** The VM has no `catch_unwind` and no general
dynamic borrow tracking; every new mechanism must fail via `PartialVMError`,
and any reliance on the internal `Rc`/`RefCell` behavior (e.g. the
`strong_count` backstops) must be re-audited once references can live inside
containers, since reference-carrying containers change `Rc` topology.

## Suggested Staging

1. **Stage 0 — no bytecode changes.** Prototype the MIR frontend targeting
   *current* Move bytecode: references-in-structs erased via arena/index
   lowering, `RefCell` as a native cell extension (natives only, no new
   opcodes), recursive types via arenas. This validates the entire pipeline
   cheaply, produces a working subset early, and measures how much real Rust
   the encodings alone cover.
2. **Stage 1 — runtime + format extension, verifier bypassed.** Land the
   file-format lifetime annotations and runtime support behind a feature
   flag; verify such modules only in trusted/experimental configurations
   (local networks, tests). Unblocks experimentation with genuine reference
   fields while the verifier design matures.
3. **Stage 2 — the verifier.** Lifetime-checked reference safety for
   untrusted bytecode: extended abstract domain, precise call summaries,
   metering, and a soundness argument extending the Move borrow checker
   formalization. This is where the bulk of the difficulty — and the novel
   contribution — lives.

## Open Questions

- Lifetime *elision* in the format: how much annotation can be defaulted
  (single-input-lifetime rules à la Rust) to keep module size and
  verification cost down?
- Should reference-carrying structs be allowed across *public* function
  boundaries from day one, or start module-internal (annotations private,
  public ABI unchanged)?
- Is `struct S<'a> { next: &'a S<'a> }` (reference-recursion, not layout
  recursion) worth allowing, given the recursion checker must be changed
  anyway?
- Range/slice references as a VM-native view vs. permanent library encoding?
- Does the `Cell` mechanism generalize to `Rc<RefCell<T>>` graphs (shared
  mutable structure) without observable identity leaking into Move
  semantics (equality, serialization of `store`-less values)?
- Interaction with the compiler-v2 pipeline: its internal reference-safety
  passes (`reference_safety_processor_v2/v3`) re-derive lifetime information
  for codegen; can that infrastructure be reused to *produce* the bytecode
  lifetime annotations for Move-source code as well, unifying the two
  producers?
