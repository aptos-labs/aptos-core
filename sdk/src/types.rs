// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0
use crate::{
    crypto::{
        ed25519::{Ed25519PrivateKey, Ed25519PublicKey},
        hash::CryptoHash,
        signing_message,
        traits::Uniform,
        CryptoMaterialError,
    },
    transaction_builder::TransactionBuilder,
    types::{
        account_address::AccountAddress,
        transaction::{authenticator::AuthenticationKey, RawTransaction, SignedTransaction},
    },
};
use anyhow::{Context, Result};
use aptos_crypto::{ed25519::Ed25519Signature, secp256r1_ecdsa, HashValue, PrivateKey, SigningKey};
use aptos_ledger::AptosLedgerError;
use aptos_rest_client::{aptos_api_types::MoveStructTag, Client, PepperRequest, ProverRequest};
pub use aptos_types::*;
use aptos_types::{
    event::EventKey,
    function_info::FunctionInfo,
    keyless::{
        Claims, Configuration, EphemeralCertificate, IdCommitment, KeylessPublicKey,
        KeylessSignature, OpenIdSig, Pepper, ZeroKnowledgeSig,
    },
    transaction::{
        authenticator::{AnyPublicKey, EphemeralPublicKey, EphemeralSignature},
        Auth,
    },
};
use bip39::{Language, Mnemonic, Seed};
use ed25519_dalek_bip32::{DerivationPath, ExtendedSecretKey};
use keyless::FederatedKeylessPublicKey;
use lazy_static::lazy_static;
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::{
    fmt,
    str::FromStr,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
    time::{Duration, SystemTime, UNIX_EPOCH},
};

pub const APTOS_COIN_TYPE_STR: &str = "0x1::aptos_coin::AptosCoin";
lazy_static! {
    pub static ref APT_METADATA_ADDRESS: AccountAddress = {
        let mut addr = [0u8; 32];
        addr[31] = 10u8;
        AccountAddress::new(addr)
    };
}

#[derive(Debug)]
enum LocalAccountAuthenticator {
    PrivateKey(AccountKey),
    Keyless(KeylessAccount),
    FederatedKeyless(FederatedKeylessAccount),
    Abstraction(AbstractedAccount), // TODO: Add support for keyless authentication
    DerivableAbstraction(DomainAbstractedAccount), // TODO: Add support for keyless authentication
}

impl LocalAccountAuthenticator {
    pub fn sign_transaction(&self, txn: RawTransaction) -> SignedTransaction {
        match self {
            LocalAccountAuthenticator::PrivateKey(key) => txn
                .sign(key.private_key(), key.public_key().clone())
                .expect("Signing a txn can't fail")
                .into_inner(),
            LocalAccountAuthenticator::Keyless(keyless_account) => {
                let sig = self.build_keyless_signature(txn.clone(), &keyless_account);
                SignedTransaction::new_keyless(txn, keyless_account.public_key.clone(), sig)
            },
            LocalAccountAuthenticator::FederatedKeyless(federated_keyless_account) => {
                let sig = self.build_keyless_signature(txn.clone(), &federated_keyless_account);
                SignedTransaction::new_federated_keyless(
                    txn,
                    federated_keyless_account.public_key.clone(),
                    sig,
                )
            },
            LocalAccountAuthenticator::Abstraction(..) => unreachable!(),
            LocalAccountAuthenticator::DerivableAbstraction(..) => unreachable!(),
        }
    }

    fn build_keyless_signature(
        &self,
        txn: RawTransaction,
        account: &impl CommonKeylessAccount,
    ) -> KeylessSignature {
        let proof = account.zk_sig().proof;
        let txn_and_zkp = keyless::TransactionAndProof {
            message: txn,
            proof: Some(proof),
        };

        let esk = account.ephem_private_key();
        let ephemeral_signature = esk.sign(&txn_and_zkp).unwrap();

        KeylessSignature {
            cert: EphemeralCertificate::ZeroKnowledgeSig(account.zk_sig().clone()),
            jwt_header_json: account.jwt_header_json().clone(),
            exp_date_secs: account.expiry_date_secs(),
            ephemeral_pubkey: account.ephem_public_key().clone(),
            ephemeral_signature,
        }
    }
}
impl<T: Into<AccountKey>> From<T> for LocalAccountAuthenticator {
    fn from(key: T) -> Self {
        Self::PrivateKey(key.into())
    }
}

/// LocalAccount represents an account on the Aptos blockchain. Internally it
/// holds the private / public key pair and the address of the account. You can
/// use this struct to help transact with the blockchain, e.g. by generating a
/// new account and signing transactions.
#[derive(Debug)]
pub struct LocalAccount {
    /// Address of the account.
    address: AccountAddress,
    /// Authenticator of the account
    auth: LocalAccountAuthenticator,
    /// Latest known sequence number of the account, it can be different from validator.
    sequence_number: AtomicU64,
}

pub fn get_apt_primary_store_address(address: AccountAddress) -> AccountAddress {
    get_paired_fa_primary_store_address(address, *APT_METADATA_ADDRESS)
}

pub fn get_paired_fa_primary_store_address(
    address: AccountAddress,
    fa_metadata_address: AccountAddress,
) -> AccountAddress {
    let mut bytes = address.to_vec();
    bytes.append(&mut fa_metadata_address.to_vec());
    bytes.push(0xFC);
    AccountAddress::from_bytes(aptos_crypto::hash::HashValue::sha3_256_of(&bytes).to_vec()).unwrap()
}

