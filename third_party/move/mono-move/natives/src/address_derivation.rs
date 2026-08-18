// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Address derivations, delegating to
//! `aptos_types::transaction::authenticator::AuthenticationKey` so the scheme
//! bytes and hashing stay in one place.

use aptos_types::transaction::authenticator::AuthenticationKey;
use mono_move_core::native::TableHandle;
use move_core_types::account_address::AccountAddress;
use sha3::{Digest, Sha3_256};

/// AUID address: `sha3_256(txn_hash || auid_counter_le || DeriveAuid)`.
pub(crate) fn auid_address(txn_hash: &[u8], auid_counter: u64) -> AccountAddress {
    AuthenticationKey::auid(txn_hash.to_vec(), auid_counter).account_address()
}

/// Object-from-object address:
/// `sha3_256(source || derive_from || DeriveObjectAddressFromObject)`.
pub(crate) fn object_address_from_object(
    source: &AccountAddress,
    derive_from: &AccountAddress,
) -> AccountAddress {
    AuthenticationKey::object_address_from_object(source, derive_from).account_address()
}

/// Table handle: `sha3_256(txn_hash || table_count_be_u32)`. Unlike the AUID and
/// object derivations, no scheme byte is appended.
pub(crate) fn table_handle(txn_hash: &[u8], table_count: u32) -> TableHandle {
    let mut hasher = Sha3_256::new();
    hasher.update(txn_hash);
    hasher.update(table_count.to_be_bytes());
    TableHandle::new(AccountAddress::new(hasher.finalize().into()))
}

#[cfg(test)]
mod tests {
    use super::*;

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
