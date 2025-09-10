// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::metrics::TIMER;
use anyhow::{ensure, Result};
use aptos_metrics_core::TimerHelper;
use aptos_storage_interface::state_store::state_update_refs::StateUpdateRefs;
use aptos_types::transaction::{PersistedAuxiliaryInfo, Transaction, TransactionOutput, Version};
use itertools::{izip, Itertools};
use std::{
    fmt::{Debug, Formatter},
    ops::Deref,
};

#[derive(Debug, Default)]
pub struct TransactionsWithOutput {
    pub transactions: Vec<Transaction>,
    pub transaction_outputs: Vec<TransactionOutput>,
    pub persisted_auxiliary_infos: Vec<PersistedAuxiliaryInfo>,
}

impl TransactionsWithOutput {
    pub fn new(
        transactions: Vec<Transaction>,
        transaction_outputs: Vec<TransactionOutput>,
        persisted_auxiliary_infos: Vec<PersistedAuxiliaryInfo>,
    ) -> Self {
        assert_eq!(transactions.len(), transaction_outputs.len());
        assert_eq!(transactions.len(), persisted_auxiliary_infos.len());
        Self {
            transactions,
            transaction_outputs,
            persisted_auxiliary_infos,
        }
    }

    pub fn new_empty() -> Self {
        Self::default()
    }

    pub fn push(
        &mut self,
        transaction: Transaction,
        transaction_output: TransactionOutput,
        persisted_auxiliary_info: PersistedAuxiliaryInfo,
    ) {
        self.transactions.push(transaction);
        self.transaction_outputs.push(transaction_output);
        self.persisted_auxiliary_infos
            .push(persisted_auxiliary_info);
    }

    pub fn len(&self) -> usize {
        self.transactions.len()
    }

    pub fn is_empty(&self) -> bool {
        self.transactions.is_empty()
    }

    pub fn iter(
        &self,
    ) -> impl Iterator<Item = (&Transaction, &TransactionOutput, &PersistedAuxiliaryInfo)> {
        izip!(
            self.transactions.iter(),
            self.transaction_outputs.iter(),
            self.persisted_auxiliary_infos.iter()
        )
    }

    pub fn last(&self) -> Option<(&Transaction, &TransactionOutput, &PersistedAuxiliaryInfo)> {
        self.transactions.last().map(|txn| {
            (
                txn,
                self.transaction_outputs.last().unwrap(),
                self.persisted_auxiliary_infos.last().unwrap(),
            )
        })
    }
}

#[ouroboros::self_referencing]
pub struct TransactionsToKeep {
    transactions_with_output: TransactionsWithOutput,
    is_reconfig: bool,
    #[borrows(transactions_with_output)]
    #[covariant]
    state_update_refs: StateUpdateRefs<'this>,
}

impl TransactionsToKeep {
    pub fn index(
        first_version: Version,
        transactions_with_output: TransactionsWithOutput,
        must_be_block: bool,
    ) -> Self {
        let _timer = TIMER.timer_with(&["transactions_to_keep__index"]);

        let (all_checkpoint_indices, is_reconfig) =
            Self::get_all_checkpoint_indices(&transactions_with_output, must_be_block);

        TransactionsToKeepBuilder {
            transactions_with_output,
            is_reconfig,
            state_update_refs_builder: |transactions_with_output| {
                let write_sets = transactions_with_output
                    .transaction_outputs
                    .iter()
                    .map(TransactionOutput::write_set);
                let last_checkpoint_index = all_checkpoint_indices.last().copied();
                StateUpdateRefs::index_write_sets(
                    first_version,
                    write_sets,
                    transactions_with_output.len(),
                    last_checkpoint_index,
                )
            },
        }
        .build()
    }

    pub fn make(
        first_version: Version,
        transactions: Vec<Transaction>,
        transaction_outputs: Vec<TransactionOutput>,
        persisted_auxiliary_infos: Vec<PersistedAuxiliaryInfo>,
    ) -> Self {
        let txns_with_output = TransactionsWithOutput::new(
            transactions,
            transaction_outputs,
            persisted_auxiliary_infos,
        );
        Self::index(first_version, txns_with_output, false)
    }

    pub fn new_empty() -> Self {
        Self::make(0, vec![], vec![], vec![])
    }

    pub fn new_dummy_success(txns: Vec<Transaction>) -> Self {
        let txn_outputs = vec![TransactionOutput::new_empty_success(); txns.len()];
        let persisted_auxiliary_infos = (0..txns.len())
            .map(|i| PersistedAuxiliaryInfo::V1 {
                transaction_index: i as u32,
            })
            .collect();
        Self::make(0, txns, txn_outputs, persisted_auxiliary_infos)
    }

    /// Whether the last txn of this block/chunk is reconfig.
    pub fn is_reconfig(&self) -> bool {
        *self.borrow_is_reconfig()
    }

