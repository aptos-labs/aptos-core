// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

#![feature(core_intrinsics)]
#![allow(internal_features)]

pub fn add(left: u8, right: u8) -> (u8, bool) {
    core::intrinsics::add_with_overflow(left, right)
}

pub fn subtract(left: i8, right: i8) -> (i8, bool) {
    core::intrinsics::sub_with_overflow(left, right)
}

pub fn multiply(left: u16, right: u16) -> (u16, bool) {
    core::intrinsics::mul_with_overflow(left, right)
}

