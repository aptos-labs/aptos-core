// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

macro_rules! delegate_read {
    ($(
        $(#[$($attr:meta)*])*
        fn $name:ident(&self $(, $arg: ident : $ty: ty $(,)?)*) -> $return_type:ty;
    )+) => {
        $(
            $(#[$($attr)*])*
            fn $name(&self, $($arg: $ty),*) -> $return_type {
                self.get_read_delegatee().$name($($arg),*)
            }
        )+
    };
}

pub(crate) use delegate_read;
