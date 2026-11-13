// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

pub fn apply<F: FnMut() -> R, R>(callback: &mut F) -> R {
    callback()
}

pub fn apply_arg<F: FnMut(T) -> R, T, R>(callback: &mut F, value: T) -> R {
    callback(value)
}

pub fn two<A: FnMut() -> u64, B: FnMut() -> u64>(a: &mut A, b: &mut B) -> u64 {
    a().wrapping_add(b())
}

pub fn collision<F: FnMut() -> u64>(__trait_FnMut_call_mut: &mut F) -> u64 {
    __trait_FnMut_call_mut()
}

pub fn outer<G: FnMut() -> u64>(g: &mut G) -> u64 {
    let mut n = g();
    let mut c = move || { n = n.wrapping_add(1); n };
    apply(&mut c)
}

pub fn nested(value: u64) -> u64 {
    let mut n = value;
    let mut c = move || { n = n.wrapping_add(1); n };
    outer(&mut c)
}

pub fn shared(value: u64) -> u64 {
    let mut c = || value;
    apply(&mut c)
}

pub fn argument(value: u64) -> u64 {
    let mut n = 0u64;
    let mut c = move |x: u64| { n = n.wrapping_add(x); n };
    apply_arg(&mut c, value)
}

pub fn multiple(value: u64) -> u64 {
    let mut a = value;
    let mut b = 0u64;
    two(&mut move || { a = a.wrapping_add(1); a }, &mut move || { b = b.wrapping_add(1); b })
}

pub trait Step { fn step(&mut self); }
pub fn two_steps<A: Step, B: Step>(a: &mut A, b: &mut B) { a.step(); b.step(); }


pub struct Pair { pub number: u64, pub flag: bool }

pub fn record(value: u64) -> Pair {
    let mut n = value;
    let mut c = move || { n = n.wrapping_add(1); Pair { number: n, flag: true } };
    apply(&mut c)
}

pub fn apply_two<F: FnMut(u64, bool) -> u64>(callback: &mut F, value: u64, enabled: bool) -> u64 {
    callback(value, enabled)
}

pub fn arguments(value: u64) -> u64 {
    let mut n = 0u64;
    let mut c = move |x: u64, enabled: bool| {
        if enabled { n = n.wrapping_add(x); }
        n
    };
    apply_two(&mut c, value, true)
}
