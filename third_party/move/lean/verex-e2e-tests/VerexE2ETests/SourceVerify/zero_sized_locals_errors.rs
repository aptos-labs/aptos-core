// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// rustc removes the assignments of zero-sized values, so a unit struct can be read
// from a local nothing assigned; its value is the type's one value.

pub struct Token;

pub struct Pair(pub Token, pub Token);

impl Token {
    pub fn value(&self) -> u32 {
        7
    }

    pub fn consume(self) -> u32 {
        8
    }
}

pub fn borrowed() -> u32 {
    let t = Token;
    t.value()
}

pub fn borrowed_false() -> u32 {
    let t = Token;
    t.value()
}

pub fn moved() -> u32 {
    let t = Token;
    let u = t;
    u.consume()
}

pub fn moved_false() -> u32 {
    let t = Token;
    let u = t;
    u.consume()
}

pub fn aggregate() -> u32 {
    let p = Pair(Token, Token);
    p.0.value() + p.1.consume()
}

pub fn aggregate_false() -> u32 {
    let p = Pair(Token, Token);
    p.0.value() + p.1.consume()
}

pub fn returned() -> Token {
    Token
}

pub fn through_return() -> u32 {
    returned().value()
}

pub fn through_return_false() -> u32 {
    returned().value()
}