    pub fn state_update_refs(&self) -> &StateUpdateRefs {
        self.borrow_state_update_refs()
    }

    pub fn ends_with_sole_checkpoint(&self) -> bool {
        let _timer = TIMER.timer_with(&["ends_with_sole_checkpoint"]);

        if self.is_reconfig() {
            !self
                .transactions
                .iter()
                .any(Transaction::is_non_reconfig_block_ending)
        } else {
            self.transactions
                .iter()
                .position(Transaction::is_non_reconfig_block_ending)
                == Some(self.len() - 1)
        }
    }

    fn get_all_checkpoint_indices(
        transactions_with_output: &TransactionsWithOutput,
        must_be_block: bool,
    ) -> (Vec<usize>, bool) {
        let _timer = TIMER.timer_with(&["get_all_checkpoint_indices"]);

        let (last_txn, last_output) = match transactions_with_output.last() {
            Some((txn, output, _)) => (txn, output),
            None => return (Vec::new(), false),
        };
        let is_reconfig = last_output.has_new_epoch_event();

        if must_be_block {
            assert!(last_txn.is_non_reconfig_block_ending() || is_reconfig);
            return (vec![transactions_with_output.len() - 1], is_reconfig);
        }

        let all = transactions_with_output
            .transactions
            .iter()
            .zip_eq(transactions_with_output.transaction_outputs.iter())
            .enumerate()
            .filter_map(|(idx, (txn, output))| {
                (txn.is_non_reconfig_block_ending() || output.has_new_epoch_event()).then_some(idx)
            })
            .collect();
        (all, is_reconfig)
    }

    pub fn ensure_at_most_one_checkpoint(&self) -> Result<()> {
        let _timer = TIMER.timer_with(&["unexpected__ensure_at_most_one_checkpoint"]);

        let mut total = self
            .transactions
            .iter()
            .filter(|t| t.is_non_reconfig_block_ending())
            .count();
        if self.is_reconfig() {
            total += self
                .transactions
                .last()
                .map_or(0, |t| !t.is_non_reconfig_block_ending() as usize);
        }

        ensure!(
            total <= 1,
            "Expecting at most one checkpoint, found {}",
            total,
        );
        Ok(())
    }
}

impl Deref for TransactionsToKeep {
    type Target = TransactionsWithOutput;

    fn deref(&self) -> &Self::Target {
        self.borrow_transactions_with_output()
    }
}

impl Debug for TransactionsToKeep {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TransactionsToKeep")
            .field(
                "transactions_with_output",
                self.borrow_transactions_with_output(),
            )
            .field("is_reconfig", &self.is_reconfig())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::{TransactionsToKeep, TransactionsWithOutput};
    use aptos_crypto::{ed25519::Ed25519PrivateKey, HashValue, PrivateKey, Uniform};
    use aptos_types::{
        account_address::AccountAddress,
        contract_event::ContractEvent,
        test_helpers::transaction_test_helpers::get_test_signed_txn,
        transaction::{
            ExecutionStatus, PersistedAuxiliaryInfo, Transaction, TransactionAuxiliaryData,
            TransactionOutput, TransactionStatus,
        },
        write_set::WriteSet,
    };

    fn dummy_txn() -> Transaction {
        let private_key = Ed25519PrivateKey::generate_for_testing();
        let public_key = private_key.public_key();
        let sender = AccountAddress::ZERO;
        Transaction::UserTransaction(get_test_signed_txn(
            sender,
            0,
            &private_key,
            public_key,
            None,
        ))
    }

    fn ckpt_txn() -> Transaction {
        Transaction::StateCheckpoint(HashValue::zero())
    }

    fn default_output() -> TransactionOutput {
        TransactionOutput::new(
            WriteSet::default(),
            vec![],
            0,
            TransactionStatus::Keep(ExecutionStatus::Success),
            TransactionAuxiliaryData::default(),
        )
    }

    fn output_with_reconfig() -> TransactionOutput {
        let reconfig_event = ContractEvent::new_v2_with_type_tag_str(
            "0x1::reconfiguration::NewEpochEvent",
            b"".to_vec(),
        );
        TransactionOutput::new(
            WriteSet::default(),
            vec![reconfig_event],
            0,
            TransactionStatus::Keep(ExecutionStatus::Success),
            TransactionAuxiliaryData::default(),
        )
    }

    fn default_aux_info() -> PersistedAuxiliaryInfo {
        PersistedAuxiliaryInfo::None
    }

