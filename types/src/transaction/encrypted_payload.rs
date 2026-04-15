// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::{
    secret_sharing::{Ciphertext, EvalProof},
    transaction::{TransactionExecutable, TransactionExecutableRef, TransactionExtraConfig},
};
use anyhow::{bail, Result};
use aptos_batch_encryption::{
    schemes::fptx_weighted::FPTXWeighted,
    traits::{AssociatedData, BatchThresholdEncryption, Plaintext},
};
use aptos_crypto::HashValue;
use aptos_crypto_derive::{BCSCryptoHash, CryptoHasher};
use move_core_types::{
    account_address::AccountAddress, identifier::Identifier, language_storage::ModuleId,
};
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
pub enum DecryptionFailureReason {
    /// Cryptographic decryption failed (e.g., invalid ciphertext, key mismatch).
    CryptoFailure,
    /// Transaction exceeded the per-block encrypted transaction batch limit.
    /// Should be retried in a subsequent block.
    BatchLimitReached,
    /// The decryption key is not available.
    ConfigUnavailable,
    /// The decryption key is not available.
    DecryptionKeyUnavailable,
    /// The claimed entry function does not match the one in the decrypted payload
    ClaimedEntryFunctionMismatch,
}

// Mirrors EntryFunction in types/src/transaction/script.rs
#[derive(Clone, Debug, Hash, Eq, PartialEq, Serialize, Deserialize)]
pub struct ClaimedEntryFunction {
    pub module: ModuleId,
    pub function: Option<Identifier>,
}

#[derive(Clone, Debug, Hash, Eq, PartialEq, Serialize, Deserialize)]
pub enum EncryptedPayload {
    Encrypted {
        ciphertext: Ciphertext,
        extra_config: TransactionExtraConfig,
        payload_hash: HashValue,
        claimed_entry_fun: Option<ClaimedEntryFunction>,
    },
    FailedDecryption {
        ciphertext: Ciphertext,
        extra_config: TransactionExtraConfig,
        payload_hash: HashValue,
        claimed_entry_fun: Option<ClaimedEntryFunction>,
        eval_proof: Option<EvalProof>,
        reason: DecryptionFailureReason,
    },
    Decrypted {
        ciphertext: Ciphertext,
        extra_config: TransactionExtraConfig,
        payload_hash: HashValue,
        claimed_entry_fun: Option<ClaimedEntryFunction>,
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

    pub fn decryption_failure_reason(&self) -> Option<&DecryptionFailureReason> {
        match self {
            EncryptedPayload::FailedDecryption { reason, .. } => Some(reason),
            _ => None,
        }
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
            claimed_entry_fun,
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
            claimed_entry_fun: claimed_entry_fun.clone(),
        };
        Ok(())
    }

    pub fn entry_fun_matches(&self, decrypted: &DecryptedPayload) -> anyhow::Result<bool> {
        let Self::Encrypted {
            claimed_entry_fun, ..
        } = self
        else {
            bail!("Payload is not in Encrypted state");
        };

        if let Some(claim) = claimed_entry_fun {
            // If there is a claim about this payload, the payload executable must be an entry
            // function
            if let TransactionExecutable::EntryFunction(entry_fun) = &decrypted.executable {
                if *entry_fun.module() != claim.module {
                    // module must match
                    return Ok(false);
                } else if let Some(claimed_function_id) = &claim.function
                    && entry_fun.function() != claimed_function_id.as_ident_str()
                {
                    // if there is a claimed function name, this must also match
                    return Ok(false);
                }
            } else {
                return Ok(false);
            }
        }
        Ok(true)
    }

    pub fn into_failed_decryption_with_reason(
        &mut self,
        eval_proof: Option<EvalProof>,
        reason: DecryptionFailureReason,
    ) -> anyhow::Result<()> {
        if let Self::Encrypted {
            ciphertext,
            extra_config,
            payload_hash,
            claimed_entry_fun,
        } = self
        {
            *self = Self::FailedDecryption {
                ciphertext: ciphertext.clone(),
                extra_config: extra_config.clone(),
                payload_hash: *payload_hash,
                claimed_entry_fun: claimed_entry_fun.clone(),
                eval_proof,
                reason,
            };
            Ok(())
        } else {
            bail!("Payload is already in FailedDecryption or Decrypted state");
        }
        // TODO(ibalajiarun): Avoid the clone
    }

    pub fn verify(&self, sender: AccountAddress) -> anyhow::Result<()> {
        let associated_data = PayloadAssociatedData::new(sender);
        FPTXWeighted::verify_ct(self.ciphertext(), &associated_data)
    }
}
