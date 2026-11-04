// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

pub fn larger<'a>(left: &'a mut u32, right: &'a mut u32) -> &'a mut u32 {
    if *left >= *right { left } else { right }
}
