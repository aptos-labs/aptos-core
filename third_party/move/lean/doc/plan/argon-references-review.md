# Review: Rust references across safe code, interior mutability, and the unsafe subset

Reviewed 2026-11-01. Covers the representation and soundness of references and
their interaction with interior mutability and the supported unsafe subset
across [`prophetic-references.md`](../designs/prophetic-references.md),
[`unsafe-pointers.md`](../designs/unsafe-pointers.md),
[`interior-mutability.md`](../designs/interior-mutability.md),
[`rust-mir-design.md`](../designs/rust-mir-design.md),
[`native-verification.md`](../designs/native-verification.md), and
[`rust-verification.md`](../designs/rust-verification.md).

---

## 1. Current representation

### 1.1 Safe references: prophetic ownership (`prophetic-references.md`)

After validation, safe references are represented as values rather than heap
pointers, following the RustHorn encoding:

- **Shared references (`&T` for `T: Freeze`):** Plain value snapshots with no
  wrapper constructor, no hole, and no loan minted. Because exclusivity
  prevents mutation of the referent while `&T` lives, the snapshot remains equal
  to the referent throughout `&T`'s lifetime.
- **Mutable references (`&mut T`):** Ownership transfer. At runtime, a live
  mutable borrow is `borrow (loan : LoanId) (current : RuntimeValue)`, while the
  lender's place holds `loanHole loan`. Relationally and in the native
  denotation, `&mut T` has carrier `(current, prophecy)`:
  - **Borrow (`let b = &mut place`):** Moves the current value `v` into `b` and
    immediately places the prophecy `π ℓ` into the lender's slot.
  - **Reborrow (`let b' = &mut *b`):** Mints a fresh loan `ℓ'`, gives `b'` the
    current value `v`, and replaces `b`'s current value with `π ℓ'`.
  - **Freeze (`&*b`):** Reads `b.current` as a value snapshot while keeping `b`'s
    mutable loan `ℓ` live so later writes through `b` still reach the lender.
  - **Death:** Certified statically by `BorrowCertificate`
    (`CheckedLoanFact.deaths`). At a loan's death anchor, the semantics assumes
    the reconciliation equation `current = π ℓ` (executably writing `current`
    back into `loanHole ℓ`).
  - **Returned references and modular calls:** Dying frames export unreconciled
    loans in `state.pending`, and callers resolve holes via `applyPending`
    without path bookkeeping (`pick`-style dynamic returns work uniformly).

### 1.2 References in aggregates, cursors, shared views, and regions (`rust-verification.md`)

- **Aggregates holding `&mut`:** A nested `&mut` keeps carrier
  `(current, prophecy)` at a projection path `Proj`. Loans die at their holder
  path (`Term.resolveAt`, a no-op on inactive enum variants). Moving a `&mut`
  out of an aggregate is normalized to a reborrow through the aggregate
  (`reborrowMovedComponents`) so conditional moves still reconcile on all paths.
  `VIR-SEMANTIC-LOAN-DEATH` checks that no holder path in the merged state at a
  death anchor is read along any continuation, including loop backedges.
- **Cursor transfer (`while let Some(r) = cur { cur = next(r) }`):** A component
  move followed by an overwrite transfers the loan to `r`, clears the stale
  component in the runtime (`clearBorrowValue`), and exposes `final(x)` in loop
  invariants.
- **Shared views of a `&mut` holder (`&S` where `S` contains `&mut U`):**
  Modeled in Step 3e by erasing loans under shared observations (`&T` denotes
  `erase T` with `erase (ref σ) = erase σ`), and evaluating `&s` at runtime via
  `value.observe`.
- **Signature lifetimes:** Parameter and result types retain declared signature
  lifetimes so returned references tie to their input lenders. Early-bound
  lifetimes (the only kind a `where` clause can constrain in rustc) are erased
  by `FnDef::fn_sig` and conservatively relate to all lifetimes; bodies are
  checked against `resultRegionDiagnostics`.
- **Generics instantiated at `&mut U`:** Blocked from generic theorem transfer
  by `refFree` and `dropsDestructor` (since dropping `&mut U` resolves its
  prophecy); verified per substituted instance via `Term.callInstance`.

### 1.3 Interior mutability (`interior-mutability.md`)

To preserve `&T` as a pure value snapshot for `Freeze` types without adding a
heap indirection to ordinary structs:

- **Identity-bearing cell handles:** `UnsafeCell<T>`, `Cell<T>`, and
  `RefCell<T>` have native carrier `AllocId` (`RuntimeValue.cellRef a`), naming
  a typed allocation `a` in `RuntimeState.memory`.
- **Snapshots of cell-bearing structs:** A shared reference `&S` to a struct
  holding both `Freeze` fields and cell fields is still a plain value copy:
  copying `&S` copies the `Freeze` fields by value and copies the `AllocId` of
  each cell field, so all aliases of `&S` address the same `memory[a]` entry.
- **Three cell disciplines:**
  - `Cell<T>`: lock-free value reads/writes on `memory[a].content` (`cellGet`,
    `cellSet`, `cellReplace`), minting no loan.
  - `RefCell<T>`: dynamic borrow counter `memory[a].refState` (`unused`,
    `reading n`, `writing`) with explicit guard values (`refGuard`,
    `refMutGuard`) whose `Drop` lowers to lexical `guardDrop` at MIR drop
    points.
  - `UnsafeCell<T>`: `cell.get()` yields a mutable raw pointer
    `rawPtr (some a) 0 none true none`; dereferencing `&*cell.get()` or
    `&mut *cell.get()` is a raw-pointer `mint` checked against `memoryLoans`.
- **Reference-bearing cells (`UnsafeCell<&'a mut T>`, `RefCell<&'a mut T>`):**
  Store `borrow ℓ_outer v` in `memory[a].content`, reborrowing `ℓ_outer` during
  inner accesses and reconciling with `π ℓ_outer` when region `'a` and the cell
  die.

