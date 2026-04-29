// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use super::iterator::ShuffledTransactionIterator;
use crate::transaction_shuffler::{use_case_aware::Config, TransactionShuffler};
use aptos_types::transaction::{
    signature_verified_transaction::SignatureVerifiedTransaction,
    use_case::UseCaseAwareTransaction, BlockExecutableTransaction, SignedTransaction,
};
use std::fmt::Debug;

pub struct UseCaseAndEncryptedTxnsAwareShuffler {
    pub config: Config,
}

impl TransactionShuffler for UseCaseAndEncryptedTxnsAwareShuffler {
    fn shuffle(&self, txns: Vec<SignedTransaction>) -> Vec<SignedTransaction> {
        self.signed_transaction_iterator(txns).collect()
    }

    fn signed_transaction_iterator(
        &self,
        txns: Vec<SignedTransaction>,
    ) -> Box<dyn Iterator<Item = SignedTransaction> + 'static> {
        let (encrypted, regular): (Vec<_>, Vec<_>) = txns
            .into_iter()
            .partition(|t| t.payload().is_encrypted_variant());

        let iter = ShuffledTransactionIterator::new(self.config.clone()).extended_with(encrypted);
        Box::new(TwoPhaseIterator {
            inner: iter,
            pending_regular: regular,
        })
    }

    fn signature_verified_transaction_iterator(
        &self,
        txns: Vec<SignatureVerifiedTransaction>,
    ) -> Box<dyn Iterator<Item = SignatureVerifiedTransaction> + 'static> {
        let (encrypted, regular): (Vec<_>, Vec<_>) = txns.into_iter().partition(|t| {
            t.try_as_signed_user_txn()
                .is_some_and(|txn| txn.payload().is_encrypted_variant())
        });

        let iter = ShuffledTransactionIterator::new(self.config.clone()).extended_with(encrypted);
        Box::new(TwoPhaseIterator {
            inner: iter,
            pending_regular: regular,
        })
    }
}

struct TwoPhaseIterator<Txn> {
    inner: ShuffledTransactionIterator<Txn>,
    pending_regular: Vec<Txn>,
}

impl<Txn: UseCaseAwareTransaction + Debug> Iterator for TwoPhaseIterator<Txn> {
    type Item = Txn;

