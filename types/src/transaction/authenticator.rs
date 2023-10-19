// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::{
    account_address::AccountAddress,
    transaction::{RawTransaction, RawTransactionWithData},
};
use anyhow::{bail, ensure, Error, Result};
use aptos_crypto::{
    ed25519::{Ed25519PublicKey, Ed25519Signature},
    hash::CryptoHash,
    multi_ed25519::{MultiEd25519PublicKey, MultiEd25519Signature},
    secp256k1_ecdsa,
    traits::Signature,
    CryptoMaterialError, HashValue, ValidCryptoMaterial, ValidCryptoMaterialStringExt,
};
use aptos_crypto_derive::{CryptoHasher, DeserializeKey, SerializeKey};
#[cfg(any(test, feature = "fuzzing"))]
use proptest_derive::Arbitrary;
use rand::{rngs::OsRng, Rng};
use serde::{Deserialize, Serialize};
use std::{convert::TryFrom, fmt, str::FromStr};
use thiserror::Error;

/// Maximum number of signatures supported in `TransactionAuthenticator`,
/// across all `AccountAuthenticator`s included.
pub const MAX_NUM_OF_SIGS: usize = 32;

/// An error enum for issues related to transaction or account authentication.
#[derive(Clone, Debug, PartialEq, Eq, Error)]
#[error("{:?}", self)]
pub enum AuthenticationError {
    /// The number of signatures exceeds the maximum supported.
    MaxSignaturesExceeded,
}

/// Each transaction submitted to the Aptos blockchain contains a `TransactionAuthenticator`. During
/// transaction execution, the executor will check if every `AccountAuthenticator`'s signature on
/// the transaction hash is well-formed and whether the sha3 hash of the
/// `AccountAuthenticator`'s `AuthenticationKeyPreimage` matches the `AuthenticationKey` stored
/// under the participating signer's account address.
#[derive(Clone, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub enum TransactionAuthenticator {
    /// Single Ed25519 signature
    Ed25519 {
        public_key: Ed25519PublicKey,
        signature: Ed25519Signature,
    },
    /// K-of-N multisignature
    MultiEd25519 {
        public_key: MultiEd25519PublicKey,
        signature: MultiEd25519Signature,
    },
    /// Multi-agent transaction.
    MultiAgent {
        sender: AccountAuthenticator,
        secondary_signer_addresses: Vec<AccountAddress>,
        secondary_signers: Vec<AccountAuthenticator>,
    },
    /// Optional Multi-agent transaction with a fee payer.
    FeePayer {
        sender: AccountAuthenticator,
        secondary_signer_addresses: Vec<AccountAddress>,
        secondary_signers: Vec<AccountAuthenticator>,
        fee_payer_address: AccountAddress,
        fee_payer_signer: AccountAuthenticator,
    },
    SingleSender {
        sender: AccountAuthenticator,
    },
}

impl TransactionAuthenticator {
    /// Create a single-signature ed25519 authenticator
    pub fn ed25519(public_key: Ed25519PublicKey, signature: Ed25519Signature) -> Self {
        Self::Ed25519 {
            public_key,
            signature,
        }
    }

    /// Create a (optional) multi-agent fee payer authenticator
    pub fn fee_payer(
        sender: AccountAuthenticator,
        secondary_signer_addresses: Vec<AccountAddress>,
        secondary_signers: Vec<AccountAuthenticator>,
        fee_payer_address: AccountAddress,
        fee_payer_signer: AccountAuthenticator,
    ) -> Self {
        Self::FeePayer {
            sender,
            secondary_signer_addresses,
            secondary_signers,
            fee_payer_address,
            fee_payer_signer,
        }
    }

    /// Create a multisignature ed25519 authenticator
    pub fn multi_ed25519(
        public_key: MultiEd25519PublicKey,
        signature: MultiEd25519Signature,
    ) -> Self {
        Self::MultiEd25519 {
            public_key,
            signature,
        }
    }

    /// Create a multi-agent authenticator
    pub fn multi_agent(
        sender: AccountAuthenticator,
        secondary_signer_addresses: Vec<AccountAddress>,
        secondary_signers: Vec<AccountAuthenticator>,
    ) -> Self {
        Self::MultiAgent {
            sender,
            secondary_signer_addresses,
            secondary_signers,
        }
    }

    /// Create a single-sender authenticator
    pub fn single_sender(sender: AccountAuthenticator) -> Self {
        Self::SingleSender { sender }
    }

    /// Return Ok if all AccountAuthenticator's public keys match their signatures, Err otherwise
    pub fn verify(&self, raw_txn: &RawTransaction) -> Result<()> {
        let num_sigs: usize = self.sender().number_of_signatures()
            + self
                .secondary_signers()
                .iter()
                .map(|auth| auth.number_of_signatures())
                .sum::<usize>();
        if num_sigs > MAX_NUM_OF_SIGS {
            return Err(Error::new(AuthenticationError::MaxSignaturesExceeded));
        }
        match self {
            Self::Ed25519 {
                public_key,
                signature,
            } => signature.verify(raw_txn, public_key),
            Self::FeePayer {
                sender,
                secondary_signer_addresses,
                secondary_signers,
                fee_payer_address,
                fee_payer_signer,
            } => {
                let message = RawTransactionWithData::new_fee_payer(
                    raw_txn.clone(),
                    secondary_signer_addresses.clone(),
                    *fee_payer_address,
                );
                sender.verify(&message)?;
                for signer in secondary_signers {
                    signer.verify(&message)?;
                }
                fee_payer_signer.verify(&message)?;
                Ok(())
            },
            Self::MultiEd25519 {
                public_key,
                signature,
            } => signature.verify(raw_txn, public_key),
            Self::MultiAgent {
                sender,
                secondary_signer_addresses,
                secondary_signers,
            } => {
                let message = RawTransactionWithData::new_multi_agent(
                    raw_txn.clone(),
                    secondary_signer_addresses.clone(),
                );
                sender.verify(&message)?;
                for signer in secondary_signers {
                    signer.verify(&message)?;
                }
                Ok(())
            },
            Self::SingleSender { sender } => sender.verify(raw_txn),
        }
    }