pub fn get_paired_fa_metadata_address(coin_type_name: &MoveStructTag) -> AccountAddress {
    let coin_type_name = coin_type_name.to_string();
    if coin_type_name == APTOS_COIN_TYPE_STR {
        *APT_METADATA_ADDRESS
    } else {
        let mut preimage = APT_METADATA_ADDRESS.to_vec();
        preimage.extend(coin_type_name.as_bytes());
        preimage.push(0xFE);
        AccountAddress::from_bytes(HashValue::sha3_256_of(&preimage).to_vec()).unwrap()
    }
}

impl LocalAccount {
    /// Create a new representation of an account locally. Note: This function
    /// does not actually create an account on the Aptos blockchain, just a
    /// local representation.
    pub fn new<T: Into<AccountKey>>(address: AccountAddress, key: T, sequence_number: u64) -> Self {
        Self {
            address,
            auth: LocalAccountAuthenticator::from(key),
            sequence_number: AtomicU64::new(sequence_number),
        }
    }

    pub fn new_keyless(
        address: AccountAddress,
        keyless_account: KeylessAccount,
        sequence_number: u64,
    ) -> Self {
        Self {
            address,
            auth: LocalAccountAuthenticator::Keyless(keyless_account),
            sequence_number: AtomicU64::new(sequence_number),
        }
    }

    pub fn new_federated_keyless(
        address: AccountAddress,
        federated_keyless_account: FederatedKeylessAccount,
        sequence_number: u64,
    ) -> Self {
        Self {
            address,
            auth: LocalAccountAuthenticator::FederatedKeyless(federated_keyless_account),
            sequence_number: AtomicU64::new(sequence_number),
        }
    }

    pub fn new_domain_aa(
        function_info: FunctionInfo,
        account_identity: Vec<u8>,
        sign_func: Arc<dyn Fn(&[u8]) -> Vec<u8> + Send + Sync>,
        sequence_number: u64,
    ) -> Self {
        Self {
            address: AuthenticationKey::domain_abstraction_address(
                bcs::to_bytes(&function_info).unwrap(),
                &account_identity,
            )
            .account_address(),
            auth: LocalAccountAuthenticator::DerivableAbstraction(DomainAbstractedAccount {
                function_info,
                account_identity,
                sign_func,
            }),
            sequence_number: AtomicU64::new(sequence_number),
        }
    }

    /// Recover an account from derive path (e.g. m/44'/637'/0'/0'/0') and mnemonic phrase,
    pub fn from_derive_path(
        derive_path: &str,
        mnemonic_phrase: &str,
        sequence_number: u64,
    ) -> Result<Self> {
        let derive_path = DerivationPath::from_str(derive_path)?;
        let mnemonic = Mnemonic::from_phrase(mnemonic_phrase, Language::English)?;
        // TODO: Make `password` as an optional argument.
        let seed = Seed::new(&mnemonic, "");
        let key = ExtendedSecretKey::from_seed(seed.as_bytes())?
            .derive(&derive_path)?
            .secret_key;
        let key = AccountKey::from(Ed25519PrivateKey::try_from(key.as_bytes().as_ref())?);
        let address = key.authentication_key().account_address();

        Ok(Self::new(address, key, sequence_number))
    }

    /// Create a new account from the given private key in hex literal.
    pub fn from_private_key(private_key: &str, sequence_number: u64) -> Result<Self> {
        let key = AccountKey::from_private_key(Ed25519PrivateKey::try_from(
            hex::decode(private_key.trim_start_matches("0x"))?.as_ref(),
        )?);
        let address = key.authentication_key().account_address();

        Ok(Self::new(address, key, sequence_number))
    }

