// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

pub struct Leaf {
    pub value: u32,
}

pub enum Container {
    Item(Leaf),
}

pub fn take(container: Container) -> Leaf {
    let Container::Item(value) = container;
    value
}
