// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// The impl of `Index` for `Vec<T>` holds only under `T`'s bounds, so rustc selects
// it only at a concrete `T`.
pub fn first<T>(values: &Vec<T>) -> &T {
    &values[0]
}
