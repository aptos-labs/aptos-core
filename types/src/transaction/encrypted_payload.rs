// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::{
    secret_sharing::{Ciphertext, EvalProof},
    transaction::{TransactionExecutable, TransactionExecutableRef, TransactionExtraConfig},
};
use anyhow::{bail, Result};
use aptos_batch_encryption::traits::{AssociatedData, Plaintext};
use aptos_crypto::HashValue;
use move_core_types::account_address::AccountAddress;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Hash, Eq, PartialEq, Serialize, Deserialize)]
pub struct DecryptedPayload {
    executable: TransactionExecutable,
    decryption_nonce: u64,
}

impl DecryptedPayload {
    pub fn unwrap(self) -> (TransactionExecutable, u64) {
        (self.executable, self.decryption_nonce)
    }
}

impl Plaintext for DecryptedPayload {}

#[derive(Clone, Debug, Hash, Eq, PartialEq, Serialize, Deserialize)]
pub struct PayloadAssociatedData {
    sender: AccountAddress,
}

impl PayloadAssociatedData {
    fn new(sender: AccountAddress) -> Self {
        Self { sender }
    }
}

impl AssociatedData for PayloadAssociatedData {}

#[derive(Clone, Debug, Hash, Eq, PartialEq, Serialize, Deserialize)]
pub enum EncryptedPayload {
    Encrypted {
        ciphertext: Ciphertext,
        extra_config: TransactionExtraConfig,
        payload_hash: HashValue,
    },
    FailedDecryption {
        ciphertext: Ciphertext,
        extra_config: TransactionExtraConfig,
        payload_hash: HashValue,
        eval_proof: EvalProof,
    },
    Decrypted {
        ciphertext: Ciphertext,
        extra_config: TransactionExtraConfig,
        payload_hash: HashValue,
        eval_proof: EvalProof,

        // decrypted things
        executable: TransactionExecutable,
        decryption_nonce: u64,
    },
}

impl EncryptedPayload {
    pub fn ciphertext(&self) -> &Ciphertext {
        match self {
            Self::Encrypted { ciphertext, .. }
            | Self::FailedDecryption { ciphertext, .. }
            | Self::Decrypted { ciphertext, .. } => ciphertext,
        }
    }

    pub fn executable(&self) -> Result<TransactionExecutable> {
        let Self::Decrypted { executable, .. } = self else {
            bail!("Transaction is encrypted");
        };
        Ok(executable.clone())
    }

    pub fn executable_ref(&self) -> Result<TransactionExecutableRef<'_>> {
        let Self::Decrypted { executable, .. } = self else {
            bail!("Transaction is encrypted");
        };
        Ok(executable.as_ref())
    }

    pub fn extra_config(&self) -> &TransactionExtraConfig {
        match self {
            EncryptedPayload::Encrypted { extra_config, .. } => extra_config,
            EncryptedPayload::FailedDecryption { extra_config, .. } => extra_config,
            EncryptedPayload::Decrypted { extra_config, .. } => extra_config,
        }
    }

    pub fn is_encrypted(&self) -> bool {
        matches!(self, Self::Encrypted { .. })
    }

    pub fn into_decrypted(
        &mut self,
        eval_proof: EvalProof,
        executable: TransactionExecutable,
        nonce: u64,
    ) -> anyhow::Result<()> {
        let Self::Encrypted {
            ciphertext,
            extra_config,
            payload_hash,
        } = self
        else {
            bail!("Payload is not in Encrypted state");
        };

        *self = Self::Decrypted {
            ciphertext: ciphertext.clone(),
            extra_config: extra_config.clone(),
            payload_hash: *payload_hash,
            eval_proof,
            executable,
            decryption_nonce: nonce,
        };
        Ok(())
    }

    pub fn into_failed_decryption(&mut self, eval_proof: EvalProof) -> anyhow::Result<()> {
        let Self::Encrypted {
            ciphertext,
            extra_config,
            payload_hash,
        } = self
        else {
            bail!("Payload is not in Encrypted state");
        };

        // TODO(ibalajiarun): Avoid the clone
        *self = Self::FailedDecryption {
            ciphertext: ciphertext.clone(),
            extra_config: extra_config.clone(),
            payload_hash: *payload_hash,
            eval_proof,
        };
        Ok(())
    }

    pub fn verify(&self, sender: AccountAddress) -> anyhow::Result<()> {
        let associated_data = PayloadAssociatedData::new(sender);
        self.ciphertext().verify(&associated_data)
    }
}

/// A wrapper around `Mutex<EncryptedPayload>` that provides interior mutability
/// for encrypted payload decryption while implementing necessary traits for
/// use in `TransactionPayload`.
#[derive(Debug)]
pub struct SyncEncryptedPayload(aptos_infallible::Mutex<EncryptedPayload>);

impl SyncEncryptedPayload {
    pub fn new(payload: EncryptedPayload) -> Self {
        Self(aptos_infallible::Mutex::new(payload))
    }

    /// Get a reference to the inner payload by locking the mutex.
    pub fn lock(&self) -> aptos_infallible::MutexGuard<'_, EncryptedPayload> {
        self.0.lock()
    }

    /// Get read-only access to the inner payload.
    /// Caller must ensure the lock is held for the duration of the borrow.
    pub fn with<F, R>(&self, f: F) -> R
    where
        F: FnOnce(&EncryptedPayload) -> R,
    {
        f(&self.0.lock())
    }

    /// Get mutable access to the inner payload.
    pub fn with_mut<F, R>(&self, f: F) -> R
    where
        F: FnOnce(&mut EncryptedPayload) -> R,
    {
        f(&mut self.0.lock())
    }

    // Delegation methods for accessing inner EncryptedPayload

    pub fn ciphertext(&self) -> Ciphertext {
        self.0.lock().ciphertext().clone()
    }

    pub fn executable(&self) -> Result<TransactionExecutable> {
        self.0.lock().executable()
    }

    pub fn extra_config(&self) -> TransactionExtraConfig {
        self.0.lock().extra_config().clone()
    }

    pub fn is_encrypted(&self) -> bool {
        self.0.lock().is_encrypted()
    }

    pub fn verify(&self, sender: AccountAddress) -> Result<()> {
        self.0.lock().verify(sender)
    }
}

impl Clone for SyncEncryptedPayload {
    fn clone(&self) -> Self {
        Self::new(self.0.lock().clone())
    }
}

impl PartialEq for SyncEncryptedPayload {
    fn eq(&self, other: &Self) -> bool {
        *self.0.lock() == *other.0.lock()
    }
}

impl Eq for SyncEncryptedPayload {}

impl std::hash::Hash for SyncEncryptedPayload {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.0.lock().hash(state)
    }
}

impl Serialize for SyncEncryptedPayload {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.0.lock().serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for SyncEncryptedPayload {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        Ok(Self::new(EncryptedPayload::deserialize(deserializer)?))
    }
}
