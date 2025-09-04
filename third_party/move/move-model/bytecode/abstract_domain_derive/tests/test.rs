// Copyright (c) Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use abstract_domain_derive::AbstractDomain;
use move_stackless_bytecode::dataflow_domains::{AbstractDomain, JoinResult};

#[derive(AbstractDomain)]
struct Unit;

/// Add a formal top and bottom element to type `T`
#[derive(PartialEq, Eq, Clone)]
pub enum Plus2<T> {
    Top,
    Mid(T),
    Bot,
}

impl<T: Eq + Clone> AbstractDomain for Plus2<T> {
    fn join(&mut self, other: &Self) -> JoinResult {
        match (&self, other) {
            (Plus2::Top, _) | (_, Plus2::Bot) => JoinResult::Unchanged,
            (Plus2::Mid(x), Plus2::Mid(y)) if x == y => JoinResult::Unchanged,
            (Plus2::Mid(_), _mid_or_top) => {
                *self = Plus2::Top;
                JoinResult::Changed
            },
            (Plus2::Bot, _mid_or_top) => {
                *self = other.clone();
                JoinResult::Changed
            },
        }
    }
}

type Three = Plus2<()>;

#[derive(Eq, PartialEq, Clone, AbstractDomain)]
struct Foo(Three, Three);

#[derive(AbstractDomain)]
struct Bar {
    x: Three,
    #[no_join]
    y: Three,
}

#[test]
fn test_unit() {
    let mut x = Unit;
    let y = Unit;
    assert!(x.join(&y) == JoinResult::Unchanged);
}

#[test]
fn test_plus2() {
    let mut top: Three = Plus2::Top;
    let mut bot: Three = Plus2::Bot;
    assert!(top.join(&bot) == JoinResult::Unchanged);
    assert!(bot.join(&top) == JoinResult::Changed);
    assert!(bot == Plus2::Top);
}

#[test]
fn test_named_tuple_derive() {
    let mut x = Foo(Plus2::Bot, Plus2::Bot);
    let y = Foo(Plus2::Top, Plus2::Mid(()));
    assert!(x.join(&y) == JoinResult::Changed);
    assert!(x == Foo(Plus2::Top, Plus2::Mid(())));
}

#[test]
fn test_struct_derive() {
    let mut x = Bar {
        x: Plus2::Bot,
        y: Plus2::Bot,
    };
    let y = Bar {
        x: Plus2::Top,
        y: Plus2::Top,
    };
    assert!(x.join(&y) == JoinResult::Changed);
    assert!(x.x == Plus2::Top && x.y == Plus2::Bot);
}