    pub fn generate_for_testing<R1>(rng: &mut R1, keyless_mode: bool) -> Self
    where
        R1: Rng + rand_core::CryptoRng,
    {
        if keyless_mode {
            let config = keyless::Configuration::new_for_testing();
            let now_secs = SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_secs();
            let esk = EphemeralPrivateKey::Ed25519 {
                inner_private_key: Ed25519PrivateKey::generate(rng),
            };
            let exp_timestamp_secs = now_secs + 7 * 86400; // + 7 days
            let exp_horizon_secs = 100 * 86400; // 100 days
            let blinder = vec![0x01; 31];
            let eph_key_pair = EphemeralKeyPair::new_with_keyless_config(
                &config,
                esk,
                exp_timestamp_secs,
                blinder,
            )
            .unwrap();

            // Simulation of OIDC provider processing.
            let iss = keyless::test_utils::get_sample_iss();
            let jwk = keyless::test_utils::get_sample_jwk();
            let aud = format!("aud_{}", hex::encode(rng.gen::<[u8; 4]>()));
            let uid_key = "sub".to_string();
            let uid_val = format!("uid_{}", hex::encode(rng.gen::<[u8; 4]>()));
            let jwt_header = keyless::test_utils::get_sample_jwt_header_json();
            let jwt_header_b64 = keyless::base64url_encode_str(&jwt_header);
            let jwt_payload = keyless::circuit_testcases::render_jwt_payload_json(
                &iss,
                &aud,
                &uid_key,
                &uid_val,
                "",
                now_secs,
                &eph_key_pair.nonce,
                now_secs + 86400,
            );
            let jwt_payload_b64 = keyless::base64url_encode_str(&jwt_payload);
            let jwt_msg = format!("{}.{}", jwt_header_b64, jwt_payload_b64);
            let jwt_sig = keyless::test_utils::oidc_provider_sign(
                *keyless::circuit_testcases::SAMPLE_JWK_SK,
                jwt_msg.as_bytes(),
            );
            let jwt_sig_b64 = base64::encode_config(jwt_sig, base64::URL_SAFE_NO_PAD);
            let jwt = format!("{}.{}", jwt_msg, jwt_sig_b64);

            let pepper = keyless::test_utils::get_sample_pepper();
            let idc = keyless::IdCommitment::new_from_preimage(&pepper, &aud, &uid_key, &uid_val)
                .unwrap();
            let public_inputs = keyless::bn254_circom::hash_public_inputs(
                &config,
                &eph_key_pair.public_key,
                &idc,
                exp_timestamp_secs,
                exp_horizon_secs,
                &iss,
                None,
                &jwt_header,
                &jwk,
                None,
            )
            .unwrap();
            let groth16_proof = keyless::proof_simulation::Groth16SimulatorBn254::create_random_proof_with_trapdoor(&[public_inputs], &keyless::circuit_constants::TEST_GROTH16_SETUP.simulation_pk, rng).unwrap();
            let zk_sig = ZeroKnowledgeSig {
                proof: keyless::ZKP::Groth16(groth16_proof),
                exp_horizon_secs,
                extra_field: None,
                override_aud_val: None,
                training_wheels_signature: None,
            };
            // zk_sig.verify_groth16_proof(public_inputs, &TEST_GROTH16_KEYS.prepared_vk).unwrap();
            let keyless_account =
                KeylessAccount::new_from_jwt(&jwt, eph_key_pair, Some(&uid_key), pepper, zk_sig)
                    .unwrap();

            Self::new_keyless(
                keyless_account.authentication_key().account_address(),
                keyless_account,
                0,
            )
        } else {
            Self::generate(rng)
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
        let address = key.authentication_key().account_address();

        Self::new(address, key, 0)
    }

    pub fn sign_transaction(&self, txn: RawTransaction) -> SignedTransaction {
        self.auth.sign_transaction(txn)
    }

    pub fn sign_with_transaction_builder(&self, builder: TransactionBuilder) -> SignedTransaction {
        let raw_txn = if builder.has_nonce() {
            // Do not increment sequence number for turbo transactions.
            builder
                .sender(self.address())
                .sequence_number(u64::MAX)
                .build()
        } else {
            builder
                .sender(self.address())
                .sequence_number(self.increment_sequence_number())
                .build()
        };
        self.sign_transaction(raw_txn)
    }

    pub fn sign_multi_agent_with_transaction_builder(
        &self,
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

        let raw_txn = if builder.has_nonce() {
            // Do not increment sequence number for turbo transactions.
            builder
                .sender(self.address())
                .sequence_number(u64::MAX)
                .build()
        } else {
            builder
                .sender(self.address())
                .sequence_number(self.increment_sequence_number())
                .build()
        };
        raw_txn
            .sign_multi_agent(
                self.private_key(),
                secondary_signer_addresses,
                secondary_signer_privkeys,
            )
            .expect("Signing multi agent txn failed")
            .into_inner()
    }

    pub fn sign_fee_payer_with_transaction_builder(
        &self,
        secondary_signers: Vec<&Self>,
        fee_payer_signer: &Self,
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
        let raw_txn = if builder.has_nonce() {
            // Do not increment sequence number for turbo transactions.
            builder
                .sender(self.address())
                .sequence_number(u64::MAX)
                .build()
        } else {
            builder
                .sender(self.address())
                .sequence_number(self.increment_sequence_number())
                .build()
        };
        raw_txn
            .sign_fee_payer(
                self.private_key(),
                secondary_signer_addresses,
                secondary_signer_privkeys,
                fee_payer_signer.address(),
                fee_payer_signer.private_key(),
            )
            .expect("Signing multi agent txn failed")
            .into_inner()
    }

    pub fn sign_aa_transaction_with_transaction_builder(
        &self,
        secondary_signers: Vec<&Self>,
        fee_payer_signer: Option<&Self>,
        builder: TransactionBuilder,
    ) -> SignedTransaction {
        let secondary_signer_addresses = secondary_signers
            .iter()
            .map(|signer| signer.address())
            .collect();
        let secondary_signer_auths = secondary_signers.iter().map(|a| a.auth()).collect();
        let raw_txn = if builder.has_nonce() {
            builder
                .sender(self.address())
                .sequence_number(u64::MAX)
                .build()
        } else {
            builder
                .sender(self.address())
                .sequence_number(self.increment_sequence_number())
                .build()
        };
        raw_txn
            .sign_aa_transaction(
                self.auth(),
                secondary_signer_addresses,
                secondary_signer_auths,
                fee_payer_signer.map(|fee_payer| (fee_payer.address(), fee_payer.auth())),
            )
            .expect("Signing aa txn failed")
            .into_inner()
    }

    pub fn address(&self) -> AccountAddress {
        self.address
    }

    pub fn private_key(&self) -> &Ed25519PrivateKey {
        match &self.auth {
            LocalAccountAuthenticator::PrivateKey(key) => key.private_key(),
            LocalAccountAuthenticator::Keyless(_) => todo!(),
            LocalAccountAuthenticator::FederatedKeyless(_) => todo!(),
            LocalAccountAuthenticator::Abstraction(..) => todo!(),
            LocalAccountAuthenticator::DerivableAbstraction(..) => todo!(),
        }
    }

    pub fn public_key(&self) -> &Ed25519PublicKey {
        match &self.auth {
            LocalAccountAuthenticator::PrivateKey(key) => key.public_key(),
            LocalAccountAuthenticator::Keyless(_) => todo!(),
            LocalAccountAuthenticator::FederatedKeyless(_) => todo!(),
            LocalAccountAuthenticator::Abstraction(..) => todo!(),
            LocalAccountAuthenticator::DerivableAbstraction(..) => todo!(),
        }
    }

    pub fn authentication_key(&self) -> AuthenticationKey {
        match &self.auth {
            LocalAccountAuthenticator::PrivateKey(key) => key.authentication_key(),
            LocalAccountAuthenticator::Keyless(keyless_account) => {
                keyless_account.authentication_key()
            },
            LocalAccountAuthenticator::FederatedKeyless(federated_keyless_account) => {
                federated_keyless_account.authentication_key()
            },
            LocalAccountAuthenticator::Abstraction(..) => todo!(),
            LocalAccountAuthenticator::DerivableAbstraction(..) => todo!(),
        }
    }

    pub fn auth(&self) -> Auth {
        match &self.auth {
            LocalAccountAuthenticator::PrivateKey(key) => Auth::Ed25519(key.private_key()),
            LocalAccountAuthenticator::Keyless(_) => todo!(),
            LocalAccountAuthenticator::FederatedKeyless(_) => todo!(),
            LocalAccountAuthenticator::Abstraction(aa) => {
                Auth::Abstraction(aa.function_info.clone(), aa.sign_func.clone())
            },
            LocalAccountAuthenticator::DerivableAbstraction(aa) => Auth::DerivableAbstraction {
                function_info: aa.function_info.clone(),
                account_identity: aa.account_identity.clone(),
                sign_function: aa.sign_func.clone(),
            },
        }
    }

    pub fn set_abstraction_auth(
        &mut self,
        function_info: FunctionInfo,
        sign_func: Arc<dyn Fn(&[u8]) -> Vec<u8> + Send + Sync>,
    ) {
        self.auth = LocalAccountAuthenticator::Abstraction(AbstractedAccount {
            function_info,
            sign_func,
        })
    }

    pub fn sequence_number(&self) -> u64 {
        self.sequence_number.load(Ordering::SeqCst)
    }

    pub fn increment_sequence_number(&self) -> u64 {
        self.sequence_number.fetch_add(1, Ordering::SeqCst)
    }

    pub fn decrement_sequence_number(&self) -> u64 {
        self.sequence_number.fetch_sub(1, Ordering::SeqCst)
    }

    pub fn set_sequence_number(&self, sequence_number: u64) {
        self.sequence_number
            .store(sequence_number, Ordering::SeqCst);
    }

    pub fn rotate_key<T: Into<AccountKey>>(&mut self, new_key: T) -> AccountKey {
        match &mut self.auth {
            LocalAccountAuthenticator::PrivateKey(key) => std::mem::replace(key, new_key.into()),
            LocalAccountAuthenticator::Keyless(_) => todo!(),
            LocalAccountAuthenticator::FederatedKeyless(_) => todo!(),
            LocalAccountAuthenticator::Abstraction(..) => todo!(),
            LocalAccountAuthenticator::DerivableAbstraction(..) => todo!(),
        }
    }

    pub fn received_event_key(&self) -> EventKey {
        EventKey::new(2, self.address)
    }

    pub fn sent_event_key(&self) -> EventKey {
        EventKey::new(3, self.address)
    }
}

/// Types of hardware wallet the SDK currently supports
#[derive(Debug)]
pub enum HardwareWalletType {
    Ledger,
}

pub trait TransactionSigner {
    fn sign_transaction(&self, txn: RawTransaction) -> Result<SignedTransaction>;