    /// Return Ok if all AccountAuthenticator's public keys match their signatures, Err otherwise
    /// Special check for fee payer transaction having optional fee payer address in the
    /// transaction. This will be removed after 1.8 has been fully released.
    pub fn verify_with_optional_fee_payer(&self, raw_txn: &RawTransaction) -> Result<()> {
        match self {
            Self::FeePayer {
                sender,
                secondary_signer_addresses,
                secondary_signers,
                fee_payer_address,
                fee_payer_signer,
            } => {
                // Some checks duplicated here to so we don't need to repeat checks.
                let num_sigs: usize = self.sender().number_of_signatures()
                    + self
                        .secondary_signers()
                        .iter()
                        .map(|auth| auth.number_of_signatures())
                        .sum::<usize>();
                if num_sigs > MAX_NUM_OF_SIGS {
                    return Err(Error::new(AuthenticationError::MaxSignaturesExceeded));
                }

                // In the fee payer model, the fee payer address can be optionally signed. We
                // realized when we designed the fee payer model, that we made it too restrictive
                // by requiring the signature over the fee payer address. So now we need to live in
                // a world where we support a multitude of different solutions. The modern approach
                // assumes that some may sign over the address and others will sign over the zero
                // address, so we verify both and only fail if the signature fails for either of
                // them. The legacy approach is to assume the address of the fee payer is signed
                // over.
                let mut to_verify = vec![sender];
                let _ = secondary_signers
                    .iter()
                    .map(|signer| to_verify.push(signer))
                    .collect::<Vec<_>>();

                let no_fee_payer_address_message = RawTransactionWithData::new_fee_payer(
                    raw_txn.clone(),
                    secondary_signer_addresses.clone(),
                    AccountAddress::ZERO,
                );

                let mut remaining = to_verify
                    .iter()
                    .filter(|verifier| verifier.verify(&no_fee_payer_address_message).is_err())
                    .collect::<Vec<_>>();

                remaining.push(&fee_payer_signer);

                let fee_payer_address_message = RawTransactionWithData::new_fee_payer(
                    raw_txn.clone(),
                    secondary_signer_addresses.clone(),
                    *fee_payer_address,
                );

                for verifier in remaining {
                    verifier.verify(&fee_payer_address_message)?;
                }

                Ok(())
            },
            _ => self.verify(raw_txn),
        }
    }

    pub fn sender(&self) -> AccountAuthenticator {
        match self {
            Self::Ed25519 {
                public_key,
                signature,
            } => AccountAuthenticator::Ed25519 {
                public_key: public_key.clone(),
                signature: signature.clone(),
            },
            Self::FeePayer { sender, .. } => sender.clone(),
            Self::MultiEd25519 {
                public_key,
                signature,
            } => AccountAuthenticator::multi_ed25519(public_key.clone(), signature.clone()),
            Self::MultiAgent { sender, .. } => sender.clone(),
            Self::SingleSender { sender } => sender.clone(),
        }
    }

    pub fn secondary_signer_addresses(&self) -> Vec<AccountAddress> {
        match self {
            Self::Ed25519 { .. } | Self::MultiEd25519 { .. } | Self::SingleSender { .. } => {
                vec![]
            },
            Self::FeePayer {
                sender: _,
                secondary_signer_addresses,
                ..
            } => secondary_signer_addresses.to_vec(),
            Self::MultiAgent {
                sender: _,
                secondary_signer_addresses,
                ..
            } => secondary_signer_addresses.to_vec(),
        }
    }

    pub fn secondary_signers(&self) -> Vec<AccountAuthenticator> {
        match self {
            Self::Ed25519 { .. } | Self::MultiEd25519 { .. } | Self::SingleSender { .. } => {
                vec![]
            },
            Self::FeePayer {
                sender: _,
                secondary_signer_addresses: _,
                secondary_signers,
                ..
            } => secondary_signers.to_vec(),
            Self::MultiAgent {
                sender: _,
                secondary_signer_addresses: _,
                secondary_signers,
            } => secondary_signers.to_vec(),
        }
    }

    pub fn fee_payer_address(&self) -> Option<AccountAddress> {
        match self {
            Self::Ed25519 { .. }
            | Self::MultiEd25519 { .. }
            | Self::MultiAgent { .. }
            | Self::SingleSender { .. } => None,
            Self::FeePayer {
                sender: _,
                secondary_signer_addresses: _,
                secondary_signers: _,
                fee_payer_address,
                ..
            } => Some(*fee_payer_address),
        }
    }

    pub fn fee_payer_signer(&self) -> Option<AccountAuthenticator> {
        match self {
            Self::Ed25519 { .. }
            | Self::MultiEd25519 { .. }
            | Self::MultiAgent { .. }
            | Self::SingleSender { .. } => None,
            Self::FeePayer {
                sender: _,
                secondary_signer_addresses: _,
                secondary_signers: _,
                fee_payer_address: _,
                fee_payer_signer,
            } => Some(fee_payer_signer.clone()),
        }
    }
}

