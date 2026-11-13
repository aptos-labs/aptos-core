// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// Moving the first element of a tuple behind a flag drops the second.
// Before the Print.lean fix the second element's place printed as `value[1u32]`
// (unparseable as a place); with the fix it prints as `value.1`.

pub struct Wrap(pub u32);

pub fn tuple_element_drop(flag: bool, value: (Wrap, u64)) -> Wrap {
    if flag { value.0 } else { Wrap(0) }
}

pub fn tuple_element_drop_false(flag: bool, value: (Wrap, u64)) -> Wrap {
    if flag { value.0 } else { Wrap(0) }
}

pub fn tuple_element_drop_vacuous(flag: bool, value: (Wrap, u64)) -> Wrap {
    if flag { value.0 } else { Wrap(0) }
}

// A Vec in a tuple: moving element 0 behind a flag drops element 1 (the Vec),
// exercising the drop path for the `.1` place printing (was `[1u32]`).
pub fn vec_in_tuple_drop(flag: bool, value: (u64, Vec<u32>)) -> u64 {
    if flag { value.0 } else { 0 }
}

pub fn vec_in_tuple_drop_false(flag: bool, value: (u64, Vec<u32>)) -> u64 {
    if flag { value.0 } else { 0 }
}
