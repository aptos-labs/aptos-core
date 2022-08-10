// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

pub mod aptos {
    pub mod extractor {
        include!("aptos.extractor.rs");
    }
    pub mod block_output {
        include!("aptos.block_output.rs");
    }
}
pub mod google {
    pub mod protobuf {
        include!("google.protobuf.rs");
    }
}
