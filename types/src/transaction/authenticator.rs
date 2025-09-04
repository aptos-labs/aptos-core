// Copyright © Velor Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::{
    account_address::AccountAddress,
    function_info::FunctionInfo,
    keyless::{
        EphemeralCertificate, FederatedKeylessPublicKey, KeylessPublicKey, KeylessSignature,
        TransactionAndProof,
    },
    transaction::{
        webauthn::PartialAuthenticatorAssertionResponse, RawTransaction, RawTransactionWithData,
    },
};
use anyhow::{bail, ensure, Error, Result};
use velor_crypto::{
    ed25519::{Ed25519PublicKey, Ed25519Signature},
    hash::CryptoHash,
    multi_ed25519::{MultiEd25519PublicKey, MultiEd25519Signature},
    secp256k1_ecdsa, secp256r1_ecdsa, signing_message,
    traits::Signature,
    CryptoMaterialError, HashValue, ValidCryptoMaterial, ValidCryptoMaterialStringExt,
};
use velor_crypto_derive::{CryptoHasher, DeserializeKey, SerializeKey};
#[cfg(any(test, feature = "fuzzing"))]
use proptest_derive::Arbitrary;
use rand::{rngs::OsRng, Rng};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
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

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AuthenticationProof {
    Key(Vec<u8>),
    Abstraction {
        function_info: FunctionInfo,
        auth_data: AbstractionAuthData,
    },
    None,
}

impl AuthenticationProof {
    pub fn is_abstracted(&self) -> bool {
        matches!(self, Self::Abstraction { .. })
    }

    pub fn optional_auth_key(&self) -> Option<Vec<u8>> {
        match self {
            Self::Key(data) => Some(data.clone()),
            Self::Abstraction { .. } => None,
            Self::None => None,
        }
    }
}

/// Each transaction submitted to the Velor blockchain contains a `TransactionAuthenticator`. During
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

    pub fn sender(&self) -> AccountAuthenticator {
        match self {
            Self::Ed25519 {
                public_key,
                signature,
            } => AccountAuthenticator::ed25519(public_key.clone(), signature.clone()),
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

    pub fn all_signers(&self) -> Vec<AccountAuthenticator> {
        match self {
            // This is to ensure that any new TransactionAuthenticator variant must update this function.
            Self::Ed25519 { .. }
            | Self::MultiEd25519 { .. }
            | Self::MultiAgent { .. }
            | Self::FeePayer { .. }
            | Self::SingleSender { .. } => {
                let mut account_authenticators: Vec<AccountAuthenticator> = vec![];
                account_authenticators.push(self.sender());
                account_authenticators.extend(self.secondary_signers());
                if let Some(fee_payer_signer) = self.fee_payer_signer() {
                    account_authenticators.push(fee_payer_signer);
                }
                account_authenticators
            },
        }
    }

    pub fn to_single_key_authenticators(&self) -> Result<Vec<SingleKeyAuthenticator>> {
        let account_authenticators = self.all_signers();
        let mut single_key_authenticators: Vec<SingleKeyAuthenticator> =
            Vec::with_capacity(MAX_NUM_OF_SIGS);
        for account_authenticator in account_authenticators {
            match account_authenticator {
                AccountAuthenticator::Ed25519 {
                    public_key,
                    signature,
                } => {
                    let authenticator = SingleKeyAuthenticator {
                        public_key: AnyPublicKey::ed25519(public_key.clone()),
                        signature: AnySignature::ed25519(signature.clone()),
                    };
                    single_key_authenticators.push(authenticator);
                },
                AccountAuthenticator::MultiEd25519 {
                    public_key,
                    signature,
                } => {
                    let public_keys = MultiKey::from(public_key);
                    let signatures: Vec<AnySignature> = signature
                        .signatures()
                        .iter()
                        .map(|sig| AnySignature::ed25519(sig.clone()))
                        .collect();
                    let signatures_bitmap = velor_bitvec::BitVec::from(signature.bitmap().to_vec());
                    let authenticator = MultiKeyAuthenticator {
                        public_keys,
                        signatures,
                        signatures_bitmap,
                    };
                    single_key_authenticators.extend(authenticator.to_single_key_authenticators()?);
                },
                AccountAuthenticator::SingleKey { authenticator } => {
                    single_key_authenticators.push(authenticator);
                },
                AccountAuthenticator::MultiKey { authenticator } => {
                    single_key_authenticators.extend(authenticator.to_single_key_authenticators()?);
                },
                AccountAuthenticator::NoAccountAuthenticator => {
                    //  This case adds no single key authenticators to the vector.
                },
                AccountAuthenticator::Abstraction { .. } => {},
            };
        }
        Ok(single_key_authenticators)
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
    Abstraction = 4,
    DeriveDomainAbstraction = 5,
    NoScheme = 250,
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
            Scheme::NoScheme => "NoScheme",
            Scheme::Abstraction => "Abstraction",
            Scheme::DeriveDomainAbstraction => "DeriveDomainAbstraction",
            Scheme::DeriveAuid => "DeriveAuid",
            Scheme::DeriveObjectAddressFromObject => "DeriveObjectAddressFromObject",
            Scheme::DeriveObjectAddressFromGuid => "DeriveObjectAddressFromGuid",
            Scheme::DeriveObjectAddressFromSeed => "DeriveObjectAddressFromSeed",
            Scheme::DeriveResourceAccountAddress => "DeriveResourceAccountAddress",
        };
        write!(f, "Scheme::{}", display)
    }
}