    fn sign_with_transaction_builder(
        &mut self,
        builder: TransactionBuilder,
    ) -> Result<SignedTransaction>;
}

/// Similar to LocalAccount, but for hardware wallets.
/// HardwareWallet does not have private key exported.
/// Anything that requires private key should be go through HardwareWallet.
#[derive(Debug)]
pub struct HardwareWalletAccount {
    address: AccountAddress,
    public_key: Ed25519PublicKey,
    derivation_path: String,
    hardware_wallet_type: HardwareWalletType,
    /// Same as LocalAccount's sequence_number.
    sequence_number: u64,
}

impl TransactionSigner for HardwareWalletAccount {
    fn sign_transaction(&self, txn: RawTransaction) -> Result<SignedTransaction> {
        let signature = self.sign_arbitrary_message(
            signing_message(&txn)
                .expect("Unable to convert txn to signing message.")
                .as_ref(),
        )?;
        Ok(SignedTransaction::new(
            txn,
            self.public_key().clone(),
            signature,
        ))
    }

    fn sign_with_transaction_builder(
        &mut self,
        builder: TransactionBuilder,
    ) -> Result<SignedTransaction> {
        let two_minutes = Duration::from_secs(2 * 60);
        let current_time = SystemTime::now().duration_since(UNIX_EPOCH)? + two_minutes;
        let seconds = current_time.as_secs();
        let turbo = builder.has_nonce();
        let sequence_number = if turbo {
            u64::MAX
        } else {
            self.sequence_number()
        };
        let raw_txn = builder
            .sender(self.address())
            .sequence_number(sequence_number)
            .expiration_timestamp_secs(seconds)
            .build();

        if !turbo {
            *self.sequence_number_mut() += 1;
        }
        self.sign_transaction(raw_txn)
    }
}

impl HardwareWalletAccount {
    pub fn new(
        address: AccountAddress,
        public_key: Ed25519PublicKey,
        derivation_path: String,
        hardware_wallet_type: HardwareWalletType,
        sequence_number: u64,
    ) -> Self {
        Self {
            address,
            public_key,
            derivation_path,
            hardware_wallet_type,
            sequence_number,
        }
    }

