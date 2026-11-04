// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// A type only a body states refuses that function: its signature and the others stand.

pub trait Shape {
    fn area(&self) -> u64;
}

pub struct Square(pub u64);

impl Shape for Square {
    fn area(&self) -> u64 {
        self.0
    }
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

pub fn through_instantiation() -> usize {
    core::mem::size_of::<f64>()
}

pub fn holds_closure(x: u32) -> u32 {
    let f = |a: u32| a + x;
    let g = f(1) as f32;
    g as u32
}

pub fn plain(a: u64) -> u64 {
    a / 2
}
