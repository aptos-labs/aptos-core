// Copyright © Velor Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0
use crate::transaction::authenticator::{AuthenticationKey, Scheme};
use anyhow::bail;
use velor_crypto::{
    ed25519::Ed25519PublicKey,
    hash::{CryptoHasher, HashValue},
    x25519,
};
pub use move_core_types::account_address::AccountAddress;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::{
    fmt::{Debug, Display, Formatter},
    str::FromStr,
};

const MULTISIG_ACCOUNT_DOMAIN_SEPARATOR: &[u8] = b"velor_framework::multisig_account";
const STAKING_CONTRACT_DOMAIN_SEPARATOR: &[u8] = b"velor_framework::staking_contract";
const VESTING_POOL_DOMAIN_SEPARATOR: &[u8] = b"velor_framework::vesting";

/// A wrapper struct that gives better error messages when the account address
/// can't be deserialized in a human readable format
///
/// TODO: Put this in the upstream AccountAddress
#[derive(Clone, Copy, Hash, Ord, PartialOrd, Eq, PartialEq)]
pub struct AccountAddressWithChecks(AccountAddress);

impl Display for AccountAddressWithChecks {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0.to_hex())
    }
}

impl Debug for AccountAddressWithChecks {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self.0.to_hex())
    }
}

impl FromStr for AccountAddressWithChecks {
    type Err = anyhow::Error;

    fn from_str(str: &str) -> Result<Self, Self::Err> {
        const NUM_CHARS: usize = AccountAddress::LENGTH * 2;
        let mut has_0x = false;
        let mut working = str.trim();

        // Checks if it has a 0x at the beginning, which is okay
        if working.starts_with("0x") {
            has_0x = true;
            working = &working[2..];
        }

        if working.len() > NUM_CHARS {
            bail!(
                "AccountAddress {} is too long {} must be {} hex characters with or without a 0x in front",
                str,
                working.len(),
               NUM_CHARS
            )
        } else if !has_0x && working.len() < NUM_CHARS {
            bail!(
                "AccountAddress {} is too short {} must be {} hex characters",
                str,
                working.len(),
                NUM_CHARS
            )
        }

        if !working.chars().all(|c| char::is_ascii_hexdigit(&c)) {
            bail!("AccountAddress {} contains a non-hex character", str)
        }

        let account_address = if has_0x {
            AccountAddress::from_hex_literal(str.trim())
        } else {
            AccountAddress::from_str(str.trim())
        }?;

        Ok(account_address.into())
    }
}

impl From<AccountAddress> for AccountAddressWithChecks {
    fn from(addr: AccountAddress) -> Self {
        AccountAddressWithChecks(addr)
    }
}

impl From<&AccountAddress> for AccountAddressWithChecks {
    fn from(addr: &AccountAddress) -> Self {
        AccountAddressWithChecks(*addr)
    }
}

impl From<AccountAddressWithChecks> for AccountAddress {
    fn from(addr: AccountAddressWithChecks) -> Self {
        addr.0
    }
}

impl From<&AccountAddressWithChecks> for AccountAddress {
    fn from(addr: &AccountAddressWithChecks) -> Self {
        addr.0
    }
}

impl Serialize for AccountAddressWithChecks {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.0.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for AccountAddressWithChecks {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Ok(if deserializer.is_human_readable() {
            AccountAddressWithChecks::from_str(&<String>::deserialize(deserializer)?)
                .map_err(serde::de::Error::custom)?
        } else {
            AccountAddressWithChecks(<AccountAddress>::deserialize(deserializer)?)
        })
    }
}

pub fn from_public_key(public_key: &Ed25519PublicKey) -> AccountAddress {
    AuthenticationKey::ed25519(public_key).account_address()
}

