// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

// edition:2021

// Cases the `unsafe_cell_reborrow` lint must flag, and ones it must not. The
// test's `dylint.toml` (in `src/lib.rs`) opts in `Ctx`, `Leaky`, `Adapter`
// and `Store`, lists `anchor_borrow` as the anchor function, and names two structs that
// cannot be checked: `NoCells`, and `Renamed`, which does not exist (flagged
// at the top of the file).
#![allow(dead_code, unused_variables, unused_unsafe, unused_mut, improper_ctypes)]

use std::cell::UnsafeCell;

/// A local that a reborrow is tied to.
struct Anchor;

trait CellBorrow<'s> {
    type Anchored;
    fn anchored(self) -> Self::Anchored;
}

impl<'s, 'l: 's, T: ?Sized> CellBorrow<'s> for &'l T {
    type Anchored = &'s T;
    fn anchored(self) -> &'s T {
        self
    }
}

impl<'s, 'l: 's, T: ?Sized> CellBorrow<'s> for &'l mut T {
    type Anchored = &'s mut T;
    fn anchored(self) -> &'s mut T {
        self
    }
}

/// Shortens `borrow` to the lifetime of `anchor`.
fn anchor_borrow<'s, B: CellBorrow<'s>>(_anchor: &'s mut Anchor, borrow: B) -> B::Anchored {
    borrow.anchored()
}

/// A site: a reborrow tied to a fresh local anchor.
macro_rules! borrow_cell {
    (let $name:ident = $borrow:expr) => {
        let mut anchor = Anchor;
        let $name = anchor_borrow(&mut anchor, $borrow);
    };
}

pub struct Heap(u64);
struct Rws(u64);
struct Pool;

trait Loader {
    fn lookup(&self, x: u64) -> u64;
}

/// A context: `&self` methods that mutate through two cells.
struct Ctx<'a> {
    heap: UnsafeCell<&'a mut Heap>,
    rws: UnsafeCell<&'a mut Rws>,
    pool: Pool,
    loader: &'a dyn Loader,
}

/// Flagged: a cell field must be private.
pub struct Leaky<'a> {
    pub cell: UnsafeCell<&'a mut Heap>,
}

fn alloc(heap: &mut Heap, rws: &mut Rws) -> u64 {
    heap.0 += 1;
    rws.0 += 1;
    heap.0
}

impl<'a> Ctx<'a> {
    // Not flagged: a leaf reborrows once and passes the components down.
    fn leaf(&self) -> u64 {
        borrow_cell!(let heap = unsafe { &mut **self.heap.get() });
        borrow_cell!(let rws = unsafe { &mut **self.rws.get() });
        alloc(heap, rws)
    }

    fn heap_only(&self) -> u64 {
        borrow_cell!(let heap = unsafe { &mut **self.heap.get() });
        heap.0
    }

    // Not flagged: holds `rws`; the callee reborrows only `heap`.
    fn disjoint(&self) -> u64 {
        borrow_cell!(let rws = unsafe { &mut **self.rws.get() });
        rws.0 += 1;
        self.heap_only()
    }

    // Not flagged: a call through a field of the struct while holding.
    fn through_field(&self) -> u64 {
        borrow_cell!(let heap = unsafe { &mut **self.heap.get() });
        self.loader.lookup(heap.0)
    }

    // Not flagged: holds nothing, so it may call anything, even outside the crate.
    fn orchestrator(&self) -> u64 {
        let a = self.leaf();
        let b = std::hint::black_box(self).leaf();
        a + b
    }

    // Flagged: `leaf` reborrows `heap`, which is held here.
    fn overlapping_call(&self) -> u64 {
        borrow_cell!(let heap = unsafe { &mut **self.heap.get() });
        let n = self.leaf();
        heap.0 + n
    }

    // Flagged: the same, through `orchestrator`.
    fn overlapping_transitive(&self) {
        borrow_cell!(let rws = unsafe { &mut **self.rws.get() });
        self.orchestrator();
    }

