# Unsafe pointers over the prophetic ownership model

Status: proposed 2026-08-28, not yet scheduled. Designs the first
unsafe-pointer profile for validated LIR on top of
[`prophetic-references.md`](prophetic-references.md), refining the U-stage
roadmap of
[`rust-mir-design.md`](rust-mir-design.md#unsafe-rust-and-raw-pointers);
prerequisite for the interior-mutability design demanded by
[`lir-design.md`](lir-design.md)'s register.

## 1. Unsafe Memory in Rust

A raw pointer (`*const T` / `*mut T`) is plain data: an address plus
*provenance* — an invisible token naming the allocation the pointer was
derived from and the access it permits. Creating, copying, and comparing
pointers is always defined, however garbage they are; only *accesses* and
*offset arithmetic* carry requirements. A legal access is an ordinary
typed load or store: `unsafe { *p = 1 }` lands the write, and every later
read with provenance for that memory observes it (`*p = v` drops the old
value first, `ptr::write` does not). Violating any requirement is
undefined behavior (UB).

UB is *completely* undefined: it never raises a panic or any observable
error — the whole execution loses all meaning, and the compiler optimizes
assuming UB cannot happen.

| Example | Verdict |
|---|---|
| `let p = 0x10 as *const u32` — fabricate, copy, compare any pointer | defined; pointers are data, only use has requirements |
| `let p = &raw mut x; unsafe { *p = 1 }`, no live reference to `x` | defined; a plain typed store, visible to later reads of `x` |
| access through a pointer derived from `&mut x` before that reference is next used | defined; the next use of the reference invalidates the pointer |
| `unsafe { *p }` after `dealloc`, owner drop, or frame return | **UB** — use-after-death (null and out-of-bounds pointers likewise) |
| `p.add(n)` past one-past-the-end, even if never dereferenced | **UB** (`wrapping_add` is defined; accesses must stay in bounds) |
| read of uninitialized bytes, an invalid value (`bool` outside 0/1), or a misaligned `*p` | **UB** (`MaybeUninit`, `read_unaligned` are the defined routes) |
| `unsafe { *(&x as *const T as *mut T) = v }` — write through a `&T`-derived pointer | **UB** even if never observed; defined only inside `UnsafeCell` |
| mutating memory covered by a live `&T`, or foreign access under a live `&mut T` | **UB** — reference guarantees bind all accesses |

Obeying every requirement restores exactly safe-Rust semantics: `unsafe`
only moves the proof obligation from the compiler to the programmer. Rust
has no finished normative memory model (the operational reference is Miri
with Stacked/Tree Borrows); this design implements the conservative core
the candidate models agree on and makes each requirement a verifier
premise (§2).

## 2. Objective: finding misuses

The point of modeling unsafe pointers is that **verification finds
misuses** and proves their absence — canonically **use-after-death**:
access through a pointer whose allocation has died (freed heap memory, or
a stack place whose owner was dropped or whose frame returned). Every
misuse in scope must be a *located, failed proof obligation* under
`#leaner_verify` and a *detected, located outcome* in the interpreter:

| Misuse | Detected as |
|---|---|
| Use-after-death (heap free, owner drop, frame exit) | liveness obligation at the access site |
| Double death (`dealloc` of a dead allocation) | liveness obligation at the `dealloc` site |
| Out-of-bounds offset or access | bounds obligation at the access site |
| Read of uninitialized memory | initializedness obligation at the load site |
| Write through a shared-derived or null pointer | permission obligation at the store site |

Each row is one safety premise of one operation's rule (§4), carried with
its `Located` source point: a failed proof names the misuse and its
location, and the interpreter checks the same premise dynamically, so
misuse fixtures test both modes differentially. A theorem whose contract
assumes a safety premise away is a conditional model theorem and must be
labeled as such (rust-mir-design's rule).

## 3. Decision

Raw pointers are **not references**: no loans, holes, prophecies, or
death markers. They form a parallel domain — pointer values plus an
explicit `memory` — meeting the value world only at explicit
materialization points. The prophetic guarantee survives because aliasing
returns *only inside `memory`*, where every access is checked. The
smallest sound profile:

- **Strict allocation provenance.** A pointer is valid only inside the
  allocation it was derived from; integer-to-pointer casts and
  cross-allocation arithmetic are rejected at validation.
- **Allocation identities are never reused.** A dangling pointer still
  *names* its dead allocation, so liveness is checkable at every access —
  use-after-death becomes decidable.
- **Typed allocations, no bytes.** An allocation holds one value tree
  (optionally uninitialized); offsets are element indices. Layout, byte
  casts, and unions stay rejected (U3 material).
- **UB is a third outcome** besides `returned`/`threw`, with a cause and
  no usable final state; verification proves it unreachable.

## 4. The model

```lean
-- Syntax: new type constructor
| rawPointer (pointee : TypeId) (mutable : Bool)

-- RuntimeValue: two new constructors
| rawPtr (alloc : Option AllocId) (offset : Nat) (mutable : Bool)
| rawAnchor (alloc : AllocId)

-- RuntimeState gains memory
structure Allocation where
  live    : Bool
  content : Option RuntimeValue   -- none = uninitialized

memory    : Array Allocation      -- indexed by AllocId, append-only
```

- `rawPtr` is plain and copyable. `alloc = none` (null, no provenance)
  makes every access UB; `mutable = false` (shared-derived) makes writes
  UB — the `UnsafeCell` exception belongs to the interior-mutability
  design.
- `rawAnchor` owns the allocation backing an escaped place. It is affine
  (moved, never copied; validation rejects copy/shared-snapshot of
  anchor-carrying values), and its death kills the allocation. One anchor
  per live materialized allocation is the memory analogue of the
  well-holed invariant.
- `memory` is append-only; death flips `live`. Interpreter and big-step
  relation share all rules, so the agreement theorem extends mechanically.

**Allocation death** parallels loan death — both are explicit events in
the one semantics. An allocation dies at (1) `dealloc p`, requiring a
live, offset-zero, mutable, anchor-free pointer (heap allocations from
`alloc` have no anchor; ownership is the library model's convention), or
(2) anchor death: the `rawAnchor` is dropped, overwritten, or its frame
exits. A pointer to a local that escaped its function is therefore
dangling *by the semantics of return* — stack use-after-death needs no
special mechanism. Dead allocations are not cleaned up behind outstanding
pointers; every §2 misuse is a premise violation against this state.

**Operations.** A new shared group, admitted only under the unsafe
profile (the raw checker rejects it elsewhere):

```lean
inductive RawOperation where
  | addrOf (mutable : Bool) (place : PlaceId)  -- materialize, yields rawPtr
  | load                                       -- checked read through rawPtr
  | store                                      -- checked write through rawPtr
  | offset                                     -- element offset within allocation
  | alloc (type : TypeId)                      -- fresh uninitialized allocation
  | dealloc
  | ptrEq
  | isValid                                    -- decides the access premises; itself never UB
```

Raw access is operation-based only: `Place.deref` stays reserved for
references, and raw pointers never appear in place paths (the frontend
lowers `(*p).f = x` to explicit raw operations). Each rule carries its
safety premises — for `store` through `rawPtr (some a) i true`: `a` live
(else use-after-death), `i` in bounds, pointee type matches. A failed
premise derives `undefined`; `Outcome` gains
`| undefined (cause : UbCause)`, and wp and contracts quantify only over
`returned`/`threw`, so absence of UB is a conjunct of every proof.
Language-level `throw` never models UB and vice versa.

**Boundary with the prophetic model.** `addrOf` (Rust's `&raw`) moves the
place's value into a fresh allocation, leaves `rawAnchor a` in the place,
and yields `rawPtr (some a) 0 mut`. Safe accesses to an anchored place
elaborate to checked accesses through the anchor, so safe code and raw
aliases share one memory cell and observe each other correctly. An escape
from a live `&mut b` materializes the borrow's `current`; because a hole
must be reconciled with a value, validation requires the escape to be
**bracketed**: value read back and allocation dead before the loan's
`endLoan`, with liveness plus initializedness as the re-entry obligation.
Prophecy reconciliation is untouched, and nothing changes in `pending`,
`applyPending`, or reference contracts: pointers have no death point and
no exclusivity, so they get no prophecies.

## 5. `StoredPtr<T>`: dynamic checking as a well-known type

For pointers stored in data structures there are exactly two options:
(a) static discharge — invariants constrain the stored pointers so the §4
premises are provable at each access; (b) dynamic checking. The design
makes (b) a well-known library type, named for where it is needed:
`StoredPtr<T>` wraps a raw pointer and stores what checking needs. In the
model that is nothing extra, since `isValid` decides the access premises
against `memory`, and its methods are ordinary code: `read p` is
`if isValid p { load } else { panic }`.

- UB is unreachable through a `StoredPtr` by construction: its accesses
  contribute no `undefined` obligations, only a defined panic outcome. A
  spec may accept the panic or prove its absence — option (a) again, but
  local and optional. Data structures of `StoredPtr` need no
  memory-safety invariants.
- Real hardware stores no liveness metadata, so the compiled counterpart
  must genuinely carry it: a generational handle — allocation id plus
  epoch against an arena or registry, slotmap/`Weak`-style — verified
  once against the raw model (UP3). A `StoredPtr` theorem transfers to
  compiled Rust exactly when that realization is used; `isValid` on a
  bare `*mut T` is not realizable.

## 6. Verification and validation

- `wpExprStep` gains one arm per `RawOperation`, conjoining its located
  safety premises with the continuation. The spec-visible predicates
  `memory.isLive` / `memory.initialized` follow the `holeInGlobals`
  precedent (constructor-keyed guarded scans, safe for symbolic states).
- Generated contracts export a raw-pointer parameter's assumptions as
  preconditions (liveness, bounds, initializedness) and its memory
  effects as postconditions: an unsafe `fn` is exactly a function whose
  contract carries safety preconditions its caller must discharge.
- Misuse fixtures are negative tests in both modes: `#leaner_verify`
  fails at the named premise and location, recorded in an
  expected-failure baseline with a trailing `// error: ...` marker, and
  the interpreter reports the located `undefined` outcome.
- `Validation/Capability.lean` admits the §4 operations only under the
  unsafe profile; integer-derived pointers, byte casts, unions, FFI,
  atomics, inline assembly, `UnsafeCell`, and custom allocators stay
  rejected with located diagnostics. The exporter keeps its U0
  serialize-or-reject obligation; the mapper lowers the admitted subset.
  RawUnit gains the type and operation group as a versioned extension.
  The borrow analysis certifies escape sites (licensing the anchor
  rewrite of safe accesses) and the bracket fact of §4.

## 7. Changes by module

| Module | Change |
|---|---|
| `Syntax.lean` | `Ty.rawPointer`; `RawOperation` (incl. `isValid`); `Operation.raw`. |
| `Semantics/Runtime.lean` | `rawPtr`/`rawAnchor`; `Allocation`, `RuntimeState.memory`; `Outcome.undefined`; pointer typing (dead allocations type at every pointee, like holes). |
| `Semantics/Operations.lean` | Materialization, checked access, alloc/dealloc, anchor-aware place resolution and drop. |
| `BigStep.lean` + `Interpreter.lean` | Same deterministic rules; `undefined` propagation; frame exit kills anchored allocations. |
| `Proofs/*` | Agreement over the new outcome; invariants (one live anchor per allocation, monotone `memory`); WP safety arms and `memory.*` normal forms. |
| `LeanerLang/Contract.lean` | Safety pre-/postconditions for raw-pointer signatures. |
| Validation + exporter/mapper | Profile gating; escape/bracket certificate facts; lowering with precise rejection. |
| Rust profile registry | `StoredPtr<T>` well-known type, lowering to `isValid`-guarded raw access. |

## 8. Milestones

- **UP1 — checked memory model.** §4 across syntax, runtime, operations,
  both engines; agreement and invariants repaired. Gate: the §2 misuse
  fixtures (`use_after_death` stack and heap, `double_death`,
  `oob_offset`, `uninit_read`, `write_through_const`) produce located
  `undefined` outcomes, and a positive `swap_via_raw` fixture runs
  end-to-end from Rust source.
- **UP2 — misuse-finding verification.** §6 wp arms, contracts, spec
  predicates. Gate: `#leaner_verify` proves `swap_via_raw` sorry-free,
  and each misuse fixture fails at exactly its named premise and
  location, recorded in expected-failure baselines.
- **UP3 — heap ownership and library models.** A `Box`-class model and
  the `StoredPtr` realization (generational handle) verified against
  this semantics, exporting pure value-level contracts — the pattern the
  interior-mutability design reuses for `UnsafeCell` caches. Gate:
  clients of the verified models verify without seeing memory
  predicates; a fixture storing `StoredPtr`s in a vector verifies with
  no memory-safety invariants.

UP1–UP2 correspond to rust-mir-design U1, UP3 to the entry half of U2; U0
remains the exporter's serialize-or-reject obligation. Each milestone is a
checkpoint commit with the full matrix green.
