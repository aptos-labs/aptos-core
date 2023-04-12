// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// use std::fmt::{Debug, Formatter, Result};

/// Represents a write op at the VM level.
#[derive(Clone, PartialEq, Eq)]
pub enum Op<T> {
    Creation(T),
    Modification(T),
    Deletion,
}

// impl<T: Debug> Debug for Op<T> {
//     fn fmt(&self, f: &mut Formatter<'_>) -> Result {
//         match self {
//             Op::Modification(value) => write!(f, "Modification({:?})", value),
//             Op::Creation(value) => write!(f, "Creation({:?})", value),
//             Op::Deletion => write!(f, "Deletion"),
//         }
//     }
// }