    fn next(&mut self) -> Option<Txn> {
        if let Some(txn) = self.inner.next() {
            return Some(txn);
        }
        if !self.pending_regular.is_empty() {
            self.inner
                .extend_with(std::mem::take(&mut self.pending_regular));
            self.inner.next()
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transaction_shuffler::use_case_aware::iterator::ShuffledTransactionIterator;
    use aptos_types::transaction::use_case::UseCaseKey;
    use move_core_types::account_address::AccountAddress;

    #[derive(Clone, Copy)]
    struct Account(u8);

    impl Account {
        fn as_account_address(self) -> AccountAddress {
            let mut addr = [0u8; 32];
            addr[31] = self.0;
            AccountAddress::new(addr)
        }
    }

    #[derive(Clone, Copy)]
    enum Contract {
        Platform,
        User(u8),
    }

    struct TestTxn {
        contract: Contract,
        sender: Account,
        encrypted: bool,
        original_idx: usize,
    }

    impl Debug for TestTxn {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(
                f,
                "t{}:{}a{}",
                self.original_idx,
                if self.encrypted { "E" } else { "" },
                self.sender.0
            )
        }
    }

    impl UseCaseAwareTransaction for TestTxn {
        fn parse_sender(&self) -> AccountAddress {
            self.sender.as_account_address()
        }

        fn parse_use_case(&self) -> UseCaseKey {
            match self.contract {
                Contract::Platform => UseCaseKey::Platform,
                Contract::User(c) => UseCaseKey::ContractAddress(Account(c).as_account_address()),
            }
        }
    }

    const A1: Account = Account(1);
    const A2: Account = Account(2);
    const A3: Account = Account(3);

    const C1: Contract = Contract::User(0xF1);
    const C2: Contract = Contract::User(0xF2);
    const PP: Contract = Contract::Platform;

    fn make_txns(specs: &[(Contract, Account, bool)]) -> Vec<TestTxn> {
        specs
            .iter()
            .enumerate()
            .map(|(i, &(contract, sender, encrypted))| TestTxn {
                contract,
                sender,
                encrypted,
                original_idx: i,
            })
            .collect()
    }

    fn two_phase_shuffle(config: Config, txns: Vec<TestTxn>) -> Vec<usize> {
        let (encrypted, regular): (Vec<_>, Vec<_>) = txns.into_iter().partition(|t| t.encrypted);

        let iter = ShuffledTransactionIterator::new(config).extended_with(encrypted);
        let mut two_phase = TwoPhaseIterator {
            inner: iter,
            pending_regular: regular,
        };

        std::iter::from_fn(move || two_phase.next())
            .map(|t| t.original_idx)
            .collect()
    }

    fn plain_shuffle(config: Config, txns: Vec<TestTxn>) -> Vec<usize> {
        ShuffledTransactionIterator::new(config)
            .extended_with(txns)
            .map(|t| t.original_idx)
            .collect()
    }

    #[test]
    fn test_encrypted_before_regular() {
        let config = Config {
            sender_spread_factor: 0,
            platform_use_case_spread_factor: 0,
            user_use_case_spread_factor: 0,
        };
        let txns = make_txns(&[
            (C1, A1, true),
            (C2, A2, false),
            (C1, A3, true),
            (C2, A1, false),
        ]);

        let order = two_phase_shuffle(config, txns);
        assert_eq!(order, vec![0, 2, 1, 3]);
    }

    #[test]
    fn test_cross_boundary_sender_spread() {
        let config = Config {
            sender_spread_factor: 2,
            platform_use_case_spread_factor: 0,
            user_use_case_spread_factor: 0,
        };
        let txns = make_txns(&[
            (C1, A1, true),  // 0: encrypted, A1
            (C2, A2, false), // 1: regular, A2
            (C1, A3, false), // 2: regular, A3
            (C2, A1, false), // 3: regular, A1
        ]);

        let order = two_phase_shuffle(config.clone(), txns);
        // Encrypted phase: [0] (A1 at output pos 0, try_delay_till = 0+1+2 = 3)
        // Regular phase starts at output pos 1.
        // A2(1) at pos 1: ok. A3(2) at pos 2: ok. A1(3) at pos 3: try_delay_till=3, ok.
        assert_eq!(order, vec![0, 1, 2, 3]);

        let txns = make_txns(&[
            (C1, A1, true),  // 0: encrypted, A1
            (C1, A1, false), // 1: regular, A1
            (C2, A2, false), // 2: regular, A2
        ]);

        let order = two_phase_shuffle(config, txns);
        // Encrypted: [0] (A1 at pos 0, try_delay_till=3)
        // Regular: A1(1) blocked until pos 3, A2(2) at pos 1. A1(1) force-popped at pos 2.
        assert_eq!(order, vec![0, 2, 1]);
    }

    #[test]
    fn test_use_case_spread_within_groups() {
        let config = Config {
            sender_spread_factor: 0,
            platform_use_case_spread_factor: 0,
            user_use_case_spread_factor: 1,
        };
        let txns = make_txns(&[
            (C1, A1, true), // 0
            (C1, A2, true), // 1
            (C2, A3, true), // 2
        ]);

        let order = two_phase_shuffle(config, txns);
        assert_eq!(order, vec![0, 2, 1]);
    }

    #[test]
    fn test_empty_encrypted() {
        let config = Config {
            sender_spread_factor: 1,
            platform_use_case_spread_factor: 0,
            user_use_case_spread_factor: 0,
        };

        let order = two_phase_shuffle(
            config.clone(),
            make_txns(&[(C1, A1, false), (C2, A1, false), (C1, A2, false)]),
        );
        let expected = plain_shuffle(
            config,
            make_txns(&[(C1, A1, false), (C2, A1, false), (C1, A2, false)]),
        );
        assert_eq!(order, expected);
    }

    #[test]
    fn test_all_encrypted() {
        let config = Config {
            sender_spread_factor: 1,
            platform_use_case_spread_factor: 0,
            user_use_case_spread_factor: 0,
        };

        let order = two_phase_shuffle(
            config.clone(),
            make_txns(&[(C1, A1, true), (C2, A1, true), (C1, A2, true)]),
        );
        let expected = plain_shuffle(
            config,
            make_txns(&[(C1, A1, true), (C2, A1, true), (C1, A2, true)]),
        );
        assert_eq!(order, expected);
    }

    #[test]
    fn test_output_is_permutation_and_encrypted_first() {
        let config = Config {
            sender_spread_factor: 2,
            platform_use_case_spread_factor: 1,
            user_use_case_spread_factor: 1,
        };
        let encrypted_original_indices = [0usize, 1, 5];
        let regular_original_indices = [2usize, 3, 4];
        let txns = make_txns(&[
            (PP, A1, true),  // 0
            (C1, A2, true),  // 1
            (C2, A3, false), // 2
            (PP, A1, false), // 3
            (C1, A2, false), // 4
            (C2, A3, true),  // 5
        ]);

        let order = two_phase_shuffle(config, txns);

        let mut sorted = order.clone();
        sorted.sort();
        assert_eq!(sorted, vec![0, 1, 2, 3, 4, 5]);

        let max_encrypted_pos = order
            .iter()
            .position(|idx| regular_original_indices.contains(idx))
            .unwrap();
        for idx in &order[..max_encrypted_pos] {
            assert!(
                encrypted_original_indices.contains(idx),
                "Non-encrypted txn {idx} found in encrypted prefix"
            );
        }
    }
}
