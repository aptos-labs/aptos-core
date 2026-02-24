// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::{
    move_types::account_address::AccountAddress,
    types::{
        chain_id::ChainId,
        transaction::{authenticator::AuthenticationKey, RawTransaction, TransactionPayload},
    },
};
use anyhow::Result;
use aptos_batch_encryption::{
    schemes::fptx_weighted::FPTXWeighted, traits::BatchThresholdEncryption,
};
pub use aptos_cached_packages::aptos_stdlib;
use aptos_crypto::{ed25519::Ed25519PublicKey, hash::CryptoHash, HashValue};
use aptos_global_constants::{GAS_UNIT_PRICE, MAX_GAS_AMOUNT};
use aptos_types::{
    function_info::FunctionInfo,
    secret_sharing::EncryptionKey,
    transaction::{
        encrypted_payload::{DecryptedPayload, EncryptedPayload, PayloadAssociatedData},
        EntryFunction, Script,
    },
};
use ark_std::rand::{rngs::StdRng, SeedableRng};
use rand::{thread_rng, Rng};
use std::sync::{Arc, RwLock};

#[derive(Debug, Default)]
pub struct EncryptionKeyState {
    pub epoch: u64,
    pub key: Option<EncryptionKey>,
}

pub struct TransactionBuilder {
    sender: Option<AccountAddress>,
    sequence_number: Option<u64>,
    payload: TransactionPayload,
    max_gas_amount: u64,
    gas_unit_price: u64,
    expiration_timestamp_secs: u64,
    chain_id: ChainId,
    encryption_key: Arc<RwLock<EncryptionKeyState>>,
}

impl TransactionBuilder {
    pub fn new(
        payload: TransactionPayload,
        expiration_timestamp_secs: u64,
        chain_id: ChainId,
    ) -> Self {
        Self {
            payload,
            chain_id,
            expiration_timestamp_secs,
            // TODO(Gas): double check this
            max_gas_amount: MAX_GAS_AMOUNT,
            gas_unit_price: std::cmp::max(GAS_UNIT_PRICE, 1),
            sender: None,
            sequence_number: None,
            encryption_key: Arc::new(RwLock::new(EncryptionKeyState::default())),
        }
    }

    pub fn sender(mut self, sender: AccountAddress) -> Self {
        self.sender = Some(sender);
        self
    }

    pub fn sequence_number(mut self, sequence_number: u64) -> Self {
        self.sequence_number = Some(sequence_number);
        self
    }

    pub fn max_gas_amount(mut self, max_gas_amount: u64) -> Self {
        self.max_gas_amount = max_gas_amount;
        self
    }

    pub fn gas_unit_price(mut self, gas_unit_price: u64) -> Self {
        self.gas_unit_price = gas_unit_price;
        self
    }

    pub fn chain_id(mut self, chain_id: ChainId) -> Self {
        self.chain_id = chain_id;
        self
    }

    pub fn expiration_timestamp_secs(mut self, expiration_timestamp_secs: u64) -> Self {
        self.expiration_timestamp_secs = expiration_timestamp_secs;
        self
    }

    pub fn has_nonce(&self) -> bool {
        self.payload.replay_protection_nonce().is_some()
    }

    pub fn upgrade_payload_with_rng(
        mut self,
        rng: &mut impl Rng,
        use_txn_payload_v2_format: bool,
        use_orderless_transactions: bool,
    ) -> Self {
        self.payload = self.payload.upgrade_payload_with_rng(
            rng,
            use_txn_payload_v2_format,
            use_orderless_transactions,
        );
        if use_orderless_transactions {
            self.sequence_number = self.sequence_number.map(|_| u64::MAX);
        }
        self
    }

    pub fn build(mut self) -> RawTransaction {
        // Read the encryption key at build time (not at builder-creation time)
        // so that epoch-change key rotations are always picked up.
        let encryption_key = self.encryption_key.read().unwrap().key.clone();
        if let Some(ref encryption_key) = encryption_key {
            assert!(!self.payload.is_encrypted_variant());
            let encrypted_payload = TransactionFactory::encrypt_payload(
                self.payload.clone(),
                self.sender.expect("sender must have been set"),
                encryption_key,
            )
            .expect("Payload must encrypt");
            self.payload = TransactionPayload::EncryptedPayload(encrypted_payload);
        }

        let sequence_number = if self.has_nonce() {
            u64::MAX
        } else {
            self.sequence_number
                .expect("sequence number must have been set")
        };
        RawTransaction::new(
            self.sender.expect("sender must have been set"),
            sequence_number,
            self.payload,
            self.max_gas_amount,
            self.gas_unit_price,
            self.expiration_timestamp_secs,
            self.chain_id,
        )
    }
}

