// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

pub struct Token(pub u32);

impl Drop for Token {
    fn drop(&mut self) {}
}

pub fn consume(_token: Token) {}
