use std::fmt::Debug;
use serde::{Deserialize, Serialize};
use aptos_crypto_derive::{BCSCryptoHash, CryptoHasher};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, CryptoHasher, BCSCryptoHash)]
pub enum SystemTransaction {
    Void,
    // to be populated...
}
