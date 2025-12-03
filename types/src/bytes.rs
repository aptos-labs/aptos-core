#![forbid(unsafe_code)] // Copyright (c) Aptos Foundation
                        // Copyright (c) Aptos Foundation
                        // SPDX-License-Identifier: Innovation-Enabling Source Code License

// SPDX-License-Identifier: Innovation-Enabling Source Code License

use bytes::Bytes;

pub trait NumToBytes {
    fn le_bytes(&self) -> Bytes;
}

impl NumToBytes for u64 {
    fn le_bytes(&self) -> Bytes {
        Bytes::copy_from_slice(&self.to_le_bytes())
    }
}
