// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub enum AccountSequenceInfo {
    Sequential(u64),
}

impl AccountSequenceInfo {
    pub fn min_seq(&self) -> u64 {
        match self {
            Self::Sequential(seqno) => *seqno,
        }
    }
}
