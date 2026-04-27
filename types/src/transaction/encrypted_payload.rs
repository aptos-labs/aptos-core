// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::{
    secret_sharing::{Ciphertext, EvalProof},
    transaction::{
        authenticator::AuthenticationKey, TransactionExecutable, TransactionExecutableRef,
        TransactionExtraConfig,
    },
};
use anyhow::{bail, Result};
use aptos_batch_encryption::traits::{AssociatedData, Plaintext};
use aptos_crypto::{hash::CryptoHash, HashValue};
use aptos_crypto_derive::{BCSCryptoHash, CryptoHasher};
use move_core_types::{
    account_address::AccountAddress, identifier::Identifier, language_storage::ModuleId,
};
use serde::{Deserialize, Serialize};

pub type DecryptionNonce = [u8; 16];

#[derive(
    Clone, Debug, Hash, Eq, PartialEq, Serialize, Deserialize, CryptoHasher, BCSCryptoHash,
)]
pub struct DecryptedPlaintext {
    executable: TransactionExecutable,
    decryption_nonce: DecryptionNonce,
}

impl DecryptedPlaintext {
    pub fn new(executable: TransactionExecutable, decryption_nonce: DecryptionNonce) -> Self {
        Self {
            executable,
            decryption_nonce,
        }
    }

    pub fn executable(&self) -> &TransactionExecutable {
        &self.executable
    }

    pub fn decryption_nonce(&self) -> &DecryptionNonce {
        &self.decryption_nonce
    }
}

impl Plaintext for DecryptedPlaintext {}

#[derive(Clone, Debug, Hash, Eq, PartialEq, Serialize, Deserialize)]
pub enum PayloadAssociatedData {
    V1 {
        sender: AccountAddress,
        signer_auth_keys: Vec<(AccountAddress, AuthenticationKey)>,
    },
}

impl PayloadAssociatedData {
    pub fn new(
        sender: AccountAddress,
        signer_auth_keys: Vec<(AccountAddress, AuthenticationKey)>,
    ) -> Self {
        Self::V1 {
            sender,
            signer_auth_keys,
        }
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
    /// The stored payload hash does not match the decrypted payload.
    PayloadHashMismatch,
    /// The payload was encrypted for a different epoch than the available decryption key.
    EpochMismatch,
}

// Mirrors EntryFunction in types/src/transaction/script.rs
#[derive(Clone, Debug, Hash, Eq, PartialEq, Serialize, Deserialize)]
pub struct ClaimedEntryFunction {
    pub module: ModuleId,
    pub function: Option<Identifier>,
}

#[derive(Clone, Debug, Hash, Eq, PartialEq, Serialize, Deserialize)]
pub struct EncryptedInner {
    pub ciphertext: Ciphertext,
    pub extra_config: TransactionExtraConfig,
    pub payload_hash: HashValue,
    /// Client-supplied label identifying which epoch's encryption key the
    /// submitter believes the ciphertext was encrypted under. This is a hint
    /// used for fast-path rejection when it diverges from the available
    /// decryption key's epoch — it is not a cryptographic commitment.
    /// Ciphertext integrity is enforced separately via `Ciphertext::verify`.
    pub encryption_epoch: u64,
    pub claimed_entry_fun: Option<ClaimedEntryFunction>,
}

#[derive(Clone, Debug, Hash, Eq, PartialEq, Serialize, Deserialize)]
pub enum EncryptedPayload {
    Encrypted(EncryptedInner),
    FailedDecryption {
        original: EncryptedInner,
        eval_proof: Option<EvalProof>,
        reason: DecryptionFailureReason,
    },
    Decrypted {
        original: EncryptedInner,
        eval_proof: EvalProof,
        decrypted: DecryptedPlaintext,
    },
}

impl EncryptedPayload {
    fn original(&self) -> &EncryptedInner {
        match self {
            Self::Encrypted(original)
            | Self::FailedDecryption { original, .. }
            | Self::Decrypted { original, .. } => original,
        }
    }

    pub fn ciphertext(&self) -> &Ciphertext {
        &self.original().ciphertext
    }

    pub fn executable(&self) -> Result<TransactionExecutable> {
        Ok(match self {
            EncryptedPayload::Encrypted(_) | EncryptedPayload::FailedDecryption { .. } => {
                TransactionExecutable::Encrypted
            },
            EncryptedPayload::Decrypted { decrypted, .. } => decrypted.executable().clone(),
        })
    }

    pub fn executable_ref(&self) -> Result<TransactionExecutableRef<'_>> {
        Ok(match self {
            EncryptedPayload::Encrypted(_) | EncryptedPayload::FailedDecryption { .. } => {
                TransactionExecutableRef::Encrypted
            },
            EncryptedPayload::Decrypted { decrypted, .. } => decrypted.executable().as_ref(),
        })
    }

    pub fn decryption_failure_reason(&self) -> Option<&DecryptionFailureReason> {
        match self {
            EncryptedPayload::FailedDecryption { reason, .. } => Some(reason),
            _ => None,
        }
    }

    pub fn extra_config(&self) -> &TransactionExtraConfig {
        &self.original().extra_config
    }

    pub fn encryption_epoch(&self) -> u64 {
        self.original().encryption_epoch
    }

    pub fn is_encrypted(&self) -> bool {
        matches!(self, Self::Encrypted(_))
    }

