// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

#![allow(dead_code)]

// TODO: There are no negative tests at the moment (e.g. deriving NumVariants on a struct or union).
// Add some, possibly using compiletest-rs: https://github.com/laumann/compiletest-rs

use aptos_num_variants::NumVariants;

#[derive(NumVariants)]
enum BasicEnum {
    A,
    B(usize),
    C { foo: String },
}

#[derive(NumVariants)]
enum ZeroEnum {}

#[derive(NumVariants)]
#[num_variants = "CUSTOM_NAME"]
enum CustomName {
    Foo,
    Bar,
    Baz,
}

#[test]
fn basic_enum() {
    assert_eq!(BasicEnum::NUM_VARIANTS, 3);
}

#[test]
fn zero_enum() {
    assert_eq!(ZeroEnum::NUM_VARIANTS, 0);
}

#[test]
fn custom_name() {
    assert_eq!(CustomName::CUSTOM_NAME, 3);
}
