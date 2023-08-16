// Copyright © Aptos Foundation

use primitive_types::H160;

#[derive(Clone, Debug)]
pub struct EthAddress(H160);

impl EthAddress {
    /// Construct Address from H160
    pub const fn new(val: H160) -> Self {
        Self(val)
    }

    pub const fn raw(&self) -> H160 {
        self.0
    }

    #[must_use]
    pub fn as_bytes(&self) -> &[u8] {
        self.0.as_bytes()
    }

}