    /// Verify the decrypted plaintext against the stored commitments and, on success,
    /// transition `self` to `Decrypted`. On mismatch, `self` is left as `Encrypted` and the
    /// failure reason is returned so the caller can mark the payload failed.
    pub fn try_into_decrypted(
        &mut self,
        eval_proof: EvalProof,
        plaintext: DecryptedPlaintext,
    ) -> Result<(), DecryptionFailureReason> {
        let Self::Encrypted(original) = self else {
            panic!("try_into_decrypted called on non-Encrypted state");
        };

        if original.payload_hash != CryptoHash::hash(&plaintext) {
            return Err(DecryptionFailureReason::PayloadHashMismatch);
        }

        if let Some(claim) = &original.claimed_entry_fun {
            let TransactionExecutable::EntryFunction(entry_fun) = plaintext.executable() else {
                return Err(DecryptionFailureReason::ClaimedEntryFunctionMismatch);
            };
            if *entry_fun.module() != claim.module {
                return Err(DecryptionFailureReason::ClaimedEntryFunctionMismatch);
            }
            if let Some(claimed_function_id) = &claim.function
                && entry_fun.function() != claimed_function_id.as_ident_str()
            {
                return Err(DecryptionFailureReason::ClaimedEntryFunctionMismatch);
            }
        }

        *self = Self::Decrypted {
            original: original.clone(),
            eval_proof,
            decrypted: plaintext,
        };
        Ok(())
    }

    pub fn into_failed_decryption_with_reason(
        &mut self,
        eval_proof: Option<EvalProof>,
        reason: DecryptionFailureReason,
    ) -> anyhow::Result<()> {
        if let Self::Encrypted(original) = self {
            *self = Self::FailedDecryption {
                original: original.clone(),
                eval_proof,
                reason,
            };
            Ok(())
        } else {
            bail!("Payload is already in FailedDecryption or Decrypted state");
        }
        // TODO(ibalajiarun): Avoid the clone
    }

    pub fn verify(
        &self,
        sender: AccountAddress,
        signer_auth_keys: Vec<(AccountAddress, AuthenticationKey)>,
    ) -> anyhow::Result<()> {
        let associated_data = PayloadAssociatedData::new(sender, signer_auth_keys);
        self.ciphertext().verify(&associated_data)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transaction::TransactionExtraConfig;

    fn encrypted_with_hash(
        payload_hash: HashValue,
        claimed_entry_fun: Option<ClaimedEntryFunction>,
    ) -> EncryptedPayload {
        EncryptedPayload::Encrypted(EncryptedInner {
            ciphertext: Ciphertext::random(),
            extra_config: TransactionExtraConfig::V1 {
                multisig_address: None,
                replay_protection_nonce: None,
            },
            payload_hash,
            encryption_epoch: 7,
            claimed_entry_fun,
        })
    }

    #[test]
    fn try_into_decrypted_accepts_matching_hash() {
        let plaintext = DecryptedPlaintext::new(TransactionExecutable::Empty, [7; 16]);
        let mut encrypted = encrypted_with_hash(CryptoHash::hash(&plaintext), None);

        encrypted
            .try_into_decrypted(EvalProof::random(), plaintext)
            .unwrap();
        assert!(matches!(encrypted, EncryptedPayload::Decrypted { .. }));
    }

    #[test]
    fn try_into_decrypted_rejects_mismatched_hash() {
        let plaintext = DecryptedPlaintext::new(TransactionExecutable::Empty, [7; 16]);
        let mut encrypted = encrypted_with_hash(HashValue::random(), None);

        assert_eq!(
            encrypted.try_into_decrypted(EvalProof::random(), plaintext),
            Err(DecryptionFailureReason::PayloadHashMismatch)
        );
        assert!(matches!(encrypted, EncryptedPayload::Encrypted(_)));
    }

    #[test]
    fn try_into_decrypted_rejects_mismatched_entry_fun() {
        use crate::transaction::EntryFunction;
        use move_core_types::ident_str;

        let entry_fun = EntryFunction::new(
            ModuleId::new(AccountAddress::ONE, ident_str!("coin").to_owned()),
            ident_str!("transfer").to_owned(),
            vec![],
            vec![],
        );
        let plaintext =
            DecryptedPlaintext::new(TransactionExecutable::EntryFunction(entry_fun), [7; 16]);
        let claim = ClaimedEntryFunction {
            module: ModuleId::new(AccountAddress::ONE, ident_str!("other_module").to_owned()),
            function: None,
        };
        let mut encrypted = encrypted_with_hash(CryptoHash::hash(&plaintext), Some(claim));

        assert_eq!(
            encrypted.try_into_decrypted(EvalProof::random(), plaintext),
            Err(DecryptionFailureReason::ClaimedEntryFunctionMismatch)
        );
        assert!(matches!(encrypted, EncryptedPayload::Encrypted(_)));
    }

    #[test]
    fn state_transitions_preserve_encryption_epoch() {
        let mut encrypted = EncryptedPayload::Encrypted(EncryptedInner {
            ciphertext: Ciphertext::random(),
            extra_config: TransactionExtraConfig::V1 {
                multisig_address: None,
                replay_protection_nonce: None,
            },
            payload_hash: HashValue::random(),
            encryption_epoch: 11,
            claimed_entry_fun: None,
        });

        assert_eq!(encrypted.encryption_epoch(), 11);

        encrypted
            .into_failed_decryption_with_reason(None, DecryptionFailureReason::EpochMismatch)
            .unwrap();
        assert_eq!(encrypted.encryption_epoch(), 11);
    }
}
