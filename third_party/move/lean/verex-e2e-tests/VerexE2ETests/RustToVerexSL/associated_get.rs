// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

pub fn lookup(values: &Vec<u32>, index: usize) -> Option<&u32> {
    values.get(index)
}

pub fn lookup_first(values: &Vec<u64>) -> Option<&u64> {
    values.get(0)
}

pub fn lookup_generic<T>(values: &[T], index: usize) -> Option<&T> {
    values.get(index)
}
