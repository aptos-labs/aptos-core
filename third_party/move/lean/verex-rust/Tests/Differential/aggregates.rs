// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

#![feature(core_intrinsics)]
#![allow(internal_features)]

pub enum Shape {
    Empty,
    One(u8),
    Two(i8, bool),
    Named { value: u8, flag: bool },
}

pub fn make(tag: u8, a: u8) -> Shape {
    match tag {
        0 => Shape::Empty,
        1 => Shape::One(a),
        2 => Shape::Two(a as i8, a > 3),
        _ => Shape::Named { value: a, flag: tag == 4 },
    }
}

pub fn read(shape: Shape) -> i16 {
    match shape {
        Shape::Empty => -1,
        Shape::One(a) => a as i16,
        Shape::Two(a, true) => a as i16,
        Shape::Two(_, false) => 7,
        Shape::Named { value, flag } => {
            if flag { value as i16 } else { -(value as i16) }
        }
    }
}

pub fn write_variant(shape: &mut Shape, a: u8) {
    match shape {
        Shape::One(held) => *held = a,
        Shape::Named { value, .. } => *value = a,
        _ => {}
    }
}

pub fn replace(shape: &mut Shape) {
    *shape = Shape::One(1);
}

pub fn option_add(a: Option<u8>, b: u8) -> Option<u8> {
    match a {
        Some(value) => Some(core::intrinsics::wrapping_add(value, b)),
        None => None,
    }
}

pub fn array_of(a: u8, b: u8) -> [u8; 4] {
    [a, b, a, b]
}

pub fn array_repeat(a: u8) -> [u8; 3] {
    [a; 3]
}

pub fn tuple_equal(a: (u8, bool), b: (u8, bool)) -> bool {
    a.0 == b.0 && a.1 == b.1
}

pub fn bool_ops(a: bool, b: bool) -> (bool, bool, bool, bool, bool) {
    (a & b, a | b, a ^ b, !a, a == b)
}

pub fn bool_order(a: bool, b: bool) -> (bool, bool, bool, bool) {
    (a < b, a <= b, a > b, a >= b)
}

pub fn discriminant(shape: &Shape) -> u8 {
    match shape {
        Shape::Empty => 0,
        Shape::One(_) => 1,
        Shape::Two(..) => 2,
        Shape::Named { .. } => 3,
    }
}

pub fn payload_of_call(a: u8) -> u8 {
    match option_add(Some(a), 1) {
        Some(value) => value,
        None => 0,
    }
}