    /// Create a new account from a Ledger device.
    /// This requires the Ledger device to be connected, unlocked and the Aptos app to be opened
    pub fn from_ledger(
        derivation_path: String,
        sequence_number: u64,
    ) -> Result<Self, AptosLedgerError> {
        let public_key = aptos_ledger::get_public_key(&derivation_path, false)?;
        let authentication_key = AuthenticationKey::ed25519(&public_key);
        let address = authentication_key.account_address();

        Ok(Self::new(
            address,
            public_key,
            derivation_path,
            HardwareWalletType::Ledger,
            sequence_number,
        ))
    }

    pub fn address(&self) -> AccountAddress {
        self.address
    }

    pub fn public_key(&self) -> &Ed25519PublicKey {
        &self.public_key
    }

    pub fn derivation_path(&self) -> &str {
        &self.derivation_path
    }

    pub fn hardware_wallet_type(&self) -> &HardwareWalletType {
        &self.hardware_wallet_type
    }

    pub fn sequence_number(&self) -> u64 {
        self.sequence_number
    }

    pub fn sequence_number_mut(&mut self) -> &mut u64 {
        &mut self.sequence_number
    }

    pub fn sign_arbitrary_message(
        &self,
        message: &[u8],
    ) -> Result<Ed25519Signature, AptosLedgerError> {
        aptos_ledger::sign_message(&self.derivation_path, message)
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
        R: rand_core::RngCore + rand_core::CryptoRng,
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

#[derive(Debug, Eq, PartialEq, Deserialize)]
pub enum EphemeralPrivateKey {
    Ed25519 {
        inner_private_key: Ed25519PrivateKey,
    },
    Secp256r1Ecdsa {
        inner_private_key: secp256r1_ecdsa::PrivateKey,
    },
}

impl EphemeralPrivateKey {
    pub fn public_key(&self) -> EphemeralPublicKey {
        match self {
            EphemeralPrivateKey::Ed25519 { inner_private_key } => {
                EphemeralPublicKey::ed25519(inner_private_key.public_key())
            },
            EphemeralPrivateKey::Secp256r1Ecdsa { inner_private_key } => {
                EphemeralPublicKey::secp256r1_ecdsa(inner_private_key.public_key())
            },
        }
    }
}

impl TryFrom<&[u8]> for EphemeralPrivateKey {
    type Error = CryptoMaterialError;

    fn try_from(bytes: &[u8]) -> Result<Self, CryptoMaterialError> {
        bcs::from_bytes::<EphemeralPrivateKey>(bytes)
            .map_err(|_e| CryptoMaterialError::DeserializationError)
    }
}

impl EphemeralPrivateKey {
    pub fn sign<T: CryptoHash + Serialize>(
        &self,
        message: &T,
    ) -> Result<EphemeralSignature, CryptoMaterialError> {
        match self {
            EphemeralPrivateKey::Ed25519 { inner_private_key } => Ok(EphemeralSignature::ed25519(
                inner_private_key.sign(message)?,
            )),
            EphemeralPrivateKey::Secp256r1Ecdsa {
                inner_private_key: _,
            } => todo!(),
        }
    }
}
#[derive(Debug)]
pub struct EphemeralKeyPair {
    private_key: EphemeralPrivateKey,
    public_key: EphemeralPublicKey,
    nonce: String,
    expiry_date_secs: u64,
    blinder: Vec<u8>,
}

impl EphemeralKeyPair {
    pub fn new(
        private_key: EphemeralPrivateKey,
        expiry_date_secs: u64,
        blinder: Vec<u8>,
    ) -> Result<Self> {
        Self::new_with_keyless_config(
            &Configuration::new_for_devnet(),
            private_key,
            expiry_date_secs,
            blinder,
        )
    }

    pub fn new_with_keyless_config(
        config: &Configuration,
        private_key: EphemeralPrivateKey,
        expiry_date_secs: u64,
        blinder: Vec<u8>,
    ) -> Result<Self> {
        let epk = private_key.public_key();
        let nonce = OpenIdSig::reconstruct_oauth_nonce(&blinder, expiry_date_secs, &epk, config)?;

        Ok(Self {
            private_key,
            public_key: epk,
            nonce,
            expiry_date_secs,
            blinder,
        })
    }

    pub fn new_ed25519(
        private_key: Ed25519PrivateKey,
        expiry_date_secs: u64,
        blinder: Vec<u8>,
    ) -> Result<Self> {
        let esk = EphemeralPrivateKey::Ed25519 {
            inner_private_key: private_key,
        };
        Self::new(esk, expiry_date_secs, blinder)
    }
}

#[derive(Debug)]
pub struct KeylessAccount {
    public_key: KeylessPublicKey,
    ephemeral_key_pair: EphemeralKeyPair,
    zk_sig: ZeroKnowledgeSig,
    jwt_header_json: String,
    jwt: Option<String>,
}

#[derive(Debug)]
pub struct FederatedKeylessAccount {
    public_key: FederatedKeylessPublicKey,
    ephemeral_key_pair: EphemeralKeyPair,
    zk_sig: ZeroKnowledgeSig,
    jwt_header_json: String,
    jwt: Option<String>,
}

pub struct AbstractedAccount {
    function_info: FunctionInfo,
    sign_func: Arc<dyn Fn(&[u8]) -> Vec<u8> + Send + Sync>,
}

impl fmt::Debug for AbstractedAccount {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AbstractedAccount")
            .field("function_info", &self.function_info)
            .field("sign_func", &"<function pointer>") // Placeholder for the function
            .finish()
    }
}

pub struct DomainAbstractedAccount {
    function_info: FunctionInfo,
    account_identity: Vec<u8>,
    sign_func: Arc<dyn Fn(&[u8]) -> Vec<u8> + Send + Sync>,
}

impl fmt::Debug for DomainAbstractedAccount {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DomainAbstractedAccount")
            .field("function_info", &self.function_info)
            .field("account_identity", &self.account_identity)
            .field("sign_func", &"<function pointer>") // Placeholder for the function
            .finish()
    }
}

impl KeylessAccount {
    pub fn new(
        iss: &str,
        aud: &str,
        uid_key: &str,
        uid_val: &str,
        jwt_header_json: &str,
        ephemeral_key_pair: EphemeralKeyPair,
        pepper: Pepper,
        zk_sig: ZeroKnowledgeSig,
    ) -> Result<Self> {
        let public_key = create_keyless_public_key(iss, aud, uid_key, uid_val, &pepper)?;
        Ok(Self {
            public_key,
            ephemeral_key_pair,
            zk_sig,
            jwt_header_json: jwt_header_json.to_string(),
            jwt: None,
        })
    }

