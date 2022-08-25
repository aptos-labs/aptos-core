// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{
    crypto::{
        ed25519::{Ed25519PrivateKey, Ed25519PublicKey},
        traits::Uniform,
    },
    transaction_builder::TransactionBuilder,
    types::{
        account_address::AccountAddress,
        transaction::{authenticator::AuthenticationKey, RawTransaction, SignedTransaction},
    },
};

use aptos_types::event::EventKey;
pub use aptos_types::*;

/// LocalAccount represents an account on the Aptos blockchain. Internally it
/// holds the private / public key pair and the address of the account. You can
/// use this struct to help transact with the blockchain, e.g. by generating a
/// new account and signing transactions.
#[derive(Debug)]
pub struct LocalAccount {
    /// Address of the account.
    address: AccountAddress,
    /// Authentication key of the account.
    key: AccountKey,
    /// Latest known sequence number of the account, it can be different from validator.
    sequence_number: u64,
}

impl LocalAccount {
    /// Create a new representation of an account locally. Note: This function
    /// does not actually create an account on the Aptos blockchain, just a
    /// local representation.
    pub fn new<T: Into<AccountKey>>(address: AccountAddress, key: T, sequence_number: u64) -> Self {
        Self {
            address,
            key: key.into(),
            sequence_number,
        }
    }

    /// Generate a new account locally. Note: This function does not actually
    /// create an account on the Aptos blockchain, it just generates a new
    /// account locally.
    pub fn generate<R>(rng: &mut R) -> Self
    where
        R: ::rand_core::RngCore + ::rand_core::CryptoRng,
    {
        let key = AccountKey::generate(rng);
        let address = key.authentication_key().derived_address();

        Self::new(address, key, 0)
    }

    pub fn sign_transaction(&self, txn: RawTransaction) -> SignedTransaction {
        txn.sign(self.private_key(), self.public_key().clone())
            .expect("Signing a txn can't fail")
            .into_inner()
    }

    pub fn sign_with_transaction_builder(
        &mut self,
        builder: TransactionBuilder,
    ) -> SignedTransaction {
        let raw_txn = builder
            .sender(self.address())
            .sequence_number(self.sequence_number())
            .build();
        *self.sequence_number_mut() += 1;
        self.sign_transaction(raw_txn)
    }

    pub fn sign_multi_agent_with_transaction_builder(
        &mut self,
        secondary_signers: Vec<&Self>,
        builder: TransactionBuilder,
    ) -> SignedTransaction {
        let secondary_signer_addresses = secondary_signers
            .iter()
            .map(|signer| signer.address())
            .collect();
        let secondary_signer_privkeys = secondary_signers
            .iter()
            .map(|signer| signer.private_key())
            .collect();
        let raw_txn = builder
            .sender(self.address())
            .sequence_number(self.sequence_number())
            .build();
        *self.sequence_number_mut() += 1;
        raw_txn
            .sign_multi_agent(
                self.private_key(),
                secondary_signer_addresses,
                secondary_signer_privkeys,
            )
            .expect("Signing multi agent txn failed")
            .into_inner()
    }

    pub fn address(&self) -> AccountAddress {
        self.address
    }

    pub fn private_key(&self) -> &Ed25519PrivateKey {
        self.key.private_key()
    }

    pub fn public_key(&self) -> &Ed25519PublicKey {
        self.key.public_key()
    }

    pub fn authentication_key(&self) -> AuthenticationKey {
        self.key.authentication_key()
    }

    pub fn sequence_number(&self) -> u64 {
        self.sequence_number
    }

    pub fn sequence_number_mut(&mut self) -> &mut u64 {
        &mut self.sequence_number
    }

    pub fn rotate_key<T: Into<AccountKey>>(&mut self, new_key: T) -> AccountKey {
        std::mem::replace(&mut self.key, new_key.into())
    }

    pub fn received_event_key(&self) -> EventKey {
        EventKey::new(2, self.address)
    }

    pub fn sent_event_key(&self) -> EventKey {
        EventKey::new(3, self.address)
    }
}

#[derive(Debug)]
pub struct AccountKey {
    private_key: Ed25519PrivateKey,
    public_key: Ed25519PublicKey,
    authentication_key: AuthenticationKey,
}

impl AccountKey {
    pub fn generate<R>(rng: &mut R) -> Self
    where
        R: ::rand_core::RngCore + ::rand_core::CryptoRng,
    {
        let private_key = Ed25519PrivateKey::generate(rng);
        Self::from_private_key(private_key)
    }

    pub fn from_private_key(private_key: Ed25519PrivateKey) -> Self {
        let public_key = Ed25519PublicKey::from(&private_key);
        let authentication_key = AuthenticationKey::ed25519(&public_key);

        Self {
            private_key,
            public_key,
            authentication_key,
        }
    }

    pub fn private_key(&self) -> &Ed25519PrivateKey {
        &self.private_key
    }

    pub fn public_key(&self) -> &Ed25519PublicKey {
        &self.public_key
    }

    pub fn authentication_key(&self) -> AuthenticationKey {
        self.authentication_key
    }
}

impl From<Ed25519PrivateKey> for AccountKey {
    fn from(private_key: Ed25519PrivateKey) -> Self {
        Self::from_private_key(private_key)
    }
}
