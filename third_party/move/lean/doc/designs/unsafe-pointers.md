# Unsafe pointers over the prophetic ownership model

The unsafe-pointer profile for validated VIR on top of
[`prophetic-references.md`](prophetic-references.md), refining the unsafe
section of
[`rust-mir-design.md`](rust-mir-design.md#unsafe-rust-and-raw-pointers);
companion to [`interior-mutability.md`](interior-mutability.md), which
[`roadmap.md`](../plan/roadmap.md), "4. Shared checking" (prophetic residue), demands. Open work:
[`roadmap.md`](../plan/roadmap.md).

A reader who writes unsafe Rust wants three answers: what the verifier
accepts and what it then asks to be proved (§7.1), what it refuses with a
diagnostic (§7.2), and where it reports UB although Rust defines the
program (§7.3). §§1–6 give the model those answers come from; a reader in
a hurry can start at §7 and follow its references back. §8 applies the
answers to the unsafe code of MonoVM.

## 1. Unsafe memory in Rust

A raw pointer (`*const T` / `*mut T`) is plain data: an address plus
*provenance*, an invisible token naming the allocation the pointer was
derived from and the access it permits. Creating, copying, and comparing
pointers is always defined, however garbage they are; only *accesses* and
*offset arithmetic* carry requirements. A legal access is an ordinary
typed load or store: `unsafe { *p = 1 }` lands the write, and every later
read with provenance for that memory observes it (`*p = v` drops the old
value first, `ptr::write` does not). Memory itself is bytes: a typed read
decodes the bytes at the pointer as a value of the pointer's type, so
reading a buffer through `p as *const u64`, reading a union field, and
`transmute` are the same operation, defined exactly when the bytes are
initialized and form a valid value of the type read. Violating any
requirement is undefined behavior (UB).

UB is *completely* undefined: it never raises a panic or any observable
error. The whole execution loses all meaning, and the compiler optimizes
assuming UB cannot happen.

| Example | Verdict |
|---|---|
| `let p = 0x10 as *const u32`: fabricate, copy, compare any pointer | defined; pointers are data, only use has requirements |
| `let p = &raw mut x; unsafe { *p = 1 }`, no live reference to `x` | defined; a plain typed store, visible to later reads of `x` |
| access through a pointer derived from `&mut x` before that reference is next written through or reborrowed | defined; a write through the reference invalidates the pointer, a read does not |
| `*(buf.add(8) as *const u64)` on a byte buffer whose bytes 8..16 were written | defined; a typed read of initialized bytes |
| `transmute::<u32, [u8; 4]>(x)`, reading a union field | defined if the bytes form a valid value of the type read |
| `unsafe { *p }` after `dealloc`, owner drop, or frame return | **UB**: use-after-death (null and out-of-bounds pointers likewise) |
| `p.add(n)` past one-past-the-end, even if never dereferenced | **UB** (`wrapping_add` is defined; accesses must stay in bounds) |
| read of uninitialized bytes, an invalid value (`bool` outside 0/1), or a misaligned `*p` | **UB** (`MaybeUninit`, `read_unaligned` are the defined routes) |
| `unsafe { *(&x as *const T as *mut T) = v }`: write through a `&T`-derived pointer | **UB** even if never observed; defined only inside `UnsafeCell` |
| mutating memory covered by a live `&T`, or foreign access under a live `&mut T` | **UB**: reference guarantees bind all accesses |

Obeying every requirement restores exactly safe-Rust semantics: `unsafe`
only moves the proof obligation from the compiler to the programmer. Rust
has no finished normative memory model (the operational reference is Miri
with Stacked/Tree Borrows); this design implements the conservative core
the candidate models agree on and makes each requirement a verifier
premise (§2).

## 2. Objective: finding misuses

The point of modeling unsafe pointers is that **verification finds
misuses** and proves their absence, canonically **use-after-death**:
access through a pointer whose allocation has died (freed heap memory, or
a stack place whose scope ended or whose frame returned). Every misuse in
scope must be a *located, failed proof obligation* under `verify` and a
*detected, located outcome* in the interpreter:

| Misuse | Detected as |
|---|---|
| Use-after-death (heap free, scope end, frame exit) | liveness obligation at the access site |
| Double death (`dealloc` of a dead allocation) | liveness obligation at the `dealloc` site |
| Out-of-bounds offset or access | bounds obligation at the access site |
| Read of uninitialized memory | initializedness obligation at the load site |
| Read of bytes that are not a valid value of the type read (`bool` byte 2, unknown enum tag) | validity obligation at the load, `transmute`, or union-read site |
| Misaligned typed access | alignment obligation at the access site (`read_unaligned` has none) |
| Overlapping `copy_nonoverlapping` | disjointness obligation at the copy site |
| Write through a shared-derived or null pointer | permission obligation at the store site |
| Foreign access under a live minted reference (`&*p`, `&mut *p`) or a live reference a pointer escaped from | aliasing obligation at the access, mint, or `dealloc` site |
| Access through a pointer invalidated by a write through the reference it was derived from, or by a foreign write to its memory | provenance obligation at the access site |

Each row is one safety premise of one operation's rule (§4), carried with
its `Located` source point: a failed proof names the misuse and its
location, and the interpreter checks the same premise dynamically, so
misuse fixtures test both modes differentially. A theorem whose contract
assumes a safety premise away is a conditional model theorem and must be
labeled as such (rust-mir-design's rule).

## 3. Decision

Raw pointers are **not references**: no loans, holes, prophecies, or
death markers. They form a parallel domain, pointer values plus an
explicit `memory`, meeting the value world only at explicit
materialization points. The prophetic guarantee survives because aliasing
returns *only inside `memory`*, where every access is checked. The
profile:

- **Strict allocation provenance.** A pointer names the allocation it was
  derived from and a byte offset in it, and is valid for access only
  inside that allocation. Integer-to-pointer casts are rejected at
  validation: an integer names no allocation for the pointer to be valid
  in.
- **Symbolic addresses.** Every allocation has a base address, chosen by
  an *address oracle* when the allocation is made (§4.1). A proof knows
  an address only through the *placement facts*: non-null, aligned as
  requested, inside the address space, disjoint from the other live
  allocations. Reading a pointer's address (`addr()`, `as usize`),
  comparing and subtracting addresses, and moving a pointer to another
  address of its allocation (`with_addr`, `map_addr`) are defined.
- **Allocation identities are never reused.** A dangling pointer still
  *names* its dead allocation, so liveness is checkable at every access:
  use-after-death becomes decidable. Addresses may be reused, as a real
  allocator reuses them.
- **Typed allocations, and byte allocations with typed views.** An
  allocation is either *typed*, holding one value of its type (a local, a
  `Box`, a node from `Box::into_raw`), or a *byte allocation*, holding a
  run of abstract bytes (`alloc::alloc(layout)`, a `u8` buffer). A pointer
  into a typed allocation accesses a *subvalue*, the whole value, a field,
  or an element, named by the pointer's offset and type through the layout
  facts (§4.2). A byte allocation is read and written through a pointer
  of any type that has a layout: the read decodes bytes into a value and
  the write encodes a value into bytes (§4.4). `transmute` and unions are
  the same encode/decode applied to a value rather than to memory. Typed
  allocations stay typed, so the proofs about ordinary owned memory never
  see bytes.
- **Monomorphic verification.** Generic code is verified per
  instantiation: every function a unit verifies has concrete types, so
  every typed view has a concrete layout (§4.4).
- **UB is a third outcome** besides `returned`/`threw`, with a cause and
  no usable final state; verification proves it unreachable.
- **Places become allocations when their address is taken.** `&raw mut x`
  turns the place `x` into an allocation that safe code keeps accessing
  through an *anchor* left in the place (§4.5).
- **Provenance by loan.** A pointer taken from a reference points into the
  lender's own memory and records the loan it was derived through. An
  access to memory under a live loan is legal only through a pointer
  derived from that loan, and a write through the reference, or a foreign
  write to the memory, invalidates the pointer: the conservative core of
  Stacked and Tree Borrows (§4.5, §4.6). Only a reference whose lender the
  frame cannot name (a parameter) gets a *shadow* allocation holding its
  value (§4.5).
- **Minted references are memory loans.** `&*p` and `&mut *p` borrow a
  memory cell as `borrow_global_mut` borrows a global slot: the result is
  an ordinary prophetic borrow, and its exclusivity becomes a premise of
  every raw operation while it lives (§4.6).

## 4. The model

### 4.1 Values and memory

```lean
-- Syntax: new type constructors
| rawPointer (pointee : TypeId) (mutable : Bool)
| union (fields : List (FieldName × TypeId))

-- RuntimeValue: three new constructors
| rawPtr (alloc : Option AllocId) (offset : Int) (meta : Option Nat)
         (mutable : Bool) (via : Option LoanId)                  -- offset in bytes; meta: slice length
| rawAnchor (alloc : AllocId)
| bytes (content : Array AbstractByte)

inductive AbstractByte where
  | uninit
  | int (value : UInt8)
  | ptr (pointer : RuntimeValue) (index : Nat)  -- the index-th byte of a thin rawPtr

-- RuntimeState gains memory
structure Allocation where
  live    : Bool
  type    : Option TypeId          -- some: typed allocation; none: byte allocation
  size    : Nat                    -- bytes
  align   : Nat                    -- the alignment requested
  base    : Nat                    -- address, from the address oracle
  shadow  : Bool                   -- stands in for an unknown lender (§4.5)
  content : Option RuntimeValue    -- typed: none = uninitialized; byte: some (bytes …)

memory    : Array Allocation       -- indexed by AllocId, append-only
```

- `rawPtr` is plain and copyable. `offset` counts bytes from the
  allocation's base, so the pointer's address is `base + offset`; it may
  lie outside the allocation (`wrapping_add` is defined), and then every
  access is UB. `alloc = none` is a pointer without provenance: null,
  `NonNull::dangling()`, or a pointer read from integer bytes; `offset` is
  its address, and every access through it is UB. `mutable = false`
  (shared-derived) makes writes UB; the `UnsafeCell` exception belongs to
  the interior-mutability design. `via` is the loan the pointer was
  derived through (§4.5): `some ℓ` for a pointer escaped from a reference
  with loan `ℓ`, kept by every operation on the pointer; `none` for a
  pointer from `addrOf`, `alloc`, or `allocBytes`, and for pointers
  without provenance. `meta` is `none` for a thin pointer and `some len`
  for a slice pointer.
- `rawAnchor` sits in a place whose address was taken (§4.5) and owns the
  allocation that now backs the place. It is pinned: validation rejects
  copying or moving a value that holds one, and a copy or move *out of*
  an anchored place reads the value through the anchor instead (§4.3).
  The allocation dies with the place (§4.3). Each anchored allocation has
  exactly one anchor.
- `bytes` is the content of a byte allocation and the value of a union
  (§4.4). Its bytes are MiniRust's abstract bytes: uninitialized, an
  integer byte, or one byte of a pointer. A pointer stored into memory
  keeps its provenance byte by byte, so a pointer read back is the same
  pointer; read at an integer type, the same bytes are its address
  (§4.4).
- A byte allocation is one made by `allocBytes` or one whose type is
  bytes already: `u8`, `MaybeUninit<u8>`, and arrays of them. For these
  the value form and the byte form coincide, so a `u8` buffer viewed
  through `*mut u64` needs no conversion. Any other typed allocation is
  accessed only at its subvalues (§4.2); viewing it as bytes is UB in the
  model (§7.3, §9).
- **Addresses.** The *address oracle* is a function from the current
  memory and a request (size, alignment, whether the allocation is a
  shadow) to a base address, or to a refusal. It is *admissible* when
  every base it returns is non-null, a multiple of the requested
  alignment, and leaves the allocation inside the address space, and when
  the base of an ordinary allocation is disjoint from every allocation
  live in that memory (a dead allocation's range may be reused). A shadow
  (§4.5) stands for a caller's memory at an unknown address, so its base
  may overlap other allocations. The oracle may refuse a heap
  request, and then `alloc`/`allocBytes` return null, as the allocator may
  (§6); it answers every request for a place or a shadow. The interpreter runs with
  one oracle (the test driver supplies a bump oracle: the next free
  aligned address), and a theorem holds for every admissible one: the
  big-step relation takes the oracle as a parameter, as it quantifies
  prophecies, and the agreement theorem between interpreter and relation
  holds per oracle. These are the *placement facts* a proof reasons with:
  address arithmetic inside one allocation reduces to offset arithmetic;
  an address in another live ordinary allocation lies outside this one's
  range, except one past the end, where the next allocation may begin;
  the address of a dangling pointer may equal that of a live one.
- `ptrEq` (`==` on raw pointers, `ptr::eq`) compares addresses, and the
  lengths of slice pointers; it is never UB. Two pointers into one
  allocation are equal exactly when their offsets are; pointers into two
  live ordinary allocations are unequal unless one is one past the end of
  its allocation. A pointer escaped from a reference points into the
  lender's allocation (§4.5), so it compares exactly with `&raw mut x` and
  with every other pointer into it; a dangling pointer compared with a
  live one, or a pointer into a shadow compared with any other, is
  undecided.
- `memory` is append-only; death flips `live`.

### 4.2 Operations

A new shared group, admitted only under the unsafe profile (the raw
checker rejects it elsewhere):

```lean
inductive RawOperation where
  | addrOf (mutable : Bool) (place : PlaceId)  -- &raw: anchor the place, yield rawPtr
  | load                                       -- checked typed read through rawPtr
  | store                                      -- checked typed write through rawPtr
  | offset                                     -- p.add n: n · size T bytes; byte_add: n bytes
  | offsetFrom                                 -- p.offset_from q, same allocation
  | cast (pointee : TypeId)                    -- p.cast::<U>(): same pointer, new view
  | alloc (type : TypeId)                      -- fresh uninitialized typed allocation
  | allocBytes                                 -- alloc::alloc(layout): fresh byte allocation
  | dealloc
  | copy (overlapping : Bool)                  -- ptr::copy / copy_nonoverlapping
  | fill                                       -- ptr::write_bytes
  | ptrEq
  | addr                                       -- p.addr(), p as usize
  | withAddr                                   -- p.with_addr(a): same provenance, new offset
  | isValid                                    -- decides the access premises; itself never UB
  | mint (mutable : Bool)                      -- &*p / &mut *p and interval forms: a memory loan
```

Raw access is operation-based only: `Place.deref` stays reserved for
references, and raw pointers never appear in place paths. The frontend
lowers `(*p).f = x` to `offset` by `f`'s layout offset, `cast` to `f`'s
type, and `store`; `addr_of_mut!((*p).f)` is the same without the store.

**What an access touches.** A pointer's pointee type `T` and byte offset
`o` name the memory an access reads or writes. In a byte allocation it is
the bytes `o .. o + size T`. In a typed allocation of type `A` it is the
*subvalue* of `A` at `(o, T)`: the whole value when `o = 0` and `T = A`,
otherwise the field or array element reached by following the layout
facts (§4.4) from `A` through nested fields, active enum variants, and
elements to a component of type `T` at offset `o`; a step into an enum
follows the active variant of the value currently in the allocation (and
fails if the enum is uninitialized or in another variant), and a path that
enters a union continues as a byte view of the union's bytes. Zero-sized
components at the same `(o, T)` carry the same unit value. A field access
therefore touches only its field and does not conflict with a live
`&mut (*p).g` on a sibling field (§4.6). When `(o, T)` names no subvalue
(`&raw mut x as *mut u8` for `x : u64`), the access is UB in the model,
which is stricter than Rust (§7.3, §9).

**Premises.** For `store` through `rawPtr (some a) o meta true via` they
are: `a` live (else use-after-death); the accessed memory inside the
allocation (bounds, using `meta` for a slice pointer); for a typed
allocation, `(o, T)` names a subvalue; for a byte allocation, the
alignment and encoding premises of §4.4; and the provenance premises of
§4.6: no foreign live loan on the memory, and a pointer derived through a
loan still valid for it. `load` adds that the subvalue or bytes are
initialized and form a valid value of `T`. `allocBytes` requires
`size > 0` and `align` a power of two. A failed premise derives
`undefined`; `Outcome` gains `| undefined (cause : UbCause)`, and wp and
contracts quantify only over `returned`/`threw`, so absence of UB is a
conjunct of every proof. Language-level `throw` never models UB and vice
versa.

`copy` moves `n` elements' worth of bytes between two pointers of the
same pointee type, provenance included and uninitialized bytes as they
are (Rust copies them, so the model does); its premises are liveness and
bounds of both ranges, the loan premises of a load on the source and a
store on the destination, and, for `copy_nonoverlapping`, disjoint
ranges. Into a typed allocation the bytes are decoded at the pointee
type; when the decode fails, the subvalue becomes uninitialized, so the
UB is at a later read, as in Rust. `fill` stores a repeated integer byte
the same way. `offsetFrom` requires both pointers in one allocation.
`cast` is a pure value operation: the view changes, the pointer does not.
`addr` yields `base + offset`; `withAddr` keeps the provenance and sets
the offset to `a - base`, so an access through the result needs the new
offset back inside the allocation. `align_offset` and `is_aligned` are
integer functions of the address.

### 4.3 Allocation death

Allocation death parallels loan death; both are explicit events in the
one semantics. An allocation dies at (1) `dealloc p`, requiring a live,
offset-zero, mutable pointer into a heap allocation (one from `alloc` or
`allocBytes` whose `(size, align)` matches the `dealloc` layout and has no
live loan in `memoryLoans`; ownership is the library model's convention,
§6), (2) the end of its anchored place's scope (MIR's `StorageDead`) or
the exit of the frame holding the place, or (3) the death of the loan a
shadow stands in for (§4.5). A
pointer to a local that escaped its function is therefore dangling *by
the semantics of return*: stack use-after-death needs no special
mechanism. Dead allocations are not cleaned up behind outstanding
pointers; every §2 misuse is a premise violation against this state.

A move out of an anchored place is a checked `load` through the anchor;
afterwards the allocation's content is uninitialized, so a read through a
pointer to the moved-from place is UB, which is stricter than Rust for
types whose stale bytes are still valid (§7.3). A later assignment to the
place is a `store` through the anchor and initializes it again.

A bump arena carved out of one `allocBytes` is one allocation: every
access inside its bounds is defined, whatever object the arena's own
bookkeeping says lives there. Object boundaries inside an arena are the
program's invariants, stated and proved as functional properties, not as
memory safety.

### 4.4 Typed views of bytes: layout facts, `transmute`, unions

**Layout facts.** A type has a *layout* when the exporter can state, for
the pinned compiler and target, how its values are laid out in bytes:
size and alignment; for integers the width and the target's endianness;
`bool` as one byte 0/1; `char` as four bytes in its valid range; for raw
and non-null pointers the pointer width, plus the length word of a slice
pointer; for arrays the element layout and stride; for structs the offset
of each field and the padding between them; for enums the tag's offset,
width, and value per variant, or its niche. These are rustc's own layout
results, read through the public API the exporter already uses,
serialized in RawUnit as a versioned table, and checked by validation for
internal consistency (fields inside the size, no overlap, alignment
dividing the size). They are trusted facts, listed in the assumption
ledger ([`assumptions.md`](../status/assumptions.md)): a theorem that
relies on a layout holds for the pinned target and compiler. Types
holding references or closures have no layout, because prophetic values
have no byte form (§7.2). Verification is monomorphic (§3), so every type
a unit views has a concrete layout.

**Encode and decode.** Two functions, defined from the layout facts,
connect values and bytes:

```lean
encode : Layout → RuntimeValue → Array AbstractByte          -- padding bytes are uninit
decode : Layout → Array AbstractByte → Option RuntimeValue   -- none: not a valid value
```

`decode` fails, and the operation is UB, when a non-padding byte is
uninitialized (except under a union or `MaybeUninit` view, which keep the
bytes as they are) or when the bytes are not a valid value of the type
(`bool` outside 0/1, a tag naming no variant, a `char` out of range).
Encoding a thin pointer yields pointer-width `AbstractByte.ptr` bytes;
encoding a slice pointer (`meta = some len`) yields the thin pointer's
`AbstractByte.ptr` bytes followed by the `usize` integer bytes of `len`,
so reading the data pointer or length word separately is defined.
Pointer bytes read at an integer type decode to the bytes of the
pointer's address, with the provenance dropped, as `addr()` does. Bytes
read at a pointer type decode to their pointer when all of the data-pointer
bytes belong to the same thin pointer in order (together with a valid
length word for a slice pointer); any other initialized bytes decode to a
pointer without provenance at the address they spell (§4.1), which is a
value, and UB to access.

**Access to a byte allocation.** A `load` at pointee type `T` and byte
offset `o` requires: the allocation live; `o + size T` within its size;
the address `base + o` a multiple of `align T`, unless the access is
unaligned (`read_unaligned`, `write_unaligned`, `copy`); and `decode T`
of the bytes at `o` succeeding, whose result is the value read. A `store`
encodes the value and overwrites `size T` bytes at `o`. The alignment
premise is a fact about the actual address, as in Rust. A proof
discharges it without knowing the address when `o` is a multiple of
`align T` and `align T` divides the allocation's requested alignment,
which is the usual case; otherwise from a path condition the program
established itself (`p.is_aligned()`, `p.align_offset(..) == 0`).
`size_of::<T>()` and `align_of::<T>()` in the program are the same facts
as constants.

**`transmute::<T, U>(v)`** is `decode U (encode T v)`: a value-level
reinterpretation whose only obligation is that the decode succeeds. rustc
already ensures `size T = size U`. A pointer-to-integer transmute yields
the address. An integer-to-pointer transmute yields a pointer without
provenance at that address, as a load of integer bytes at a pointer type
does: a value, and UB to access, as in Rust.

**Unions** are values of the form `bytes` with the union's size. Reading
field `f : T` decodes `T` at `f`'s offset (so an uninitialized or invalid
field read is UB, as in Rust); writing `f` encodes the value over `f`'s
bytes and leaves the others unchanged. `MaybeUninit<T>` is the union of
`()` and `ManuallyDrop<T>`, so `MaybeUninit::uninit()` is all-uninit
bytes, `write` is a field store, and `assume_init` is a field read with
its validity obligation: no well-known type is needed for it.

### 4.5 Boundary with the prophetic model

**Taking the address of a place.** `addrOf` (Rust's `&raw`) on a place
`x.path` anchors the root local `x` if it is not anchored yet (moving `x`'s
value into a fresh allocation `a` and leaving `rawAnchor a` in `x`) and
yields `rawPtr (some a) o meta mut none`, where `o` is `path`'s byte offset
from the layout facts (`0` for `x` itself). The place stays usable by safe
code: the frontend turns a read of an anchored place (or a sub-place of
one) into a `load` at its offset through the anchor, an assignment into a
`store`, a move into a `load` that leaves the subvalue uninitialized
(§4.3), and `&x.path`/`&mut x.path` into a `mint` (§4.6). Safe code and raw
aliases thus share one memory cell and observe each other correctly, and
`&raw mut x.f` equals `(&raw mut x).byte_add(o)`. A root local is anchored
at most once; the anchor dies with the local (§4.3).

**Taking a pointer from a reference.** A reference is a value, not an
address: a mutable borrow holds its current value, and a shared borrow is
a snapshot. An *escape*, `&raw mut *r`, `&raw const *r`, `r as *const T`,
and the APIs that do the same inside (`<[T]>::as_mut_ptr`, §6), yields a
pointer into the *lender's memory* with *provenance by loan*: `rawPtr
(some a) o meta mutable (some ℓ)`, where `ℓ` is the loan of `r`, `(a, o)`
the position of the cell `ℓ` lends by the layout facts, and `mutable` from
the reference. The lender is known when the borrow analysis can name the
cell: the chain of reborrows behind `r` roots at a sub-place of a place of
the current frame, or at a memory cell (a minted reference, §4.6).

- *Lender a place of the frame.* The escape anchors the root local if it is
  not anchored yet, exactly as `addrOf` does, and the live loans of the
  place become memory loans (§4.6) on the cells of the allocation: a loan
  of `x.f` lends the subvalue `f`. Nothing changes for the lender: the
  loan's death reconciles with the cell, which is the memory the anchor
  rewrite reads, and its prophecy is the cell's final content
  (`memory'[cell] = π ℓ`), as for a minted loan. The pointer is therefore
  equal to `&raw mut x.f` exactly.
- *Lender a memory cell.* The pointer is the cell's position with `via =
  some ℓ`.
- *A shared reference.* A safe shared borrow is a snapshot with no loan,
  so the escape registers a freeze loan on the cell for the reference's
  region, as `mint false` does (§4.6), and the pointer is read-only with
  that loan as `via`.

**The escaped state.** At the escape of `r : &mut T` with loan `ℓ`, the
current value of `r` is written into the cell and `ℓ`'s escape becomes
*open*: the cell holds the value, and the references on the chain from
the cell to `r` hold stale currents. While the escape is open or frozen
(below):

- a read through `r`, or through an ancestor on the chain once `r` is
  dead, first re-reads the cell, with the premise that it is initialized
  and a valid `T` (a `copy` may have left it otherwise); the escape stays
  open;
- a write through `r`, and a reborrow of `r` (`&mut *r`, including the
  implicit reborrow when `r` is passed to a call or returned), closes the
  escapes of `ℓ` and of its descendants: the current becomes the cell's
  content (with the premise that the cell is initialized and a valid `T`,
  unless the write overwrites `*r` in full at a type without drop glue),
  the cell is exclusively lent again, and every pointer derived through a
  closed loan is invalid, so an access through it is UB;
- `r`'s death leaves the escape open and the value in the cell, where the
  lender finds it: the reconciliation of `ℓ` reads the cell, with the
  premise that the cell is initialized and a valid `T` (so a native
  prophecy `π ℓ : NTy.carrier T` always exists on non-UB paths and cannot
  make the continuation vacuously true), and `pending` and `applyPending`
  are untouched, since a loan of a frame's place dies in that frame and a
  minted loan returned by a function keeps its registry entry, escape
  included, as before (§4.6). The pointers stay usable under the provenance
  rules of §4.6: a *foreign write* to the cell (by safe code through the
  anchor, by a pointer not derived through `ℓ`, by an ancestor reference)
  closes the escape, and a *foreign read* makes it *frozen* once the loan
  has been written: reads through its pointers remain legal, writes are UB.
  A loan is *written* from the first write through its reference, a
  reborrow of it, or a pointer derived through it or a descendant; a
  foreign read of a loan never written leaves the escape open, as Tree
  Borrows keeps such a reference *reserved*.

A later escape of `r` reopens `ℓ`'s escape, and with it every pointer
derived through `ℓ` before the close, as both aliasing models do. A
pointer derived through `ℓ` may be offset anywhere in the allocation, but
accesses only `ℓ`'s cell: outside it, it has no permission, like the
reference (§4.6).

These rules are the conservative core of Stacked and Tree Borrows for a
pointer derived from a reference: a write through the reference or a
reborrow of it (Stacked Borrows) and a foreign write (both) invalidate the
pointer, a read through the reference keeps it, and a foreign read
leaves it readable but freezes it once the loan was written (Tree
Borrows; Stacked Borrows never freezes raw pointers). Where the model is
stricter than one of the two, §7.3 says so.

**Shadows.** A reference whose lender the frame cannot name, a parameter,
a reference returned by a call that is not a mint (§6), or one read from a
data structure, escapes into a *shadow allocation*: a fresh allocation of
the referent's type, marked `shadow` so that its address is unconstrained
(§4.1), standing in for the unknown cell. `ℓ` becomes a memory loan on the
shadow and everything above applies, except that the memory is the
caller's, so the escape cannot outlive the loan: the shadow dies at the
close or at `ℓ`'s death, its content becoming the current value (with the
obligation that it is initialized and a valid `T`) that the ordinary death
or `pending` export carries to the lender, and every pointer into it
dangles. A `&'static T` yields a shadow that never dies. Shadows are why
escaping APIs enter by contract (§6) and the source of the §7.3 rows on
parameters.

Pointers have no death point and no exclusivity, so they get no
prophecies, and reference contracts need nothing new.

### 4.6 Minted references

The converse direction, `&*p : &T` and `&mut *p : &mut T` with their path
and interval forms (`&mut (*p).f`, `slice::from_raw_parts_mut`,
`NonNull::as_mut`), is a loan whose lender is a memory *cell*: an
allocation and either a byte interval (byte allocation) or a subvalue
path (typed allocation, §4.2), the `borrow_global_mut` precedent with a
dynamic key. rustc does not check the minted reference against raw
accesses, and its lifetime is not constrained by the pointer, so the model
checks it:

```lean
-- RuntimeState: loan registry beside globalLoans
structure MemoryLoan where
  cell      : MemoryCell        -- alloc, byte interval or subvalue path
  exclusive : Bool              -- &mut: exclusive loan; &: freeze
  parent    : Option LoanId     -- the loan this one reborrows or was minted through
  escape    : EscapeState       -- closed, open, or frozen (§4.5)
  written   : Bool              -- written through its reference or a derived pointer (§4.5)

memoryLoans : List (Nat × MemoryLoan)
```

The registry holds every loan whose lender is in `memory`: minted loans,
and the loans of an anchored place (§4.5). An entry outlives its loan
while its escape is open or frozen.

- `mint true` (`&mut *p`): the load premises (live, in bounds,
  initialized and valid) plus *no live foreign loan intersecting the
  cell* (below). It reads the value `v`, records `ℓ ↦ cell` (`exclusive =
  true`) in `memoryLoans` (which enforces the hole's exclusivity on `cell`
  without placing a `loanHole` into `AbstractByte`s), and yields
  `borrow ℓ v` with prophecy `π ℓ`. When `ℓ` dies with its escape closed,
  death reconciles `current = π ℓ`, requires the allocation live, and—when
  the loan was written through its reference—writes `current` (encoded via
  `encode` in a byte allocation, or per written sub-place) back to `cell`;
  leaving an unwritten borrow's bytes in place preserves pointer provenance
  and struct padding in byte allocations. When `ℓ` dies with its escape
  open or frozen, death reconciles `π ℓ` with the cell's decoded value
  (§4.5). A dereference of a minted borrow likewise requires the allocation
  live, so the UB is located at the use rather than at the death point.
- `mint false` (`&*p`): the load premises plus no intersecting exclusive
  loan. The result is a snapshot as for safe shared borrows, and the mint
  records a *freeze* loan `ℓ ↦ cell` (`exclusive = false`) in `memoryLoans`
  whose death point covers every copy (the loan's lifetime closure). A safe
  shared borrow needs no loan because rustc proves the lender immutable;
  nothing proves it for a minted one. Because `dealloc` and anchor death
  require no live loan (exclusive or freeze) in the allocation, freeing or
  exiting the frame of an allocation while a minted `&T` or `&mut T` still
  has a later use fails at the death of the allocation.
- **Provenance.** A pointer is *derived through* `ℓ` when its `via` is
  `some ℓ`; `offset`, `cast`, `withAddr`, `copy`, and storing and
  reloading a pointer keep `via`. A mint through a pointer derived through
  `ℓ` records `parent := ℓ`, a reborrow records the loan it reborrows, and
  the loan of a sub-place (`&mut r.g`) lends the sub-cell. The loans
  *foreign* to an access through a pointer with `via = v` are the loans
  intersecting the accessed cell other than `v` and its ancestors (all of
  them for `v = none`); an access through the reference of `ℓ` counts as
  `via = some ℓ`, except that a write or reborrow through it also counts
  `ℓ`'s own escape as foreign (§4.5). Two cells intersect when their
  intervals overlap or one subvalue path is a prefix of the other, so
  `split_at_mut` and disjoint fields mint legally.
- **Raw premises.** A `load` requires no live exclusive foreign loan; a
  `store`, `fill`, the destination of `copy`, and `mint true` require no
  live foreign loan of either kind; `dealloc` and anchor death require no
  live loan in the allocation. A pointer with `via = some ℓ` additionally
  requires `ℓ`'s escape open or frozen, open for a store, and the access
  inside `ℓ`'s cell. Effects: a store (and `mint true`) closes the escapes
  of the foreign loans, and a load (and `mint false`) freezes their open
  escapes that are written; since a live foreign loan makes the access
  UB, these are dead loans whose pointers are still in use. A store
  through a pointer with `via = some ℓ`, or through `ℓ`'s reference, marks
  `ℓ` and its ancestors written.
- **Static and symbolic.** Death points are static, so the live-loan
  premises are offset and interval disequalities against the statically
  known live loans: integer arithmetic, like the bounds premise. While a
  loan is live every foreign access to its cell is UB, so its escape state
  changes only at static points, the escape and the uses of its reference,
  and whether a read through a reference re-reads the cell (§4.5) is
  static too. After the loan's death the state changes at raw accesses
  whose targets may be symbolic, so the registry's escape states are
  spec-visible like liveness (§5).
- Death points come from the borrow analysis like any loan's, and minted
  loans die **non-lexically**: at the reference's last use, not at the
  end of its scope. Lexical deaths would be sound but would reject
  idiomatic unsafe code (`Vec::push`, linked lists), where the pointer is
  used again after the reference's last use. A minted reference (`&mut T`
  or `&T`) returned by a function dies in the caller, whose region for it
  is fixed by the signature or inferred from its uses, as for any
  reference-returning function; the callee excludes the loan from its exit
  deaths, and the registry entry persists and is rekeyed on reborrow like
  `globalLoans`. Argument loans span the call, which yields the protectors
  of Stacked Borrows.
- Fidelity: with non-lexical deaths a live loan is one whose reference is
  used later, so a failed premise matches the Stacked and Tree Borrows
  rule that a foreign access followed by a use is UB. The model is
  stricter than Tree Borrows, where a foreign read freezes an active
  `&mut` instead of failing: sound, incomplete (§7.3). After the loan's
  death the escape states of §4.5 are Tree Borrows' states for the loan's
  node, open for reserved (not written) and active (written), frozen,
  closed for disabled, with Stacked
  Borrows' rule that a write or reborrow through the reference invalidates
  the pointers derived from it.

The prophetic model is unchanged: a minted mutable borrow is an ordinary
`borrow ℓ v`, its prophecy flows through memory at reconciliation
(`memory'[cell] = π ℓ`), and a minted shared borrow carries its freeze
loan `ℓ` in `BorrowCertificate` and `memoryLoans`.

## 5. Verification and validation

- The weakest precondition gains one arm per raw operation, conjoining
  its located safety premises with the continuation. The spec-visible
  predicates `memory.isLive` / `memory.initialized` / `memory.lent` /
  `memory.escape` follow the `holeInGlobals` precedent (constructor-keyed
  guarded scans, safe for symbolic states). `decode` on symbolic bytes is unfolded only
  against the layout of the type read, so a load of a `u64` from a byte
  allocation produces an eight-byte condition, not a scan of the buffer.
  Address facts enter as the placement facts of §4.1, so a proof never
  sees a concrete address.
- Generated contracts export a raw-pointer parameter's assumptions as
  preconditions (liveness, bounds, initializedness, validity) and its
  memory effects as postconditions: an unsafe `fn` is exactly a function
  whose contract carries safety preconditions its caller must discharge.
  A mint-returning unsafe function exports the mint premises as
  preconditions, registers the returned result's loan `ℓ ↦ cell` in
  `memoryLoans'` for the caller's region of the result (both for
  `&mut`-returning functions like `as_mut` and `from_raw_parts_mut` and
  for `&`-returning functions like `as_ref`, `from_raw_parts`, and
  `as_ref_unchecked`), and for `&mut` exports `memory'[cell] = π result` at
  the loan's reconciliation.
- Validation admits the §4 operations only under the unsafe profile and
  rejects the constructs of §7.2 with located diagnostics. The exporter
  serializes or rejects every unsafe-related MIR form and emits the layout
  table of §4.4 for the types the unit views through pointers; the mapper
  lowers the admitted subset, and RawUnit carries the type, the operation
  group, and the layout table as a versioned extension. The borrow
  analysis certifies the anchored places (licensing the anchor rewrite of
  safe accesses), the escape sites with their lenders, the reads through
  references that re-read the cell and the writes and reborrows that close
  an escape (§4.5), and the non-lexical deaths of minted loans, including
  the freeze loans of minted and escaped shared references.

## 6. Library models

Heap ownership and pointer-based data structures enter through library
models verified once against this semantics. They export pure
value-level contracts, so their clients verify without seeing memory
predicates; the interior-mutability design reuses the pattern for
`UnsafeCell` caches.

**The allocation API.** `std::alloc::alloc(layout)` is `allocBytes`
(requiring `layout.size() > 0` and `layout.align()` a power of two) or
null (the allocator may fail), so a proof covers both results; a null
branch that calls `handle_alloc_error` diverges and needs no further
proof. `dealloc` is `dealloc` with the matching layout `(a.size, a.align)`
as a premise. `Layout` values are plain data. `NonNull<T>` is a newtype
over `*mut T` whose `new_unchecked` has the non-null precondition and
whose `as_ref`/`as_mut` are mints; `NonNull::dangling()` is a pointer
without provenance (§7.3).

**Owned heap memory.** A `Box`-class model owns a typed allocation and
exports the contract of the value it holds; `Box::into_raw` hands the
allocation to the pointer, and `Box::from_raw` takes it back with
liveness and single-ownership as preconditions, so a double free is a
failed liveness obligation at the second `from_raw`. A `Vec`-class model
(`Vec`, `String`) owns a byte allocation that it views at its element
type, so a partly initialized buffer is representable; `as_ptr` and
`as_mut_ptr` escape the receiver borrow `&self` / `&mut self` into that
allocation (`via = some ℓ`, remaining usable after the temporary receiver
borrow dies until the vector is next accessed as a value, reallocated, or
dropped, §4.5), and a reallocation or drop kills the allocation, so
pointers taken before dangle, as in Rust. A method returning a reference
(`Deref`, `Index`, `get_mut`) is a mint with its loan (§4.6), so a raw
write into the buffer while a slice of it is used is a failed loan
premise. What these models need beyond §4 for pointers with `via = none`
is the guarantee that no stray pointer writes their allocation (§9,
framing).

**Escaping APIs.** An API that returns a pointer derived from a reference
parameter (`<[T]>::as_ptr`, `<[T]>::as_mut_ptr`, `NonNull::from`,
`ptr::from_mut`) enters as a contract applied at the call site, where the
argument's lender is visible: the result is the escape of the argument
(§4.5), its `via` the argument's loan and its position the lender's cell
moved by a stated byte offset (zero for these, a field's offset for an
accessor), and the escape opens at the call. The body cannot establish
this contract, because inside it the parameter's lender is a shadow
(§4.5), so it is a stated model like `alloc`'s, and a user-written
function of this shape is declared with the same contract rather than
verified. A function that only uses the pointer inside its body is
verified against the shadow and needs no declaration.

**Arenas.** An arena that carves objects out of a byte buffer is, in the
model, a byte allocation its clients access through typed views (§4.4),
or, when it is an external crate whose internals are not verified, a
library model whose `alloc(&self, v: T) -> *mut T` is a fresh allocation
that dies with the arena. A typed arena (one element type per chunk) is
a typed allocation once elements are individually initialized.

**`StoredPtr<T>`: dynamic checking.** For pointers stored in data
structures there are exactly two options: (a) static discharge, where
invariants constrain the stored pointers so the §4 premises are provable
at each access; (b) dynamic checking. The design makes (b) a well-known
library type, named for where it is needed: `StoredPtr<T>` wraps a raw
pointer and stores what checking needs. In the model that is nothing
extra, since `isValid` decides the access premises against `memory`, and
its methods are ordinary code: `read p` is
`if isValid p { load } else { panic }`.

- UB is unreachable through a `StoredPtr` by construction: its accesses
  contribute no `undefined` obligations, only a defined panic outcome. A
  spec may accept the panic or prove its absence (option (a) again, but
  local and optional). Data structures of `StoredPtr` need no
  memory-safety invariants.
- Real hardware stores no liveness metadata, so the compiled counterpart
  must genuinely carry it: a generational handle, allocation id plus
  epoch against an arena or registry, slotmap/`Weak`-style, verified once
  against the raw model. A `StoredPtr` theorem transfers to compiled Rust
  exactly when that realization is used; `isValid` on a bare `*mut T` is
  not realizable.

## 7. What is allowed

Three answers exist for a construct. *Supported*: the verifier accepts it
and asks for the proof obligations listed, which is also what the
interpreter checks at run time. *Rejected*: validation stops with a
located diagnostic naming the construct; the program never reaches the
semantics. *Stricter than Rust*: the construct is accepted, but the model
derives UB for some executions Rust defines, or cannot prove a fact Rust
guarantees, so a proof fails that should have succeeded. Strictness keeps
verification sound but is silent incompleteness, so every case is named
in §7.3 and measured.

### 7.1 Supported

| Construct | Proof obligations |
|---|---|
| Creating, copying, comparing pointers; `is_null`; `ptr::null_mut()`; `wrapping_add` | none (§4.1) |
| `&raw const x`, `&raw mut x`, `addr_of!`, `addr_of_mut!` of a local or owned place | none; the root local is accessed through its anchor afterwards, and the allocation dies with the local (§4.5, §4.3) |
| `&raw mut *r`, `&raw const *r`, `r as *const T`, `<[T]>::as_mut_ptr()` for a reference `r` whose lender the frame can name: a borrow of a place of the frame, a minted reference, a reborrow of either | none at the escape; the pointer points into the lender's memory and stays usable until `r` is next written through or reborrowed (passed to a call, returned), or, after `r`'s death, until the memory is written through another path; once another path reads it, only reads remain; the cell is initialized and valid at `r`'s next read, close, or reconciliation (§4.5) |
| The same for a reference whose lender the frame cannot name: a parameter, a reference returned by a call that is not a mint, a reference read from a data structure | none at the escape; the pointer is usable until the next use of `r` or `r`'s death, whichever is first, and the value is initialized and valid then (shadow, §4.5, §7.3) |
| `*p`, `p.read()`, `p.write(v)`, `*p = v`, `ptr::read`/`ptr::write`, `(*p).f`, `addr_of_mut!((*p).f)` into a typed allocation | allocation live; the pointer's offset and type name a subvalue (the whole value, a field, an element, following active enum variants) inside it; initialized (read); writable (`*mut`); no intersecting minted loan (§4.2, §4.6) |
| The same through a pointer into a byte allocation, including `p.cast::<U>()`, `p as *mut U` | the above, with: `o + size U` within the allocation; the address a multiple of `align U`, usually from `o` a multiple of `align U` dividing the allocation's alignment; the bytes decode as a valid `U` (§4.4) |
| `read_unaligned`, `write_unaligned` | as above without the alignment obligation |
| `p.add(n)`, `p.sub(n)`, `p.offset(i)`, `byte_add`, `offset_from` | the result inside the allocation (one past the end allowed); `offset_from` on pointers of one allocation |
| `std::alloc::alloc(layout)`, `alloc_zeroed`, `dealloc` | `alloc`: `layout.size() > 0` and `layout.align()` a power of two, and the result may be null and must be checked; `dealloc`: pointer live, offset zero, mutable, matching `(size, align)` layout, no live loan in the allocation (§4.3, §6) |
| `Box::new`, `Box::into_raw`, `Box::from_raw`, `Box::leak`; `Vec::as_ptr`, `Vec::as_mut_ptr` | `from_raw`: allocation live and not already owned; a `Vec` pointer: the vector not accessed as a value, reallocated, or dropped since (§6) |
| `copy_nonoverlapping`, `ptr::copy`, `write_bytes` | both ranges live and in bounds; destination writable and unlent; source not exclusively lent; `copy_nonoverlapping`: ranges disjoint (§4.2) |
| `transmute::<T, U>(v)`, `from_ne_bytes`/`to_ne_bytes` and friends | `T` and `U` have layouts; the bytes of `v` decode as a valid `U` (§4.4) |
| Unions: declaring, writing a field, reading a field; `MaybeUninit<T>` | the field read decodes as a valid value: initialized bytes, valid tag, 0/1 for `bool` (§4.4) |
| `size_of::<T>()`, `align_of::<T>()`, `Layout::new`, `Layout::from_size_align` | none; `T` has a layout |
| Reading addresses: `p.addr()`, `p as usize`, `expose_provenance`, `transmute::<*mut T, usize>`; comparing and subtracting them (`p < q`, `p as usize - q as usize`); `align_offset`, `is_aligned` | none; the result is the address of §4.1, known through the placement facts. Exposure is unobservable, since no integer becomes a pointer again |
| Moving a pointer within its provenance: `p.with_addr(a)`, `p.map_addr(f)`, tag bits in low address bits | none at the operation; an access through the result needs its address back inside the allocation (§4.2) |
| `&*p`, `&mut *p`, `&(*p).f`, `&mut (*p).f`, `slice::from_raw_parts`, `from_raw_parts_mut`, `NonNull::as_ref`/`as_mut`, `ptr.as_ref()` | the load obligations of the cell; `&mut`: no live minted loan intersecting the cell; `&`: no live exclusive one; the allocation live at every use of the reference (§4.6) |
| Raw accesses while a minted reference, or a reference a pointer escaped from, is live | the access is through a pointer derived from the reference's loan, or does not intersect its cell: disjoint intervals or fields (§4.6) |
| `NonNull<T>`, `NonNull::new`, `new_unchecked`, `NonNull::dangling()` | `new_unchecked`: the pointer is non-null; `dangling()`: never accessed (§6, §7.3) |
| `unsafe fn` and `unsafe` blocks | an `unsafe fn` exports its safety conditions as preconditions; an `unsafe` block proves its operations' obligations from the enclosing precondition and path conditions (§5) |
| `unsafe impl Send`/`Sync` | none: the theorems speak about single-threaded executions, and thread safety is not verified |
| Pointers stored in data structures (parent links, free lists, intrusive lists) | either invariants from which the access obligations follow, or `StoredPtr<T>` with its panic outcome (§6) |

A pointer into a bump arena carved out of one allocation is in bounds
whenever it is inside the arena's buffer (§4.3); object boundaries inside
the buffer are functional properties the program states itself.

### 7.2 Rejected at validation

Each row is a located diagnostic naming the construct.

| Construct | Why |
|---|---|
| Integer-to-pointer casts: `a as *mut T`, `with_exposed_provenance`, and so reading back a pointer stored as an integer (`usize::from_ne_bytes(..) as *mut u8`) | strict provenance: an integer names no allocation for the pointer to be valid in (§3). A pointer kept as data is stored as a pointer, through a `*mut *mut T` view, which keeps its provenance (§4.1) |
| A typed view (`cast`, `transmute`, union field) at a type without a layout: one holding references or closures | prophetic values have no byte form (§4.4) |
| Raw pointers in place paths (`(*p).f = x` as a place) | the frontend lowers these to raw operations; a residual one is a frontend bug (§4.2) |
| `UnsafeCell`, `Cell`, `RefCell`, `Mutex` internals, `static mut` | interior mutability (`UnsafeCell`, `Cell`, `RefCell`) is covered by [`interior-mutability.md`](interior-mutability.md); `Mutex` internals and `static mut` remain rejected |
| FFI (`extern` functions and blocks), inline assembly, atomics, `#[global_allocator]` | no foreign-call contracts and no concurrency model; allocators are the trusted `alloc` model |
| Copying or moving a value that holds a `rawAnchor` | an anchor is pinned to its place (§4.1) |

### 7.3 Stricter than Rust

The verifier accepts these constructs, and the model reports UB in
executions Rust defines, or cannot prove a fact Rust guarantees. A proof
that fails on one of them is not evidence of a bug in the program.

| Construct | Rust | The model |
|---|---|---|
| A read through another path (the minting pointer, the lender, a foreign pointer) while a minted `&mut *p`, or a `&mut` a pointer escaped from, is still used later | defined under Tree Borrows: the reference is frozen, and only later writes through it are UB | UB at the read (§4.6); Tree Borrows' states apply only after the loan's death (§4.5) |
| A pointer escaped from `r : &mut T` used after a write through `r` or a reborrow of `r` | defined under Tree Borrows, where the pointer shares `r`'s tag; UB under Stacked Borrows | UB: the write or reborrow closes the escape (§4.5) |
| A pointer derived from a reference whose lender the frame cannot name (a parameter) and kept past the reference's next use or death | defined: the pointer stays usable until the memory is written through another path | the shadow dies with the loan, so later accesses are use-after-death (§4.5); a function returning such a pointer is declared by the §6 contract |
| Address comparison between a pointer into a shadow and another pointer | equal or unequal as the addresses are | undecided: the shadow's address is unconstrained (§4.1); a proof relying on it fails |
| Viewing a typed allocation at a type that is not one of its subvalues: `&raw mut x as *mut u8` for `x : u64`, `write_bytes` over a typed node, a `u64` read across two `u32` fields | defined: memory is bytes | UB at the access (§4.2); the conversion of §9 is the extension |
| Reading a moved-from anchored place through its pointer | defined for a type whose bytes are still valid | UB: the move leaves the allocation uninitialized (§4.3) |
| Zero-sized accesses through a pointer without provenance (`NonNull::dangling()`, the buffer of an empty `Vec`) | defined: zero-sized accesses need no allocation | UB (§4.1); a non-null provenance-free pointer with defined zero-length accesses is an extension |
| Anchor-gated access, if adopted (§9) | defined: any pointer with provenance may access | UB for a pointer to memory another value owns; `StoredPtr` (§6) is the route |

**Measuring strictness.** Every Rust fixture of the unsafe profile also
runs under Miri with Tree Borrows, rust-mir-design's differential test.
UB that Miri reports and the interpreter does not is a soundness bug of
the model. UB that only the interpreter reports must be explained by a
row above; an unexplained one is a missing row or a bug.

## 8. Unsafe code in MonoVM

MonoVM (`third_party/move/mono-move`) keeps Move values in raw byte
memory: frames and a heap with a copying collector, both carved out of
regions from `alloc::alloc`. Its unsafe code falls into the idioms below,
named by representative places. *Supported* means verifiable under §7.1
with the obligations listed there; *rewrite* means rejected or UB as
written, but supported after a local change that keeps the behavior;
*library model* means the code is not verified but enters through a
stated contract (§6).

| Idiom (where) | Verdict | What verification asks |
|---|---|---|
| Memory regions from `alloc::alloc` / `alloc_zeroed` at `MAX_ALIGN`, freed in `Drop` (`MemoryRegion::new`, `alloc/src/memory_region.rs`): frames, heap spaces | supported | each region is a byte allocation (§4.1); the null result aborts through `handle_alloc_error` (§6); `Drop`'s `dealloc` requires no slice over the region still in use |
| Typed reads and writes at byte offsets: `read_u64`, `write_u64`, `read_u32`, `read_ptr` (`core/src/memory.rs`), object header fields (`runtime/src/memory.rs`), frame slots (`runtime/src/interpreter.rs`), the BCS and comparison walks (`runtime/src/value_conv/bcs.rs`, `runtime/src/value_cmp.rs`) | supported | typed views of a byte allocation (§4.4): in bounds; bytes initialized, so a `new_uninit` or `new_scratch` region read before written fails here; a valid value of the type; the address a multiple of the type's alignment, which follows from the offset and the region's `MAX_ALIGN` alignment |
| Unaligned variants: `read_account_address`, `read_unaligned` in `natives/src/aggregator_v2.rs` and `natives/src/ristretto255_point.rs`, the 64-bit slot moves of the interpreter, `read_array` in `runtime/src/value_cmp.rs` | supported | as above without the alignment obligation |
| Generic `read_int<T: Copy>` / `write_int<T>` (`runtime/src/interpreter.rs`) and `VMValue::read_from_frame` / `write_to_frame` (`core/src/native/value.rs`), choosing `read` or `read_unaligned` by `align_of::<T>() <= MAX_ALIGN` | supported | verified per instantiation (`u8` to `U256`, `I256`, `[u8; N]`) (§3); the branch on `align_of` is a constant in each |
| Pointers stored in memory: `read_ptr` / `write_ptr`, `read_fat_ptr` / `write_fat_ptr` (`core/src/memory.rs`), object fields, forwarding pointers (`runtime/src/memory.rs`), frame metadata (saved frame and function pointers), closure function references (`write_closure_func_ref_and_mask`, `runtime/src/interpreter.rs`) | supported | pointer bytes keep their provenance, so a pointer read back is the pointer stored (§4.1); every use of it is still checked for liveness |
| Function pointers read from frame metadata or closures and used as `&Function`: `saved_caller_ptr`, `exec_call_closure` (`runtime/src/interpreter.rs`), `caller_frame` (`runtime/src/native_context.rs`), `gc_collect` (`runtime/src/heap/mod.rs`) | supported | `as *const Function` is a pure cast back to the view the pointer was stored from (§4.2); the `&Function` is a minted shared reference into the leaked box's allocation (§4.6, §6), so the function must not have been freed by `free_unchecked` while a frame or closure still names it |
| Object headers below the object pointer: `read_descriptor`, `read_obj_size`, `write_object_header` (`runtime/src/memory.rs`) | supported | negative offsets stay in bounds: that the header start lies inside the region is a functional invariant of `heap_alloc` |
| Bump allocation: `heap_alloc`, `fits_in_buffer`, `Heap::used` (`runtime/src/heap/mod.rs`) | supported | the integer comparisons are address arithmetic within one region, which reduces to offsets (§4.1); object boundaries and the bump cursor staying inside the buffer are functional invariants of the heap (§4.3) |
| Copying collection: `gc_collect`, `gc_copy_object`, `gc_scan_to_space` (`runtime/src/heap/mod.rs`), `try_deep_copy` (`runtime/src/heap/deep_copy.rs`) | supported | `copy_nonoverlapping` from from-space to to-space is disjoint because two live allocations are (§4.1); forwarding pointers are stored pointers. The region pool (`take_region` / `return_region`, `alloc/src/global_arena.rs`) holds its regions in a `RefCell`, so it enters as a library model (§6) whose contract makes a returned region dead and a taken one fresh, as MonoVM's own Miri configuration does; a root the collector failed to relocate then fails the liveness obligation at its next access |
| `write_bytes` for zeroing (`heap_alloc`) and poisoning (`MemoryRegion::new_uninit`, `recycle`) | supported | `fill` (§4.2) |
| Slices over raw memory: `slice::from_raw_parts(_mut)` in `Vector::as_bytes` (`core/src/native/value.rs`), `arg_raw` (`runtime/src/native_context.rs`), `serialize_impl` (`runtime/src/value_conv/bcs.rs`), `natives/src/table.rs`; `&*(ptr as *const TableHandle)` in `Ref::get` (`core/src/native/value.rs`) | supported | minted references (§4.6): the bytes initialized and valid at the mint; no raw store into the slice's bytes and no `dealloc` of its region while the slice is used |
| Range test `is_heap_ptr`: `ptr as usize` against the heap buffer's bounds (`runtime/src/heap/mod.rs`) | supported | address comparison (§4.1): for a pointer into the buffer the test reduces to offsets; for one into another live allocation the placement facts put it outside the buffer's range, except at the buffer's end. The inclusive bound `p <= end` therefore accepts a foreign pointer to the first byte of an allocation placed right after the buffer, so the proof that `relocate` moves only heap objects needs `p < end`; a heap object pointer never equals `end`, as the comment there notes, so the change loses nothing |
| Alignment assertion `dst.addr().is_multiple_of(align)` (`write_value`, `runtime/src/value_conv/rust.rs`) | supported | `addr()` (§4.1): provable from the destination's offset and the region's alignment |
| `mem::swap` through a `Vec<u8>` temporary: `copy_nonoverlapping` into `tmp.as_mut_ptr()`, `ptr::copy` between the referents (`native_swap`, `natives/src/mem.rs`) | supported | `Vec::as_mut_ptr` and `as_ptr` are pointers into the vector's own allocation (§6); `ptr::copy` allows overlap; both referents live and in bounds (§4.2) |
| A stack array filled through `as_mut_ptr()`: `addr.as_mut_ptr()` in the GUID decoding of `natives/src/event.rs` | supported | `<[u8]>::as_mut_ptr` escapes a temporary `&mut addr` whose lender is the local `addr`, so the pointer points into `addr`'s allocation and stays usable after the temporary dies, until `addr` is written through another path (§4.5, §6); a `[u8; N]` is a byte allocation, so the `u8` view needs no conversion (§4.1) |
| Pointers stored as integers in event payloads: `usize::from_ne_bytes(..) as *mut u8`, `(new as usize).to_ne_bytes()` (`EventStore::relocate_roots`, `natives/src/event.rs`) | rewrite | reading the pointer back is an integer-to-pointer cast (§7.2). Writing it through a `*mut *mut u8` view of the payload (`msg_data.as_mut_ptr().add(off).cast::<*mut u8>().write(p)`) and reading it back the same way keeps the provenance (§4.1); the payload then holds pointer bytes, which `serialize` already reads with `read_ptr` |
| Leaked boxes: `LeakedBoxPtr` over `Box::into_raw` / `Box::from_raw`, `as_ref_unchecked<'a>` (`alloc/src/leaked.rs`), `FunctionPtr` (`core/src/function.rs`) | supported | the `Box` model (§6): `free_unchecked` is `Box::from_raw` with the liveness obligation, so a double free fails there; `as_ref_unchecked` mints a shared reference with a freeze loan, so the pointee is not written and the box not freed while the reference is used (§4.6) |
| `VersionedLeakedBoxPtr` over `AtomicPtr` (`alloc/src/leaked.rs`); the module and script caches of `global-context` | rejected | atomics and concurrent maps (§7.2); the execution engine sees a loaded function only through the leaked box pointer of the row above |
| Global arena on bumpalo: `GlobalArenaShard::alloc` as `NonNull::from(bump.alloc(v))`, `GlobalArenaPtr::as_ref_unchecked` (`alloc/src/global_arena.rs`) | library model | bumpalo's `alloc` is a fresh allocation that dies with the arena's reset or drop, and the `&mut T` it returns a minted reference into it (§6); `NonNull::from` escapes that reference into the allocation, so the pointer stays usable after the reference dies (§4.5). `as_ref_unchecked` mints a shared reference, so `reset_all_arenas_unchecked` while a pointer is used fails the liveness obligation |
| `GlobalArenaPtr::from_static(&'static T)` for the primitive-type constants (`alloc/src/global_arena.rs`, `core/src/types.rs`) | supported | a pointer from a `&'static T` is a read-only escape of a reference whose lender is a static the frame cannot name, so its shadow never dies (§4.5) |
| Element-type change `Vector::transmute_unchecked<U>` (`core/src/native/value.rs`) | supported | changes only the phantom type of a handle; the bytes are reinterpreted at the element reads, with the validity obligation of §4.4 |
| `NonNull::dangling()` as a placeholder (`VMRegisters::idle`, `runtime/src/interpreter.rs`) | supported | a pointer without provenance, defined as long as it is not accessed (§7.3) |
| Interior mutability: `RootPool` over `UnsafeCell` (`core/src/root_pool.rs`), `ProductionNativeContext` holding `UnsafeCell<&mut Heap>` with the gas meter and read-write set (`runtime/src/native_context.rs`), `SharedArena` over `RefCell<Heap>` (`runtime/src/heap/mod.rs`), the `RefCell` per native extension (`core/src/native/extension.rs`) and in the transaction executor | supported ([`interior-mutability.md`](interior-mutability.md)) | covered by the companion interior-mutability design ([`interior-mutability.md`](interior-mutability.md) §8): identity-bearing cell allocations (`cellRef a`) in `memory`, dynamic `RefCellBorrow` states with lexical `guardDrop`, and reference-bearing cells (§4.6) |
| `unsafe impl Send` / `Sync` (`MemoryRegion`, `LeakedBoxPtr`, `GlobalArenaPtr`) | admitted | no obligation; theorems speak about one thread, and sharing the global arena across threads is not verified |
| `extern "C"` entry points (`lean-link`) | rejected | FFI (§7.2); the crate is the Verex test link, not part of the verified VM |

In sum, the memory core of MonoVM (regions, typed views at byte offsets,
stored pointers, bump allocation, copying collection, address arithmetic,
the heap range test) is in the supported fragment, and verification there
finds what matters most for a moving collector: stale pointers into a
returned space. One local rewrite remains, the event pointers stored as
integers; the natives' context and the root pool are covered by
[`interior-mutability.md`](interior-mutability.md).

## 9. Open questions

- **Lenders outside the frame.** A reference parameter's lender is a
  shadow (§4.5), so a pointer derived from it dies with the loan, cannot
  be returned or stored, and compares undecided with other pointers
  (§7.3); functions that return such a pointer enter by contract (§6). The
  exact treatment gives each reference parameter a symbolic cell in the
  function's contract, an allocation and offset the caller instantiates
  with the argument's lender, so that pointers into it survive the call
  and relate to the caller's pointers. It costs the contract layer an
  allocation variable per reference parameter and is worth adopting when a
  corpus verifies the body of an escaping API rather than stating it.
- **Framing for library models.** §4 alone lets a stray pointer store
  into a `Vec` buffer, so every pure library contract would need a
  footprint clause: the frame problem the prophetic design rejects. The
  candidate: `alloc` and `allocBytes` mint an affine `rawAnchor` too,
  stored in the owning value (the role of std's `Unique`/`RawVec`), and a
  raw access requires its allocation's anchor reachable from the accessing
  frame through owned values and mutable-borrow currents, discharged by a
  type invariant `ptr.alloc = anchor.alloc`. Exclusivity is then
  structural again: Verus's `PointsTo` inferred from ownership rather than
  written as ghost code. It is a profile restriction beyond Rust's UB, so
  it needs its own diagnostic rather than `undefined`, and the
  reachability premise must stay cheap on symbolic states. Shared
  ownership (`Rc`, back pointers) stays `StoredPtr` and
  interior-mutability work.
- **Re-viewing a typed allocation as bytes.** An access to a typed
  allocation at a type that is not one of its subvalues (`&raw mut x as
  *mut u8` for `x : u64`) is UB in the model (§7.3). Two options: keep
  the rule, since byte buffers come from `allocBytes` and `u8` arrays
  anyway; or convert the allocation in place at its first such access,
  `content := bytes (encode T v)`, after which it is a byte allocation and
  safe accesses to the anchored place decode with the usual validity
  obligation (which is Rust's rule: writing byte 2 into a `bool` through a
  `*mut u8` makes the next read of the `bool` UB). The conversion is well
  defined wherever the type has a layout and costs nothing for
  allocations never re-viewed; the recommendation is to adopt it when a
  corpus needs it (`write_bytes` over a typed node, hashing a struct's
  bytes), and to keep the rule until then.

## 10. Related work

- RustHorn (Matsushita, Tsukada, Kobayashi; ESOP 2020, TOPLAS 2021): the
  prophecy encoding of the reference model.
- RustHornBelt (Matsushita, Denis, Jourdan, Dreyer; PLDI 2022): the
  semantic foundation of prophetic specifications for unsafe library
  internals. Minting is a lifetime-logic full borrow with a fresh
  prophecy, which §4.6's memory loan mirrors operationally.
- Stacked Borrows (Jung, Dang, Kang, Dreyer; POPL 2020) and Tree Borrows
  (Villani, Hostert, Dreyer, Jung; PLDI 2025): the operational aliasing
  models. §4.5's provenance by loan is Stacked Borrows' tag reduced to one
  loan per pointer, with Tree Borrows' reserved, active, frozen, and
  disabled states as the escape states of a dead loan (§4.6).
- MiniRust (Jung et al.): the abstract bytes of §4.1 (uninitialized,
  integer, or a provenance-carrying pointer byte) and the encode/decode
  view of typed accesses in §4.4 follow its specification of Rust's
  memory interface; the model keeps MiniRust's addresses symbolic,
  constrained by the placement facts of §4.1.
- Creusot's `Perm::as_mut(ptr, Ghost<&mut Perm>) -> &mut T`, ensuring
  `*(^own).val() == ^result`: §4.6's mint contract with the permission
  explicit.
- Verus (Lattuada et al.; OOPSLA 2023): linear ghost `PointsTo`
  permissions, which §9's framing candidate infers.
- Gillian-Rust (Ayoun, Denis, Maksimović, Gardner; PLDI 2025):
  RustHornBelt's lifetime logic and prophecies in a semi-automated
  separation-logic verifier, linked to Creusot for safe code.
- RefinedRust (Gäher, Sammler, Jung, Krebbers, Dreyer; PLDI 2024).