    pub fn new_from_jwt(
        jwt: &str,
        ephemeral_key_pair: EphemeralKeyPair,
        uid_key: Option<&str>,
        pepper: Pepper,
        zk_sig: ZeroKnowledgeSig,
    ) -> Result<Self> {
        let claims = extract_claims_from_jwt(jwt)?;
        let uid_key = uid_key.unwrap_or("sub").to_string();
        let uid_val = claims.get_uid_val(&uid_key)?;
        let aud = claims.oidc_claims.aud;

        let mut account = Self::new(
            &claims.oidc_claims.iss,
            &aud,
            &uid_key,
            &uid_val,
            &extract_header_json_from_jwt(jwt)?,
            ephemeral_key_pair,
            pepper,
            zk_sig,
        )?;
        account.jwt = Some(jwt.to_string());
        Ok(account)
    }

    pub fn authentication_key(&self) -> AuthenticationKey {
        AuthenticationKey::any_key(AnyPublicKey::keyless(self.public_key.clone()))
    }

    pub fn public_key(&self) -> &KeylessPublicKey {
        &self.public_key
    }
}

impl FederatedKeylessAccount {
    pub fn new(
        iss: &str,
        aud: &str,
        uid_key: &str,
        uid_val: &str,
        jwt_header_json: &str,
        ephemeral_key_pair: EphemeralKeyPair,
        pepper: Pepper,
        zk_sig: ZeroKnowledgeSig,
        jwk_addr: AccountAddress,
    ) -> Result<Self> {
        let public_key =
            create_federated_public_key(iss, aud, uid_key, uid_val, &pepper, jwk_addr)?;
        Ok(Self {
            public_key,
            ephemeral_key_pair,
            zk_sig,
            jwt_header_json: jwt_header_json.to_string(),
            jwt: None,
        })
    }

    pub fn new_from_jwt(
        jwt: &str,
        ephemeral_key_pair: EphemeralKeyPair,
        jwk_addr: AccountAddress,
        uid_key: Option<&str>,
        pepper: Pepper,
        zk_sig: ZeroKnowledgeSig,
    ) -> Result<Self> {
        let claims = extract_claims_from_jwt(jwt)?;
        let uid_key = uid_key.unwrap_or("sub").to_string();
        let uid_val = claims.get_uid_val(&uid_key)?;
        let aud = claims.oidc_claims.aud;

        let mut account = Self::new(
            &claims.oidc_claims.iss,
            &aud,
            &uid_key,
            &uid_val,
            &extract_header_json_from_jwt(jwt)?,
            ephemeral_key_pair,
            pepper,
            zk_sig,
            jwk_addr,
        )?;
        account.jwt = Some(jwt.to_string());
        Ok(account)
    }

    pub fn authentication_key(&self) -> AuthenticationKey {
        AuthenticationKey::any_key(AnyPublicKey::federated_keyless(self.public_key.clone()))
    }

