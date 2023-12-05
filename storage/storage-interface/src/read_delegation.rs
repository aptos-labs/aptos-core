// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::DbReader;

pub trait ReadDelegation {
    fn get_read_delegatee(&self) -> &dyn DbReader {
        unimplemented!("Implement desired method or get_delegatee().");
    }
}
