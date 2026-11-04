// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// What a body states and the unit cannot refuses that function alone: no other function
// reads inside a body.

pub trait Shape {
    fn area(&self) -> u64;
}

pub struct Square(pub u64);

impl Shape for Square {
    fn area(&self) -> u64 {
        self.0
    }
}

pub struct Celsius {
    pub degrees: f32,
}

pub fn through_dyn(s: u64) -> u64 {
    let square = Square(s);
    let shape: &dyn Shape = &square;
    shape.area()
}

pub fn through_pointer(x: u64) -> u64 {
    let p = &x as *const u64;
    unsafe { *p }
}

pub fn through_float(x: u32) -> u32 {
    let f = x as f32;
    f as u32
}

pub fn through_struct(x: u32) -> u32 {
    let c = Celsius { degrees: x as f32 };
    c.degrees as u32
}

pub fn through_instantiation() -> usize {
    core::mem::size_of::<f64>()
}

pub fn through_associated_type<I: Iterator>(mut i: I) -> bool {
    i.next().is_some()
}

pub fn through_impl_trait(s: impl Shape) -> u64 {
    s.area()
}

pub fn through_closure(x: u32) -> u32 {
    let f = |a: u32| {
        let g = a as f32;
        g as u32
    };
    f(x)
}

pub fn holds_closure(x: u32) -> u32 {
    let f = |a: u32| a + x;
    let g = f(1) as f32;
    g as u32
}

pub fn calls_without_contract(a: u64) -> u64 {
    through_pointer(a)
}

pub fn calls_with_contract(a: u64) -> u64 {
    through_dyn(a)
}

pub fn plain(a: u64) -> u64 {
    a / 2
}
