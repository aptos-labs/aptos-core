// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

pub struct Sized_<const N: usize>;

impl<const N: usize> Sized_<N> {
    const VALUE: usize = N;
}

pub fn three() -> usize {
    Sized_::<3>::VALUE
}