    // Flagged: the same cell reborrowed twice.
    fn twice(&self) {
        borrow_cell!(let a = unsafe { &mut **self.heap.get() });
        borrow_cell!(let b = unsafe { &mut **self.heap.get() });
    }

    // Flagged: no anchor, so nothing stops the reborrow from being returned.
    fn unanchored(&self) -> &mut Heap {
        unsafe { &mut **self.heap.get() }
    }

    // Flagged: the anchor is a parameter, which outlives this function.
    fn foreign_anchor(&self, anchor: &mut Anchor) {
        let heap = anchor_borrow(anchor, unsafe { &mut **self.heap.get() });
    }

    // Not flagged: the return type could carry a reference to the heap, but
    // the anchor stops the compiler from letting one out.
    fn peek(&self) -> Option<&Heap> {
        borrow_cell!(let heap = unsafe { &**self.heap.get() });
        None
    }

    // Not flagged: a reference to what another cell holds is a component the
    // caller lent.
    fn takes_component(&self, heap: &mut Heap) {
        borrow_cell!(let rws = unsafe { &mut **self.rws.get() });
        heap.0 += rws.0;
    }

    // Flagged: the cell field used outside a reborrow site.
    fn misuse(&self) {
        let cell = &self.heap;
    }

    // Flagged: the struct reaches code outside the crate while holding.
    fn to_external(&self) {
        borrow_cell!(let heap = unsafe { &mut **self.heap.get() });
        std::hint::black_box(self);
    }

    // Flagged: a closure capturing the struct is passed as an argument.
    fn closure_arg(&self) {
        let v = Some(1).map(|_| self.leaf());
    }

    // Flagged: a method called on a type parameter while holding.
    fn generic_call<V: Value>(&self, v: V) {
        borrow_cell!(let heap = unsafe { &mut **self.heap.get() });
        v.write(heap.0);
    }

    // Flagged: a call through a function value while holding.
    fn fn_value(&self, f: impl Fn()) {
        borrow_cell!(let heap = unsafe { &mut **self.heap.get() });
        f();
    }

    // Flagged: a method called on a trait object while holding.
    fn dyn_local(&self) {
        borrow_cell!(let heap = unsafe { &mut **self.heap.get() });
        let l: &dyn Loader = &NoLoader;
        l.lookup(heap.0);
    }

    // Flagged twice: as above, and a trait-object parameter could carry a
    // route back to the struct.
    fn dyn_param(&self, l: &dyn Loader) {
        borrow_cell!(let heap = unsafe { &mut **self.heap.get() });
        l.lookup(heap.0);
    }

    // Flagged: a pattern names the cell field without a site.
    fn by_pattern(&self) {
        let Ctx { heap, .. } = self;
    }

    // Not flagged: the site runs once per iteration, but each reborrow is tied
    // to that iteration's anchor.
    fn in_loop(&self) {
        for _ in 0..2 {
            borrow_cell!(let heap = unsafe { &mut **self.heap.get() });
            heap.0 += 1;
        }
    }

    // Flagged: `hands_out` passes the struct outside the crate, so it may
    // come back in.
    fn leak_in_callee(&self) {
        borrow_cell!(let heap = unsafe { &mut **self.heap.get() });
        hands_out(self);
    }

    // Flagged: the struct leaves inside a struct of this crate.
    fn leak_wrapped(&self) {
        borrow_cell!(let heap = unsafe { &mut **self.heap.get() });
        std::hint::black_box(Wrapper { ctx: self });
    }

    // Flagged: the struct goes to a generic parameter; what `generic_helper`
    // calls on it cannot be followed.
    fn to_generic(&self) {
        generic_helper(self);
    }

    // Flagged: an operator that resolves to an impl in this crate is a call.
    fn by_operator(&self) -> u64 {
        borrow_cell!(let heap = unsafe { &mut **self.heap.get() });
        Token(self) + Token(self)
    }

    // Flagged: dropping the guard calls `leaf` while `heap` is held.
    fn drops_guard(&self) {
        borrow_cell!(let heap = unsafe { &mut **self.heap.get() });
        let guard = Guard(self);
    }

