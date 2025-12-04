// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

#![forbid(unsafe_code)]

use bytes::Bytes;

pub trait NumToBytes {
    fn le_bytes(&self) -> Bytes;
}

impl NumToBytes for u64 {
    fn le_bytes(&self) -> Bytes {
        Bytes::copy_from_slice(&self.to_le_bytes())
    }
}
