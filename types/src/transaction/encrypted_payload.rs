// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Innovation-Enabling Source Code License

use crate::transaction::{TransactionExecutable, TransactionExecutableRef, TransactionExtraConfig};
use anyhow::{bail, Result};
use aptos_crypto::HashValue;
use serde::{Deserialize, Serialize};

pub type CipherText = Vec<u8>;
pub type EvalProof = Vec<u8>;

#[derive(Clone, Debug, Hash, Eq, PartialEq, Serialize, Deserialize)]
pub enum EncryptedPayload {
    Encrypted {
        #[serde(with = "serde_bytes")]
        ciphertext: CipherText,
        extra_config: TransactionExtraConfig,
        payload_hash: HashValue,
    },
    FailedDecryption {
        #[serde(with = "serde_bytes")]
        ciphertext: CipherText,
        extra_config: TransactionExtraConfig,
        payload_hash: HashValue,
        #[serde(with = "serde_bytes")]
        eval_proof: EvalProof,
    },
    Decrypted {
        #[serde(with = "serde_bytes")]
        ciphertext: CipherText,
        extra_config: TransactionExtraConfig,
        payload_hash: HashValue,
        #[serde(with = "serde_bytes")]
        eval_proof: EvalProof,

        // decrypted things
        executable: TransactionExecutable,
        decryption_nonce: u64,
    },
}

impl EncryptedPayload {
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
}