    // Flagged: a dereference through a `Deref` impl in this crate is a call.
    fn by_deref(&self) -> u64 {
        borrow_cell!(let heap = unsafe { &mut **self.heap.get() });
        let view = View { ctx: self };
        view.0
    }

    // Not flagged: a generic parameter, but no cell is held here.
    fn generic_no_hold(&self, f: impl Fn()) {
        f();
    }
}

fn hands_out(ctx: &Ctx<'_>) {
    std::hint::black_box(ctx);
}

fn generic_helper<C: CtxTrait>(ctx: &C) -> u64 {
    ctx.leaf()
}

struct Wrapper<'c, 'a> {
    ctx: &'c Ctx<'a>,
}

struct Token<'c, 'a>(&'c Ctx<'a>);

impl std::ops::Add for Token<'_, '_> {
    type Output = u64;

    fn add(self, other: Self) -> u64 {
        self.0.leaf() + other.0.leaf()
    }
}

struct View<'c, 'a> {
    ctx: &'c Ctx<'a>,
}

impl std::ops::Deref for View<'_, '_> {
    type Target = Heap;

    fn deref(&self) -> &Heap {
        self.ctx.leaf();
        &Heap(0)
    }
}

struct Guard<'c, 'a>(&'c Ctx<'a>);

impl Drop for Guard<'_, '_> {
    fn drop(&mut self) {
        self.0.leaf();
    }
}

struct NoLoader;

impl Loader for NoLoader {
    fn lookup(&self, x: u64) -> u64 {
        x
    }
}

trait Value {
    fn write(self, n: u64);
}

trait CtxTrait {
    fn leaf(&self) -> u64;
}

impl CtxTrait for Ctx<'_> {
    fn leaf(&self) -> u64 {
        Ctx::leaf(self)
    }
}

// Not flagged: a second cell struct whose methods reborrow once each, one of
// them shared.
struct Adapter<'a> {
    set: UnsafeCell<&'a mut Rws>,
}

impl Adapter<'_> {
    fn read(&self) -> u64 {
        borrow_cell!(let set = unsafe { &**self.set.get() });
        set.0
    }

    fn write(&self) {
        borrow_cell!(let set = unsafe { &mut **self.set.get() });
        set.0 = 1;
    }
}

// An owned cell is the same hazard, reborrowed with one `*` instead of two.
struct Store {
    inner: UnsafeCell<Vec<u64>>,
}

impl Store {
    // Not flagged: one reborrow, nothing called that could reborrow again.
    fn push(&self, x: u64) {
        borrow_cell!(let inner = unsafe { &mut *self.inner.get() });
        inner.push(x);
    }

    // Flagged: the callback runs while the store is reborrowed.
    fn each(&self, mut f: impl FnMut(u64)) {
        borrow_cell!(let inner = unsafe { &*self.inner.get() });
        for x in inner.iter() {
            f(*x);
        }
    }
}

// Not flagged: the check is opt-in, and this struct is not listed.
struct NotListed<'a> {
    cell: UnsafeCell<&'a mut Heap>,
}

impl NotListed<'_> {
    fn twice(&self) {
        let a = unsafe { &mut **self.cell.get() };
        let b = unsafe { &mut **self.cell.get() };
    }
}

// Flagged: listed, but there is no `UnsafeCell` field to check.
struct NoCells {
    plain: u64,
}

#[cfg_attr(dylint_lib = "aptos_lints", allow(unsafe_cell_reborrow))]
fn exempted(ctx: &Ctx<'_>) -> u64 {
    let heap = unsafe { &mut **ctx.heap.get() };
    ctx.leaf()
}

// ---------------------------------------------------------------------------
// Cases from a soundness audit. Each one re-enters `heap` while it is held,
// in a way an earlier version of the lint accepted.
// ---------------------------------------------------------------------------

struct OtherErr;
struct CtxErr<'c, 'a>(&'c Ctx<'a>);

