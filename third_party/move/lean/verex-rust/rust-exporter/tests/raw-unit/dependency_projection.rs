// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// `Vec`'s indexing resolves to `<Vec<T, A> as Index<I>>::index`, whose result
// names the associated type `Output`: a type parameter of the declaration, which
// the call instantiates with what rustc normalized it to.
pub fn element(values: &Vec<u32>, index: usize) -> u32 {
    values[index]
}
