// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

pub fn bitwise(left: bool, right: bool) -> (bool, bool, bool) {
    (left & right, left | right, left ^ right)
}

