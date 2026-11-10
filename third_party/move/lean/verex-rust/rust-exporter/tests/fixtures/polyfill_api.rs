// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

pub trait Read { type Item; fn read(&self) -> Self::Item; }
pub struct Wrapper<T: Read> { pub value: T }
impl<T: Read<Item = u32>> Wrapper<T> {
    pub fn method(&self) -> u32 { self.value.read() }
}
pub struct Ordered<'a, 'b: 'a> { pub a: &'a u32, pub b: &'b u32 }
pub fn predicates<'a: 'b, 'b, T: Read<Item = u32> + Copy>(_x: &'a T) {}
pub fn higher_ranked<T: for<'a> Fn(&'a u32) -> u32>(_f: T) {}
pub fn constant<const N: usize>() {}
pub fn promoted<T>() -> &'static u32 { &5 }
pub fn macro_site(x: bool) { assert_eq!(x, true); }
pub fn iterate<K>(values: &[K]) { for _ in values.iter() {} }
