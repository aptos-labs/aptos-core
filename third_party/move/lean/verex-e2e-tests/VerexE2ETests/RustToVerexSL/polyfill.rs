// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

pub fn constant<const N: usize>() -> usize { 7 }
pub fn bool_constant<const B: bool>() -> bool { true }

pub fn promoted() -> &'static u64 { &5 }
pub fn promoted_array() -> u32 { (&[1, 2, 3])[1] }
pub fn is_none(x: Option<u32>) -> bool { x == None }

#[derive(PartialEq)]
pub enum Unit { A, B }
pub fn is_a(x: Unit) -> bool { x == Unit::A }
pub fn assertion(x: u32) { assert_eq!(x, 1); }

pub fn iter_count<K>(slice: &[K]) -> usize {
    let mut count = 0;
    for _ in slice.iter() { count += 1; }
    count
}

pub fn twice<T: Copy>(x: T) -> (T, T) { (x, x) }
pub fn unused_copy<T: Copy>() {}
pub trait Copier: Copy {}
pub fn super_copy<T: Copier>(x: T) -> (T, T) { (x, x) }

pub fn shorten<'a: 'b, 'b>(x: &'a u32) -> &'b u32 { x }
pub struct Bounded<'a: 'b, 'b> { pub x: &'a u32, pub y: &'b u32 }
impl<'a: 'b, 'b> Bounded<'a, 'b> {
    pub fn get(self) -> &'b u32 { self.x }
}