#[derive(Clone, Debug)]
enum TransactionExpiration {
    Relative { expiration_duration: u64 },
    Absolute { expiration_timestamp: u64 },
}

#[derive(Clone, Debug)]
pub struct TransactionFactory {
    max_gas_amount: u64,
    gas_unit_price: u64,
    transaction_expiration: TransactionExpiration,
    chain_id: ChainId,

    use_txn_payload_v2_format: bool,
    use_replay_protection_nonce: bool,
    /// Shared encryption key. All clones of a factory share the same key,
    /// allowing it to be updated (e.g., on epoch change) and have all
    /// generators see the new key immediately.
    encryption_key: Arc<RwLock<EncryptionKeyState>>,
}

impl TransactionFactory {
    pub fn new(chain_id: ChainId) -> Self {
        Self {
            // TODO(Gas): double check if this right
            max_gas_amount: MAX_GAS_AMOUNT,
            gas_unit_price: GAS_UNIT_PRICE,
            transaction_expiration: TransactionExpiration::Relative {
                expiration_duration: 30,
            },
            chain_id,
            use_txn_payload_v2_format: false,
            use_replay_protection_nonce: false,
            encryption_key: Arc::new(RwLock::new(EncryptionKeyState::default())),
        }
    }

    /// Updates the encryption key and epoch at runtime. Since the state is
    /// shared across all clones of this factory, this affects all transaction
    /// generators that were created from this factory.
    pub fn update_encryption_key_state(&self, epoch: u64, key_bytes: Option<&[u8]>) -> Result<()> {
        let new_key = key_bytes.map(bcs::from_bytes).transpose()?;
        let mut state = self.encryption_key.write().unwrap();
        state.epoch = epoch;
        state.key = new_key;
        Ok(())
    }

    /// Detaches this factory from any shared encryption key, giving it its
    /// own independent (empty) key. Useful for init/setup factories that must
    /// never encrypt, even if sibling factories share the same Arc.
    pub fn without_encryption(mut self) -> Self {
        self.encryption_key = Arc::new(RwLock::new(EncryptionKeyState::default()));
        self
    }

    /// Returns a handle to the shared encryption key. This can be used to
    /// update the key from outside the factory (e.g., from an emitter worker
    /// that detects an epoch change).
    pub fn encryption_key_handle(&self) -> Arc<RwLock<EncryptionKeyState>> {
        self.encryption_key.clone()
    }

    pub fn with_max_gas_amount(mut self, max_gas_amount: u64) -> Self {
        self.max_gas_amount = max_gas_amount;
        self
    }

    pub fn with_gas_unit_price(mut self, gas_unit_price: u64) -> Self {
        self.gas_unit_price = gas_unit_price;
        self
    }

    pub fn with_transaction_expiration_time(mut self, transaction_expiration_time: u64) -> Self {
        self.transaction_expiration = TransactionExpiration::Relative {
            expiration_duration: transaction_expiration_time,
        };
        self
    }

    /// Sets the transaction expiration timestamp to an absolute value.
    /// Generally only useful for testing.
    pub fn with_absolute_transaction_expiration_timestamp(
        mut self,
        transaction_expiration_timestamp: u64,
    ) -> Self {
        self.transaction_expiration = TransactionExpiration::Absolute {
            expiration_timestamp: transaction_expiration_timestamp,
        };
        self
    }

    pub fn with_chain_id(mut self, chain_id: ChainId) -> Self {
        self.chain_id = chain_id;
        self
    }

    /// Enables or disables nonce-based replay protection (orderless transactions).
    /// When enabled, this also enables the v2 payload format, which is a prerequisite.
    /// Note: disabling does not revert `use_txn_payload_v2_format` since it may have
    /// been set independently.
    pub fn with_use_replay_protection_nonce(mut self, use_replay_protection_nonce: bool) -> Self {
        self.use_replay_protection_nonce = use_replay_protection_nonce;
        if use_replay_protection_nonce {
            self.use_txn_payload_v2_format = true;
        }
        self
    }

    pub fn get_max_gas_amount(&self) -> u64 {
        self.max_gas_amount
    }

