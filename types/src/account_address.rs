// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0
use crate::transaction::authenticator::AuthenticationKey;
use aptos_crypto::{
    ed25519::Ed25519PublicKey,
    hash::{CryptoHasher, HashValue},
    x25519,
};

pub use move_deps::move_core_types::account_address::AccountAddress;

const SALT: &[u8] = b"aptos_framework::staking_contract";
const VESTING_POOL_SALT: &[u8] = b"aptos_framework::vesting";

pub fn from_public_key(public_key: &Ed25519PublicKey) -> AccountAddress {
    AuthenticationKey::ed25519(public_key).derived_address()
}

// Note: This is inconsistent with current types because AccountAddress is derived
// from consensus key which is of type Ed25519PublicKey. Since AccountAddress does
// not mean anything in a setting without remote authentication, we use the network
// public key to generate a peer_id for the peer.
// See this issue for potential improvements: https://github.com/aptos-labs/aptos-core/issues/3960
pub fn from_identity_public_key(identity_public_key: x25519::PublicKey) -> AccountAddress {
    let mut array = [0u8; AccountAddress::LENGTH];
    let pubkey_slice = identity_public_key.as_slice();
    // keep only the last 16 bytes
    array.copy_from_slice(&pubkey_slice[x25519::PUBLIC_KEY_SIZE - AccountAddress::LENGTH..]);
    AccountAddress::new(array)
}

pub fn default_owner_stake_pool_address(owner: AccountAddress) -> AccountAddress {
    default_stake_pool_address(owner, owner)
}

pub fn default_stake_pool_address(
    owner: AccountAddress,
    operator: AccountAddress,
) -> AccountAddress {
    create_stake_pool_address(owner, operator, &[])
}

pub fn create_stake_pool_address(
    owner: AccountAddress,
    operator: AccountAddress,
    seed: &[u8],
) -> AccountAddress {
    let mut full_seed = vec![];
    full_seed.extend(bcs::to_bytes(&owner).unwrap());
    full_seed.extend(bcs::to_bytes(&operator).unwrap());
    full_seed.extend(SALT);
    full_seed.extend(seed);
    create_resource_address(owner, &full_seed)
}

pub fn create_vesting_contract_address(
    admin: AccountAddress,
    nonce: u64,
    seed: &[u8],
) -> AccountAddress {
    let mut full_seed = vec![];
    full_seed.extend(bcs::to_bytes(&admin).unwrap());
    full_seed.extend(bcs::to_bytes(&nonce).unwrap());
    full_seed.extend(VESTING_POOL_SALT);
    full_seed.extend(seed);
    create_resource_address(admin, &full_seed)
}

pub fn create_vesting_pool_address(
    admin: AccountAddress,
    operator: AccountAddress,
    nonce: u64,
    seed: &[u8],
) -> AccountAddress {
    let contract = create_vesting_contract_address(admin, nonce, seed);
    create_stake_pool_address(contract, operator, seed)
}

pub fn create_resource_address(address: AccountAddress, seed: &[u8]) -> AccountAddress {
    let mut input = bcs::to_bytes(&address).unwrap();
    input.extend(seed);
    let hash = HashValue::sha3_256_of(&input);
    AccountAddress::from_bytes(&hash.as_ref()).unwrap()
}

// Define the Hasher used for hashing AccountAddress types. In order to properly use the
// CryptoHasher derive macro we need to have this in its own module so that it doesn't conflict
// with the imported `AccountAddress` from move-core-types. It needs to have the same name since
// the hash salt is calculated using the name of the type.
mod hasher {
    #[derive(serde::Deserialize, aptos_crypto_derive::CryptoHasher)]
    struct AccountAddress;
}

pub trait HashAccountAddress {
    fn hash(&self) -> HashValue;
}

impl HashAccountAddress for AccountAddress {
    fn hash(&self) -> HashValue {
        let mut state = hasher::AccountAddressHasher::default();
        state.update(self.as_ref());
        state.finish()
    }
}

#[cfg(test)]
mod test {
    use super::{AccountAddress, HashAccountAddress};
    use aptos_crypto::hash::HashValue;
    use hex::FromHex;

    #[test]
    fn address_hash() {
        let address: AccountAddress =
            AccountAddress::from_hex_literal("0xca843279e3427144cead5e4d5999a3d0").unwrap();

        let hash_vec =
            &Vec::from_hex("459532feaa6841de67a6b57e0df5eab275618e94e5d0e1d32ae259116f99715b")
                .expect("You must provide a valid Hex format");

        let mut hash = [0u8; 32];
        let bytes = &hash_vec[..32];
        hash.copy_from_slice(bytes);
        assert_eq!(address.hash(), HashValue::new(hash));
    }
}