### 1.4 Unsafe pointers and the safe-reference boundary (`unsafe-pointers.md`)

The unsafe profile introduces raw pointers (`rawPtr (alloc : Option AllocId)
(offset : Int) (meta : Option Nat) (mutable : Bool) (via : Option LoanId)`) and
`RuntimeState.memory` while enforcing safe-Rust reference invariants at every
boundary where pointers and references meet:

1. **Taking a place's address (`&raw const x`, `&raw mut x` -> `addrOf`):**
   Pins the root local `x` by moving its value into a fresh allocation `a` in
   `memory` and leaving `rawAnchor a` in `x`. Subsequent safe reads, writes,
   moves, and borrows of `x.path` are lowered to checked `load`, `store`, and
   `mint` operations through `a`.
2. **Escaping a reference to a pointer (`&raw mut *r`, `r as *const T`,
   `as_mut_ptr`):**
   Yields a pointer into the lender's memory cell with **provenance by loan**
   (`via = some ℓ`, where `ℓ` is `r`'s loan). If the lender is a frame local,
   the local is anchored into `memory`; if the lender is unknown (a reference
   parameter), `r` escapes into a fresh `shadow` allocation that dies with `ℓ`.
   At escape, `r.current` is written to the cell and `ℓ`'s escape state becomes
   `open`:
   - Reads through `r` re-read the cell (requiring it initialized and valid).
   - Writes through `r` or mutable reborrows (`&mut *r`) close `ℓ`'s escape and
     invalidate all pointers with `via = some ℓ`.
   - Foreign writes close the escape; foreign reads freeze a written escape.
3. **Minting a reference from a pointer (`&*p`, `&mut *p` -> `mint`):**
   Borrows a memory cell `(alloc, interval/subvalue)` and registers a
   `MemoryLoan` in `RuntimeState.memoryLoans` (`exclusive = true` for `&mut *p`,
   `exclusive = false` freeze loan for `&*p`) with **non-lexical death** at the
   minted reference's last use. While a minted loan is live in `memoryLoans`,
   conflicting raw loads, stores, mints, or `dealloc` derive
   `Outcome.undefined` (UB).

---

## 2. Soundness review

### Verdict

**The safe prophetic core is sound, and the unsafe/interior-mutability bridge is
well-conceived, but there are 5 soundness hazards where the interaction between
prophecies, raw-pointer escapes, cell liveness, and `RefMut` write-back can
violate safe-Rust reference guarantees.**

 What is already sound and well-defended:

- **Certified loan death (`VIR-SEMANTIC-LOAN-DEATH`):** Checking that no holder
  path is read after a loan's death anchor (including across loop backedges)
  and treating aggregate component moves as reborrows prevents premature
  reconciliation `current = π ℓ` from assuming a false equation.
- **Early-bound lifetime erasure with `resultRegionDiagnostics`:** Because rustc
  categorizes any region constrained by a `where` clause (`'a: 'b`) as
  early-bound, erasing early-bound regions to "relates to all" while retaining
  unconstrained late-bound regions in `FnDef::fn_sig` is conservative on callers
  and checked on bodies.
- **Non-lexical `memoryLoans` for minted references (`&*p`, `&mut *p`):**
  Recording both exclusive (`&mut *p`) and freeze (`&*p`) loans in `memoryLoans`
  until the reference's last use—and checking every raw `load`, `store`, `copy`,
  `mint`, and `dealloc` against `memoryLoans`—soundly enforces Stacked/Tree
  Borrows protectors for minted references.

---

### 5 soundness hazards to fix

#### Hazard 1. Early prophecy reconciliation when a temporary reference escapes to a raw pointer (`s.as_mut_ptr()`, `&raw mut *r`)

- **Where:** [`unsafe-pointers.md` §4.5 ("The escaped state")](../designs/unsafe-pointers.md#45-boundary-with-the-prophetic-model)
  and [§6 ("Escaping APIs")](../designs/unsafe-pointers.md#6-library-models).
- **The issue:**
  - In Rust, the standard way to obtain a raw pointer from a reference `s : &mut T`
    (such as `let p = s.as_mut_ptr();` or `let p = &raw mut *s;`) creates a
    **temporary reborrow** `tmp = &mut *s` with loan `ℓ_temp` and prophecy
    `π ℓ_temp`.
  - By the prophetic reborrow rule
    ([`prophetic-references.md` §2.2](../designs/prophetic-references.md#22-prophecies)),
    creating `tmp = &mut *s` immediately sets `s.current := π ℓ_temp`.
  - The temporary reference `tmp` is never used again after producing `p`, so
    `tmp` (loan `ℓ_temp`) **dies immediately at the semicolon**—*before* any raw
    writes `unsafe { *p = v; }` happen!
  - According to [`unsafe-pointers.md` §4.5](../designs/unsafe-pointers.md#45-boundary-with-the-prophetic-model):
    *"`r`'s death leaves the escape open and the value in the cell, where the
    lender finds it: the reconciliation of `ℓ` reads the cell... The pointers
    stay usable under the provenance rules of §4.6."*
  - If `ℓ_temp` reconciles its prophecy `π ℓ_temp` with `memory[cell]` **at
    `tmp`'s death** (before `*p = v` executes), then `π ℓ_temp`—and therefore
    `s.current`—is equated to the **old, pre-write value** of the cell! When `s`
    later dies (without being read again as `*s`), `s` exports `s.current` (the
    old pre-write value) as `π ℓ_s`, silently losing every raw write performed
    through `p` after `tmp`'s death (or making the path vacuous if `ℓ_s` also
    reconciles with the new cell value).
- **Fix required:**
  - When a loan `ℓ` has an `open` (or `frozen`) escape at the moment its
    reference `r` dies, **reconciliation of `π ℓ` with `memory[cell]` must be
    deferred until the escape closes** (at the next write/reborrow/death of the
    parent loan `ℓ_parent`, or at frame/anchor exit)—or equivalently, dying
    with an open escape must transfer the open escape up to `ℓ_parent` so
    `ℓ_parent` reconciles directly against `memory[cell]` when `ℓ_parent`
    closes or dies.

#### Hazard 2. Safe shared reborrows (`&*r`) and direct observations (`==`, `value.observe`) of an escaped `&mut T`

- **Where:** [`unsafe-pointers.md` §4.5](../designs/unsafe-pointers.md#45-boundary-with-the-prophetic-model),
  [`prophetic-references.md` §2.1–2.2](../designs/prophetic-references.md#2-the-model),
  and [`rust-verification.md` ("Shared views of a holder")](../designs/rust-verification.md#shared-views-of-a-holder).
- **The issue:**
  - Suppose `r : &mut i32` escapes via `let q = &raw mut *r;` (so `ℓ`'s escape
    is `open` and `q` has `via = some ℓ`), and then safe code takes a shared
    reborrow `let s: &i32 = &*r;`:
    ```rust
    let q = &raw mut *r; // opens escape of ℓ; q has `via = some ℓ`
    let s: &i32 = &*r;   // safe shared reborrow (freeze) of `*r`
    unsafe { *q = 42; }  // raw write through `q` while `s: &i32` is live!
    let obs = *s;        // reads snapshot `s`
    ```
  - Rustc's borrow checker accepts this because `q = &raw mut *r` does not hold
    a rustc loan; in Rust, `*q = 42` is **UB** because it mutates memory covered
    by the live shared reference `s : &i32`.
  - However, in the current design:
    1. [`unsafe-pointers.md` §4.5](../designs/unsafe-pointers.md#45-boundary-with-the-prophetic-model)
       states that a *mutable* reborrow (`&mut *r`) closes `ℓ`'s escape, while a
       read through `r` re-reads the cell and **leaves `ℓ`'s escape `open`**.
    2. [`prophetic-references.md` §2.1–2.2](../designs/prophetic-references.md#2-the-model)
       models a safe shared reborrow (`freeze` / `&*r`) as a pure value snapshot
       that **mints no loan** in `memoryLoans`.
    3. Because `ℓ`'s escape stays `open` and `s` registers no child freeze loan
       in `memoryLoans`, `unsafe { *q = 42; }` (`via = some ℓ`) passes all raw
       store premises while `s : &i32` is live, breaking `&T` snapshot
       soundness!
  - Additionally, while `ℓ`'s escape is `open`, `r`'s `borrow ℓ current` holds a
    **stale `current`** (`unsafe-pointers.md` §4.5). Any operation that inspects
    `borrow ℓ current` directly via `RuntimeValue.observe` without a place
    dereference—such as reference equality `r == r2`
    ([`prophetic-references.md` §2.1](../designs/prophetic-references.md#21-values))
    or taking a shared view `&holder` of a struct holding `r`
    ([`rust-verification.md`](../designs/rust-verification.md#shared-views-of-a-holder))—will
    read the stale `current` instead of `memory[cell]`.
- **Fix required:**
  - Lowering a shared reborrow (`&*r` / `freeze`) of a reference backed by a
    `MemoryLoan` (or with an open escape) must re-read `memory[cell]` **and**
    register a child freeze `MemoryLoan` `ℓ_s` (`parent = some ℓ`,
    `exclusive = false`) in `memoryLoans` for `s`'s non-lexical lifetime (since
    a child loan `ℓ_s` is foreign to `via = some ℓ`, any raw write through `q`
    while `s` is live will then derive UB).
  - Reference comparisons (`==`, `!=`, `compare`) and shared-view observations
    (`&holder`) on a reference with an open escape must re-read `memory[cell]`
    (and for `&holder`, register freeze loans for the observed references'
    cells).

#### Hazard 3. `TerminatorKind::Drop` vs. `StatementKind::StorageDead` in `optimized_mir` at `-Copt-level=0`

- **Where:** [`interior-mutability.md` §3 & §4.3](../designs/interior-mutability.md#43-unsafecellt-and-the-raw-pointer-boundary),
  [`unsafe-pointers.md` §4.3](../designs/unsafe-pointers.md#43-allocation-death),
  and [`Structurize.lean`](../../verex-ir/VerexIR/Import/Structurize.lean).
- **The issue:**
  In Rust's MIR, value destruction (`TerminatorKind::Drop`) and local backing-storage
  death (`StatementKind::StorageDead` + frame `Return`) are distinct, and both
  interact with `optimized_mir` at `-Copt-level=0`:
  1. **`ElaborateDrops` strips `TerminatorKind::Drop` for `UnsafeCell<T>` and
     `Cell<T>`:**
     - Before `optimized_mir` is produced, rustc's `ElaborateDrops` pass removes
       `TerminatorKind::Drop` for any type where `needs_drop::<T>()` is `false`.
     - While `Ref<'_, T>` and `RefMut<'_, T>` implement `Drop` (so their
       `guardDrop` terminators **are** preserved in `optimized_mir`),
       **`UnsafeCell<T>` and `Cell<T>` do not implement `Drop`**
       (`needs_drop::<UnsafeCell<u32>>()` is `false`).
     - Because [`interior-mutability.md` §3 & §4.3](../designs/interior-mutability.md#3-decision)
       models `UnsafeCell<T>` and `Cell<T>` as movable `cellRef a` values (rather
       than pinned `rawAnchor`s) and ties `memory[a].live := false` to **value
       drop**, `optimized_mir` contains no `Drop` terminator for
       `let c = UnsafeCell::new(0);` at scope exit or function return. Unless
       frame exit or synthetic drop insertion kills unmoved `cellRef` locals,
       `memory[a].live` remains `true` after `return c.get();`, missing stack
       use-after-return UB.
  2. **`RemoveStorageMarkers` strips `StorageDead` at `-Copt-level=0`:**
     - [`unsafe-pointers.md` §4.3](../designs/unsafe-pointers.md#43-allocation-death)
       relies on MIR's `StorageDead` to kill an anchored local's allocation at
       the end of an inner block scope (`{ let x = 0; p = &raw const x; } *p`).
     - However, because `verex-rust-export` pins `-Copt-level=0`, rustc's
       `RemoveStorageMarkers` pass deletes **all** `StorageLive` and
       `StorageDead` statements from `optimized_mir` (extending local storage to
       the enclosing function frame, which is sound for codegen but in the
       **unsound direction** for verifying inner-scope use-after-death). And in
       Lean, [`Structurize.lean`](../../verex-ir/VerexIR/Import/Structurize.lean)
       currently erases `.storageDead` as an `inertStatement`.
  3. **Outer allocation death (`dealloc` / overwrite):**
     - If a `cellRef a_inner` is stored inside an outer heap allocation
       `a_outer` that is freed via raw `dealloc(p)` (without running `Drop`) or
       inside an anchored local `a_outer` that dies at `StorageDead`/frame exit,
       setting `memory[a_outer].live := false` does not kill `a_inner`.
- **Fix required (static `VerexSL` analysis, without trusting `rustc` `StorageDead` passes):**
  - Rather than adding `-Zmir-enable-passes=-RemoveStorageMarkers` and trusting
    `rustc`'s storage-marker pass as part of the TCB (which would conflict with
    the principle in [`prophetic-references.md` §2.2](../designs/prophetic-references.md#22-prophecies)
    that *"rustc does not guard VerexSL"*), `VerexSL` should statically compute
    and certify allocation death on the structured `VerexSL` program itself—using
    the same architecture as `CheckedLoanFact.deaths` and
    `VIR-SEMANTIC-LOAN-DEATH` in [`prophetic-references.md` §3](../designs/prophetic-references.md#3-validation-loan-elimination):
    1. **Static owner-liveness / last-use death for `rawAnchor` and `cellRef`
       locals:** In `VerexSL`, an anchored local (`rawAnchor a l`) or a local
       carrying `cellRef a` dies as soon as (a) the local is no longer directly
       referenced along any continuation (its last direct use) **and** (b) all
       safe borrows (`&x` / `&mut x`) derived from it have died, or at frame
       `Return` if it is used until exit. Because every direct mention of a Rust
       local `x` must syntactically occur inside `x`'s lexical scope `{ ... }`,
       `x`'s last direct use in `VerexSL` is guaranteed to occur **at or before**
       Rust's lexical scope exit (`StorageDead`). Killing `a` at that certified
       last-use point is therefore **strictly more conservative (sound)** than
       Rust's `StorageDead` and catches inner-scope escapes such as
       `{ let x = 0; p = &raw const x; } *p` and
       `{ let c = UnsafeCell::new(0); p = c.get(); } *p` without relying on
       `StorageDead` or `Drop` glue.
    2. **Overwrite and outer-deallocation death:** Whenever a place holding
       `cellRef a_inner` (directly or nested in a struct/enum) is overwritten or
       its enclosing allocation `a_outer` is killed (by owner death or raw
       `dealloc`), `VerexSL` recursively requires no live loans on and sets
       `live := false` for every nested `cellRef a_inner`.

#### Hazard 4. `RefMut` buffered write-back vs. `std::mem::forget` followed by `get_mut()` / `into_inner()`

- **Where:** [`interior-mutability.md` §4.1, §4.5, and §7.2](../designs/interior-mutability.md#45-refcellt-and-observable-guard-drop).
- **The issue:**
  - In [`interior-mutability.md` §4.5](../designs/interior-mutability.md#45-refcellt-and-observable-guard-drop),
    `RefCell::borrow_mut` copies the cell's value into `refMutGuard a path
    current`, `DerefMut::deref_mut` mutates `guard.current` prophetically, and
    the updated `guard.current` is written back to `memory[a].content` **only
    when `guardDrop` runs**.
  - In 100% safe Rust, a caller can mutate through a `RefMut<'b, T>` guard,
    call `std::mem::forget(guard)` (suppressing `guardDrop`), wait for the
    lexical borrow lifetime `'b` on `&'b cell` to end, and then call
    `cell.get_mut()` or `cell.into_inner()`:
    ```rust
    let mut cell = RefCell::new(1);
    {
        let mut g = cell.borrow_mut();
        *g = 2;
        std::mem::forget(g); // safe! `'b` ends at block exit
    }
    assert_eq!(cell.into_inner(), 2); // real Rust: 2; buffered model: 1!
    ```
  - Because real Rust's `RefMut` mutates the `UnsafeCell` in place during
    `DerefMut` while `get_mut()` and `into_inner()` bypass the borrow counter
    (relying on exclusive ownership of `cell`), buffering the write in
    `guard.current` until `guardDrop` causes `cell.into_inner()` /
    `cell.get_mut()` to observe the **stale value `1` instead of `2`** (an
    unsoundness in safe Rust!).
- **Fix required:**
  - Either reject `std::mem::forget` on `RefMut` (and types containing `RefMut`),
    **or** (cleaner) have `DerefMut::deref_mut(&mut guard)` mint its prophetic
    borrow directly against `memory[a].content` at `path` (reconciling into
    `memory[a].content` as soon as the short `&'g mut U` borrow from `deref_mut`
    dies, which rustc guarantees happens before `guard` can be moved into
    `mem::forget(guard)`), leaving `guardDrop` responsible only for resetting
    `memory[a].refState := .unused` and releasing the guard's `MemoryLoan`.

#### Hazard 5. Framing of `cellRef` `AllocId`s across `&mut S` calls

- **Where:** [`interior-mutability.md` §3 & §5](../designs/interior-mutability.md#5-verification-and-validation)
  and [`prophetic-references.md` §2.2](../designs/prophetic-references.md#22-prophecies).
- **The issue:**
  - When a struct `s : S` containing both `Freeze` fields and cell fields
    (`cell : Cell<u32>` with `cellRef a`) is passed as `&mut S` to a helper
    function `fn bump_freeze(s: &mut S)` whose contract only constrains
    `ensures s.freeze == old(s).freeze + 1`, the prophetic model assigns `s` on
    exit the symbolic prophecy `π ℓ : S`, whose `cell` field is a **fresh
    symbolic `AllocId` `a_π`**.
  - Unless the contract or an automatic type/call framing invariant preserves
    `s.cell == old(s).cell`, the caller loses the fact that `s.cell` still
    names allocation `a` (and if the caller also held a shared copy of `a` or
    relied on pairwise disjointness `a_heap ≠ a_rws` from `S`'s invariant, that
    fact is lost across any `&mut S` call that does not restate every `AllocId`
    equality).
- **Fix required:**
  - Note this alongside Open Question 1 of
    [`interior-mutability.md` §9](../designs/interior-mutability.md#9-open-questions):
    for any `&mut T` call whose callee does not replace the cell owner, framing
    or the struct well-formedness invariant should preserve `cellRef` `AllocId`
    equalities automatically unless the field is modified.

---

## 3. Summary table

| Mechanism | Soundness status | Key condition / caveat |
|---|---|---|
| Safe `&T` as value snapshot (`T: Freeze`) | **Sound** | Requires that raw writes through escaped pointers cannot occur while a safe shared reborrow `&*r` is live (Hazard 2). |
| Safe `&mut T` as `(current, prophecy)` + `VIR-SEMANTIC-LOAN-DEATH` | **Sound** | Death anchors checked against all continuations (including loop backedges); aggregate component moves treated as reborrows. |
| References in aggregates & cursor transfer (Step 3b) | **Sound** | Runtime clears moved components (`clearBorrowValue`) on transfer so stale slots cannot intercept loan-id write-backs. |
| Shared views of `&mut` holders (`&S`, Step 3e) | **Sound in safe Rust; needs care with escapes** | Erasing loans under `&S` (`value.observe`) is sound when `s` has no open raw-pointer escape; if `s` has an open escape, `&s` must re-read `memory` and freeze the cell (Hazard 2). |
| Declared signature lifetimes & early-bound erasure | **Sound** | Relies on rustc making all `where`-constrained lifetimes early-bound (`relates to all`) and checks bodies with `resultRegionDiagnostics`. |
| Generics instantiated at `&mut U` | **Sound** | Blocked from generic transfer by `refFree` + `dropsDestructor`; verified per substituted instance (`Term.callInstance`). |
| `&raw` place anchoring (`rawAnchor`) | **Sound** | Local is pinned; all subsequent safe accesses lower to `load`/`store`/`mint` on the backing allocation. |
| Reference escape (`&raw mut *r`, `as_mut_ptr`) | **Unsound on temporary reborrows as-is** | Reconciling `π ℓ` at `r`'s death while `ℓ`'s escape remains `open` loses raw writes made after `r` dies (Hazard 1); must defer reconciliation until the escape closes. |
| Minted references (`&*p`, `&mut *p`) + non-lexical `memoryLoans` | **Sound** | Enforces Stacked/Tree Borrows protectors against raw loads, stores, and `dealloc` while the minted reference is used. |
| `UnsafeCell<T>` / `Cell<T>` / `RefCell<T>` as `cellRef AllocId` | **Needs static owner-death and `RefMut` fixes** | Must statically certify and kill `rawAnchor`/`cellRef` allocations at owner last-use/overwrite/frame exit/outer `dealloc` without relying on `StorageDead` or `Drop` glue (Hazard 3), and reconcile `RefMut` writes at `DerefMut` death rather than `guardDrop` to stay sound under `mem::forget` (Hazard 4). |
