// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

pub fn ordering(left: bool, right: bool) -> (bool, bool, bool, bool) {
    (left < right, left <= right, left > right, left >= right)
}
