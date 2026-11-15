// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// Generic callees at a type argument holding a mutable reference: the interpreter runs the
// generic body at the concrete instantiation, the term its compilation at the rows.

pub fn id<T>(x: T) -> T {
    x
}

pub struct Wrapper<T> {
    pub inner: T,
}

pub fn wrap<T>(inner: T) -> Wrapper<T> {
    Wrapper { inner }
}

pub fn unwrap<T>(wrapper: Wrapper<T>) -> T {
    wrapper.inner
}

pub fn some<T>(x: T) -> Option<T> {
    Some(x)
}

pub fn twice<T>(x: T) -> T {
    id(id(x))
}

pub fn id_write(v: u32, w: u32) -> u32 {
    let mut a = v;
    let r = id(&mut a);
    *r = w;
    a
}

pub fn id_lender(a: &mut u32, w: u32) {
    let r = id(&mut *a);
    *r = w;
}

pub fn id_returned(a: &mut u32) -> &mut u32 {
    id(a)
}

pub fn twice_lender(a: &mut u32, w: u32) {
    let r = twice(&mut *a);
    *r = w;
}

pub fn wrapper_field(a: &mut u32, w: u32) {
    let wrapper = wrap(&mut *a);
    *wrapper.inner = w;
}

pub fn wrapper_through(a: &mut u32, w: u32) {
    let r = unwrap(wrap(&mut *a));
    *r = w;
}

pub fn option_write(a: &mut u32, w: u32) {
    match some(&mut *a) {
        Some(r) => *r = w,
        None => {},
    }
}
