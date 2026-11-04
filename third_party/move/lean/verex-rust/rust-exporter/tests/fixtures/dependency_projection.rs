// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// `Vec`'s indexing resolves to `<Vec<T, A> as Index<I>>::index`, whose result
// is the associated type `Output`, which VIR does not yet express.
pub fn element(values: &Vec<u32>, index: usize) -> u32 {
    values[index]
}
