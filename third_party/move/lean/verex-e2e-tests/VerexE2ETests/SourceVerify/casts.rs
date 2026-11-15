// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

pub fn truncate(a: u32) -> u8 { a as u8 }
pub fn to_unsigned(a: i16) -> u16 { a as u16 }
pub fn sign_extend(a: i16) -> i32 { a as i32 }
pub fn zero_extend(a: u8) -> u32 { a as u32 }
pub fn to_usize(a: u32) -> usize { a as usize }
pub fn from_usize(a: usize) -> u64 { a as u64 }
pub fn to_isize(a: i32) -> isize { a as isize }
