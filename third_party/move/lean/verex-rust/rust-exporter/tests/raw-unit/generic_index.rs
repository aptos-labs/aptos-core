// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// Selection of the concrete Vec<T> impl uses the caller's T: Sized bound.
pub fn first<T>(values: &Vec<T>) -> &T {
    &values[0]
}