impl fmt::Display for TransactionAuthenticator {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Ed25519 { .. } => {
                write!(
                    f,
                    "TransactionAuthenticator[scheme: Ed25519, sender: {}]",
                    self.sender()
                )
            },
            Self::FeePayer {
                sender,
                secondary_signer_addresses,
                secondary_signers,
                fee_payer_address,
                fee_payer_signer,
            } => {
                let mut sec_addrs: String = "".to_string();
                for sec_addr in secondary_signer_addresses {
                    sec_addrs = format!("{}\n\t\t\t{:#?},", sec_addrs, sec_addr);
                }
                let mut sec_signers: String = "".to_string();
                for sec_signer in secondary_signers {
                    sec_signers = format!("{}\n\t\t\t{:#?},", sec_signers, sec_signer);
                }
                write!(
                    f,
                    "TransactionAuthenticator[\n\
                        \tscheme: MultiAgent, \n\
                        \tsender: {}\n\
                        \tsecondary signer addresses: {}\n\
                        \tsecondary signers: {}\n\n
                        \tfee payer address: {}\n\n
                        \tfee payer signer: {}]",
                    sender, sec_addrs, sec_signers, fee_payer_address, fee_payer_signer,
                )
            },
            Self::MultiEd25519 { .. } => {
                write!(
                    f,
                    "TransactionAuthenticator[scheme: MultiEd25519, sender: {}]",
                    self.sender()
                )
            },
            Self::MultiAgent {
                sender,
                secondary_signer_addresses,
                secondary_signers,
            } => {
                let mut sec_addrs: String = "".to_string();
                for sec_addr in secondary_signer_addresses {
                    sec_addrs = format!("{}\n\t\t\t{:#?},", sec_addrs, sec_addr);
                }
                let mut sec_signers: String = "".to_string();
                for sec_signer in secondary_signers {
                    sec_signers = format!("{}\n\t\t\t{:#?},", sec_signers, sec_signer);
                }
                write!(
                    f,
                    "TransactionAuthenticator[\n\
                        \tscheme: MultiAgent, \n\
                        \tsender: {}\n\
                        \tsecondary signer addresses: {}\n\
                        \tsecondary signers: {}]",
                    sender, sec_addrs, sec_signers,
                )
            },
            Self::SingleSender { sender } => {
                write!(
                    f,
                    "TransactionAuthenticator[scheme: SingleSender, sender: {}]",
                    sender
                )
            },
        }
    }
}

#[derive(Debug)]
#[repr(u8)]
pub enum Scheme {
    Ed25519 = 0,
    MultiEd25519 = 1,
    SingleKey = 2,
    MultiKey = 3,
    /// Scheme identifier used to derive addresses (not the authentication key) of objects and
    /// resources accounts. This application serves to domain separate hashes. Without such
    /// separation, an adversary could create (and get a signer for) a these accounts
    /// when a their address matches matches an existing address of a MultiEd25519 wallet.
    /// Add new derived schemes below.
    DeriveAuid = 251,
    DeriveObjectAddressFromObject = 252,
    DeriveObjectAddressFromGuid = 253,
    DeriveObjectAddressFromSeed = 254,
    DeriveResourceAccountAddress = 255,
}

impl fmt::Display for Scheme {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let display = match self {
            Scheme::Ed25519 => "Ed25519",
            Scheme::MultiEd25519 => "MultiEd25519",
            Scheme::SingleKey => "SingleKey",
            Scheme::MultiKey => "MultiKey",
            Scheme::DeriveAuid => "DeriveAuid",
            Scheme::DeriveObjectAddressFromObject => "DeriveObjectAddressFromObject",
            Scheme::DeriveObjectAddressFromGuid => "DeriveObjectAddressFromGuid",
            Scheme::DeriveObjectAddressFromSeed => "DeriveObjectAddressFromSeed",
            Scheme::DeriveResourceAccountAddress => "DeriveResourceAccountAddress",
        };
        write!(f, "Scheme::{}", display)
    }
}

/// An `AccountAuthenticator` is an an abstraction of a signature scheme. It must know:
/// (1) How to check its signature against a message and public key
/// (2) How to convert its public key into an `AuthenticationKeyPreimage` structured as
/// (public_key | signature_scheme_id).
/// Each on-chain `Account` must store an `AuthenticationKey` (computed via a sha3 hash of `(public
/// key bytes | scheme as u8)`).
#[derive(Clone, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub enum AccountAuthenticator {
    /// Ed25519 Single signature
    Ed25519 {
        public_key: Ed25519PublicKey,
        signature: Ed25519Signature,
    },
    /// Ed25519 K-of-N multisignature
    MultiEd25519 {
        public_key: MultiEd25519PublicKey,
        signature: MultiEd25519Signature,
    },
    SingleKey {
        authenticator: SingleKeyAuthenticator,
    },
    MultiKey {
        authenticator: MultiKeyAuthenticator,
    },
    // ... add more schemes here
}

impl AccountAuthenticator {
    /// Unique identifier for the signature scheme
    pub fn scheme(&self) -> Scheme {
        match self {
            Self::Ed25519 { .. } => Scheme::Ed25519,
            Self::MultiEd25519 { .. } => Scheme::MultiEd25519,
            Self::SingleKey { .. } => Scheme::SingleKey,
            Self::MultiKey { .. } => Scheme::MultiKey,
        }
    }

    /// Create a single-signature ed25519 authenticator
    pub fn ed25519(public_key: Ed25519PublicKey, signature: Ed25519Signature) -> Self {
        Self::Ed25519 {
            public_key,
            signature,
        }
    }

    /// Create a multisignature ed25519 authenticator
    pub fn multi_ed25519(
        public_key: MultiEd25519PublicKey,
        signature: MultiEd25519Signature,
    ) -> Self {
        Self::MultiEd25519 {
            public_key,
            signature,
        }
    }

