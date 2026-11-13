// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// A shared view of a value holding a mutable reference, in an aggregate or a closure.

pub struct Holder<'a> {
    pub r: &'a mut u8,
}

pub struct View<'b, 'a> {
    pub h: &'b Holder<'a>,
}

pub struct RefView<'b, 'a> {
    pub v: &'b &'a mut u8,
}

pub struct VecHolder<'a> {
    pub v: &'a mut Vec<u8>,
}

fn option_reader(o: Option<&Holder<'_>>) -> u8 {
    match o {
        Some(h) => *h.r,
        None => 0,
    }
}

pub fn option_view(a: u8) -> u8 {
    let mut x = a;
    let h = Holder { r: &mut x };
    let seen = option_reader(Some(&h));
    *h.r = seen % 100 + 1;
    x
}

fn struct_reader(v: View<'_, '_>) -> u8 {
    *v.h.r
}

pub fn struct_view(a: u8) -> u8 {
    let mut x = a;
    let h = Holder { r: &mut x };
    let seen = struct_reader(View { h: &h });
    *h.r = seen % 100 + 2;
    x
}

fn copy_out(s: &RefView<'_, '_>) -> u8 {
    let c = s.v;
    **c
}

pub fn copy_out_at(a: u8) -> u8 {
    let mut x = a;
    let r = &mut x;
    let seen = copy_out(&RefView { v: &r });
    *r = seen % 100 + 3;
    x
}

pub fn closure_view(a: u8) -> u8 {
    let mut x = a;
    let r = &mut x;
    let read = || *r;
    let seen = read() % 100 + read() % 100;
    *r = seen % 100;
    x
}

pub fn closure_moved(a: u8) -> u8 {
    let mut x = a;
    let r = &mut x;
    let read = move || *r;
    let seen = read() % 100 + read() % 100;
    drop(read);
    seen % 100 + x % 100
}
