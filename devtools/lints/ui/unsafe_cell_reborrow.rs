// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

// Cases the `unsafe_cell_reborrow` lint must flag, and ones it must not. The
// test's `dylint.toml` lists `Inert` as an inert trait and `CtxTrait` as a
// context trait.
#![allow(dead_code, unused_variables, unused_unsafe, unused_mut)]

use std::cell::UnsafeCell;

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
        let heap = unsafe { &mut **self.heap.get() };
        let rws = unsafe { &mut **self.rws.get() };
        alloc(heap, rws)
    }

    fn heap_only(&self) -> u64 {
        let heap = unsafe { &mut **self.heap.get() };
        heap.0
    }

    // Not flagged: holds `rws`; the callee reborrows only `heap`.
    fn disjoint(&self) -> u64 {
        let rws = unsafe { &mut **self.rws.get() };
        rws.0 += 1;
        self.heap_only()
    }

    // Not flagged: a call through a field of the struct while holding.
    fn through_field(&self) -> u64 {
        let heap = unsafe { &mut **self.heap.get() };
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
        let heap = unsafe { &mut **self.heap.get() };
        let n = self.leaf();
        heap.0 + n
    }

    // Flagged: the same, through `orchestrator`.
    fn overlapping_transitive(&self) {
        let rws = unsafe { &mut **self.rws.get() };
        self.orchestrator();
    }

    // Flagged: the same cell reborrowed twice.
    fn twice(&self) {
        let a = unsafe { &mut **self.heap.get() };
        let b = unsafe { &mut **self.heap.get() };
    }

    // Flagged: the reborrow escapes through the return type.
    fn escape(&self) -> &mut Heap {
        unsafe { &mut **self.heap.get() }
    }

    // Flagged: an out-parameter that can hold a reborrow.
    fn out_param(&self, out: &mut Option<&mut Heap>) {}

    // Not flagged: a plain `&mut Heap` parameter cannot store anything.
    fn takes_component(&self, heap: &mut Heap) {
        heap.0 += 1;
    }

    // Flagged: the cell field used outside a reborrow site.
    fn misuse(&self) {
        let cell = &self.heap;
    }

    // Flagged: the struct reaches code outside the crate while holding.
    fn to_external(&self) {
        let heap = unsafe { &mut **self.heap.get() };
        std::hint::black_box(self);
    }

    // Flagged: a closure capturing the struct is passed as an argument.
    fn closure_arg(&self) {
        let v = Some(1).map(|_| self.leaf());
    }

    // Flagged: a method called on a type parameter while holding.
    fn generic_call<V: Value>(&self, v: V) {
        let heap = unsafe { &mut **self.heap.get() };
        v.write(heap.0);
    }

    // Not flagged: the same on an inert trait; its impls are checked instead.
    fn inert_call<V: Inert>(&self, v: V) {
        let heap = unsafe { &mut **self.heap.get() };
        v.write(heap.0);
    }

    // Flagged: a call through a function value while holding.
    fn fn_value(&self, f: impl Fn()) {
        let heap = unsafe { &mut **self.heap.get() };
        f();
    }

    // Flagged: a method called on a trait object while holding.
    fn dyn_local(&self) {
        let heap = unsafe { &mut **self.heap.get() };
        let l: &dyn Loader = &NoLoader;
        l.lookup(heap.0);
    }

    // Flagged twice: as above, and a trait-object parameter can hold a
    // reborrow.
    fn dyn_param(&self, l: &dyn Loader) {
        let heap = unsafe { &mut **self.heap.get() };
        l.lookup(heap.0);
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

trait Inert {
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

// Not flagged: an inert impl for plain data.
struct Plain(u64);
impl Inert for Plain {
    fn write(self, _: u64) {}
}

// Flagged twice: an inert impl for a type bounded by the context trait, whose
// method calls the context.
struct Evil<'c, C: CtxTrait>(&'c C);
impl<'c, C: CtxTrait> Inert for Evil<'c, C> {
    fn write(self, _: u64) {
        self.0.leaf();
    }
}

// Flagged: an inert impl for a type holding a trait object of the context trait.
struct Hidden<'c>(&'c dyn CtxTrait);
impl Inert for Hidden<'_> {
    fn write(self, _: u64) {}
}

// Not flagged: a second cell struct whose methods reborrow once each, one of
// them shared.
struct Adapter<'a> {
    set: UnsafeCell<&'a mut Rws>,
}

impl Adapter<'_> {
    fn read(&self) -> u64 {
        let set = unsafe { &**self.set.get() };
        set.0
    }

    fn write(&self) {
        let set = unsafe { &mut **self.set.get() };
        set.0 = 1;
    }
}

#[cfg_attr(dylint_lib = "aptos_lints", allow(unsafe_cell_reborrow))]
fn exempted(ctx: &Ctx<'_>) -> u64 {
    let heap = unsafe { &mut **ctx.heap.get() };
    ctx.leaf()
}

fn main() {}