    /// Create a single-signature authenticator
    pub fn single_key(authenticator: SingleKeyAuthenticator) -> Self {
        Self::SingleKey { authenticator }
    }

    /// Create a multi-signature authenticator
    pub fn multi_key(authenticator: MultiKeyAuthenticator) -> Self {
        Self::MultiKey { authenticator }
    }

    /// Return Ok if the authenticator's public key matches its signature, Err otherwise
    pub fn verify<T: Serialize + CryptoHash>(&self, message: &T) -> Result<()> {
        match self {
            Self::Ed25519 {
                public_key,
                signature,
            } => signature.verify(message, public_key),
            Self::MultiEd25519 {
                public_key,
                signature,
            } => signature.verify(message, public_key),
            Self::SingleKey { authenticator } => authenticator.verify(message),
            Self::MultiKey { authenticator } => authenticator.verify(message),
        }
    }

    /// Return the raw bytes of `self.public_key`
    pub fn public_key_bytes(&self) -> Vec<u8> {
        match self {
            Self::Ed25519 { public_key, .. } => public_key.to_bytes().to_vec(),
            Self::MultiEd25519 { public_key, .. } => public_key.to_bytes().to_vec(),
            Self::SingleKey { authenticator } => authenticator.public_key_bytes(),
            Self::MultiKey { authenticator } => authenticator.public_key_bytes(),
        }
    }

    /// Return the raw bytes of `self.signature`
    pub fn signature_bytes(&self) -> Vec<u8> {
        match self {
            Self::Ed25519 { signature, .. } => signature.to_bytes().to_vec(),
            Self::MultiEd25519 { signature, .. } => signature.to_bytes().to_vec(),
            Self::SingleKey { authenticator } => authenticator.signature_bytes(),
            Self::MultiKey { authenticator } => authenticator.signature_bytes(),
        }
    }

    /// Return an authentication key derived from `self`'s public key and scheme id
    pub fn authentication_key(&self) -> AuthenticationKey {
        AuthenticationKey::from_preimage(self.public_key_bytes(), self.scheme())
    }

    /// Return the number of signatures included in this account authenticator.
    pub fn number_of_signatures(&self) -> usize {
        match self {
            Self::Ed25519 { .. } => 1,
            Self::MultiEd25519 { signature, .. } => signature.signatures().len(),
            Self::SingleKey { .. } => 1,
            Self::MultiKey { authenticator } => authenticator.signatures.len(),
        }
    }
}

/// A struct that represents an account authentication key. An account's address is the last 32
/// bytes of authentication key used to create it
#[derive(
    Clone,
    Copy,
    CryptoHasher,
    Debug,
    DeserializeKey,
    Eq,
    Hash,
    Ord,
    PartialEq,
    PartialOrd,
    SerializeKey,
)]
#[cfg_attr(any(test, feature = "fuzzing"), derive(Arbitrary))]
pub struct AuthenticationKey([u8; AuthenticationKey::LENGTH]);

impl AuthenticationKey {
    /// The number of bytes in an authentication key.
    pub const LENGTH: usize = AccountAddress::LENGTH;

    /// Create an authentication key from `bytes`
    pub const fn new(bytes: [u8; Self::LENGTH]) -> Self {
        Self(bytes)
    }

    /// Return an authentication key that is impossible (in expectation) to sign for--useful for
    /// intentionally relinquishing control of an account.
    pub const fn zero() -> Self {
        Self([0; 32])
    }

    /// Create an authentication key from a preimage by taking its sha3 hash
    pub fn from_preimage(mut public_key_bytes: Vec<u8>, scheme: Scheme) -> AuthenticationKey {
        public_key_bytes.push(scheme as u8);
        AuthenticationKey::new(*HashValue::sha3_256_of(&public_key_bytes).as_ref())
    }

    /// Construct a preimage from a transaction-derived AUID as (txn_hash || auid_scheme_id)
    pub fn auid(txn_hash: Vec<u8>, auid_counter: u64) -> Self {
        let mut hash_arg = Vec::new();
        hash_arg.extend(txn_hash);
        hash_arg.extend(auid_counter.to_le_bytes().to_vec());
        Self::from_preimage(hash_arg, Scheme::DeriveAuid)
    }

    /// Create an authentication key from an Ed25519 public key
    pub fn ed25519(public_key: &Ed25519PublicKey) -> AuthenticationKey {
        Self::from_preimage(public_key.to_bytes().to_vec(), Scheme::Ed25519)
    }

    /// Create an authentication key from a MultiEd25519 public key
    pub fn multi_ed25519(public_key: &MultiEd25519PublicKey) -> Self {
        Self::from_preimage(public_key.to_bytes(), Scheme::MultiEd25519)
    }

    /// Create an authentication key from an AnyPublicKey
    pub fn any_key(public_key: AnyPublicKey) -> AuthenticationKey {
        Self::from_preimage(public_key.to_bytes(), Scheme::SingleKey)
    }

    /// Create an authentication key from multiple AnyPublicKeys
    pub fn multi_key(public_keys: MultiKey) -> AuthenticationKey {
        Self::from_preimage(public_keys.to_bytes(), Scheme::MultiKey)
    }

    /// Return the authentication key as an account address
    pub fn account_address(&self) -> AccountAddress {
        AccountAddress::new(self.0)
    }

    /// Construct a vector from this authentication key
    pub fn to_vec(&self) -> Vec<u8> {
        self.0.to_vec()
    }

