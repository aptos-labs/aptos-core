// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

pub fn direct(c: u32) -> u32 {
    let g = |a: u32| a + c;
    g(1) + g(2)
}

pub fn no_upvars(x: u32) -> u32 {
    let g = |a: u32, b: u32| a + b;
    g(x, 1)
}

pub fn counter(n0: u32) -> u32 {
    let mut n = n0;
    let mut inc = |d: u32| {
        n += d;
    };
    inc(1);
    inc(2);
    n
}

pub fn consume(v: Vec<u32>) -> Vec<u32> {
    let take = move || v;
    take()
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

pub fn nested(x: u32) -> u32 {
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

pub fn make() -> impl Fn(u32) -> u32 {
    |a: u32| a + 1
}

pub fn use_make(x: u32) -> u32 {
    let c = make();
    let d = c;
    d(x)
}

pub fn unused(x: u32) -> u32 {
    let _g = |a: u32| a + 1;
    x
}

// Its parent never uses the closure, so the closure stands at the parent's entry.
#[allow(unreachable_code)]
pub fn never_makes() -> impl Fn(u32) -> u32 {
    std::process::abort();
    |a: u32| a + 1
}

pub fn use_never_makes(x: u32) -> u32 {
    let c = never_makes();
    c(x)
}

pub fn looped(n: u8) -> u8 {
    let sum = |m: u8| {
        let mut s = 0;
        let mut i = 0;
        while i < m % 8 {
            s += i;
            i += 1;
        }
        s
    };
    let mut total = 0;
    let mut j = 0;
    while j < n % 8 {
        total += sum(j);
        j += 1;
    }
    total
}

pub mod m {
    pub struct S {
        pub v: u32,
    }

    impl S {
        pub fn get(&self) -> u32 {
            let k = |a: u32| a + self.v;
            k(1)
        }
    }

    pub fn inside(c: u32) -> u32 {
        let g = |a: u32| a + c;
        g(1)
    }
}
