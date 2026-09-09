// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

pub struct Token(pub u32);

impl Drop for Token {
    fn drop(&mut self) {}
}

pub fn consume(_token: Token) {}