    pub fn public_key(&self) -> &FederatedKeylessPublicKey {
        &self.public_key
    }
}

fn create_keyless_public_key(
    iss: &str,
    aud: &str,
    uid_key: &str,
    uid_val: &str,
    pepper: &Pepper,
) -> Result<KeylessPublicKey> {
    let idc = IdCommitment::new_from_preimage(pepper, aud, uid_key, uid_val)?;
    Ok(KeylessPublicKey {
        iss_val: iss.to_owned(),
        idc,
    })
}

fn create_federated_public_key(
    iss: &str,
    aud: &str,
    uid_key: &str,
    uid_val: &str,
    pepper: &Pepper,
    jwk_addr: AccountAddress,
) -> Result<FederatedKeylessPublicKey> {
    let idc = IdCommitment::new_from_preimage(pepper, aud, uid_key, uid_val)?;
    Ok(FederatedKeylessPublicKey {
        pk: KeylessPublicKey {
            iss_val: iss.to_owned(),
            idc,
        },
        jwk_addr,
    })
}

pub async fn derive_keyless_account(
    rest_client: &Client,
    jwt: &str,
    ephemeral_key_pair: EphemeralKeyPair,
    jwk_addr: Option<AccountAddress>,
) -> Result<LocalAccount> {
    let pepper = get_pepper_from_jwt(rest_client, jwt, &ephemeral_key_pair).await?;
    let zksig = get_proof_from_jwt(rest_client, jwt, &ephemeral_key_pair, &pepper).await?;

    let account = match jwk_addr {
        Some(jwk_addr) => {
            let federated_account = FederatedKeylessAccount::new_from_jwt(
                jwt,
                ephemeral_key_pair,
                jwk_addr,
                Some("sub"),
                pepper.clone(),
                zksig,
            )?;
            LocalAccount::new_federated_keyless(
                federated_account.authentication_key().account_address(),
                federated_account,
                0, // We'll update this with the actual sequence number below
            )
        },
        None => {
            let keyless_account = KeylessAccount::new_from_jwt(
                jwt,
                ephemeral_key_pair,
                Some("sub"),
                pepper.clone(),
                zksig,
            )?;
            LocalAccount::new_keyless(
                keyless_account.authentication_key().account_address(),
                keyless_account,
                0, // We'll update this with the actual sequence number below
            )
        },
    };

    // Look up the on-chain address and sequence number
    let address = rest_client
        .lookup_address(account.authentication_key().account_address(), false)
        .await?;
    let sequence_number = rest_client
        .get_account_sequence_number(account.authentication_key().account_address())
        .await?;

    // Create the final account with the correct address and sequence number
    Ok(match account.auth {
        LocalAccountAuthenticator::Keyless(keyless_account) => LocalAccount::new_keyless(
            address.into_inner(),
            keyless_account,
            sequence_number.into_inner(),
        ),
        LocalAccountAuthenticator::FederatedKeyless(federated_keyless_account) => {
            LocalAccount::new_federated_keyless(
                address.into_inner(),
                federated_keyless_account,
                sequence_number.into_inner(),
            )
        },
        _ => unreachable!("We only create keyless or federated keyless accounts here"),
    })
}

pub fn extract_claims_from_jwt(jwt: &str) -> Result<Claims> {
    let parts: Vec<&str> = jwt.split('.').collect();
    let jwt_payload_json =
        base64::decode_config(parts.get(1).context("jwt malformed")?, base64::URL_SAFE)?;
    let claims: Claims = serde_json::from_slice(&jwt_payload_json)?;
    Ok(claims)
}

pub fn extract_header_json_from_jwt(jwt: &str) -> Result<String> {
    let parts: Vec<&str> = jwt.split('.').collect();
    let header_bytes = base64::decode(parts.first().context("jwt malformed")?)?;

    Ok(String::from_utf8(header_bytes)?)
}

trait CommonKeylessAccount {
    fn zk_sig(&self) -> &ZeroKnowledgeSig;
    fn ephem_private_key(&self) -> &EphemeralPrivateKey;
    fn ephem_public_key(&self) -> &EphemeralPublicKey;
    fn jwt_header_json(&self) -> &String;
    fn expiry_date_secs(&self) -> u64;
}

impl CommonKeylessAccount for &KeylessAccount {
    fn zk_sig(&self) -> &ZeroKnowledgeSig {
        &self.zk_sig
    }

    fn ephem_private_key(&self) -> &EphemeralPrivateKey {
        &self.ephemeral_key_pair.private_key
    }

    fn ephem_public_key(&self) -> &EphemeralPublicKey {
        &self.ephemeral_key_pair.public_key
    }

    fn jwt_header_json(&self) -> &String {
        &self.jwt_header_json
    }

    fn expiry_date_secs(&self) -> u64 {
        self.ephemeral_key_pair.expiry_date_secs
    }
}

impl CommonKeylessAccount for &FederatedKeylessAccount {
    fn zk_sig(&self) -> &ZeroKnowledgeSig {
        &self.zk_sig
    }

    fn ephem_private_key(&self) -> &EphemeralPrivateKey {
        &self.ephemeral_key_pair.private_key
    }

    fn ephem_public_key(&self) -> &EphemeralPublicKey {
        &self.ephemeral_key_pair.public_key
    }

    fn jwt_header_json(&self) -> &String {
        &self.jwt_header_json
    }