// Note: This is inconsistent with current types because AccountAddress is derived
// from consensus key which is of type Ed25519PublicKey. Since AccountAddress does
// not mean anything in a setting without remote authentication, we use the network
// public key to generate a peer_id for the peer.
// See this issue for potential improvements: https://github.com/velor-chain/velor-core/issues/3960
pub fn from_identity_public_key(identity_public_key: x25519::PublicKey) -> AccountAddress {
    let mut array = [0u8; AccountAddress::LENGTH];
    let pubkey_slice = identity_public_key.as_slice();
    // keep only the last 16 bytes
    array.copy_from_slice(&pubkey_slice[x25519::PUBLIC_KEY_SIZE - AccountAddress::LENGTH..]);
    AccountAddress::new(array)
}

pub fn create_collection_address(creator: AccountAddress, collection: &str) -> AccountAddress {
    create_object_address(creator, collection.as_bytes())
}

pub fn create_token_address(
    creator: AccountAddress,
    collection: &str,
    name: &str,
) -> AccountAddress {
    let mut seed = vec![];
    seed.extend(collection.as_bytes());
    seed.extend(b"::");
    seed.extend(name.as_bytes());
    create_object_address(creator, &seed)
}

pub fn create_derived_object_address(
    creator: AccountAddress,
    object_address: AccountAddress,
) -> AccountAddress {
    let mut input = bcs::to_bytes(&creator).unwrap();
    input.extend(bcs::to_bytes(&object_address).unwrap());
    input.push(Scheme::DeriveObjectAddressFromObject as u8);
    let hash = HashValue::sha3_256_of(&input);
    AccountAddress::from_bytes(hash.as_ref()).unwrap()
}

pub fn create_object_address(creator: AccountAddress, seed: &[u8]) -> AccountAddress {
    let mut input = bcs::to_bytes(&creator).unwrap();
    input.extend(seed);
    input.push(Scheme::DeriveObjectAddressFromSeed as u8);
    let hash = HashValue::sha3_256_of(&input);
    AccountAddress::from_bytes(hash.as_ref()).unwrap()
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
    full_seed.extend(STAKING_CONTRACT_DOMAIN_SEPARATOR);
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
    full_seed.extend(VESTING_POOL_DOMAIN_SEPARATOR);
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
    input.push(Scheme::DeriveResourceAccountAddress as u8);
    let hash = HashValue::sha3_256_of(&input);
    AccountAddress::from_bytes(hash.as_ref()).unwrap()
}

pub fn create_multisig_account_address(
    creator: AccountAddress,
    creator_nonce: u64,
) -> AccountAddress {
    let mut full_seed = vec![];
    full_seed.extend(MULTISIG_ACCOUNT_DOMAIN_SEPARATOR);
    full_seed.extend(bcs::to_bytes(&creator_nonce).unwrap());
    create_resource_address(creator, &full_seed)
}

// Define the Hasher used for hashing AccountAddress types. In order to properly use the
// CryptoHasher derive macro we need to have this in its own module so that it doesn't conflict
// with the imported `AccountAddress` from move-core-types. It needs to have the same name since
// the hash salt is calculated using the name of the type.
mod hasher {
    #[derive(serde::Deserialize, velor_crypto_derive::CryptoHasher)]
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
    use velor_crypto::hash::HashValue;
    use hex::FromHex;

    #[test]
    fn address_hash() {
        let address =
            AccountAddress::from_hex_literal("0xca843279e3427144cead5e4d5999a3d0").unwrap();

        let hash_vec =
            &Vec::from_hex("459532feaa6841de67a6b57e0df5eab275618e94e5d0e1d32ae259116f99715b")
                .expect("You must provide a valid Hex format");

        let mut hash = [0u8; 32];
        let bytes = &hash_vec[..32];
        hash.copy_from_slice(bytes);
        assert_eq!(address.hash(), HashValue::new(hash));
    }

    #[test]
    fn token_address() {
        let address = AccountAddress::from_hex_literal("0xb0b").unwrap();
        println!(
            "{:?}",
            super::create_token_address(address, "bob's collection", "bob's token")
        );
        println!(
            "{:?}",
            super::create_collection_address(address, "bob's collection")
        );
        println!(
            "{:?}",
            super::create_resource_address(address, &[0x0B, 0x00, 0x0B])
        );
    }
}