    pub fn get_gas_unit_price(&self) -> u64 {
        self.gas_unit_price
    }

    /// Returns relative duration of transaction expiration.
    pub fn get_transaction_expiration_time(&self) -> u64 {
        match self.transaction_expiration {
            TransactionExpiration::Relative {
                expiration_duration,
            } => expiration_duration,
            TransactionExpiration::Absolute { .. } => {
                unimplemented!()
            },
        }
    }

    pub fn get_chain_id(&self) -> ChainId {
        self.chain_id
    }

    pub fn payload(&self, payload: TransactionPayload) -> TransactionBuilder {
        self.transaction_builder(payload)
    }

    pub fn entry_function(&self, func: EntryFunction) -> TransactionBuilder {
        // TODO[Orderless]: Change this to use TransactionPayload::Payload once it's available
        self.payload(TransactionPayload::EntryFunction(func))
    }

    pub fn create_user_account(&self, public_key: &Ed25519PublicKey) -> TransactionBuilder {
        self.payload(aptos_stdlib::aptos_account_create_account(
            AuthenticationKey::ed25519(public_key).account_address(),
        ))
    }

    pub fn add_dispatchable_authentication_function(
        &self,
        function_info: FunctionInfo,
    ) -> TransactionBuilder {
        self.payload(
            aptos_stdlib::account_abstraction_add_authentication_function(
                function_info.module_address,
                function_info.module_name.into_bytes(),
                function_info.function_name.into_bytes(),
            ),
        )
    }

    pub fn implicitly_create_user_account_and_transfer(
        &self,
        public_key: &Ed25519PublicKey,
        amount: u64,
    ) -> TransactionBuilder {
        self.payload(aptos_stdlib::aptos_account_transfer(
            AuthenticationKey::ed25519(public_key).account_address(),
            amount,
        ))
    }

    pub fn transfer(&self, to: AccountAddress, amount: u64) -> TransactionBuilder {
        self.payload(aptos_stdlib::aptos_coin_transfer(to, amount))
    }

    pub fn account_transfer(&self, to: AccountAddress, amount: u64) -> TransactionBuilder {
        self.payload(aptos_stdlib::aptos_account_transfer(to, amount))
    }

    pub fn create_multisig_account(
        &self,
        additional_owners: Vec<AccountAddress>,
        signatures_required: u64,
    ) -> TransactionBuilder {
        self.payload(aptos_stdlib::multisig_account_create_with_owners(
            additional_owners,
            signatures_required,
            vec![],
            vec![],
        ))
    }

    pub fn create_multisig_account_with_existing_account(
        &self,
        owners: Vec<AccountAddress>,
        signatures_required: u64,
    ) -> TransactionBuilder {
        self.payload(
            aptos_stdlib::multisig_account_create_with_existing_account_call(
                owners,
                signatures_required,
                vec![],
                vec![],
            ),
        )
    }

    pub fn create_multisig_account_with_existing_account_and_revoke_auth_key(
        &self,
        owners: Vec<AccountAddress>,
        signatures_required: u64,
    ) -> TransactionBuilder {
        self.payload(
            aptos_stdlib::multisig_account_create_with_existing_account_and_revoke_auth_key_call(
                owners,
                signatures_required,
                vec![],
                vec![],
            ),
        )
    }

    pub fn create_multisig_transaction(
        &self,
        multisig_account: AccountAddress,
        payload: Vec<u8>,
    ) -> TransactionBuilder {
        self.payload(aptos_stdlib::multisig_account_create_transaction(
            multisig_account,
            payload,
        ))
    }

    pub fn approve_multisig_transaction(
        &self,
        multisig_account: AccountAddress,
        transaction_id: u64,
    ) -> TransactionBuilder {
        self.payload(aptos_stdlib::multisig_account_approve_transaction(
            multisig_account,
            transaction_id,
        ))
    }

    pub fn reject_multisig_transaction(
        &self,
        multisig_account: AccountAddress,
        transaction_id: u64,
    ) -> TransactionBuilder {
        self.payload(aptos_stdlib::multisig_account_reject_transaction(
            multisig_account,
            transaction_id,
        ))
    }

    pub fn create_multisig_transaction_with_payload_hash(
        &self,
        multisig_account: AccountAddress,
        payload: Vec<u8>,
    ) -> TransactionBuilder {
        self.payload(aptos_stdlib::multisig_account_create_transaction_with_hash(
            multisig_account,
            HashValue::sha3_256_of(&payload).to_vec(),
        ))
    }