    fn expiry_date_secs(&self) -> u64 {
        self.ephemeral_key_pair.expiry_date_secs
    }
}

async fn get_proof_from_jwt(
    rest_client: &Client,
    jwt: &str,
    ephemeral_key_pair: &EphemeralKeyPair,
    pepper: &Pepper,
) -> Result<ZeroKnowledgeSig> {
    let default_config = Configuration::new_for_devnet();
    let prover_request = ProverRequest {
        jwt_b64: jwt.to_string(),
        epk: bcs::to_bytes(&ephemeral_key_pair.public_key)?,
        epk_blinder: ephemeral_key_pair.blinder.clone(),
        exp_date_secs: ephemeral_key_pair.expiry_date_secs,
        exp_horizon_secs: default_config.max_exp_horizon_secs,
        pepper: pepper.to_bytes().to_vec(),
        uid_key: "sub".to_string(),
    };
    let response = rest_client.make_prover_request(prover_request).await?;
    Ok(response)
}

async fn get_pepper_from_jwt(
    rest_client: &Client,
    jwt: &str,
    ephemeral_key_pair: &EphemeralKeyPair,
) -> Result<Pepper> {
    let pepper_request = PepperRequest {
        jwt_b64: jwt.to_string(),
        epk: bcs::to_bytes(&ephemeral_key_pair.public_key)?,
        epk_blinder: ephemeral_key_pair.blinder.clone(),
        exp_date_secs: ephemeral_key_pair.expiry_date_secs,
        uid_key: "sub".to_string(),
    };
    let response = rest_client.make_pepper_request(pepper_request).await?;
    Ok(response)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::coin_client::CoinClient;
    use aptos_crypto::ed25519::Ed25519PrivateKey;
    use aptos_rest_client::{AptosBaseUrl, FaucetClient};
    use reqwest::Url;

    #[test]
    fn test_recover_account_from_derive_path() {
        // Same constants in test cases of TypeScript
        // https://github.com/aptos-labs/aptos-core/blob/main/ecosystem/typescript/sdk/src/aptos_account.test.ts
        let derive_path = "m/44'/637'/0'/0'/0'";
        let mnemonic_phrase =
            "shoot island position soft burden budget tooth cruel issue economy destroy above";
        let expected_address = "0x7968dab936c1bad187c60ce4082f307d030d780e91e694ae03aef16aba73f30";

        // Validate if the expected address.
        let account = LocalAccount::from_derive_path(derive_path, mnemonic_phrase, 0).unwrap();
        assert_eq!(account.address().to_hex_literal(), expected_address);

        // Return an error for empty derive path.
        assert!(LocalAccount::from_derive_path("", mnemonic_phrase, 0).is_err());

        // Return an error for empty mnemonic phrase.
        assert!(LocalAccount::from_derive_path(derive_path, "", 0).is_err());
    }

    #[test]
    fn test_create_account_from_private_key() {
        let key = AccountKey::generate(&mut rand::rngs::OsRng);
        let (private_key_hex_literal, public_key_hex_literal) = (
            hex::encode(key.private_key().to_bytes().as_ref()),
            key.authentication_key().account_address().to_hex_literal(),
        );

        // Test private key hex literal without `0x` prefix.
        let account = LocalAccount::from_private_key(&private_key_hex_literal, 0).unwrap();
        assert_eq!(account.address().to_hex_literal(), public_key_hex_literal);

        // Test private key hex literal with `0x` prefix.
        let account =
            LocalAccount::from_private_key(&format!("0x{}", private_key_hex_literal), 0).unwrap();
        assert_eq!(account.address().to_hex_literal(), public_key_hex_literal);

        // Test invalid private key hex literal.
        assert!(LocalAccount::from_private_key("invalid_private_key", 0).is_err());
    }

    #[ignore]
    #[tokio::test]
    async fn test_derive_keyless_account() {
        let aptos_rest_client = Client::builder(AptosBaseUrl::Devnet).build();
        // This JWT is taken from https://github.com/aptos-labs/aptos-ts-sdk/blob/f644e61beb70e69dfd489e75287c67b527385135/tests/e2e/api/keyless.test.ts#L11
        // As is the ephemeralKeyPair
        // This ephemeralKeyPair expires December 29, 2024.
        let jwt = "eyJhbGciOiJSUzI1NiIsInR5cCI6IkpXVCIsImtpZCI6InRlc3QtcnNhIn0.eyJpc3MiOiJ0ZXN0Lm9pZGMucHJvdmlkZXIiLCJhdWQiOiJ0ZXN0LWtleWxlc3MtZGFwcCIsInN1YiI6InRlc3QtdXNlci0wIiwiZW1haWwiOiJ0ZXN0QGFwdG9zbGFicy5jb20iLCJlbWFpbF92ZXJpZmllZCI6dHJ1ZSwiaWF0IjoxNzI1NDc1MTEyLCJleHAiOjI3MDAwMDAwMDAsIm5vbmNlIjoiNzA5NTI0MjMzMzk2NDQ1NzI2NzkzNDcyMzc2ODA4MDMwMzMyNDQ2MjgyMTE5MTc1NjQwOTQ1MDA5OTUxOTc4MTA1MTkxMDE4NzExOCJ9.eHqJLdje0FRD3UPmSw8sFHRYe9lwqSydAMcfHcpxkFwew2OTy6bWFsLQTdJp-eCZPhNzlfBXwNxaAJZksCWFWkzCz2913a5b88XRT9Im7JBDtA1e1IBXrnfXG0MDpsVRAuRNzLWqDi_4Fl1OELvoEOK-Tl4cmIwOhBr943S-b14PRVhrQ1XBD5MXaHWcJyxMaEtZfu_xxCQ-jjR---iguD243Ze98JlcOIV8VmEBg3YiSyVdMDZ8cgRia0DI8DwFn7rIxaV2H5FXb9JcehLgNP82-gsfEGV0iAXuBk7ZvRzMVA-srE9JvxVOyq5UkYu0Ss9LjKzX0KVojl7Au_OxGA";
        let sk_bytes =
            hex::decode("1111111111111111111111111111111111111111111111111111111111111111")
                .unwrap();
        let esk = Ed25519PrivateKey::try_from(sk_bytes.as_slice()).unwrap();
        let ephemeral_key_pair =
            EphemeralKeyPair::new_ed25519(esk, 1735475012, vec![0; 31]).unwrap();
        let mut account = derive_keyless_account(&aptos_rest_client, jwt, ephemeral_key_pair, None)
            .await
            .unwrap();
        println!("Address: {}", account.address().to_hex_literal());
        let balance = aptos_rest_client
            .view_apt_account_balance(account.address())
            .await
            .unwrap()
            .into_inner();
        if balance < 10000000 {
            println!("Funding account");
            let faucet_client = FaucetClient::new_from_rest_client(
                Url::from_str("https://faucet.devnet.aptoslabs.com").unwrap(),
                aptos_rest_client.clone(),
            );
            faucet_client
                .fund(account.address(), 10000000)
                .await
                .unwrap();
        }
        println!(
            "Balance: {}",
            aptos_rest_client
                .view_apt_account_balance(account.address())
                .await
                .unwrap()
                .into_inner()
        );
        let coin_client = CoinClient::new(&aptos_rest_client);
        let signed_txn = coin_client
            .get_signed_transfer_txn(
                &mut account,
                AccountAddress::from_hex_literal(
                    "0x7968dab936c1bad187c60ce4082f307d030d780e91e694ae03aef16aba73f30",
                )
                .unwrap(),
                1111111,
                None,
            )
            .await
            .unwrap();
        println!(
            "Sent 1111111 to 0x7968dab936c1bad187c60ce4082f307d030d780e91e694ae03aef16aba73f30"
        );
        aptos_rest_client
            .submit_without_deserializing_response(&signed_txn)
            .await
            .unwrap();
    }
}