/// An `AccountAuthenticator` is an abstraction of a signature scheme. It must know:
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
    NoAccountAuthenticator,
    Abstraction {
        function_info: FunctionInfo,
        auth_data: AbstractionAuthData,
    }, // ... add more schemes here
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, Hash)]
pub enum AbstractionAuthData {
    V1 {
        #[serde(with = "serde_bytes")]
        signing_message_digest: Vec<u8>,
        #[serde(with = "serde_bytes")]
        authenticator: Vec<u8>,
    },
    DerivableV1 {
        #[serde(with = "serde_bytes")]
        signing_message_digest: Vec<u8>,
        #[serde(with = "serde_bytes")]
        abstract_signature: Vec<u8>,
        #[serde(with = "serde_bytes")]
        abstract_public_key: Vec<u8>,
    },
}

impl AbstractionAuthData {
    pub fn signing_message_digest(&self) -> &Vec<u8> {
        match self {
            Self::V1 {
                signing_message_digest,
                ..
            }
            | Self::DerivableV1 {
                signing_message_digest,
                ..
            } => signing_message_digest,
        }
    }
}

impl AccountAuthenticator {
    /// Unique identifier for the signature scheme
    pub fn scheme(&self) -> Scheme {
        match self {
            Self::Ed25519 { .. } => Scheme::Ed25519,
            Self::MultiEd25519 { .. } => Scheme::MultiEd25519,
            Self::SingleKey { .. } => Scheme::SingleKey,
            Self::MultiKey { .. } => Scheme::MultiKey,
            Self::NoAccountAuthenticator => Scheme::NoScheme,
            Self::Abstraction { .. } => Scheme::Abstraction,
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

    /// Create a abstracted authenticator
    pub fn abstraction(
        function_info: FunctionInfo,
        signing_message_digest: Vec<u8>,
        authenticator: Vec<u8>,
    ) -> Self {
        Self::Abstraction {
            function_info,
            auth_data: AbstractionAuthData::V1 {
                signing_message_digest,
                authenticator,
            },
        }
    }

    /// Create a domain abstracted authenticator
    pub fn derivable_abstraction(
        function_info: FunctionInfo,
        signing_message_digest: Vec<u8>,
        abstract_signature: Vec<u8>,
        abstract_public_key: Vec<u8>,
    ) -> Self {
        Self::Abstraction {
            function_info,
            auth_data: AbstractionAuthData::DerivableV1 {
                signing_message_digest,
                abstract_signature,
                abstract_public_key,
            },
        }
    }

    pub fn is_abstracted(&self) -> bool {
        matches!(self, Self::Abstraction { .. })
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
            Self::NoAccountAuthenticator => bail!("No signature to verify."),
            // Abstraction delayed the authentication after prologue.
            Self::Abstraction { auth_data, .. } => {
                ensure!(auth_data.signing_message_digest() == &HashValue::sha3_256_of(signing_message(message)?.as_slice()).to_vec(), "The signing message digest provided in Abstraction Authenticator is not expected");
                Ok(())
            },
        }
    }

    /// Return the raw bytes of `self.public_key`
    pub fn public_key_bytes(&self) -> Vec<u8> {
        match self {
            Self::Ed25519 { public_key, .. } => public_key.to_bytes().to_vec(),
            Self::MultiEd25519 { public_key, .. } => public_key.to_bytes().to_vec(),
            Self::SingleKey { authenticator } => authenticator.public_key_bytes(),
            Self::MultiKey { authenticator } => authenticator.public_key_bytes(),
            Self::NoAccountAuthenticator => vec![],
            Self::Abstraction { .. } => vec![],
        }
    }

    /// Return the raw bytes of `self.signature`
    pub fn signature_bytes(&self) -> Vec<u8> {
        match self {
            Self::Ed25519 { signature, .. } => signature.to_bytes().to_vec(),
            Self::MultiEd25519 { signature, .. } => signature.to_bytes().to_vec(),
            Self::SingleKey { authenticator } => authenticator.signature_bytes(),
            Self::MultiKey { authenticator } => authenticator.signature_bytes(),
            Self::NoAccountAuthenticator => vec![],
            Self::Abstraction { .. } => vec![],
        }
    }

    /// Return an authentication proof derived from `self`'s public key and scheme id
    pub fn authentication_proof(&self) -> AuthenticationProof {
        match self {
            Self::NoAccountAuthenticator => AuthenticationProof::None,
            Self::Abstraction {
                function_info,
                auth_data,
            } => AuthenticationProof::Abstraction {
                function_info: function_info.clone(),
                auth_data: auth_data.clone(),
            },
            Self::Ed25519 { .. }
            | Self::MultiEd25519 { .. }
            | Self::SingleKey { .. }
            | Self::MultiKey { .. } => AuthenticationProof::Key(
                AuthenticationKey::from_preimage(self.public_key_bytes(), self.scheme()).to_vec(),
            ),
        }
    }

    /// Return the number of signatures included in this account authenticator.
    pub fn number_of_signatures(&self) -> usize {
        match self {
            Self::Ed25519 { .. } => 1,
            Self::MultiEd25519 { signature, .. } => signature.signatures().len(),
            Self::SingleKey { .. } => 1,
            Self::MultiKey { authenticator } => authenticator.signatures.len(),
            Self::NoAccountAuthenticator => 0,
            Self::Abstraction { .. } => 0,
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
    pub fn auid(mut txn_hash: Vec<u8>, auid_counter: u64) -> Self {
        txn_hash.extend(auid_counter.to_le_bytes().to_vec());
        Self::from_preimage(txn_hash, Scheme::DeriveAuid)
    }

    pub fn object_address_from_object(
        source: &AccountAddress,
        derive_from: &AccountAddress,
    ) -> AuthenticationKey {
        let mut bytes = source.to_vec();
        bytes.append(&mut derive_from.to_vec());
        Self::from_preimage(bytes, Scheme::DeriveObjectAddressFromObject)
    }

    pub fn domain_abstraction_address(
        func_info_bcs_bytes: Vec<u8>,
        account_identity: &[u8],
    ) -> AuthenticationKey {
        let mut bytes = func_info_bcs_bytes;
        bytes.append(&mut bcs::to_bytes(account_identity).expect("must serialize byte array"));
        Self::from_preimage(bytes, Scheme::DeriveDomainAbstraction)
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
    const AIP_80_PREFIX: &'static str = "";

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
    signatures_bitmap: velor_bitvec::BitVec,
}

impl MultiKeyAuthenticator {
    pub fn new(public_keys: MultiKey, signatures: Vec<(u8, AnySignature)>) -> Result<Self> {
        ensure!(
            public_keys.len() < (u8::MAX as usize),
            "Too many public keys, {}, in MultiKeyAuthenticator.",
            public_keys.len(),
        );

        let mut signatures_bitmap = velor_bitvec::BitVec::with_num_bits(public_keys.len() as u16);
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

    pub fn signatures(&self) -> Vec<(u8, &AnySignature)> {
        let mut values = vec![];
        for (idx, signature) in
            std::iter::zip(self.signatures_bitmap.iter_ones(), self.signatures.iter())
        {
            values.push((idx as u8, signature));
        }
        values
    }

    pub fn to_single_key_authenticators(&self) -> Result<Vec<SingleKeyAuthenticator>> {
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
            "Mismatch in number of signatures and the number of bits set in the signatures_bitmap, {} != {}.",
            self.signatures_bitmap.count_ones(),
            self.signatures.len(),
        );
        ensure!(
            self.signatures.len() >= self.public_keys.signatures_required() as usize,
            "Not enough signatures for verification, {} < {}.",
            self.signatures.len(),
            self.public_keys.signatures_required(),
        );
        let authenticators: Vec<SingleKeyAuthenticator> =
            std::iter::zip(self.signatures_bitmap.iter_ones(), self.signatures.iter())
                .map(|(idx, sig)| SingleKeyAuthenticator {
                    public_key: self.public_keys.public_keys[idx].clone(),
                    signature: sig.clone(),
                })
                .collect();
        Ok(authenticators)
    }

    pub fn verify<T: Serialize + CryptoHash>(&self, message: &T) -> Result<()> {
        let authenticators = self.to_single_key_authenticators()?;
        authenticators
            .iter()
            .try_for_each(|authenticator| authenticator.verify(message))?;
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

impl From<MultiEd25519PublicKey> for MultiKey {
    fn from(multi_ed25519_public_key: MultiEd25519PublicKey) -> Self {
        let public_keys: Vec<AnyPublicKey> = multi_ed25519_public_key
            .public_keys()
            .iter()
            .map(|key| AnyPublicKey::ed25519(key.clone()))
            .collect();
        let signatures_required = *multi_ed25519_public_key.threshold();
        MultiKey {
            public_keys,
            signatures_required,
        }
    }
}

impl MultiKey {
    pub fn new(public_keys: Vec<AnyPublicKey>, signatures_required: u8) -> Result<Self> {
        ensure!(
            signatures_required > 0,
            "The number of required signatures is 0."
        );

        ensure!(
            public_keys.len() <= MAX_NUM_OF_SIGS, // This max number of signatures is also the max number of public keys.
            "The number of public keys is greater than {}.",
            MAX_NUM_OF_SIGS
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
    WebAuthn {
        signature: PartialAuthenticatorAssertionResponse,
    },
    Keyless {
        signature: KeylessSignature,
    },
}

impl AnySignature {
    pub fn ed25519(signature: Ed25519Signature) -> Self {
        Self::Ed25519 { signature }
    }

    pub fn secp256k1_ecdsa(signature: secp256k1_ecdsa::Signature) -> Self {
        Self::Secp256k1Ecdsa { signature }
    }

    pub fn webauthn(signature: PartialAuthenticatorAssertionResponse) -> Self {
        Self::WebAuthn { signature }
    }

    pub fn keyless(signature: KeylessSignature) -> Self {
        Self::Keyless { signature }
    }

    pub fn name(&self) -> &'static str {
        match self {
            Self::Ed25519 { .. } => "Ed25519",
            Self::Secp256k1Ecdsa { .. } => "Secp256k1Ecdsa",
            Self::WebAuthn { .. } => "WebAuthn",
            Self::Keyless { .. } => "Keyless",
        }
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
            (Self::WebAuthn { signature }, _) => signature.verify(message, public_key),
            (Self::Keyless { signature }, AnyPublicKey::Keyless { public_key: _ }) => {
                Self::verify_keyless_ephemeral_signature(message, signature)
            },
            (Self::Keyless { signature }, AnyPublicKey::FederatedKeyless { public_key: _ }) => {
                Self::verify_keyless_ephemeral_signature(message, signature)
            },
            _ => bail!("Invalid key, signature pairing"),
        }
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        bcs::to_bytes(self).expect("Only unhandleable errors happen here.")
    }

    fn verify_keyless_ephemeral_signature<T: Serialize + CryptoHash>(
        message: &T,
        signature: &KeylessSignature,
    ) -> Result<()> {
        // Verifies the ephemeral signature on (TXN [+ ZKP]). The rest of the verification,
        // i.e., [ZKPoK of] OpenID signature verification is done in
        // `VelorVM::run_prologue`.
        //
        // This is because the JWK, under which the [ZKPoK of an] OpenID signature verifies,
        // can only be fetched from on chain inside the `VelorVM`.
        //
        // This deferred verification is what actually ensures the `signature.ephemeral_pubkey`
        // used below is the right pubkey signed by the OIDC provider.

        let mut txn_and_zkp = TransactionAndProof {
            message,
            proof: None,
        };

        // Add the ZK proof into the `txn_and_zkp` struct, if we are in the ZK path
        match &signature.cert {
            EphemeralCertificate::ZeroKnowledgeSig(proof) => txn_and_zkp.proof = Some(proof.proof),
            EphemeralCertificate::OpenIdSig(_) => {},
        }

        signature
            .ephemeral_signature
            .verify(&txn_and_zkp, &signature.ephemeral_pubkey)
    }
}

impl TryFrom<&[u8]> for AnySignature {
    type Error = CryptoMaterialError;

    fn try_from(bytes: &[u8]) -> Result<Self, CryptoMaterialError> {
        bcs::from_bytes::<AnySignature>(bytes)
            .map_err(|_e| CryptoMaterialError::DeserializationError)
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
    Secp256r1Ecdsa {
        public_key: secp256r1_ecdsa::PublicKey,
    },
    Keyless {
        public_key: KeylessPublicKey,
    },
    FederatedKeyless {
        public_key: FederatedKeylessPublicKey,
    },
}

impl AnyPublicKey {
    pub fn ed25519(public_key: Ed25519PublicKey) -> Self {
        Self::Ed25519 { public_key }
    }

    pub fn secp256k1_ecdsa(public_key: secp256k1_ecdsa::PublicKey) -> Self {
        Self::Secp256k1Ecdsa { public_key }
    }

    pub fn secp256r1_ecdsa(public_key: secp256r1_ecdsa::PublicKey) -> Self {
        Self::Secp256r1Ecdsa { public_key }
    }

    pub fn keyless(public_key: KeylessPublicKey) -> Self {
        Self::Keyless { public_key }
    }

    pub fn federated_keyless(public_key: FederatedKeylessPublicKey) -> Self {
        Self::FederatedKeyless { public_key }
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        bcs::to_bytes(self).expect("Only unhandleable errors happen here.")
    }
}

impl TryFrom<&[u8]> for AnyPublicKey {
    type Error = CryptoMaterialError;

    fn try_from(bytes: &[u8]) -> Result<Self, CryptoMaterialError> {
        bcs::from_bytes::<AnyPublicKey>(bytes)
            .map_err(|_e| CryptoMaterialError::DeserializationError)
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "fuzzing", derive(arbitrary::Arbitrary))]
pub enum EphemeralSignature {
    Ed25519 {
        signature: Ed25519Signature,
    },
    WebAuthn {
        signature: PartialAuthenticatorAssertionResponse,
    },
}

impl EphemeralSignature {
    pub fn ed25519(signature: Ed25519Signature) -> Self {
        Self::Ed25519 { signature }
    }

    pub fn web_authn(signature: PartialAuthenticatorAssertionResponse) -> Self {
        Self::WebAuthn { signature }
    }

    pub fn verify<T: Serialize + CryptoHash>(
        &self,
        message: &T,
        public_key: &EphemeralPublicKey,
    ) -> Result<()> {
        match (self, public_key) {
            (Self::Ed25519 { signature }, EphemeralPublicKey::Ed25519 { public_key }) => {
                signature.verify(message, public_key)
            },
            (Self::WebAuthn { signature }, EphemeralPublicKey::Secp256r1Ecdsa { public_key }) => {
                signature.verify(message, &AnyPublicKey::secp256r1_ecdsa(public_key.clone()))
            },
            _ => {
                bail!("Unsupported ephemeral signature and public key combination");
            },
        }
    }

    pub fn verify_arbitrary_msg(
        &self,
        message: &[u8],
        public_key: &EphemeralPublicKey,
    ) -> Result<()> {
        match (self, public_key) {
            (Self::Ed25519 { signature }, EphemeralPublicKey::Ed25519 { public_key }) => {
                signature.verify_arbitrary_msg(message, public_key)
            },
            (Self::WebAuthn { signature }, EphemeralPublicKey::Secp256r1Ecdsa { public_key }) => {
                signature.verify_arbitrary_msg(
                    message,
                    &AnyPublicKey::secp256r1_ecdsa(public_key.clone()),
                )
            },
            _ => {
                bail!("Unsupported ephemeral signature and public key combination");
            },
        }
    }
}

impl TryFrom<&[u8]> for EphemeralSignature {
    type Error = CryptoMaterialError;

    fn try_from(bytes: &[u8]) -> Result<Self, CryptoMaterialError> {
        bcs::from_bytes::<EphemeralSignature>(bytes)
            .map_err(|_e| CryptoMaterialError::DeserializationError)
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Hash)]
#[cfg_attr(feature = "fuzzing", derive(arbitrary::Arbitrary))]
pub enum EphemeralPublicKey {
    Ed25519 {
        public_key: Ed25519PublicKey,
    },
    Secp256r1Ecdsa {
        public_key: secp256r1_ecdsa::PublicKey,
    },
}

impl EphemeralPublicKey {
    pub fn ed25519(public_key: Ed25519PublicKey) -> Self {
        Self::Ed25519 { public_key }
    }

    pub fn secp256r1_ecdsa(public_key: secp256r1_ecdsa::PublicKey) -> Self {
        Self::Secp256r1Ecdsa { public_key }
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        bcs::to_bytes(self).expect("Only unhandleable errors happen here.")
    }
}

impl TryFrom<&[u8]> for EphemeralPublicKey {
    type Error = CryptoMaterialError;

    fn try_from(bytes: &[u8]) -> Result<Self, CryptoMaterialError> {
        bcs::from_bytes::<EphemeralPublicKey>(bytes)
            .map_err(|_e| CryptoMaterialError::DeserializationError)
    }
}

impl<'de> Deserialize<'de> for EphemeralPublicKey {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        if deserializer.is_human_readable() {
            let s = <String>::deserialize(deserializer)?;
            EphemeralPublicKey::try_from(
                hex::decode(s).map_err(serde::de::Error::custom)?.as_slice(),
            )
            .map_err(serde::de::Error::custom)
        } else {
            // In order to preserve the Serde data model and help analysis tools,
            // make sure to wrap our value in a container with the same name
            // as the original type.
            #[derive(::serde::Deserialize)]
            #[serde(rename = "EphemeralPublicKey")]
            enum Value {
                Ed25519 {
                    public_key: Ed25519PublicKey,
                },
                Secp256r1Ecdsa {
                    public_key: secp256r1_ecdsa::PublicKey,
                },
            }

            let value = Value::deserialize(deserializer)?;
            Ok(match value {
                Value::Ed25519 { public_key } => EphemeralPublicKey::Ed25519 { public_key },
                Value::Secp256r1Ecdsa { public_key } => {
                    EphemeralPublicKey::Secp256r1Ecdsa { public_key }
                },
            })
        }
    }
}

impl Serialize for EphemeralPublicKey {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        if serializer.is_human_readable() {
            hex::encode(self.to_bytes().as_slice()).serialize(serializer)
        } else {
            // See comment in deserialize.
            #[derive(::serde::Serialize)]
            #[serde(rename = "EphemeralPublicKey")]
            enum Value {
                Ed25519 {
                    public_key: Ed25519PublicKey,
                },
                Secp256r1Ecdsa {
                    public_key: secp256r1_ecdsa::PublicKey,
                },
            }

            let value = match self {
                EphemeralPublicKey::Ed25519 { public_key } => Value::Ed25519 {
                    public_key: public_key.clone(),
                },
                EphemeralPublicKey::Secp256r1Ecdsa { public_key } => Value::Secp256r1Ecdsa {
                    public_key: public_key.clone(),
                },
            };

            value.serialize(serializer)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        keyless::test_utils::{
            get_sample_esk, get_sample_groth16_sig_and_pk, get_sample_openid_sig_and_pk,
            maul_raw_groth16_txn,
        },
        transaction::{webauthn::AssertionSignature, SignedTransaction},
    };
    use velor_crypto::{
        ed25519::Ed25519PrivateKey,
        secp256k1_ecdsa,
        secp256r1_ecdsa::{PublicKey, Signature},
        PrivateKey, SigningKey, Uniform,
    };
    use hex::FromHex;

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
        let sender0_auth = SingleKeyAuthenticator {
            public_key: any_sender0_pub,
            signature: signature0.clone(),
        };
        let signature1 = AnySignature::secp256k1_ecdsa(sender1.sign(&raw_txn).unwrap());
        let sender1_auth = SingleKeyAuthenticator {
            public_key: any_sender1_pub,
            signature: signature1.clone(),
        };

        let mk_auth_0 =
            MultiKeyAuthenticator::new(multi_key.clone(), vec![(0, signature0.clone())]).unwrap();
        mk_auth_0.to_single_key_authenticators().unwrap_err();
        let account_auth = AccountAuthenticator::multi_key(mk_auth_0);
        let signed_txn = SignedTransaction::new_single_sender(raw_txn.clone(), account_auth);
        signed_txn.verify_signature().unwrap_err();

        let mk_auth_1 =
            MultiKeyAuthenticator::new(multi_key.clone(), vec![(1, signature1.clone())]).unwrap();
        mk_auth_1.to_single_key_authenticators().unwrap_err();
        let account_auth = AccountAuthenticator::multi_key(mk_auth_1);
        let signed_txn = SignedTransaction::new_single_sender(raw_txn.clone(), account_auth);
        signed_txn.verify_signature().unwrap_err();

        let mk_auth_01 = MultiKeyAuthenticator::new(multi_key.clone(), vec![
            (0, signature0.clone()),
            (1, signature1.clone()),
        ])
        .unwrap();
        let single_key_authenticators = mk_auth_01.to_single_key_authenticators().unwrap();
        assert_eq!(single_key_authenticators, vec![
            sender0_auth.clone(),
            sender1_auth.clone()
        ]);
        let account_auth = AccountAuthenticator::multi_key(mk_auth_01);
        let signed_txn = SignedTransaction::new_single_sender(raw_txn.clone(), account_auth);
        signed_txn.verify_signature().unwrap();

        let mk_auth_02 = MultiKeyAuthenticator::new(multi_key.clone(), vec![
            (0, signature0.clone()),
            (2, signature1.clone()),
        ])
        .unwrap();
        let single_key_authenticators = mk_auth_02.to_single_key_authenticators().unwrap();
        assert_eq!(single_key_authenticators, vec![
            sender0_auth.clone(),
            sender1_auth.clone()
        ]);
        let account_auth = AccountAuthenticator::multi_key(mk_auth_02);
        let signed_txn = SignedTransaction::new_single_sender(raw_txn.clone(), account_auth);
        signed_txn.verify_signature().unwrap();

        let mk_auth_12 = MultiKeyAuthenticator::new(multi_key.clone(), vec![
            (1, signature1.clone()),
            (2, signature1.clone()),
        ])
        .unwrap();
        let single_key_authenticators = mk_auth_12.to_single_key_authenticators().unwrap();
        assert_eq!(single_key_authenticators, vec![
            sender1_auth.clone(),
            sender1_auth.clone()
        ]);
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
        fee_payer_unset_at_signing.verify(&raw_txn).unwrap_err();

        let fee_payer_partial_at_signing_0 = TransactionAuthenticator::fee_payer(
            sender_zero.clone(),
            vec![second_sender_0_addr, second_sender_1_addr],
            second_signers_zero.clone(),
            fee_payer_addr,
            fee_payer_signer_actual.clone(),
        );
        fee_payer_partial_at_signing_0.verify(&raw_txn).unwrap();

        let fee_payer_partial_at_signing_1 = TransactionAuthenticator::fee_payer(
            sender_zero.clone(),
            vec![second_sender_0_addr, second_sender_1_addr],
            second_signers_actual.clone(),
            fee_payer_addr,
            fee_payer_signer_actual.clone(),
        );
        fee_payer_partial_at_signing_1.verify(&raw_txn).unwrap();

        let fee_payer_bad_0 = TransactionAuthenticator::fee_payer(
            sender_bad.clone(),
            vec![second_sender_0_addr, second_sender_1_addr],
            second_signers_actual.clone(),
            fee_payer_addr,
            fee_payer_signer_actual.clone(),
        );
        fee_payer_bad_0.verify(&raw_txn).unwrap_err();

        let fee_payer_bad_1 = TransactionAuthenticator::fee_payer(
            sender_zero.clone(),
            vec![second_sender_0_addr, second_sender_1_addr],
            second_signers_bad.clone(),
            fee_payer_addr,
            fee_payer_signer_actual.clone(),
        );
        fee_payer_bad_1.verify(&raw_txn).unwrap_err();

        let fee_payer_bad_2 = TransactionAuthenticator::fee_payer(
            sender_zero.clone(),
            vec![second_sender_0_addr, second_sender_1_addr],
            second_signers_actual.clone(),
            fee_payer_addr,
            fee_payer_signer_bad.clone(),
        );
        fee_payer_bad_2.verify(&raw_txn).unwrap_err();
    }

    #[test]
    fn to_single_key_authenticators() {
        let sender = Ed25519PrivateKey::generate_for_testing();
        let sender_pub = sender.public_key();
        let second_any_pub = AnyPublicKey::ed25519(sender_pub.clone());
        let sender_auth_key = AuthenticationKey::ed25519(&sender_pub);
        let sender_addr = sender_auth_key.account_address();

        let second_sender0 = Ed25519PrivateKey::generate_for_testing();
        let second_sender0_pub = second_sender0.public_key();
        let second_sender0_any_pub = AnyPublicKey::ed25519(second_sender0_pub.clone());
        let second_sender0_auth_key = AuthenticationKey::any_key(second_sender0_any_pub.clone());
        let second_sender0_addr = second_sender0_auth_key.account_address();

        let second_sender1 = secp256k1_ecdsa::PrivateKey::generate_for_testing();
        let second_sender1_pub = second_sender1.public_key();
        let second_sender1_any_pub = AnyPublicKey::secp256k1_ecdsa(second_sender1_pub.clone());
        let second_sender1_auth_key = AuthenticationKey::any_key(second_sender1_any_pub.clone());
        let second_sender1_addr = second_sender1_auth_key.account_address();

        let fee_payer0 = Ed25519PrivateKey::generate_for_testing();
        let fee_payer0_pub = fee_payer0.public_key();
        let fee_payer0_any_pub = AnyPublicKey::ed25519(fee_payer0_pub.clone());

        let fee_payer1 = secp256k1_ecdsa::PrivateKey::generate_for_testing();
        let fee_payer1_pub = fee_payer1.public_key();
        let fee_payer1_any_pub = AnyPublicKey::secp256k1_ecdsa(fee_payer1_pub.clone());

        let fee_payer2 = secp256k1_ecdsa::PrivateKey::generate_for_testing();
        let fee_payer2_pub = fee_payer2.public_key();
        let fee_payer2_any_pub = AnyPublicKey::secp256k1_ecdsa(fee_payer2_pub.clone());

        let keys = vec![
            fee_payer0_any_pub.clone(),
            fee_payer1_any_pub.clone(),
            fee_payer2_any_pub.clone(),
        ];
        let multi_key = MultiKey::new(keys, 2).unwrap();

        let multi_key_fee_payer_auth_key = AuthenticationKey::multi_key(multi_key.clone());
        let multi_key_fee_payer_addr = multi_key_fee_payer_auth_key.account_address();

        let raw_txn = crate::test_helpers::transaction_test_helpers::get_test_signed_transaction(
            sender_addr,
            0,
            &sender,
            sender_pub.clone(),
            None,
            0,
            0,
            None,
        )
        .into_raw_transaction();

        let sender_sig = sender.sign(&raw_txn).unwrap();
        let second_sender0_sig = AnySignature::ed25519(second_sender0.sign(&raw_txn).unwrap());
        let second_sender1_sig =
            AnySignature::secp256k1_ecdsa(second_sender1.sign(&raw_txn).unwrap());
        let fee_payer0_sig = AnySignature::ed25519(fee_payer0.sign(&raw_txn).unwrap());
        let fee_payer1_sig = AnySignature::secp256k1_ecdsa(fee_payer1.sign(&raw_txn).unwrap());

        let sender_sk_auth = SingleKeyAuthenticator {
            public_key: second_any_pub,
            signature: AnySignature::ed25519(sender_sig.clone()),
        };
        let second_sender0_sk_auth = SingleKeyAuthenticator {
            public_key: second_sender0_any_pub,
            signature: second_sender0_sig.clone(),
        };
        let second_sender1_sk_auth = SingleKeyAuthenticator {
            public_key: second_sender1_any_pub,
            signature: second_sender1_sig.clone(),
        };
        let fee_payer0_sk_auth = SingleKeyAuthenticator {
            public_key: fee_payer0_any_pub,
            signature: fee_payer0_sig.clone(),
        };
        let fee_payer1_sk_auth = SingleKeyAuthenticator {
            public_key: fee_payer1_any_pub,
            signature: fee_payer1_sig.clone(),
        };

        let sender_auth = AccountAuthenticator::Ed25519 {
            public_key: sender_pub,
            signature: sender_sig,
        };
        let second_sender0_auth = AccountAuthenticator::single_key(second_sender0_sk_auth.clone());
        let second_sender1_auth = AccountAuthenticator::single_key(second_sender1_sk_auth.clone());
        let fee_payer_multi_key_auth = AccountAuthenticator::multi_key(
            MultiKeyAuthenticator::new(multi_key.clone(), vec![
                (0, fee_payer0_sig.clone()),
                (1, fee_payer1_sig.clone()),
            ])
            .unwrap(),
        );

        let txn_auth = TransactionAuthenticator::fee_payer(
            sender_auth.clone(),
            vec![second_sender0_addr, second_sender1_addr],
            vec![second_sender0_auth.clone(), second_sender1_auth.clone()],
            multi_key_fee_payer_addr,
            fee_payer_multi_key_auth.clone(),
        );

        let authenticators = txn_auth.all_signers();
        assert_eq!(authenticators, vec![
            sender_auth,
            second_sender0_auth,
            second_sender1_auth,
            fee_payer_multi_key_auth
        ]);

        let single_key_authenticators = txn_auth.to_single_key_authenticators().unwrap();
        assert_eq!(single_key_authenticators, vec![
            sender_sk_auth,
            second_sender0_sk_auth,
            second_sender1_sk_auth,
            fee_payer0_sk_auth,
            fee_payer1_sk_auth
        ]);
    }

    #[test]
    fn verify_webauthn_single_key_auth() {
        let public_key_bytes = Vec::from_hex("04c5d2864ca7d52815f25f452b34dd0a14c38279fd1c1d93b971cb7f5180fd5b86463f085b6c8ae8071e7ecadf6ccffa588a8ae424d5a6486d4fc58546f5007dc8").unwrap();
        let public_key = PublicKey::try_from(public_key_bytes.as_slice()).unwrap();

        let raw_txn_bcs_bytes: Vec<u8> = vec![
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 13, 97, 112, 116, 111, 115, 95, 97, 99,
            99, 111, 117, 110, 116, 14, 116, 114, 97, 110, 115, 102, 101, 114, 95, 99, 111, 105,
            110, 115, 1, 7, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 1, 10, 97, 112, 116, 111, 115, 95, 99, 111, 105, 110, 9, 65, 112,
            116, 111, 115, 67, 111, 105, 110, 0, 2, 32, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 8, 232, 3, 0, 0, 0, 0, 0, 0, 232,
            3, 0, 0, 0, 0, 0, 0, 100, 0, 0, 0, 0, 0, 0, 0, 128, 116, 23, 188, 190, 0, 0, 0, 89,
        ];
        let raw_transaction: RawTransaction =
            bcs::from_bytes(raw_txn_bcs_bytes.as_slice()).unwrap();

        let authenticator_data: Vec<u8> = vec![
            73, 150, 13, 229, 136, 14, 140, 104, 116, 52, 23, 15, 100, 118, 96, 91, 143, 228, 174,
            185, 162, 134, 50, 199, 153, 92, 243, 186, 131, 29, 151, 99, 29, 0, 0, 0, 0,
        ];

        let client_data_json: Vec<u8> = vec![
            123, 34, 116, 121, 112, 101, 34, 58, 34, 119, 101, 98, 97, 117, 116, 104, 110, 46, 103,
            101, 116, 34, 44, 34, 99, 104, 97, 108, 108, 101, 110, 103, 101, 34, 58, 34, 101, 85,
            102, 49, 97, 88, 119, 100, 116, 72, 75, 110, 73, 89, 85, 88, 107, 84, 103, 72, 120,
            109, 87, 116, 89, 81, 95, 85, 48, 99, 51, 79, 56, 76, 100, 109, 120, 51, 80, 84, 65,
            95, 103, 34, 44, 34, 111, 114, 105, 103, 105, 110, 34, 58, 34, 104, 116, 116, 112, 58,
            47, 47, 108, 111, 99, 97, 108, 104, 111, 115, 116, 58, 53, 49, 55, 51, 34, 44, 34, 99,
            114, 111, 115, 115, 79, 114, 105, 103, 105, 110, 34, 58, 102, 97, 108, 115, 101, 125,
        ];

        let signature: Vec<u8> = vec![
            113, 168, 216, 132, 231, 240, 12, 39, 184, 16, 246, 230, 166, 142, 70, 117, 131, 2, 3,
            155, 44, 87, 236, 192, 192, 28, 110, 2, 33, 143, 17, 200, 62, 221, 102, 227, 147, 24,
            126, 96, 10, 168, 199, 184, 85, 11, 3, 212, 24, 148, 12, 118, 60, 116, 123, 117, 228,
            159, 139, 235, 130, 6, 114, 60,
        ];
        let secp256r1_signature = Signature::try_from(signature.as_slice()).unwrap();

        let paar = PartialAuthenticatorAssertionResponse::new(
            AssertionSignature::Secp256r1Ecdsa {
                signature: secp256r1_signature.clone(),
            },
            authenticator_data,
            client_data_json,
        );
        let sk_auth = SingleKeyAuthenticator::new(
            AnyPublicKey::secp256r1_ecdsa(public_key),
            AnySignature::webauthn(paar),
        );
        let account_auth = AccountAuthenticator::single_key(sk_auth);
        let signed_txn = SignedTransaction::new_single_sender(raw_transaction, account_auth);

        let verification_result = signed_txn.verify_signature();
        assert!(verification_result.is_ok());
    }

    #[test]
    fn test_keyless_openid_txn() {
        let esk = get_sample_esk();
        let (mut sig, pk) = get_sample_openid_sig_and_pk();
        let sender_addr =
            AuthenticationKey::any_key(AnyPublicKey::keyless(pk.clone())).account_address();
        let mut raw_txn = crate::test_helpers::transaction_test_helpers::get_test_raw_transaction(
            sender_addr,
            0,
            None,
            None,
            None,
            None,
        );
        sig.ephemeral_signature = EphemeralSignature::ed25519(
            esk.sign(&TransactionAndProof {
                message: raw_txn.clone(),
                proof: None,
            })
            .unwrap(),
        );

        let single_key_auth =
            SingleKeyAuthenticator::new(AnyPublicKey::keyless(pk), AnySignature::keyless(sig));
        let account_auth = AccountAuthenticator::single_key(single_key_auth);
        let signed_txn =
            SignedTransaction::new_single_sender(raw_txn.clone(), account_auth.clone());
        signed_txn.verify_signature().unwrap();

        // Badly-signed TXN
        raw_txn.expiration_timestamp_secs += 1;
        let signed_txn = SignedTransaction::new_single_sender(raw_txn, account_auth);
        assert!(signed_txn.verify_signature().is_err());
    }

    #[test]
    fn test_keyless_groth16_txn() {
        let esk = get_sample_esk();
        let (mut sig, pk) = get_sample_groth16_sig_and_pk();
        let sender_addr =
            AuthenticationKey::any_key(AnyPublicKey::keyless(pk.clone())).account_address();
        let mut raw_txn = crate::test_helpers::transaction_test_helpers::get_test_raw_transaction(
            sender_addr,
            0,
            None,
            None,
            None,
            None,
        );
        let mut txn_and_zkp = TransactionAndProof {
            message: raw_txn.clone(),
            proof: None,
        };
        match &mut sig.cert {
            EphemeralCertificate::ZeroKnowledgeSig(proof) => {
                txn_and_zkp.proof = Some(proof.proof);
            },
            EphemeralCertificate::OpenIdSig(_) => panic!("Internal inconsistency"),
        }
        sig.ephemeral_signature = EphemeralSignature::ed25519(esk.sign(&txn_and_zkp).unwrap());

        let single_key_auth =
            SingleKeyAuthenticator::new(AnyPublicKey::keyless(pk), AnySignature::keyless(sig));
        let account_auth = AccountAuthenticator::single_key(single_key_auth);
        let signed_txn =
            SignedTransaction::new_single_sender(raw_txn.clone(), account_auth.clone());

        signed_txn.verify_signature().unwrap();

        // Badly-signed TXN
        raw_txn.expiration_timestamp_secs += 1;
        let signed_txn = SignedTransaction::new_single_sender(raw_txn, account_auth);
        assert!(signed_txn.verify_signature().is_err());
    }

    #[test]
    fn test_groth16_txn_fails_non_malleability_check() {
        let (sig, pk) = get_sample_groth16_sig_and_pk();
        let sender_addr =
            AuthenticationKey::any_key(AnyPublicKey::keyless(pk.clone())).account_address();
        let raw_txn = crate::test_helpers::transaction_test_helpers::get_test_raw_transaction(
            sender_addr,
            0,
            None,
            None,
            None,
            None,
        );
        let signed_txn = maul_raw_groth16_txn(pk, sig, raw_txn);

        assert!(signed_txn.verify_signature().is_err());
    }

    #[test]
    fn test_multi_key_with_33_keys_fails() {
        let mut keys = Vec::new();
        for _ in 0..33 {
            let private_key = Ed25519PrivateKey::generate(&mut rand::thread_rng());
            let public_key = private_key.public_key();
            keys.push(AnyPublicKey::ed25519(public_key));
        }

        let result = MultiKey::new(keys, 3);
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err().to_string(),
            "The number of public keys is greater than 32."
        );
    }
}
