// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// Closures called directly: a closure without a specification is inlined at its
// call. Each function has a false twin, whose clause is reported here.

pub fn direct(c: u32) -> u32 {
    let g = |a: u32| a + c;
    g(1) + g(2)
}

pub fn direct_false(c: u32) -> u32 {
    let g = |a: u32| a + c;
    g(1) + g(2)
}

pub fn no_upvars(x: u32) -> u32 {
    let g = |a: u32, b: u32| a + b;
    g(x, 1)
}

pub fn no_upvars_false(x: u32) -> u32 {
    let g = |a: u32, b: u32| a + b;
    g(x, 1)
}

pub fn once(v: Vec<u32>) -> usize {
    let take = move || v.len();
    take()
}

pub fn once_false(v: Vec<u32>) -> usize {
    let take = move || v.len();
    take()
}

pub fn consume(v: Vec<u32>) -> Vec<u32> {
    let take = move || v;
    take()
}

pub fn consume_false(v: Vec<u32>) -> Vec<u32> {
    let take = move || v;
    take()
}

pub fn panics(c: u32) -> u32 {
    let g = |a: u32| a + c;
    g(1)
}

pub fn panics_guarded(c: u32) -> u32 {
    let g = |a: u32| a + c;
    g(1)
}

pub fn nested(x: u32) -> u32 {
    let outer = |y: u32| {
        let inner = |z: u32| z + y + x;
        inner(1)
    };
    outer(2)
}

pub fn nested_false(x: u32) -> u32 {
    let outer = |y: u32| {
        let inner = |z: u32| z + y + x;
        inner(1)
    };
    outer(2)
}

pub fn copied(c: u32) -> u32 {
    let g = |a: u32| a + c;
    let h = g;
    g(1) + h(2)
}

pub fn zero_sized_copy(x: u32) -> u32 {
    let g = |a: u32| a + 1;
    let h = g;
    h(x) + g(x)
}

pub fn zero_sized_capture(x: u32) -> u32 {
    let g = |a: u32| a + 1;
    let h = move |b: u32| g(b);
    h(x)
}

pub fn zero_sized_capture_false(x: u32) -> u32 {
    let g = |a: u32| a + 1;
    let h = move |b: u32| g(b);
    h(x)
}

// `&mut` and move upvars.

pub fn counter(n0: u32) -> u32 {
    let mut n = n0;
    let mut inc = |d: u32| {
        n += d;
    };
    inc(1);
    inc(2);
    n
}

pub fn counter_false(n0: u32) -> u32 {
    let mut n = n0;
    let mut inc = |d: u32| {
        n += d;
    };
    inc(1);
    inc(2);
    n
}

pub fn write_through(a: &mut u32, b: u32) {
    let mut write = |x: u32| {
        *a = x;
    };
    write(b);
}

pub fn write_through_false(a: &mut u32, b: u32) {
    let mut write = |x: u32| {
        *a = x;
    };
    write(b);
}

pub fn moved_counter(start: u32) -> u32 {
    let mut count = start;
    let mut next = move || {
        count += 1;
        count
    };
    next();
    next()
}

pub fn moved_counter_false(start: u32) -> u32 {
    let mut count = start;
    let mut next = move || {
        count += 1;
        count
    };
    next();
    next()
}

pub fn moved_copy_keeps(start: u32) -> u32 {
    let mut count = start;
    let mut next = move || {
        count += 1;
        count
    };
    next();
    count
}

pub fn moved_copy_keeps_false(start: u32) -> u32 {
    let mut count = start;
    let mut next = move || {
        count += 1;
        count
    };
    next();
    count
}

pub fn moved_mut(x: &mut u32) {
    let mut set = move || {
        *x = 4;
    };
    set();
}

pub fn moved_mut_false(x: &mut u32) {
    let mut set = move || {
        *x = 4;
    };
    set();
}

pub fn generic<T>(x: T) -> T {
    let take = move || x;
    take()
}

pub struct Guard(pub u32);

impl Drop for Guard {
    fn drop(&mut self) {}
}

// A closure holding a value with a destructor runs it.
pub fn holds_guard(guard: Guard) -> u32 {
    let read = move || guard.0;
    read()
}

// The body sees its `&mut` upvar through a shared view of the holder.
pub fn shared_view(x: &mut u32) -> u32 {
    let read = move || *x;
    read()
}

pub fn shared_view_false(x: &mut u32) -> u32 {
    let read = move || *x;
    read()
}

// A closure holding a closure: by value, and by mutable reference, the outer writes
// through the inner's upvar.
pub fn holds_closure(c: u32) -> u32 {
    let g = |a: u32| a + c;
    let h = move |b: u32| g(b);
    h(1)
}

pub fn holds_closure_false(c: u32) -> u32 {
    let g = |a: u32| a + c;
    let h = move |b: u32| g(b);
    h(1)
}

pub fn holds_mut_closure(n0: u32) -> u32 {
    let mut n = n0;
    let mut inc = |d: u32| {
        n += d;
    };
    let mut twice = move |d: u32| {
        inc(d);
        inc(d);
    };
    twice(1);
    n
}

pub fn holds_mut_closure_false(n0: u32) -> u32 {
    let mut n = n0;
    let mut inc = |d: u32| {
        n += d;
    };
    let mut twice = move |d: u32| {
        inc(d);
        inc(d);
    };
    twice(1);
    n
}

pub fn borrows_mut_closure(n0: u32) -> u32 {
    let mut n = n0;
    let mut inc = |d: u32| {
        n += d;
    };
    let mut twice = |d: u32| {
        inc(d);
        inc(d);
    };
    twice(1);
    n
}

pub fn borrows_mut_closure_false(n0: u32) -> u32 {
    let mut n = n0;
    let mut inc = |d: u32| {
        n += d;
    };
    let mut twice = |d: u32| {
        inc(d);
        inc(d);
    };
    twice(1);
    n
}

// A zero-sized closure returned by its parent is a value of its type in the caller.
pub fn make() -> impl Fn(u32) -> u32 {
    |a: u32| a + 1
}

pub fn use_make(x: u32) -> u32 {
    let c = make();
    let d = c;
    d(x)
}

pub fn use_make_false(x: u32) -> u32 {
    let c = make();
    let d = c;
    d(x)
}

// A closure no body uses is dead code.
pub fn unused(x: u32) -> u32 {
    let _g = |a: u32| a + 1;
    x
}

pub fn holds_guards(guards: Vec<Guard>) -> usize {
    let count = move || guards.len();
    count()
}
