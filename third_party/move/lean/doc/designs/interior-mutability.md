# Interior mutability over the prophetic and raw-memory models

The interior-mutability profile (`UnsafeCell<T>`, `Cell<T>`, `RefCell<T>`)
for validated VIR on top of
[`prophetic-references.md`](prophetic-references.md) and
[`unsafe-pointers.md`](unsafe-pointers.md), fulfilling the requirement of
[`vir-design.md`](vir-design.md) ("explicit identity-bearing cell state and
observable guard drop, not prophecy references disguised as a common
operation") and M6 of the Rust plan
([`rust-verification.md`](../plan/rust-verification.md#m6--reference-bearing-values-and-interior-mutability)).
Open work: [`roadmap.md`](../plan/roadmap.md).

A reader verifying Rust code with interior mutability wants three answers:
what the verifier accepts and what it asks to be proved (§7.1), what it
refuses with a diagnostic (§7.2), and where it is stricter than Rust (§7.3).
§§1–6 give the model those answers come from; §8 applies them to every
interior-mutability site in MonoVM.

## 1. Interior mutability in Rust

In safe Rust, a shared reference `&T` ordinarily guarantees that the
referent will not change while the reference lives;
[`prophetic-references.md`](prophetic-references.md) exploits this by
representing `&T` as a pure value snapshot. Interior mutability is the
single exception: a value of type `UnsafeCell<T>` may be mutated through a
shared reference `&UnsafeCell<T>` (or through `&S` for a struct `S`
containing `UnsafeCell<T>`). Every safe interior-mutable type in the
standard library (`Cell<T>`, `RefCell<T>`, `OnceCell<T>`, `Rc<T>`) is built
around `UnsafeCell<T>`.

Rust separates interior mutability into three layers with distinct failure
modes:

1. **`UnsafeCell<T>` (unchecked, unsafe).** Calling `cell.get()` on
   `&UnsafeCell<T>` returns a mutable raw pointer `*mut T`. Dereferencing
   that pointer as `&*cell.get()` or `&mut *cell.get()` is `unsafe`: the
   caller must ensure that no mutable reference coexists with any other
   reference or raw access to the cell's contents. Violating this aliasing
   discipline—for example, holding `&mut *cell.get()` across a callback that
   re-enters a method on `&self`—is immediate **undefined behavior (UB)**.
2. **`Cell<T>` (safe, value-only, infallible).** Wraps `UnsafeCell<T>` and
   never hands out references to the inner `T` through `&Cell<T>`: `get`
   copies `T` out, and `set` / `replace` overwrite `T` in place. Because no
   reference into the cell can live across a call, aliasing conflicts are
   impossible by construction: operations on `&Cell<T>` **never panic and
   are never UB**.
3. **`RefCell<T>` (safe, dynamically borrow-checked).** Wraps
   `UnsafeCell<T>` alongside a runtime borrow counter (`unused`, `reading n`,
   or `writing`). `borrow()` and `borrow_mut()` return RAII guard values
   (`Ref<'_, T>` and `RefMut<'_, T>`) whose `Drop` implementations release
   the borrow when the guard leaves scope. A conflicting borrow at run time
   is **never UB**: `borrow()` / `borrow_mut()` **panic** (`Outcome.threw`),
   and `try_borrow()` / `try_borrow_mut()` return `Err`.

| Example | Verdict |
|---|---|
| `let c = Cell::new(1); let r1 = &c; let r2 = &c; r1.set(2); r2.get()` | defined (`2`); no reference into `c` exists |
| `let c = RefCell::new(v); *c.borrow_mut() += 1; *c.borrow()` | defined; the `RefMut` temporary drops before `borrow()` |
| `let g1 = c.borrow_mut(); let g2 = c.borrow();` (`g1` still in scope) | defined **panic** (`BorrowError` under `try_borrow`), even if `g1` is never read again before drop |
| `Ref::map(c.borrow(), \|x\| &x.f)` and `RefMut::map(c.borrow_mut(), \|x\| &mut x.f)` | defined; the mapped guard keeps the borrow on `c` until dropped |
| `unsafe { &mut *u.get() }` on `&UnsafeCell<T>`, no other loan on `u` live | defined; an exclusive memory loan on `u` |
| holding `let m = unsafe { &mut *u.get() }` across a call that reads or writes `u` | **UB**: foreign access under a live minted `&mut` |
| two distinct `UnsafeCell` fields `self.heap` and `self.rws` borrowed mutably at once through `&self` | defined; distinct cells have disjoint allocations |
| `&x as *const T as *mut T` write when `T` contains no `UnsafeCell` (`T: Freeze`) | **UB**: shared borrow of a `Freeze` type is read-only |

## 2. Objective: finding misuses

Verification of code using interior mutability must prove the absence of
both **unsafe aliasing UB** (on `UnsafeCell<T>`) and **unexpected dynamic
borrow failures** (panics or `Err` returns on `RefCell<T>`):

| Misuse | Detected as |
|---|---|
| Re-entrant or overlapping access to `UnsafeCell<T>` while `&mut *cell.get()` (or `&*cell.get()` against a write) is live | aliasing UB obligation (`Outcome.undefined`) at the second access or `mint` site |
| Raw write through `*cell.get()` while a minted `&*cell.get()` or `&mut *cell.get()` is still used | loan UB obligation (`Outcome.undefined`) at the store site |
| `RefCell::borrow()` while a `RefMut` guard on the same cell is still in scope (not yet dropped) | panic outcome (`Outcome.threw`) at `borrow()`, or `Err` branch at `try_borrow()` |
| `RefCell::borrow_mut()` while any `Ref` or `RefMut` guard on the same cell is still in scope | panic outcome (`Outcome.threw`) at `borrow_mut()`, or `Err` branch at `try_borrow_mut()` |
| Holding a `Ref` / `RefMut` guard on a `NativeExtension` across an allocation that triggers GC (`relocate_all_roots`), `checkpoint`, or `rollback` | `try_borrow_mut` failure (`Err`) or panic obligation at the GC / checkpoint call site |
| Using a pointer from `UnsafeCell::get()` or `RefCell::as_ptr()` after the owning struct or frame died | liveness UB obligation (`Outcome.undefined`) at the access site |
| Dropping an `UnsafeCell<T>` or `RefCell<T>` while a minted reference or guard into it is still live | live-loan obligation at the cell's death site |

## 3. Decision

Interior-mutable cells are **identity-bearing allocations in `memory`**,
referenced by value as `cellRef (alloc : AllocId)`. They build directly on
the allocation and `memoryLoans` machinery of
[`unsafe-pointers.md`](unsafe-pointers.md) without changing the
representation of any cell-free (`Freeze`) type:

- **Zero tax on `Freeze` types.** A type that contains no `UnsafeCell`
  (`T: Freeze`) has the exact same native carrier (`NTy.carrier`) and
  prophetic reference semantics as in
  [`prophetic-references.md`](prophetic-references.md) and
  [`native-verification.md`](native-verification.md).
- **Identity-bearing cell values.** `UnsafeCell<T>`, `Cell<T>`, and
  `RefCell<T>` have native carrier `AllocId` (`RuntimeValue.cellRef a`),
  naming a typed allocation `a` of type `T` in `RuntimeState.memory`.
- **Shared references `&S` stay value snapshots.** When a struct `S` holds
  both `Freeze` fields and cell fields (such as `RootPool` or
  `ProductionNativeContext`), a shared reference `&S` is still a plain copy
  of `S`'s value carrier: copying `&S` copies the `Freeze` fields by value
  and copies the `AllocId` of each cell field. Every copy of `&S` therefore
  names the **same** cell allocations in `memory` and observes every
  mutation to them, with no anchoring or indirection on the outer struct.
- **Cells move with their owner and die with their owner.** Unlike a
  `rawAnchor` (which pins a local place whose address was taken via `&raw`),
  a `cellRef a` moves inside structs, enums, and containers. Because
  `UnsafeCell<T>`, `Cell<T>`, and `RefCell<T>` are `!Copy`, each live cell
  allocation has a single owning value; when that owner is dropped (or consumed by
  `into_inner`), allocation `a` dies.
- **Three cell disciplines on one allocation table.**
  1. `UnsafeCell<T>` exposes `cell.get() -> *mut T` as
     `rawPtr (some a) 0 none true none`—a **mutable** raw pointer into `a`,
     even when obtained from `&UnsafeCell<T>`. Dereferencing it via `&*p` or
     `&mut *p` is an ordinary `mint` (§4.6 of `unsafe-pointers.md`), so
     `memoryLoans` enforces single-writer / multiple-reader aliasing and
     flags re-entrancy as UB.
  2. `Cell<T>` reads and overwrites `memory[a].content` directly by value
     (`cellGet`, `cellSet`, `cellReplace`), minting no loan and never
     panicking or failing.
  3. `RefCell<T>` pairs `memory[a].content` with a `RefCellBorrow` state
     (`unused`, `reading count`, `writing`) and returns explicit guard
     values (`refGuard`, `refMutGuard`) whose `Drop` lowers to `guardDrop`
     at MIR drop points.
- **Lexical/MIR-exact guard drop.** While raw-pointer mints (`&mut *p`) die
  non-lexically at their last use, `Ref<'b, T>` and `RefMut<'b, T>` guards
  release their dynamic borrow **only at `guardDrop`** (the MIR `Drop` of
  the guard variable or temporary, or an explicit `std::mem::drop`). Holding
  an unused guard across a second `borrow_mut()` is therefore detected as a
  panic/error, matching Rust.
- **Reference-bearing cells (`UnsafeCell<&'a mut T>`, `RefCell<&'a mut T>`).**
  A cell whose payload type `T` is (or contains) a mutable borrow
  `&'a mut U` stores `borrow ℓ_outer v` in `memory[a].content`. Borrowing
  `&mut U` through the cell reborrows `ℓ_outer` for the duration of the
  inner borrow or guard; when region `'a` and the cell die, `ℓ_outer`
  reconciles its prophecy `π ℓ_outer` with the final value in
  `memory[a].content`.

## 4. The model

### 4.1 Values, types, and cell state

```lean
-- Syntax: interior-mutable type constructor
| cell (kind : CellKind) (inner : TypeId)
| borrowGuard (mutable : Bool) (pointee : TypeId)   -- Ref<'b, T> / RefMut<'b, T>

inductive CellKind where
  | unsafeCell   -- std::cell::UnsafeCell<T>
  | cell         -- std::cell::Cell<T>
  | refCell      -- std::cell::RefCell<T>

inductive RefCellBorrow where
  | unused
  | reading (count : Nat)   -- count ≥ 1 active Ref guards
  | writing                 -- 1 active RefMut guard

-- RuntimeValue: cell handle and RefCell guards
| cellRef (alloc : AllocId)
| refGuard (alloc : AllocId) (value : RuntimeValue)
| refMutGuard (alloc : AllocId) (path : SubvaluePath) (current : RuntimeValue)

-- Allocation (extending unsafe-pointers.md §4.1)
structure Allocation where
  live      : Bool
  type      : Option TypeId          -- some T for all cells
  size      : Nat
  align     : Nat
  base      : Nat
  shadow    : Bool
  refState  : RefCellBorrow          -- used by RefCell; .unused otherwise
  content   : Option RuntimeValue
```

- `cellRef a` is the runtime and native representation (`NTy.carrier
  (.cell k T) = AllocId`) of `UnsafeCell<T>`, `Cell<T>`, and `RefCell<T>`.
  The value stored inside the cell lives in `memory[a].content` (with
  native representation `NTy.carrier T` in typed verification state).
- `refGuard a v` is a `Ref<'b, U>` guard borrowing allocation `a`: `v` is
  the observed snapshot of `U` (initially the whole `T` in `memory[a]`, or a
  projected subvalue after `Ref::map`).
- `refMutGuard a path v` is a `RefMut<'b, U>` guard exclusively borrowing
  the subvalue at `path` of allocation `a` (`path = .root` before
  `RefMut::map`). The guard owns the in-flight value `current : U` while
  `memory[a].refState = .writing`, and writes `current` back to `(a, path)`
  when dropped (`guardDrop`).
- **Native carriers (`NTy.carrier`).**
  - `NTy.carrier (.cell k T) = AllocId`.
  - `NTy.carrier (.borrowGuard false U) = AllocId × NTy.carrier U`.
  - `NTy.carrier (.borrowGuard true U) = AllocId × SubvaluePath × NTy.carrier U`.
  No `RuntimeValue` appears in verification conditions or proof goals,
  preserving the native-only invariant of
  [`native-verification.md`](native-verification.md).

### 4.2 Operations

```lean
inductive CellOperation where
  | newCell (kind : CellKind) (type : TypeId)  -- UnsafeCell/Cell/RefCell::new(v)
  | intoInner                                  -- consume cell, return inner value, kill alloc
  | getMut                                     -- &mut Cell/RefCell/UnsafeCell -> &mut T
  | unsafeGet                                  -- UnsafeCell::get(&self) -> *mut T
  | cellGet                                    -- Cell::get(&self) -> T (T: Copy)
  | cellSet                                    -- Cell::set(&self, v)
  | cellReplace                                -- Cell::replace(&self, v) -> T
  | refBorrow (fallible : Bool)                -- RefCell::borrow / try_borrow
  | refBorrowMut (fallible : Bool)             -- RefCell::borrow_mut / try_borrow_mut
  | refDeref                                   -- Deref::deref(&Ref<U>) -> &U
  | refMutDeref                                -- DerefMut::deref_mut(&mut RefMut<U>) -> &mut U
  | refMap ( mutable : Bool )                  -- Ref::map / RefMut::map
  | refClone                                   -- Ref::clone
  | refAsPtr                                   -- RefCell::as_ptr(&self) -> *mut T
  | guardDrop (mutable : Bool)                 -- Drop::drop of Ref / RefMut
```

`Cell<T>` and `RefCell<T>` operations are admitted in the safe Rust profile;
`unsafeGet` (or dereferencing the pointer from `refAsPtr`) requires the
unsafe profile of [`unsafe-pointers.md`](unsafe-pointers.md).

### 4.3 `UnsafeCell<T>` and the raw-pointer boundary

- **Creation and destruction.** `newCell .unsafeCell T` allocates a fresh
  typed allocation `a` in `memory` with `live = true`, `type = some T`,
  `refState = .unused`, and `content = some v`, yielding `cellRef a`.
  Dropping the owning value of `cellRef a` (or calling `intoInner`) requires
  `a` live and no live loan on `a` in `memoryLoans`, and flips
  `memory[a].live := false`.
- **`UnsafeCell::get(&self) -> *mut T` (`unsafeGet`).** Given `cellRef a`
  (obtained from `&UnsafeCell<T>`, which is a copy of `cellRef a`),
  `unsafeGet` returns `rawPtr (some a) 0 none true none`—a mutable pointer
  into `a` with `mutable = true` and `via = none`. It has no safety
  premises of its own; premises attach to subsequent `load`, `store`, or
  `mint` operations on the pointer (`unsafe-pointers.md` §4.2, §4.6).
- **Minted borrows into `UnsafeCell<T>`.**
  - `unsafe { &mut *cell.get() }` lowers to `mint true` on `(a, .root)`: it
    requires `a` live, initialized, and valid, and **no live foreign loan on
    `a` in `memoryLoans`**. It records an exclusive `MemoryLoan` `ℓ` on `a`
    that dies non-lexically at the last use of the minted `&mut T`, and
    writes the updated value back to `memory[a].content` at reconciliation.
  - `unsafe { &*cell.get() }` lowers to `mint false` on `(a, .root)`: it
    requires `a` live, initialized, and valid, and **no live exclusive loan
    on `a` in `memoryLoans`**. It records a freeze `MemoryLoan` `ℓ` on `a`
    until the last use of the minted `&T` and returns the snapshot `v`.
- **Exemption from outer shared-reference freezes.** Taking a safe shared
  borrow `&s` of a struct `s` does not register a freeze loan on the
  allocations named by `s`'s `cellRef` fields. If `&s` itself escapes to a
  raw pointer `&raw const *r` (`r : &S`), the freeze loan registered on `s`'s
  backing allocation covers only the `Freeze` subvalues of `S`; layout
  intervals inside an `UnsafeCell` remain writable (`mutable = true`), as in
  Stacked and Tree Borrows.

### 4.4 `Cell<T>`: lock-free value replacement

Because `Cell<T>` never hands out `&T` or `&mut T` through `&Cell<T>`, no
loan into `a` can be live while `Cell<T>` is accessed sharedly (unless the
user explicitly called `Cell::as_ptr` in `unsafe` code, which is checked by
`memoryLoans`):

- **`cellGet` (`Cell::get(&self) -> T` for `T: Copy`).** Premise: `a` live,
  initialized, and no live exclusive loan on `a` in `memoryLoans` (trivially
  true in safe code). Returns `v` from `memory[a].content`.
- **`cellSet` (`Cell::set(&self, v)`).** Premise: `a` live and no live loan
  on `a` in `memoryLoans` (trivially true in safe code). Sets
  `memory[a].content := some v`.
- **`cellReplace` (`Cell::replace(&self, v) -> T`).** Same premises as
  `cellSet`; returns the old `v_0` and sets `memory[a].content := some v`.
  `Cell::take(&self)` is `cellReplace` with `T::default()`.
- **`getMut` (`Cell::get_mut(&mut self) -> &mut T`).** Because the caller
  holds `&mut Cell<T>` exclusively, `getMut` is `mint true` on `a`.

In pure safe Rust, every `Cell<T>` operation on a live `&Cell<T>` has zero
unproved safety obligations and reduces in `wp` to a direct read or write of
`memory[a]`.

### 4.5 `RefCell<T>` and observable guard drop

`RefCell<T>` enforces single-writer / multiple-reader exclusivity dynamically
through `memory[a].refState`:

- **`refBorrow` (`borrow` / `try_borrow`).** Given `cellRef a` (with `a`
  live and no live raw exclusive `MemoryLoan` on `a`):
  - If `memory[a].refState = .unused`, sets `refState := .reading 1`,
    registers a freeze `MemoryLoan` on `a`, and yields `refGuard a v` (or
    `Ok(refGuard a v)` for `try_borrow`).
  - If `memory[a].refState = .reading n`, sets `refState := .reading (n + 1)`
    and yields `refGuard a v` (or `Ok(refGuard a v)`).
  - If `memory[a].refState = .writing`, `try_borrow` returns
    `Err(BorrowError)` and `borrow` **panics** (`Outcome.threw`, never
    `Outcome.undefined`).
- **`refBorrowMut` (`borrow_mut` / `try_borrow_mut`).** Given `cellRef a`
  (with `a` live and no live raw `MemoryLoan` on `a`):
  - If `memory[a].refState = .unused`, sets `refState := .writing`,
    registers an exclusive `MemoryLoan` on `a` (so raw accesses through
    `as_ptr()` while the guard lives are UB), and yields
    `refMutGuard a .root v` (or `Ok(...)` for `try_borrow_mut`).
  - If `memory[a].refState` is `.reading _` or `.writing`, `try_borrow_mut`
    returns `Err(BorrowMutError)` and `borrow_mut` **panics**
    (`Outcome.threw`).
- **Using guards (`refDeref`, `refMutDeref`, `refClone`).**
  - `Deref::deref` on `refGuard a v` returns the snapshot `v : U`.
  - `Deref::deref` on `refMutGuard a path current` returns the observation
    `current : U`.
  - `DerefMut::deref_mut` on `&mut guard` (where `guard = refMutGuard a path
    current`) is an **ordinary prophetic mutable borrow of `guard.current`**:
    it yields `borrow ℓ current` and sets `guard.current := π ℓ`. Because
    `DerefMut::deref_mut<'g>(&'g mut self) -> &'g mut U` borrows the local
    `guard` for `'g`, rustc's borrow checker ensures `'g` ends before `guard`
    can be used again or dropped, so `guard.current` is reconciled with
    `π ℓ` before `guardDrop` runs.
  - `Ref::clone` on `refGuard a v` increments `memory[a].refState` from
    `.reading n` to `.reading (n + 1)` and returns `refGuard a v`.
- **Subvalue projection (`Ref::map`, `RefMut::map`).**
  - `Ref::map(refGuard a v, f)` evaluates `f(&v)` to a shared projection
    `v_sub : V` and returns `refGuard a v_sub`, keeping `memory[a].refState`
    unchanged.
  - `RefMut::map(refMutGuard a path current, f)` evaluates `f` on a mutable
    borrow of `current` that selects sub-place `subpath : U → V`, writes any
    unmapped sibling fields of `current` back to `(a, path)`, and returns
    `refMutGuard a (path ++ subpath) v_sub`, keeping
    `memory[a].refState = .writing`.
- **Guard drop (`guardDrop`).** Emitted at the MIR `Drop` of a `Ref` or
  `RefMut` local/temporary (and on `std::mem::drop(guard)`):
  - `guardDrop false` on `refGuard a _`: decrements `.reading (n + 1)` to
    `.reading n` (or `.unused` when `n = 0`, releasing the freeze loan in
    `memoryLoans`).
  - `guardDrop true` on `refMutGuard a path current`: requires `a` live and
    no live escaped loan into `(a, path)`, writes `current` into
    `memory[a].content` at `path`, releases the exclusive loan in
    `memoryLoans`, and sets `memory[a].refState := .unused`.

### 4.6 Reference-bearing cells (`UnsafeCell<&'a mut T>`, `RefCell<&'a mut T>`)

MonoVM stores mutable references inside cells when a context struct must
implement a `&self` trait while delegating to caller-owned `&'a mut`
components (`ProductionNativeContext<'a>` holding `UnsafeCell<&'a mut Heap>`
and `UnsafeCell<&'a mut ResourceReadWriteSet>`, and
`KeylessExecutionContext<'a>` holding
`RefCell<&'a mut InterpreterContext<'guard>>`).

The model handles reference-bearing cells without new primitives:

1. **Construction.** When `caller` passes `r : &'a mut T` (`borrow ℓ_outer
   v_0`, prophecy `π ℓ_outer`) into `UnsafeCell::new(r)` or
   `RefCell::new(r)`, allocation `a` is created in `memory` with
   `content = some (borrow ℓ_outer v_0)`. `BorrowCertificate` records that
   `ℓ_outer` is carried by `cellRef a` for region `'a`, so `ℓ_outer` is
   excluded from `new`'s exit deaths.
2. **Inner reborrow.**
   - For `UnsafeCell<&'a mut T>`, `unsafe { &mut **cell.get() }` mints an
     exclusive loan `ℓ_cell` on `a` (yielding `&mut (&'a mut T)`) and
     reborrows the inner `borrow ℓ_outer v` as `borrow ℓ_inner v` (updating
     `memory[a].content`'s current value to `π ℓ_inner`). When the short
     inner reborrow dies at the end of the method, `ℓ_inner` reconciles its
     updated value into `memory[a].content` and `ℓ_cell` dies, leaving `a`
     free for the next method call.
   - For `RefCell<&'a mut T>`, `let mut g = cell.borrow_mut()` moves
     `borrow ℓ_outer v` into `g.current` while `refState = .writing`;
     reborrows through `g` update `g.current`, and `guardDrop` writes
     `borrow ℓ_outer v_next` back to `memory[a].content` and resets
     `refState := .unused`.
3. **Region exit and reconciliation.** When the owning struct (`ctx`) and
   region `'a` die in the caller, `a` dies (requiring `refState = .unused`
   and no live `MemoryLoan` on `a`), and `ℓ_outer` reconciles the final
   `current` in `memory[a].content` with `π ℓ_outer`, delivering the final
   `Heap`, `ResourceReadWriteSet`, or `InterpreterContext` state back to the
   caller's lender place.

## 5. Verification and validation

- **Weakest precondition (`wp`).** Each `CellOperation` has a native `wp`
  combinator over the typed cell entries in `memory`:
  - `cellGet` / `cellSet` / `cellReplace` read and update the typed slot
    `memory[a].val : NTy.carrier T` directly.
  - `refBorrow` / `refBorrowMut` branch on `memory[a].refState`: for
    `borrow` / `borrow_mut`, a prove-no-panic mode conjoins
    `refState ≠ .writing` (or `refState = .unused`) as a proof obligation;
    for `try_borrow` / `try_borrow_mut`, both the `Ok` and `Err` branches
    are verified against the concrete `refState`.
  - `guardDrop` updates `memory[a].refState` and (for `RefMut`)
    `memory[a].val := guard.current`.
- **Disjointness of cell fields.** Every `newCell` call produces a fresh
  `AllocId` `a` distinct from all existing allocations (`a ∉ dom memory`).
  A constructor like `ProductionNativeContext::new` that initializes
  `gas: UnsafeCell::new(..)`, `heap: UnsafeCell::new(..)`,
  `rws: UnsafeCell::new(..)`, `pool: RootPool::new()`, and
  `returns_started: Cell::new(false)` establishes pairwise disequality
  (`a_gas ≠ a_heap ≠ a_rws ≠ a_pool ≠ a_returns`) as part of its
  postcondition and type invariant. Subsequent `&self` methods therefore
  discharge non-aliasing between `self.heap`, `self.rws`, `self.pool`, and
  `self.returns_started` automatically by `omega` / `decide` on `AllocId`s.
- **Validation and `BorrowCertificate`.** Validation classifies types into
  `Freeze` (no `UnsafeCell` reachable in value fields) and cell-bearing,
  records the MIR `Drop` points of `Ref` and `RefMut` locals/temporaries as
  `guardDrop` nodes (preserving lexical drop order rather than non-lexical
  last use), and tracks outer loans `ℓ_outer` carried inside
  reference-bearing cells across function boundaries (§4.6).

## 6. Library models

Higher-level interior-mutable containers verify once or enter through typed
contracts over the §4 cell state:

- **Type-keyed `RefCell` maps (`NativeExtensions`).** A heterogenous map of
  the shape `HashMap<TypeId, RefCell<Box<dyn Trait>>>` (MonoVM's
  `NativeExtensions`, `core/src/native/extension.rs`) is modeled—following
  [`static-memory.md`](static-memory.md)'s per-`ResourceType` decomposition—as
  one `Option (AllocId)` slot per registered extension type `T` (`AllocId`
  naming a `RefCell<T>` in `memory`).
  - `add::<T>(&mut self, ext: T)` allocates `a_T := newCell .refCell T ext`
    and sets slot `T` to `some a_T` (panicking if already `some`).
  - `get::<T>(&self) -> VMResult<Ref<'_, T>>` and
    `get_mut::<T>(&self) -> VMResult<RefMut<'_, T>>` read slot `T` (`Err` if
    `none`) and run `refBorrow true` / `refBorrowMut true` on `a_T`.
  - Because distinct types `T₁ ≠ T₂` map to distinct slots `a_T₁ ≠ a_T₂`,
    simultaneous mutable borrows of distinct extensions (`get_mut::<Counter>()`
    and `get_mut::<Flag>()`) succeed automatically, while borrowing the same
    extension twice returns `Err`.
  - Bulk operations (`relocate_all_roots`, `checkpoint`, `rollback`) iterate
    over all installed slots `a_T` and require `memory[a_T].refState =
    .unused` (returning `Err` if a native left an extension guard live
    across an allocation or checkpoint).
- **Region pools (`ArenaSlot.regions` in `alloc/src/global_arena.rs`).**
  `RefCell<[Vec<MemoryRegion>; NUM_BUCKETS]>` is a direct instance of
  `RefCell<T>`: `take_region` and `return_region` borrow the bucket array
  mutably for a leaf push/pop and drop the `RefMut` guard before returning,
  so `refState` is `.unused` at entry and exit of every call.
- **`OnceCell<T>` and `LazyCell<T>`.** Modeled as a cell allocation `a` of
  type `Option T` whose value transitions monotonically from `none` to
  `some v`; `get(&self)` returns a minted shared reference `&T` once
  initialized (safe because the cell is never mutated or overwritten after
  initialization, unless `take` or `get_mut` is called with `&mut self`).

## 7. What is allowed

### 7.1 Supported

| Construct | Proof obligations |
|---|---|
| `Cell::new(v)`, `Cell::get(&self)` (`T: Copy`), `Cell::set(&self, v)`, `Cell::replace(&self, v)`, `Cell::take(&self)`, `Cell::into_inner(self)`, `Cell::get_mut(&mut self)` | none in safe code (allocation liveness is guaranteed by `&self` / `self`; `get_mut` is exclusive by `&mut self`, §4.4) |
| `RefCell::new(v)`, `RefCell::into_inner(self)`, `RefCell::get_mut(&mut self)` | none in safe code (§4.5) |
| `RefCell::try_borrow(&self)`, `RefCell::try_borrow_mut(&self)` | none; branches on `memory[a].refState` (`Ok` vs. `Err`, §4.5) |
| `RefCell::borrow(&self)`, `RefCell::borrow_mut(&self)` | `borrow`: `memory[a].refState ≠ .writing`; `borrow_mut`: `memory[a].refState = .unused` (when proving panic-freedom; otherwise produces a defined `threw` outcome, §4.5) |
| `Deref` / `DerefMut` on `Ref<'_, T>` and `RefMut<'_, T>`, `Ref::clone`, `Ref::map`, `RefMut::map`, guard `Drop` | none; `DerefMut` borrows `guard.current` prophetically, and `guardDrop` writes back and updates `refState` at the MIR `Drop` point (§4.5) |
| `RefCell::as_ptr(&self) -> *mut T` | none at creation; dereferencing the pointer carries the raw access obligations of `unsafe-pointers.md` §4.2 plus no conflicting `Ref` / `RefMut` guard (`memoryLoans`, §4.5) |
| `UnsafeCell::new(v)`, `UnsafeCell::into_inner(self)`, `UnsafeCell::get_mut(&mut self)`, `UnsafeCell::get(&self) -> *mut T` | `get(&self)` has no obligation at call; dereferencing or minting `&*cell.get()` / `&mut *cell.get()` requires `a` live, initialized, valid, and no conflicting loan on `a` in `memoryLoans` (§4.3) |
| Multiple `UnsafeCell`, `Cell`, or `RefCell` fields in one struct accessed through `&self` (`ProductionNativeContext`) | pairwise distinct `AllocId`s (established at construction) make simultaneous borrows of distinct fields disjoint (§5) |
| `UnsafeCell<&'a mut T>` and `RefCell<&'a mut T>` (reference-bearing cells) | inner accesses reborrow the cell's `&'a mut T` without overlapping; at region `'a` exit the cell has no active guard/loan and reconciles with `π ℓ_outer` (§4.6) |

### 7.2 Rejected at validation

| Construct | Why |
|---|---|
| Sharing `UnsafeCell`, `Cell`, or `RefCell` across threads (`Sync` impls that expose concurrent mutation, `Mutex`, `RwLock`, `Atomic*`, `static mut`) | single-threaded execution model only; concurrent synchronization is out of scope (`unsafe-pointers.md` §7.2) |
| Viewing a `Ref<'_, T>`, `RefMut<'_, T>`, or a struct containing `cellRef` as raw bytes (`transmute`, union byte reinterpretation) | cell handles and borrow guards carry semantic allocation/loan identities and have no raw byte representation (`unsafe-pointers.md` §4.4) |
| `std::mem::forget` of a `RefMut<'_, T>` guard when the cell is later read or borrowed again in the same execution | supported as leaving `refState = .writing` permanently (so later borrows panic/error), but a specification requiring the write-back of the forgotten `RefMut` will not observe it |

### 7.3 Stricter than Rust

| Construct | Rust | The model |
|---|---|---|
| Raw byte access (`&raw const cell as *const u8`) to a struct holding a `cellRef` | defined: `UnsafeCell<T>` is `#[repr(transparent)]` over `T` in memory | rejected / UB unless `cell` is accessed through `UnsafeCell::get` / `RefCell::as_ptr` or its typed subvalues (§7.2); the `cellRef` indirection is erased at compilation |
| `RefMut::map_split` (splitting one `RefMut` into two guards on disjoint sub-slices/fields) | defined when the closure returns disjoint sub-borrows | unsupported unless lowered to two disjoint subvalue paths `(a, path₁)` and `(a, path₂)` sharing one split writer count |
| Re-borrowing a `RefCell` inside a `Drop` implementation of a value stored inside that same `RefCell` during `into_inner` or cell drop | `into_inner` moves the value out before dropping it | allocation `a` dies before the moved-out value drops |

## 8. Interior mutability in MonoVM

Every interior-mutability site in MonoVM (`third_party/move/mono-move`) that
was deferred in [`unsafe-pointers.md` (§8)](unsafe-pointers.md#8-unsafe-code-in-monovm)
falls into the supported fragment of §§4–6:

| Idiom (where) | Cell kind | Verdict | What verification asks |
|---|---|---|---|
| `RootPool { inner: UnsafeCell<RootPoolInner> }` and handles `ReferenceHandle<'a>` / `ObjectHandle<'a>` holding `&'a RootPool` (`core/src/root_pool.rs`) | `UnsafeCell<RootPoolInner>` | supported | `&RootPool` is a copyable snapshot of `cellRef a_pool`, so any number of `ReferenceHandle` and `ObjectHandle` values coexist (§3). Leaf methods (`alloc`, `release`, `entry`, `has_live_roots`) mint a short `&mut` or `&` borrow on `a_pool` that dies before returning (§4.3). `relocate_each` holds a minted `&mut RootPoolInner` across `relocate(entry.base)`, so its contract requires `relocate` not to access `a_pool` (which the heap scanner satisfies) |
| `ProductionNativeContext<'a>` fields `gas: UnsafeCell<&'a mut GasMeter>`, `heap: UnsafeCell<&'a mut Heap>`, `rws: UnsafeCell<&'a mut ResourceReadWriteSet>` (`runtime/src/native_context.rs`) | Reference-bearing `UnsafeCell<&'a mut T>` (§4.6) | supported | `ProductionNativeContext::new` creates three distinct cell allocations `a_gas ≠ a_heap ≠ a_rws` holding the caller's `&'a mut` borrows. Methods like `new_byte_vector_vector`, `new_vector`, `box_arg`, `bcs_deserialize_value`, `table_borrow`, and `table_remove` mint `&mut **self.heap.get()` and `&mut **self.rws.get()` simultaneously; this succeeds because `a_heap ≠ a_rws` (§5). When a method calls a helper (`bcs_serialize_arg` calling `bcs_serialize_value`), neither holds a live loan on `a_heap` or `a_rws` across the call. At native return, `ctx` drops and reconciles the final `Heap`, `ResourceReadWriteSet`, and `GasMeter` with the interpreter's borrows (§4.6) |
| `ProductionNativeContext<'a>` field `returns_started: Cell<bool>` (`runtime/src/native_context.rs`) | `Cell<bool>` (§4.4) | supported | `self.returns_started.get()` and `.set(true)` are pure reads and writes of `memory[a_returns] : Bool`, disjoint from `a_heap`, `a_rws`, and `a_pool`; proves the state-machine invariant that `arg`, `arg_raw`, `box_arg`, and `new_vector*` return `Err` once `set_return` or `set_return_raw` has succeeded |
| `SharedArena { heap: RefCell<Heap> }` and `with_heap_mut` (`runtime/src/heap/mod.rs`) | `RefCell<Heap>` (§4.5) | supported | `with_heap_mut(&self, f)` calls `self.heap.borrow_mut()` (requiring `refState = .unused`), passes `&mut Heap` to `f` (requiring `f` not to re-enter `self.heap`), and runs `guardDrop` at the end of `f(&mut self.heap.borrow_mut())`, restoring `refState = .unused`. Because `Heap`inside `SharedArena` is append-only (never GC'd or reset), pointers allocated during `f` remain valid in `memory` after `guardDrop` |
| `NativeExtensions { map: FxHashMap<TypeId, RefCell<Box<dyn NativeExtension>>> }` (`core/src/native/extension.rs`) and concrete extensions (`NativeTableContext`, `NativeAggregatorContext`, `NativeRistrettoPointContext`, `NativeObjectContext`, `EventStore`) | Type-keyed `RefCell<T>` map (§6) + per-extension internal `RefCell`s (§4.5) | supported | Each registered extension type `T` has its own `RefCell<T>` slot `a_T` (§6). `get::<T>()` and `get_mut::<T>()` use `try_borrow` / `try_borrow_mut` + `Ref::map` / `RefMut::map` (§4.5), so double-borrowing the same extension returns `Err(native_invariant_violation)` (never UB), while borrowing distinct extensions succeeds (`a_T₁ ≠ a_T₂`). `relocate_all_roots`, `checkpoint`, and `rollback` require all extension guards dropped (`refState = .unused`) before GC or session boundaries; a native that holds an extension `RefMut` across an allocation that triggers GC is flagged at the allocation call site |
| `ArenaSlot { bump: Bump, regions: RefCell<[Vec<MemoryRegion>; NUM_BUCKETS]> }` (`alloc/src/global_arena.rs`) | `RefCell<[Vec<MemoryRegion>; NUM_BUCKETS]>` (§4.5, §6) | supported | `take_region` and `return_region` borrow `self.guard.regions.borrow_mut()` for a single leaf `Vec::pop` or `Vec::push` and drop the `RefMut` guard before returning; `refState` is `.unused` before and after every call |
| `KeylessExecutionContext { interp: RefCell<&'a mut InterpreterContext<'guard>>, ... }` (`aptos-transaction-executor/src/user_txn/keyless.rs`) | Reference-bearing `RefCell<&'a mut T>` (§4.6) | supported | Combines `RefCell` guard discipline (§4.5) with a reference-bearing cell (§4.6): each entry point borrows `self.interp.borrow_mut()`, runs a VM step on `&mut InterpreterContext`, and drops the guard before returning; when `KeylessExecutionContext` drops, the inner `&'a mut InterpreterContext` reconciles with the caller |
| `InMemoryStateViewProvider { inner: RefCell<ProviderState> }` and `TestResourceProvider { materialized: RefCell<Materialized> }` (`aptos-state-view-providers/src/lib.rs`, `testsuite/src/resource_provider.rs`) | `RefCell<T>` (§4.5) | supported | Lazy materialization caches: each `ResourceProvider` trait method (`get_resource`, `get_table_item`) borrows `inner.borrow_mut()` locally to populate the cache, allocates in a `SharedArena`, and drops the `RefMut` guard before returning the arena pointer |

With this design in place alongside [`unsafe-pointers.md`](unsafe-pointers.md),
no interior-mutability construct in MonoVM's execution engine, memory
manager, or native-function interface remains rejected.

## 9. Open questions

- **Automatic vs. explicit cell-disjointness invariants.** When a struct has
  fixed `UnsafeCell` / `Cell` / `RefCell` fields (`ProductionNativeContext`),
  pairwise distinctness of their `AllocId`s (`a_heap ≠ a_rws`) is finite and
  can be emitted automatically as a type well-formedness invariant of every
  instance constructed by safe/checked code, so user specs never write
  `self.heap ≠ self.rws` by hand. For dynamic collections of cells (`Vec<RefCell<T>>`),
  pairwise distinctness of elements is a quantified container invariant
  (`∀ i ≠ j, v[i] ≠ v[j]`). The recommendation is to emit the finite
  struct-field disjointness invariant automatically from validation.
- **`RefMut::map_split`.** Standard-library `RefMut::map_split` splits one
  `RefMut<'b, T>` into `(RefMut<'b, U>, RefMut<'b, V>)` sharing a single
  dynamic writer borrow on the underlying `RefCell`. MonoVM does not use
  `map_split`; if a future crate needs it, `RefCellBorrow.writing` can carry
  a split-guard count `(count : Nat)` analogous to `.reading count`, with
  disjoint `SubvaluePath`s required at the split.

## 10. Related work

- RustBelt (Jung, Jourdan, Krebbers, Dreyer; POPL 2018): semantic typing of
  `UnsafeCell`, `Cell`, and `RefCell` via custom lifetime-logic protocols,
  showing that `Cell` and `RefCell` are safe abstractions over `UnsafeCell`.
- RustHornBelt (Matsushita, Denis, Jourdan, Dreyer; PLDI 2022): integrates
  prophetic mutable borrows with RustBelt's semantic type system for library
  types that encapsulate interior mutability.
- Creusot (`PermCell` / `Ghost` permissions) and Verus (`PCell` / `PointsTo`):
  model `UnsafeCell` via explicit ghost permission tokens passed in specs;
  our model keeps `Cell` and `RefCell` ghost-free by indexing `memory` by the
  cell's own `AllocId` (`cellRef a`) and tracking `RefCellBorrow` directly in
  the state.
- Aeneas (Ho, Protzenko; ICFP 2022): pure functional translation of safe
  Rust that rejects interior mutability unless lifted to state-monad
  parameters; our `memory` projection achieves the same state-passing effect
  locally for cell-bearing types while keeping `Freeze` types pure.
