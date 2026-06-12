// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Address derivations replicated from
//! `aptos_types::transaction::authenticator::AuthenticationKey`.
//
// TODO: unify with aptos-core's `AuthenticationKey` so we don't end up having two
// duplicate implementation of the same scheme and derivation algorithm.

use move_core_types::account_address::AccountAddress;
use sha3::{Digest, Sha3_256};

/// `Scheme::DeriveAuid` discriminant.
const DERIVE_AUID_SCHEME: u8 = 251;
/// `Scheme::DeriveObjectAddressFromObject` discriminant.
const DERIVE_OBJECT_FROM_OBJECT_SCHEME: u8 = 252;

/// `sha3_256(preimage || scheme)` as an address, mirroring
/// `AuthenticationKey::from_preimage`.
fn address_from_preimage(mut preimage: Vec<u8>, scheme: u8) -> AccountAddress {
    preimage.push(scheme);
    let digest = Sha3_256::digest(&preimage);
    AccountAddress::new(digest.into())
}

/// AUID address: `sha3_256(txn_hash || auid_counter_le || DeriveAuid)`.
pub(crate) fn auid_address(txn_hash: &[u8], auid_counter: u64) -> AccountAddress {
    let mut preimage = Vec::with_capacity(txn_hash.len() + 8);
    preimage.extend_from_slice(txn_hash);
    preimage.extend_from_slice(&auid_counter.to_le_bytes());
    address_from_preimage(preimage, DERIVE_AUID_SCHEME)
}

/// Object-from-object address:
/// `sha3_256(source || derive_from || DeriveObjectAddressFromObject)`.
pub(crate) fn object_address_from_object(
    source: &AccountAddress,
    derive_from: &AccountAddress,
) -> AccountAddress {
    let mut preimage = source.to_vec();
    preimage.extend_from_slice(derive_from.as_ref());
    address_from_preimage(preimage, DERIVE_OBJECT_FROM_OBJECT_SCHEME)
}

#[cfg(test)]
mod tests {
    use super::*;

    // Pin the on-chain-frozen scheme discriminants.
    #[test]
    fn scheme_bytes_are_frozen() {
        assert_eq!(DERIVE_AUID_SCHEME, 251);
        assert_eq!(DERIVE_OBJECT_FROM_OBJECT_SCHEME, 252);
    }

    // Known-answer tests, also cross-checked end-to-end against the legacy VM's
    // `AuthenticationKey` in the differential suite.
    #[test]
    fn auid_known_answer() {
        let addr = auid_address(&[0u8; 32], 1);
        assert_eq!(
            addr.to_hex_literal(),
            "0x777e34c52ecee7cd877e439f7cbf8f5a2394c369855c7bb8a140fced68b3aed6"
        );
    }

    #[test]
    fn object_from_object_known_answer() {
        let source = AccountAddress::from_hex_literal("0xa").unwrap();
        let derive_from = AccountAddress::from_hex_literal("0xb").unwrap();
        let addr = object_address_from_object(&source, &derive_from);
        assert_eq!(
            addr.to_hex_literal(),
            "0xc168433b37d568f2c5cb143f04e177e102d9e40247cefdcb41b8dcc56caa44b0"
        );
    }
}
