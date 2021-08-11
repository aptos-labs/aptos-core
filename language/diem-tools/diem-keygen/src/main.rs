// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

use diem_crypto::ValidCryptoMaterialStringExt;
use diem_keygen::KeyGen;
use diem_types::{account_address::AccountAddress, transaction::authenticator::AuthenticationKey};

fn main() {
    let mut keygen = KeyGen::from_os_rng();
    let (privkey, pubkey) = keygen.generate_keypair();

    println!("Private Key:");
    println!("{}", privkey.to_encoded_string().unwrap());

    println!();

    let auth_key = AuthenticationKey::ed25519(&pubkey).to_vec();
    let prefix_length = auth_key.len() - AccountAddress::LENGTH;
    let auth_key_prefix = &auth_key[..prefix_length];
    let account_addr = &auth_key[prefix_length..];

    println!("Auth Key Prefix:");
    println!("{}", hex::encode(auth_key_prefix));
    println!();

    println!("Account Address:");
    println!("0x{}", hex::encode(account_addr));
    println!();
}
