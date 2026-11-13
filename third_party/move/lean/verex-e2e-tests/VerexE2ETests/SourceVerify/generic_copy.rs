// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// `T: Copy` as the body shows it: the bound itself is not exported.

pub fn read<T: Copy>(m: &T) -> T {
    *m
}

pub fn read_mut<T: Copy>(m: &mut T) -> T {
    *m
}

pub fn pair<T: Copy>(x: T) -> (T, T) {
    (x, x)
}

pub fn field<T: Copy>(m: &(T, u8)) -> T {
    m.0
}

pub fn option<T: Copy>(m: &Option<T>) -> Option<T> {
    *m
}

pub fn through_call<T: Copy>(m: &T) -> T {
    read(m)
}

pub fn upvar<T: Copy>(x: T) -> T {
    let g = || x;
    g()
}

pub fn at_u8(x: u8) -> u8 {
    let (a, b) = pair(read(&x));
    a % 100 + b % 100
}

pub fn consumed<T>(x: T) -> T {
    let y = x;
    y
}

pub fn consumed_tuple<T>(x: (T, u8)) -> (T, u8) {
    let y = x;
    y
}