    /// Create a random authentication key. For testing only
    pub fn random() -> Self {
        let mut rng = OsRng;
        let buf: [u8; Self::LENGTH] = rng.gen();
        AuthenticationKey::new(buf)
    }
}

impl ValidCryptoMaterial for AuthenticationKey {
    fn to_bytes(&self) -> Vec<u8> {
        self.to_vec()
    }
}

impl fmt::Display for AccountAuthenticator {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "AccountAuthenticator[scheme id: {:?}, public key: {}, signature: {}]",
            self.scheme(),
            hex::encode(self.public_key_bytes()),
            hex::encode(self.signature_bytes())
        )
    }
}

impl TryFrom<&[u8]> for AuthenticationKey {
    type Error = CryptoMaterialError;

    fn try_from(bytes: &[u8]) -> std::result::Result<AuthenticationKey, CryptoMaterialError> {
        if bytes.len() != Self::LENGTH {
            return Err(CryptoMaterialError::WrongLengthError);
        }
        let mut addr = [0u8; Self::LENGTH];
        addr.copy_from_slice(bytes);
        Ok(AuthenticationKey(addr))
    }
}

impl TryFrom<Vec<u8>> for AuthenticationKey {
    type Error = CryptoMaterialError;

    fn try_from(bytes: Vec<u8>) -> std::result::Result<AuthenticationKey, CryptoMaterialError> {
        AuthenticationKey::try_from(&bytes[..])
    }
}

impl FromStr for AuthenticationKey {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self> {
        ensure!(
            !s.is_empty(),
            "authentication key string should not be empty.",
        );
        let bytes_out = ::hex::decode(s)?;
        let key = AuthenticationKey::try_from(bytes_out.as_slice())?;
        Ok(key)
    }
}

impl AsRef<[u8]> for AuthenticationKey {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

impl fmt::LowerHex for AuthenticationKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", hex::encode(self.0))
    }
}

impl fmt::Display for AuthenticationKey {
    fn fmt(&self, f: &mut fmt::Formatter) -> std::fmt::Result {
        // Forward to the LowerHex impl with a "0x" prepended (the # flag).
        write!(f, "{:#x}", self)
    }
}

#[derive(Clone, Debug, Eq, Hash, PartialEq, Serialize, Deserialize)]
pub struct MultiKeyAuthenticator {
    public_keys: MultiKey,
    signatures: Vec<AnySignature>,
    signatures_bitmap: aptos_bitvec::BitVec,
}

impl MultiKeyAuthenticator {
    pub fn new(public_keys: MultiKey, signatures: Vec<(u8, AnySignature)>) -> Result<Self> {
        ensure!(
            public_keys.len() < (u8::MAX as usize),
            "Too many public keys, {}, in MultiKeyAuthenticator.",
            public_keys.len(),
        );

        let mut signatures_bitmap = aptos_bitvec::BitVec::with_num_bits(public_keys.len() as u16);
        let mut any_signatures = vec![];

        for (idx, signature) in signatures {
            ensure!(
                (idx as usize) < public_keys.len(),
                "Signature index is out of public key range, {} < {}.",
                idx,
                public_keys.len(),
            );
            ensure!(
                !signatures_bitmap.is_set(idx as u16),
                "Duplicate signature index, {}.",
                idx
            );
            signatures_bitmap.set(idx as u16);
            any_signatures.push(signature);
        }

        Ok(MultiKeyAuthenticator {
            public_keys,
            signatures: any_signatures,
            signatures_bitmap,
        })
    }

    pub fn public_keys(&self) -> &MultiKey {
        &self.public_keys
    }

    pub fn signatures(&self) -> Vec<(u8, AnySignature)> {
        let mut values = vec![];
        for (idx, signature) in
            std::iter::zip(self.signatures_bitmap.iter_ones(), self.signatures.iter())
        {
            values.push((idx as u8, signature.clone()));
        }
        values
    }

    pub fn verify<T: Serialize + CryptoHash>(&self, message: &T) -> Result<()> {
        ensure!(
            self.signatures_bitmap.last_set_bit().is_some(),
            "There were no signatures set in the bitmap."
        );

        ensure!(
            (self.signatures_bitmap.last_set_bit().unwrap() as usize) < self.public_keys.len(),
            "Mismatch in the position of the last signature and the number of PKs, {} >= {}.",
            self.signatures_bitmap.last_set_bit().unwrap(),
            self.public_keys.len(),
        );
        ensure!(
            self.signatures_bitmap.count_ones() as usize == self.signatures.len(),
            "Mismatch in number of signatures and the number of bits set in the signatures_bitmap, {} != {}.", self.signatures_bitmap.count_ones(), self.signatures.len(),
        );
        ensure!(
            self.signatures.len() >= self.public_keys.signatures_required() as usize,
            "Not enough signatures for verification, {} < {}.",
            self.signatures.len(),
            self.public_keys.signatures_required(),
        );
        for (idx, signature) in
            std::iter::zip(self.signatures_bitmap.iter_ones(), self.signatures.iter())
        {
            signature.verify(&self.public_keys.public_keys[idx], message)?;
        }
        Ok(())
    }

    pub fn public_key_bytes(&self) -> Vec<u8> {
        self.public_keys.to_bytes()
    }

