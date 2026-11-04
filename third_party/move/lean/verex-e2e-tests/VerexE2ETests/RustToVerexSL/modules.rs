// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

pub struct Pair {
    left: u32,
    right: u32,
}

impl Pair {
    pub fn first(self) -> u32 {
        inner::nested::id(self.left)
    }
}

pub mod inner {
    pub fn second(pair: crate::Pair) -> u32 {
        pair.right
    }

    pub mod nested {
        pub struct Wrapper {
            pub value: u32,
        }

        pub fn id(value: u32) -> u32 {
            Wrapper { value }.value
        }

        pub fn through_root(value: u32) -> u32 {
            crate::root(value)
        }
    }
}

pub fn root(value: u32) -> u32 {
    fn helper(value: u32) -> u32 {
        value
    }
    helper(value)
}
