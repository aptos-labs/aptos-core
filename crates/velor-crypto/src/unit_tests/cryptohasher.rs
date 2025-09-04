// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

//! Test file for the procedural macros CryptoHasher and BCSCryptoHash.

use crate as velor_crypto;
use crate::{
    hash::{CryptoHash, CryptoHasher, HASH_PREFIX},
    HashValue,
};
use velor_crypto_derive::{BCSCryptoHash, CryptoHasher};
use serde::{Deserialize, Serialize};
use tiny_keccak::{Hasher, Sha3};

// The expected use case.
#[derive(Serialize, Deserialize, CryptoHasher, BCSCryptoHash)]
pub struct Foo {
    a: u64,
    b: u32,
}

// Used for testing the seed in FooHasher.
pub struct Bar {}

// Complex example with generics and serde-rename.
#[derive(Serialize, Deserialize, CryptoHasher, BCSCryptoHash)]
#[serde(rename = "Foo")]
pub struct Baz<T> {
    a: T,
    b: u32,
}

#[derive(Serialize, Deserialize, CryptoHasher, BCSCryptoHash)]
pub struct Duplo<A, B> {
    a: A,
    b: B,
    c: u32,
}

impl CryptoHash for Bar {
    type Hasher = FooHasher;

    fn hash(&self) -> HashValue {
        let state = Self::Hasher::default();
        state.finish()
    }
}

#[test]
fn test_cryptohasher_name() {
    let mut salt = HASH_PREFIX.to_vec();
    salt.extend_from_slice(b"Foo");

    let value = Bar {};
    let expected = {
        let mut digest = Sha3::v256();
        digest.update(HashValue::sha3_256_of(&salt[..]).as_ref());
        let mut hasher_bytes = [0u8; 32];
        digest.finalize(&mut hasher_bytes);
        hasher_bytes
    };
    let actual = CryptoHash::hash(&value);
    assert_eq!(&expected, actual.as_ref());
}

#[test]
fn test_bcs_cryptohash() {
    let mut salt = HASH_PREFIX.to_vec();
    salt.extend_from_slice(b"Foo");

    let value = Foo { a: 5, b: 1025 };
    let expected = {
        let mut digest = Sha3::v256();
        digest.update(HashValue::sha3_256_of(&salt[..]).as_ref());
        digest.update(&bcs::to_bytes(&value).unwrap());
        let mut hasher_bytes = [0u8; 32];
        digest.finalize(&mut hasher_bytes);
        hasher_bytes
    };
    let actual = CryptoHash::hash(&value);
    assert_eq!(&expected, actual.as_ref());
}

#[test]
fn test_bcs_cryptohash_with_generics() {
    let value = Baz { a: 5u64, b: 1025 };
    let expected = CryptoHash::hash(&Foo { a: 5, b: 1025 });
    let actual = CryptoHash::hash(&value);
    assert_eq!(expected, actual);
}

fn prefixed_sha3(input: &[u8]) -> [u8; 32] {
    let mut sha3 = ::tiny_keccak::Sha3::v256();
    let salt: Vec<u8> = [HASH_PREFIX, input].concat();
    sha3.update(&salt);
    let mut output = [0u8; 32];
    sha3.finalize(&mut output);
    output
}

#[test]
fn test_cryptohasher_salt_access() {
    // the salt for this simple struct is expected to be its name
    assert_eq!(FooHasher::seed(), &prefixed_sha3(b"Foo"));
    assert_eq!(<Foo as CryptoHash>::Hasher::seed(), &prefixed_sha3(b"Foo"));
    assert_eq!(
        <Baz<usize> as CryptoHash>::Hasher::seed(),
        &prefixed_sha3(b"Foo")
    );
    assert_eq!(
        <Baz<String> as CryptoHash>::Hasher::seed(),
        &prefixed_sha3(b"Foo")
    );
    assert_eq!(<Bar as CryptoHash>::Hasher::seed(), &prefixed_sha3(b"Foo"));

    assert_eq!(
        <Duplo<(), ()> as CryptoHash>::Hasher::seed(),
        &prefixed_sha3(b"Duplo")
    );

    // WARNING: There is no domain separation between `Foo<A>` and `Foo<B>`. This might be on purpose,
    // so as to avoid changing the hash when the type of A or B needs to be changed in the code, but
    // it means we should exercise extreme caution when using the CryptoHasher derive.
    assert_eq!(
        <Duplo<usize, u8> as CryptoHash>::Hasher::seed(),
        &prefixed_sha3(b"Duplo")
    );
}