    pub fn signature_bytes(&self) -> Vec<u8> {
        bcs::to_bytes(&(&self.signatures, &self.signatures_bitmap))
            .expect("Only unhandleable errors happen here.")
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct MultiKey {
    public_keys: Vec<AnyPublicKey>,
    signatures_required: u8,
}

impl MultiKey {
    pub fn new(public_keys: Vec<AnyPublicKey>, signatures_required: u8) -> Result<Self> {
        ensure!(
            signatures_required > 0,
            "The number of required signatures is 0."
        );

        ensure!(
            public_keys.len() >= signatures_required as usize,
            "The number of public keys is smaller than the number of required signatures, {} < {}",
            public_keys.len(),
            signatures_required
        );

        Ok(Self {
            public_keys,
            signatures_required,
        })
    }

    pub fn is_empty(&self) -> bool {
        self.public_keys.is_empty()
    }

    pub fn len(&self) -> usize {
        self.public_keys.len()
    }

    pub fn public_keys(&self) -> &[AnyPublicKey] {
        &self.public_keys
    }

    pub fn signatures_required(&self) -> u8 {
        self.signatures_required
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        bcs::to_bytes(&self).expect("Only unhandleable errors happen here.")
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct SingleKeyAuthenticator {
    public_key: AnyPublicKey,
    signature: AnySignature,
}

impl SingleKeyAuthenticator {
    pub fn new(public_key: AnyPublicKey, signature: AnySignature) -> Self {
        Self {
            public_key,
            signature,
        }
    }

    pub fn public_key(&self) -> &AnyPublicKey {
        &self.public_key
    }

    pub fn public_key_bytes(&self) -> Vec<u8> {
        self.public_key.to_bytes()
    }

    pub fn signature(&self) -> &AnySignature {
        &self.signature
    }

    pub fn signature_bytes(&self) -> Vec<u8> {
        bcs::to_bytes(&self.signature).expect("Only unhandleable errors happen here.")
    }

    pub fn verify<T: Serialize + CryptoHash>(&self, message: &T) -> Result<()> {
        self.signature.verify(&self.public_key, message)
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub enum AnySignature {
    Ed25519 {
        signature: Ed25519Signature,
    },
    Secp256k1Ecdsa {
        signature: secp256k1_ecdsa::Signature,
    },
}

impl AnySignature {
    pub fn ed25519(signature: Ed25519Signature) -> Self {
        Self::Ed25519 { signature }
    }

    pub fn secp256k1_ecdsa(signature: secp256k1_ecdsa::Signature) -> Self {
        Self::Secp256k1Ecdsa { signature }
    }

    pub fn verify<T: Serialize + CryptoHash>(
        &self,
        public_key: &AnyPublicKey,
        message: &T,
    ) -> Result<()> {
        match (self, public_key) {
            (Self::Ed25519 { signature }, AnyPublicKey::Ed25519 { public_key }) => {
                signature.verify(message, public_key)
            },
            (Self::Secp256k1Ecdsa { signature }, AnyPublicKey::Secp256k1Ecdsa { public_key }) => {
                signature.verify(message, public_key)
            },
            _ => bail!("Invalid key, signature pairing"),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub enum AnyPublicKey {
    Ed25519 {
        public_key: Ed25519PublicKey,
    },
    Secp256k1Ecdsa {
        public_key: secp256k1_ecdsa::PublicKey,
    },
}

impl AnyPublicKey {
    pub fn ed25519(public_key: Ed25519PublicKey) -> Self {
        Self::Ed25519 { public_key }
    }

    pub fn secp256k1_ecdsa(public_key: secp256k1_ecdsa::PublicKey) -> Self {
        Self::Secp256k1Ecdsa { public_key }
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        bcs::to_bytes(self).expect("Only unhandleable errors happen here.")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transaction::SignedTransaction;
    use aptos_crypto::{
        ed25519::Ed25519PrivateKey, secp256k1_ecdsa, PrivateKey, SigningKey, Uniform,
    };

    #[test]
    fn test_from_str_should_not_panic_by_given_empty_string() {
        assert!(AuthenticationKey::from_str("").is_err());
    }

    #[test]
    fn verify_ed25519_single_key_auth() {
        let sender = Ed25519PrivateKey::generate_for_testing();
        let sender_pub = sender.public_key();

        let ed_sender_auth = AuthenticationKey::ed25519(&sender_pub);
        let ed_sender_addr = ed_sender_auth.account_address();

        let single_sender_auth =
            AuthenticationKey::any_key(AnyPublicKey::ed25519(sender_pub.clone()));
        let single_sender_addr = single_sender_auth.account_address();
        assert!(ed_sender_addr != single_sender_addr);

        let raw_txn = crate::test_helpers::transaction_test_helpers::get_test_signed_transaction(
            single_sender_addr,
            0,
            &sender,
            sender_pub.clone(),
            None,
            0,
            0,
            None,
        )
        .into_raw_transaction();

        let signature = sender.sign(&raw_txn).unwrap();
        let sk_auth = SingleKeyAuthenticator::new(
            AnyPublicKey::ed25519(sender_pub),
            AnySignature::ed25519(signature),
        );
        let account_auth = AccountAuthenticator::single_key(sk_auth);
        let signed_txn = SignedTransaction::new_single_sender(raw_txn, account_auth);
        signed_txn.verify_signature().unwrap();
    }

    #[test]
    fn verify_secp256k1_ecdsa_single_key_auth() {
        let fake_sender = Ed25519PrivateKey::generate_for_testing();
        let fake_sender_pub = fake_sender.public_key();

        let sender = secp256k1_ecdsa::PrivateKey::generate_for_testing();
        let sender_pub = sender.public_key();

        let single_sender_auth =
            AuthenticationKey::any_key(AnyPublicKey::secp256k1_ecdsa(sender_pub.clone()));
        let single_sender_addr = single_sender_auth.account_address();

        let raw_txn = crate::test_helpers::transaction_test_helpers::get_test_signed_transaction(
            single_sender_addr,
            0,
            &fake_sender,
            fake_sender_pub.clone(),
            None,
            0,
            0,
            None,
        )
        .into_raw_transaction();

        let signature = sender.sign(&raw_txn).unwrap();
        let sk_auth = SingleKeyAuthenticator::new(
            AnyPublicKey::secp256k1_ecdsa(sender_pub),
            AnySignature::secp256k1_ecdsa(signature),
        );
        let account_auth = AccountAuthenticator::single_key(sk_auth);
        let signed_txn = SignedTransaction::new_single_sender(raw_txn, account_auth);
        signed_txn.verify_signature().unwrap();
    }

    #[test]
    fn verify_multi_key_auth() {
        let sender0 = Ed25519PrivateKey::generate_for_testing();
        let sender0_pub = sender0.public_key();
        let any_sender0_pub = AnyPublicKey::ed25519(sender0_pub.clone());
        let sender1 = secp256k1_ecdsa::PrivateKey::generate_for_testing();
        let sender1_pub = sender1.public_key();
        let any_sender1_pub = AnyPublicKey::secp256k1_ecdsa(sender1_pub);

        let keys = vec![
            any_sender0_pub.clone(),
            any_sender1_pub.clone(),
            any_sender1_pub.clone(),
        ];
        let multi_key = MultiKey::new(keys, 2).unwrap();

        let sender_auth = AuthenticationKey::multi_key(multi_key.clone());
        let sender_addr = sender_auth.account_address();

        let raw_txn = crate::test_helpers::transaction_test_helpers::get_test_signed_transaction(
            sender_addr,
            0,
            &sender0,
            sender0_pub,
            None,
            0,
            0,
            None,
        )
        .into_raw_transaction();

        let signature0 = AnySignature::ed25519(sender0.sign(&raw_txn).unwrap());
        let signature1 = AnySignature::secp256k1_ecdsa(sender1.sign(&raw_txn).unwrap());

        let mk_auth_0 =
            MultiKeyAuthenticator::new(multi_key.clone(), vec![(0, signature0.clone())]).unwrap();
        let account_auth = AccountAuthenticator::multi_key(mk_auth_0);
        let signed_txn = SignedTransaction::new_single_sender(raw_txn.clone(), account_auth);
        signed_txn.verify_signature().unwrap_err();

        let mk_auth_1 =
            MultiKeyAuthenticator::new(multi_key.clone(), vec![(1, signature1.clone())]).unwrap();
        let account_auth = AccountAuthenticator::multi_key(mk_auth_1);
        let signed_txn = SignedTransaction::new_single_sender(raw_txn.clone(), account_auth);
        signed_txn.verify_signature().unwrap_err();

        let mk_auth_01 = MultiKeyAuthenticator::new(multi_key.clone(), vec![
            (0, signature0.clone()),
            (1, signature1.clone()),
        ])
        .unwrap();
        let account_auth = AccountAuthenticator::multi_key(mk_auth_01);
        let signed_txn = SignedTransaction::new_single_sender(raw_txn.clone(), account_auth);
        signed_txn.verify_signature().unwrap();

        let mk_auth_02 = MultiKeyAuthenticator::new(multi_key.clone(), vec![
            (0, signature0.clone()),
            (2, signature1.clone()),
        ])
        .unwrap();
        let account_auth = AccountAuthenticator::multi_key(mk_auth_02);
        let signed_txn = SignedTransaction::new_single_sender(raw_txn.clone(), account_auth);
        signed_txn.verify_signature().unwrap();

        let mk_auth_12 = MultiKeyAuthenticator::new(multi_key.clone(), vec![
            (1, signature1.clone()),
            (2, signature1.clone()),
        ])
        .unwrap();
        let account_auth = AccountAuthenticator::multi_key(mk_auth_12);
        let signed_txn = SignedTransaction::new_single_sender(raw_txn.clone(), account_auth);
        signed_txn.verify_signature().unwrap();

        MultiKeyAuthenticator::new(multi_key.clone(), vec![
            (0, signature0.clone()),
            (0, signature0.clone()),
        ])
        .unwrap_err();
    }

    #[test]
    fn verify_fee_payer_with_optional_fee_payer_address() {
        // This massive test basically verifies that various combinations of signatures work
        // with the appropriate verifiers:
        // verify -- only works with properly set fee payer address
        // verify_with_optional_fee_payer -- works with either fee payer address or zero
        // both of them reject anything else
        let sender = Ed25519PrivateKey::generate_for_testing();
        let sender_pub = sender.public_key();
        let sender_auth = AuthenticationKey::ed25519(&sender_pub);
        let sender_addr = sender_auth.account_address();

        let fee_payer = Ed25519PrivateKey::generate_for_testing();
        let fee_payer_pub = fee_payer.public_key();
        let fee_payer_auth = AuthenticationKey::ed25519(&fee_payer_pub);
        let fee_payer_addr = fee_payer_auth.account_address();

        let second_sender_0 = Ed25519PrivateKey::generate_for_testing();
        let second_sender_0_pub = second_sender_0.public_key();
        let second_sender_0_auth = AuthenticationKey::ed25519(&second_sender_0_pub);
        let second_sender_0_addr = second_sender_0_auth.account_address();

        let second_sender_1 = Ed25519PrivateKey::generate_for_testing();
        let second_sender_1_pub = second_sender_1.public_key();
        let second_sender_1_auth = AuthenticationKey::ed25519(&second_sender_1_pub);
        let second_sender_1_addr = second_sender_1_auth.account_address();

        let raw_txn = crate::test_helpers::transaction_test_helpers::get_test_signed_transaction(
            sender_addr,
            0,
            &sender,
            sender_pub,
            None,
            0,
            0,
            None,
        )
        .into_raw_transaction();

        let original_fee_payer = raw_txn
            .clone()
            .sign_fee_payer(
                &sender,
                vec![second_sender_0_addr, second_sender_1_addr],
                vec![&second_sender_0, &second_sender_0],
                fee_payer_addr,
                &fee_payer,
            )
            .unwrap()
            .into_inner();
        original_fee_payer.clone().check_signature().unwrap();
        original_fee_payer
            .clone()
            .check_fee_payer_signature()
            .unwrap();

        let (_sender_actual, second_signers_actual, fee_payer_signer_actual) =
            match original_fee_payer.authenticator_ref() {
                TransactionAuthenticator::FeePayer {
                    sender,
                    secondary_signer_addresses: _,
                    secondary_signers,
                    fee_payer_address: _,
                    fee_payer_signer,
                } => (sender, secondary_signers, fee_payer_signer),
                _ => panic!("Impossible path."),
            };

        let new_fee_payer = raw_txn
            .clone()
            .sign_fee_payer(
                &sender,
                vec![second_sender_0_addr, second_sender_1_addr],
                vec![&second_sender_0, &second_sender_0],
                AccountAddress::ZERO,
                &fee_payer,
            )
            .unwrap()
            .into_inner();
        new_fee_payer.clone().check_signature().unwrap();
        new_fee_payer.clone().check_fee_payer_signature().unwrap();

        let (sender_zero, second_signers_zero, fee_payer_signer_zero) =
            match new_fee_payer.authenticator_ref() {
                TransactionAuthenticator::FeePayer {
                    sender,
                    secondary_signer_addresses: _,
                    secondary_signers,
                    fee_payer_address: _,
                    fee_payer_signer,
                } => (sender, secondary_signers, fee_payer_signer),
                _ => panic!("Impossible path."),
            };

        let bad_fee_payer = raw_txn
            .clone()
            .sign_fee_payer(
                &sender,
                vec![second_sender_0_addr, second_sender_1_addr],
                vec![&second_sender_0, &second_sender_0],
                AccountAddress::ONE,
                &fee_payer,
            )
            .unwrap()
            .into_inner();

        let (sender_bad, second_signers_bad, fee_payer_signer_bad) =
            match bad_fee_payer.authenticator_ref() {
                TransactionAuthenticator::FeePayer {
                    sender,
                    secondary_signer_addresses: _,
                    secondary_signers,
                    fee_payer_address: _,
                    fee_payer_signer,
                } => (sender, secondary_signers, fee_payer_signer),
                _ => panic!("Impossible path."),
            };

        let fee_payer_unset_at_signing = TransactionAuthenticator::fee_payer(
            sender_zero.clone(),
            vec![second_sender_0_addr, second_sender_1_addr],
            second_signers_zero.clone(),
            fee_payer_addr,
            fee_payer_signer_zero.clone(),
        );
        fee_payer_unset_at_signing
            .clone()
            .verify_with_optional_fee_payer(&raw_txn)
            .unwrap_err();
        fee_payer_unset_at_signing
            .clone()
            .verify(&raw_txn)
            .unwrap_err();

        let fee_payer_partial_at_signing_0 = TransactionAuthenticator::fee_payer(
            sender_zero.clone(),
            vec![second_sender_0_addr, second_sender_1_addr],
            second_signers_zero.clone(),
            fee_payer_addr,
            fee_payer_signer_actual.clone(),
        );
        fee_payer_partial_at_signing_0
            .clone()
            .verify_with_optional_fee_payer(&raw_txn)
            .unwrap();
        fee_payer_partial_at_signing_0
            .clone()
            .verify(&raw_txn)
            .unwrap_err();

        let fee_payer_partial_at_signing_1 = TransactionAuthenticator::fee_payer(
            sender_zero.clone(),
            vec![second_sender_0_addr, second_sender_1_addr],
            second_signers_actual.clone(),
            fee_payer_addr,
            fee_payer_signer_actual.clone(),
        );
        fee_payer_partial_at_signing_1
            .clone()
            .verify_with_optional_fee_payer(&raw_txn)
            .unwrap();
        fee_payer_partial_at_signing_1.verify(&raw_txn).unwrap_err();

        let fee_payer_bad_0 = TransactionAuthenticator::fee_payer(
            sender_bad.clone(),
            vec![second_sender_0_addr, second_sender_1_addr],
            second_signers_actual.clone(),
            fee_payer_addr,
            fee_payer_signer_actual.clone(),
        );
        fee_payer_bad_0
            .clone()
            .verify_with_optional_fee_payer(&raw_txn)
            .unwrap_err();
        fee_payer_bad_0.clone().verify(&raw_txn).unwrap_err();

        let fee_payer_bad_1 = TransactionAuthenticator::fee_payer(
            sender_zero.clone(),
            vec![second_sender_0_addr, second_sender_1_addr],
            second_signers_bad.clone(),
            fee_payer_addr,
            fee_payer_signer_actual.clone(),
        );
        fee_payer_bad_1
            .clone()
            .verify_with_optional_fee_payer(&raw_txn)
            .unwrap_err();
        fee_payer_bad_1.clone().verify(&raw_txn).unwrap_err();

        let fee_payer_bad_2 = TransactionAuthenticator::fee_payer(
            sender_zero.clone(),
            vec![second_sender_0_addr, second_sender_1_addr],
            second_signers_actual.clone(),
            fee_payer_addr,
            fee_payer_signer_bad.clone(),
        );
        fee_payer_bad_2
            .clone()
            .verify_with_optional_fee_payer(&raw_txn)
            .unwrap_err();
        fee_payer_bad_2.clone().verify(&raw_txn).unwrap_err();
    }
}
