// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use velor_crypto::ValidCryptoMaterialStringExt;
use velor_keygen::KeyGen;
use velor_types::transaction::authenticator::AuthenticationKey;

fn main() {
    let mut keygen = KeyGen::from_os_rng();
    let (privkey, pubkey) = keygen.generate_ed25519_keypair();

    println!("Private Key:");
    println!("{}", privkey.to_encoded_string().unwrap());

    println!();

    let auth_key = AuthenticationKey::ed25519(&pubkey);
    let account_addr = auth_key.account_address();

    println!("Auth Key:");
    println!("{}", auth_key.to_encoded_string().unwrap());
    println!();

    println!("Account Address:");
    println!("{}", account_addr);
    println!();
}
