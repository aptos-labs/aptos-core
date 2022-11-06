// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Clone, Hash, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
pub struct XibblePath {
    xibbles: Vec<u8>,
}

impl fmt::Debug for XibblePath {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.xibbles.iter().try_for_each(|x| write!(f, "{:x}", x))
    }
}
