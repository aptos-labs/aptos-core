// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use aptos_crypto::bls12381;
use aptos_safety_rules::Error;
use aptos_types::ledger_info::{LedgerInfo, LedgerInfoWithSignatures};

pub trait CommitSignerProvider: Send + Sync {
    fn sign_commit_vote(
        &self,
        ledger_info: LedgerInfoWithSignatures,
        new_ledger_info: LedgerInfo,
    ) -> Result<bls12381::Signature, Error>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pipeline::tests::test_utils::{
        prepare_executed_blocks_with_ordered_ledger_info, prepare_safety_rules,
    };
    use aptos_crypto::HashValue;
    use aptos_types::{aggregate_signature::AggregateSignature, block_info::BlockInfo};

    #[test]
    fn commit_signer_preserves_safety_rules_validation() {
        let (signer, validators) = prepare_safety_rules();
        let (blocks, ordered) = prepare_executed_blocks_with_ordered_ledger_info(&validators[0]);
        let executed = LedgerInfo::new(
            blocks.last().unwrap().block_info(),
            HashValue::from_u64(0xBEEF),
        );
        assert!(signer
            .sign_commit_vote(ordered.clone(), executed.clone())
            .is_ok());
        assert!(matches!(
            signer.sign_commit_vote(
                ordered.clone(),
                LedgerInfo::new(BlockInfo::random(1), HashValue::from_u64(0xBEEF))
            ),
            Err(Error::InconsistentExecutionResult(_, _))
        ));
        assert!(matches!(
            signer.sign_commit_vote(
                LedgerInfoWithSignatures::new(
                    ordered.ledger_info().clone(),
                    AggregateSignature::empty()
                ),
                executed
            ),
            Err(Error::InvalidQuorumCertificate(_))
        ));
    }
}
