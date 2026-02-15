// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::{
    secret_sharing::{Ciphertext, EvalProof},
    transaction::{TransactionExecutable, TransactionExecutableRef, TransactionExtraConfig},
};
use anyhow::{bail, Result};
use aptos_batch_encryption::traits::{AssociatedData, Plaintext};
use aptos_crypto::HashValue;
use aptos_crypto_derive::{BCSCryptoHash, CryptoHasher};
use move_core_types::account_address::AccountAddress;
use serde::{Deserialize, Serialize};

#[derive(
    Clone, Debug, Hash, Eq, PartialEq, Serialize, Deserialize, CryptoHasher, BCSCryptoHash,
)]
pub struct DecryptedPayload {
    executable: TransactionExecutable,
    decryption_nonce: u64,
}

impl DecryptedPayload {
    pub fn new(executable: TransactionExecutable, decryption_nonce: u64) -> Self {
        Self {
            executable,
            decryption_nonce,
        }
    }

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
    pub fn new(sender: AccountAddress) -> Self {
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
        Ok(match self {
            EncryptedPayload::Encrypted { .. } | EncryptedPayload::FailedDecryption { .. } => {
                TransactionExecutable::Encrypted
            },
            EncryptedPayload::Decrypted { executable, .. } => executable.clone(),
        })
    }

    pub fn executable_ref(&self) -> Result<TransactionExecutableRef<'_>> {
        Ok(match self {
            EncryptedPayload::Encrypted { .. } | EncryptedPayload::FailedDecryption { .. } => {
                TransactionExecutableRef::Encrypted
            },
            EncryptedPayload::Decrypted { executable, .. } => executable.as_ref(),
        })
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
