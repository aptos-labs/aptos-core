// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use aptos_crypto::ValidCryptoMaterialStringExt;
use aptos_keygen::KeyGen;
use aptos_types::transaction::authenticator::AuthenticationKey;

fn main() {
    let mut keygen = KeyGen::from_os_rng();
    let (privkey, pubkey) = keygen.generate_keypair();

    println!("Private Key:");
    println!("{}", privkey.to_encoded_string().unwrap());

    println!();

    let auth_key = AuthenticationKey::ed25519(&pubkey).to_vec();
    let account_addr = &auth_key[..];

    println!("Auth Key:");
    println!("{}", hex::encode(account_addr));
    println!();

    println!("Account Address:");
    println!("0x{}", hex::encode(account_addr));
    println!();
}