    pub fn mint(&self, to: AccountAddress, amount: u64) -> TransactionBuilder {
        self.payload(aptos_stdlib::aptos_coin_mint(to, amount))
    }

    //
    // Internal Helpers
    //

    pub fn script(&self, script: Script) -> TransactionBuilder {
        // TODO[Orderless]: Change this to use TransactionPayload::Payload once it's available
        self.payload(TransactionPayload::Script(script))
    }

    fn transaction_builder(&self, payload: TransactionPayload) -> TransactionBuilder {
        TransactionBuilder {
            sender: None,
            sequence_number: None,
            payload: if self.use_txn_payload_v2_format {
                payload.upgrade_payload_with_fn(
                    true,
                    self.use_replay_protection_nonce
                        .then_some(|| thread_rng().r#gen()),
                )
            } else {
                payload
            },
            max_gas_amount: self.max_gas_amount,
            gas_unit_price: self.gas_unit_price,
            expiration_timestamp_secs: self.expiration_timestamp(),
            chain_id: self.chain_id,
            encryption_key: self.encryption_key.clone(),
        }
    }

    /// Encrypts a transaction payload using the provided encryption key.
    /// This is a helper method used internally by `with_encrypted_payload` and `payload`.
    fn encrypt_payload(
        payload: TransactionPayload,
        sender: AccountAddress,
        encryption_key: &EncryptionKey,
    ) -> Result<EncryptedPayload> {
        // Convert payload to executable
        let executable = payload.executable()?;
        let extra_config = payload.extra_config();

        // Generate decryption nonce
        let decryption_nonce: u64 = rand::random();

        // Create DecryptedPayload for encryption
        let decrypted_payload = DecryptedPayload::new(executable, decryption_nonce);

        // Create associated data
        let associated_data = PayloadAssociatedData::new(sender);

        // Encrypt the payload
        // Note: FPTXWeighted::encrypt requires CryptoRng from ark_std::rand, so we use StdRng
        let mut rng = StdRng::from_entropy();
        let ciphertext = FPTXWeighted::encrypt(
            encryption_key,
            &mut rng,
            &decrypted_payload,
            &associated_data,
        )?;

        // Compute payload hash from the executable
        // We need to compute hash before encryption, so we use the executable directly
        let payload_hash = CryptoHash::hash(&decrypted_payload);

        // Create encrypted payload
        Ok(EncryptedPayload::Encrypted {
            ciphertext,
            extra_config,
            payload_hash,
        })
    }

    fn expiration_timestamp(&self) -> u64 {
        match self.transaction_expiration {
            TransactionExpiration::Relative {
                expiration_duration,
            } => {
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs()
                    + expiration_duration
            },
            TransactionExpiration::Absolute {
                expiration_timestamp,
            } => expiration_timestamp,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use aptos_types::transaction::{ReplayProtector, TransactionPayload};

    fn test_payload() -> TransactionPayload {
        aptos_stdlib::aptos_account_transfer(AccountAddress::ONE, 100)
    }

    #[test]
    fn test_orderless_transaction_factory() {
        let factory =
            TransactionFactory::new(ChainId::test()).with_use_replay_protection_nonce(true);

        let builder = factory.payload(test_payload());

        // The builder should have a nonce set for orderless transactions.
        assert!(builder.has_nonce());

        // Building the transaction with u64::MAX as sequence number (orderless
        // convention) should produce a raw transaction with the nonce in the
        // payload.
        let raw_txn = builder
            .sender(AccountAddress::ONE)
            .sequence_number(u64::MAX)
            .build();

        assert!(matches!(
            raw_txn.replay_protector(),
            ReplayProtector::Nonce(_)
        ));
        assert!(raw_txn.payload_ref().replay_protection_nonce().is_some());
    }

    #[test]
    fn test_regular_transaction_factory() {
        let factory = TransactionFactory::new(ChainId::test());

        let builder = factory.payload(test_payload());

        // Without orderless enabled, there should be no nonce.
        assert!(!builder.has_nonce());

        let raw_txn = builder
            .sender(AccountAddress::ONE)
            .sequence_number(5)
            .build();

        assert_eq!(
            raw_txn.replay_protector(),
            ReplayProtector::SequenceNumber(5)
        );
        assert!(raw_txn.payload_ref().replay_protection_nonce().is_none());
    }
}