impl From<CtxErr<'_, '_>> for OtherErr {
    fn from(e: CtxErr<'_, '_>) -> Self {
        e.0.leaf();
        OtherErr
    }
}

struct Shown<'c, 'a>(&'c Ctx<'a>);

impl std::fmt::Display for Shown<'_, '_> {
    fn fmt(&self, _: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.leaf();
        Ok(())
    }
}

fn makes_guard(ctx: &Ctx<'_>) {
    let _guard = Guard(ctx);
}

fn calls_dyn(d: &dyn CtxTrait) -> u64 {
    d.leaf()
}

struct DynWrap<'x>(&'x dyn CtxTrait);

fn calls_wrap(w: DynWrap<'_>) -> u64 {
    w.0.leaf()
}

trait RefGo {
    fn go(self) -> u64;
}

impl RefGo for &Ctx<'_> {
    fn go(self) -> u64 {
        self.leaf()
    }
}

fn generic_ref<T>(t: &T) -> u64
where
    for<'x> &'x T: RefGo,
{
    t.go()
}

extern "C" {
    fn ffi_poke(ctx: &Ctx<'_>);
}

impl Ctx<'_> {
    // Flagged: the `From` impl that `?` calls reaches the struct. The call
    // is inside the desugaring of `?`.
    fn in_question_mark(&self) -> Result<(), OtherErr> {
        borrow_cell!(let heap = unsafe { &mut **self.heap.get() });
        Err(CtxErr(self))?;
        Ok(())
    }

    // Flagged: the same through `format!`, a macro of another crate.
    fn in_format(&self) -> String {
        borrow_cell!(let heap = unsafe { &mut **self.heap.get() });
        format!("{}", Shown(self))
    }

    // Flagged: the callee drops a guard whose destructor reborrows `heap`.
    fn drop_in_callee(&self) {
        borrow_cell!(let heap = unsafe { &mut **self.heap.get() });
        makes_guard(self);
    }

    // Flagged: turned into a trait object by a `let`, not at a call.
    fn hidden_by_let(&self) -> u64 {
        borrow_cell!(let heap = unsafe { &mut **self.heap.get() });
        let d: &dyn CtxTrait = self;
        calls_dyn(d)
    }

    // Flagged: the same by a cast.
    fn hidden_by_cast(&self) -> u64 {
        borrow_cell!(let heap = unsafe { &mut **self.heap.get() });
        calls_dyn(self as &dyn CtxTrait)
    }

    // Flagged: the same in a constructor argument.
    fn hidden_in_constructor(&self) -> u64 {
        borrow_cell!(let heap = unsafe { &mut **self.heap.get() });
        calls_wrap(DynWrap(self))
    }

    // Flagged: the callee's bound is on `&T`, not on `T`.
    fn bound_on_reference(&self) -> u64 {
        borrow_cell!(let heap = unsafe { &mut **self.heap.get() });
        generic_ref(self)
    }

    // Flagged: an `extern` declaration is local but its code is not.
    fn to_extern(&self) {
        borrow_cell!(let heap = unsafe { &mut **self.heap.get() });
        unsafe { ffi_poke(self) };
    }
}

impl Ctx<'_> {
    // Flagged: `Vec` is another crate's type with a destructor, and what it
    // holds reaches the struct. The lint cannot see what that destructor
    // does, so it assumes the worst.
    fn drops_foreign(&self) {
        borrow_cell!(let heap = unsafe { &mut **self.heap.get() });
        let others: Vec<&Ctx<'_>> = Vec::new();
    }
}

struct Carrier<T>(T);

impl<T> Carrier<T> {
    // Flagged: `self` is of generic type, and this is not a cell struct.
    fn consume(self, ctx: &Ctx<'_>) {
        borrow_cell!(let heap = unsafe { &mut **ctx.heap.get() });
        drop(self);
        heap.0 += 1;
    }
}

// Flagged: the closure holds a cell and captures a value of generic type.
fn captures_generic<T>(ctx: &Ctx<'_>, t: T) {
    let closure = move || {
        borrow_cell!(let heap = unsafe { &mut **ctx.heap.get() });
        drop(t);
        heap.0 += 1;
    };
    closure();
}

