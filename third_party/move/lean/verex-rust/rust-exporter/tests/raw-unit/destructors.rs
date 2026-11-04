// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// Which ADTs state a destructor: those with a `Drop` impl and opaque ones reaching
// one through their hidden fields or holding a type argument.

use std::{marker::PhantomData, rc::Rc};

pub struct Guard(pub u32);

impl Drop for Guard {
    fn drop(&mut self) {}
}

pub struct Field(pub Guard);

pub struct Holder<T>(pub T);

pub struct Argument(pub Holder<Guard>);

pub struct Phantom<T>(pub PhantomData<T>);

pub struct PhantomGuard(pub PhantomData<Guard>);

pub struct Shared<'a>(pub &'a Guard);

pub enum Plain<'a> {
    Leaf,
    Node(&'a Plain<'a>),
}

pub enum List {
    Nil,
    Cons(u32, Box<List>),
}

pub struct Pair<T>(pub (T, u32), pub [T; 2]);

pub fn guard(_value: &Guard) {}
pub fn field(_value: &Field) {}
pub fn holder(_value: &Holder<u32>) {}
pub fn argument(_value: &Argument) {}
pub fn phantom(_value: &Phantom<Guard>) {}
pub fn phantom_guard(_value: &PhantomGuard) {}
pub fn shared(_value: &Shared<'_>) {}
pub fn plain(_value: &Plain<'_>) {}
pub fn list(_value: &List) {}
pub fn pair(_value: &Pair<u32>) {}
pub fn vector(_value: &Vec<u32>) {}
pub fn string(_value: &String) {}
pub fn reference_counted(_value: &Rc<u32>) {}
pub fn cell(_value: &std::cell::Cell<u32>) {}
pub fn ordering(_value: &std::cmp::Ordering) {}
