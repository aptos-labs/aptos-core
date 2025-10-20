use crate::transaction::{TransactionExecutable, TransactionExecutableRef, TransactionExtraConfig};
use anyhow::{bail, Result};
use aptos_crypto::HashValue;
use serde::{Deserialize, Serialize};
use std::ops::Deref;

#[derive(Clone, Debug, Hash, Eq, PartialEq, Serialize, Deserialize)]
pub enum EncryptedPayload {
    V1(EncryptedPayloadV1),
}

impl Deref for EncryptedPayload {
    type Target = EncryptedPayloadV1;

    fn deref(&self) -> &Self::Target {
        let Self::V1(payload) = self;
        payload
    }
}

#[derive(Clone, Debug, Hash, Eq, PartialEq, Serialize, Deserialize)]
pub enum EncryptedPayloadV1 {
    Encrypted {
        cipher_text: Vec<u8>,
        payload_hash: HashValue,
        extra_config: TransactionExtraConfig,
    },
    Decrypted {
        cipher_text: Vec<u8>,
        payload_hash: HashValue,
        extra_config: TransactionExtraConfig,

        encryption_nonce: u64,
        executable: TransactionExecutable,
    },
}

impl EncryptedPayloadV1 {
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

    pub(crate) fn extra_config(&self) -> &TransactionExtraConfig {
        match self {
            EncryptedPayloadV1::Encrypted { extra_config, .. } => extra_config,
            EncryptedPayloadV1::Decrypted { extra_config, .. } => extra_config,
        }
    }
}
