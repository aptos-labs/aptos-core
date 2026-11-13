// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// A drop that could run a destructor has no denotation, at its location; other
// uses of the type stay verifiable.

pub struct Guard(pub u32);

impl Drop for Guard {
    fn drop(&mut self) {}
}

pub struct Field(pub Guard);

pub struct Holder<T>(pub T);

pub struct Plain(pub u32);

pub fn consume(_guard: Guard) {}

pub fn consume_field(_field: Field) {}

pub fn consume_holder(_holder: Holder<Guard>) {}

pub fn forget<T>(_value: T) {}

pub fn forget_through<T>(value: T) {
    forget(value);
}

pub fn identity<T>(value: T) -> T {
    value
}

pub fn forget_guard(guard: Guard) {
    forget(guard);
}

pub fn forget_guard_through(guard: Guard) {
    forget_through(guard);
}

pub fn forget_plain(plain: Plain) {
    forget(plain);
}

pub fn keep_guard(guard: Guard) -> Guard {
    identity(guard)
}

pub fn forget_borrow(guard: &Guard) {
    forget(guard);
}