struct TupleCell(UnsafeCell<Vec<u64>>);

impl TupleCell {
    // Flagged: a tuple-struct pattern binds the cell.
    fn by_tuple_pattern(&self) {
        let TupleCell(cell) = self;
    }
}

struct Nested {
    cell: UnsafeCell<u64>,
    // Flagged: the lint only understands a field of type `UnsafeCell<T>`.
    // It cannot check this form, so it rejects it.
    in_array: [UnsafeCell<u64>; 2],
    // Flagged: the same inside `Option`.
    in_option: Option<UnsafeCell<u64>>,
    // Not flagged: `Cell` is a type of its own.
    flag: std::cell::Cell<bool>,
}

fn main() {}

impl std::fmt::Display for Ctx<'_> {
    // Flagged: `Formatter` is another crate's type that contains a trait
    // object, which could carry a route back to the struct.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        borrow_cell!(let heap = unsafe { &mut **self.heap.get() });
        f.write_str("ctx")
    }
}

trait Family {
    type Out;
}

// Does not mention `Ctx` itself; only its `Out` does.
struct ToCtx;

impl Family for ToCtx {
    type Out = *const Ctx<'static>;
}

struct Assoc<F: Family> {
    inner: F::Out,
}

impl std::fmt::Display for Assoc<ToCtx> {
    fn fmt(&self, _: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        unsafe { (*self.inner).leaf() };
        Ok(())
    }
}

impl Ctx<'_> {
    // Flagged: `Assoc<ToCtx>` reaches the struct through a field whose type
    // is an associated type.
    fn through_associated_type(&self) {
        borrow_cell!(let heap = unsafe { &mut **self.heap.get() });
        let w = Assoc::<ToCtx> {
            inner: self as *const Ctx<'_> as *const Ctx<'static>,
        };
        let _ = w.to_string();
    }
}

fn apply<T>(v: T, f: fn(T) -> u64) -> u64 {
    f(v)
}

struct Local;

fn from_bound<T>(t: T) -> u64
where
    Local: From<T>,
{
    let _ = Local::from(t);
    0
}

impl<'c, 'a> From<&'c Ctx<'a>> for Local {
    fn from(ctx: &'c Ctx<'a>) -> Self {
        ctx.leaf();
        Local
    }
}

fn shows<F: Family>(w: Assoc<F>) -> String
where
    Assoc<F>: std::fmt::Display,
{
    w.to_string()
}

impl Ctx<'_> {
    // Flagged: a type parameter receives the struct. It needs no bound: the
    // callee hands the value to a function value.
    fn to_unbounded(&self) -> u64 {
        apply(self, Ctx::leaf)
    }

    // Flagged: the bound that lets the callee use the value does not have
    // the type parameter as its self type.
    fn to_from_bound(&self) -> u64 {
        from_bound(self)
    }

    // Flagged: the value reaches the struct through an associated type of
    // the type argument, not through the argument itself.
    fn to_generic_struct(&self) -> String {
        shows(Assoc::<ToCtx> {
            inner: self as *const Ctx<'_> as *const Ctx<'static>,
        })
    }

    // Flagged: the cast leaves a pointer whose type no longer shows the
    // struct.
    fn erased_by_cast(&self) -> *const () {
        self as *const Self as *const ()
    }

    // Flagged: an `async` body can return to its caller at an `.await` with
    // the reborrow still live.
    async fn in_async(&self) {
        borrow_cell!(let heap = unsafe { &mut **self.heap.get() });
        std::future::ready(()).await;
    }
}

// Each level is a new type, so a walk of it never ends.
struct Node<T> {
    val: T,
    next: *const Node<(T,)>,
}

// Flagged: the lint gives up on `Node<u64>` and assumes the worst about it.
fn endless(ctx: &Ctx<'_>, node: &Node<u64>) {
    borrow_cell!(let heap = unsafe { &mut **ctx.heap.get() });
}