    #[test]
    fn test_regular_block_without_reconfig() {
        let txns = vec![dummy_txn(), dummy_txn(), ckpt_txn()];
        let outputs = vec![default_output(), default_output(), default_output()];
        let aux_infos = vec![default_aux_info(), default_aux_info(), default_aux_info()];
        let txn_with_outputs = TransactionsWithOutput::new(txns, outputs, aux_infos);

        {
            let (all_ckpt_indices, is_reconfig) =
                TransactionsToKeep::get_all_checkpoint_indices(&txn_with_outputs, true);
            assert_eq!(all_ckpt_indices, vec![2]);
            assert!(!is_reconfig);
        }

        {
            let (all_ckpt_indices, is_reconfig) =
                TransactionsToKeep::get_all_checkpoint_indices(&txn_with_outputs, false);
            assert_eq!(all_ckpt_indices, vec![2]);
            assert!(!is_reconfig);
        }
    }

    #[test]
    fn test_regular_block_with_reconfig() {
        let txns = vec![dummy_txn(), dummy_txn(), dummy_txn()];
        let outputs = vec![default_output(), default_output(), output_with_reconfig()];
        let aux_infos = vec![default_aux_info(), default_aux_info(), default_aux_info()];
        let txn_with_outputs = TransactionsWithOutput::new(txns, outputs, aux_infos);

        {
            let (all_ckpt_indices, is_reconfig) =
                TransactionsToKeep::get_all_checkpoint_indices(&txn_with_outputs, true);
            assert_eq!(all_ckpt_indices, vec![2]);
            assert!(is_reconfig);
        }

        {
            let (all_ckpt_indices, is_reconfig) =
                TransactionsToKeep::get_all_checkpoint_indices(&txn_with_outputs, false);
            assert_eq!(all_ckpt_indices, vec![2]);
            assert!(is_reconfig);
        }
    }

    #[test]
    fn test_chunk_with_no_ckpt() {
        let txns = vec![dummy_txn(), dummy_txn(), dummy_txn()];
        let outputs = vec![default_output(), default_output(), default_output()];
        let aux_infos = vec![default_aux_info(), default_aux_info(), default_aux_info()];
        let txn_with_outputs = TransactionsWithOutput::new(txns, outputs, aux_infos);

        let (all_ckpt_indices, is_reconfig) =
            TransactionsToKeep::get_all_checkpoint_indices(&txn_with_outputs, false);
        assert!(all_ckpt_indices.is_empty());
        assert!(!is_reconfig);
    }

    #[test]
    fn test_chunk_with_ckpts_no_reconfig() {
        let txns = vec![
            dummy_txn(),
            ckpt_txn(),
            dummy_txn(),
            ckpt_txn(),
            dummy_txn(),
        ];
        let outputs = vec![
            default_output(),
            default_output(),
            default_output(),
            default_output(),
            default_output(),
        ];
        let aux_infos = vec![
            default_aux_info(),
            default_aux_info(),
            default_aux_info(),
            default_aux_info(),
            default_aux_info(),
        ];
        let txn_with_outputs = TransactionsWithOutput::new(txns, outputs, aux_infos);

        let (all_ckpt_indices, is_reconfig) =
            TransactionsToKeep::get_all_checkpoint_indices(&txn_with_outputs, false);
        assert_eq!(all_ckpt_indices, vec![1, 3]);
        assert!(!is_reconfig);
    }

    #[test]
    fn test_chunk_with_ckpts_with_reconfig_in_the_middle() {
        let txns = vec![
            dummy_txn(),
            ckpt_txn(),
            dummy_txn(),
            dummy_txn(),
            dummy_txn(),
        ];
        let outputs = vec![
            default_output(),
            default_output(),
            default_output(),
            output_with_reconfig(),
            default_output(),
        ];
        let aux_infos = vec![
            default_aux_info(),
            default_aux_info(),
            default_aux_info(),
            default_aux_info(),
            default_aux_info(),
        ];
        let txn_with_outputs = TransactionsWithOutput::new(txns, outputs, aux_infos);

        let (all_ckpt_indices, is_reconfig) =
            TransactionsToKeep::get_all_checkpoint_indices(&txn_with_outputs, false);
        assert_eq!(all_ckpt_indices, vec![1, 3]);
        assert!(!is_reconfig);
    }

    #[test]
    fn test_chunk_with_ckpts_with_reconfig_at_end() {
        let txns = vec![
            dummy_txn(),
            ckpt_txn(),
            dummy_txn(),
            dummy_txn(),
            dummy_txn(),
        ];
        let outputs = vec![
            default_output(),
            default_output(),
            default_output(),
            default_output(),
            output_with_reconfig(),
        ];
        let aux_infos = vec![
            default_aux_info(),
            default_aux_info(),
            default_aux_info(),
            default_aux_info(),
            default_aux_info(),
        ];
        let txn_with_outputs = TransactionsWithOutput::new(txns, outputs, aux_infos);

        let (all_ckpt_indices, is_reconfig) =
            TransactionsToKeep::get_all_checkpoint_indices(&txn_with_outputs, false);
        assert_eq!(all_ckpt_indices, vec![1, 4]);
        assert!(is_reconfig);
    }
}
