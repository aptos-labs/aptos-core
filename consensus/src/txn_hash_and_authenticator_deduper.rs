// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    counters::{TXN_DEDUP_FILTERED, TXN_DEDUP_SECONDS},
    transaction_deduper::TransactionDeduper,
};
use aptos_experimental_runtimes::thread_manager::optimal_min_len;
use aptos_types::transaction::SignedTransaction;
use rayon::prelude::*;
use std::collections::{HashMap, HashSet};

/// An implementation of TransactionDeduper. Duplicate filtering is done using the pair
/// (raw_txn.hash(), authenticator). Both the hash and signature are required because dedup
/// happens before signatures are verified and transaction prologue is checked. (So, e.g., a bad
/// transaction could contain a txn and signature that are unrelated.) If the checks are done
/// beforehand only one of the txn hash or signature would be required.
///
/// The implementation is written to avoid and/or parallelize the most expensive operations. Below
/// are the steps:
/// 1. Mark possible duplicates (sequential): Using a helper HashMap, mark transactions with 2+
///    (sender, seq_no) pairs as possible duplicates. If no possible duplicates, return the original
///    transactions.
/// 2. Calculate txn hashes (parallel): For all possible duplicates, calculate the txn hash. This
///    is an expensive operation.
/// 3. Filter duplicates (sequential): Using a helper HashSet with the txn hashes calculated above
///    and signatures, filter actual duplicate transactions.
///
/// Possible future optimizations:
/// a. Note the possible duplicates in Step 1 are independent of each other, so they could be
///    grouped independently and run in parallel in Step 3.
/// b. Txn hashes are calculated at many places within a validator. A per-txn hash cache could speed
///    up dedup or later operations.
/// c. If signature verification is moved to before dedup, then only the signature has to be matched
///    for duplicates and not the hash.
pub(crate) struct TxnHashAndAuthenticatorDeduper {}

impl TransactionDeduper for TxnHashAndAuthenticatorDeduper {
    fn dedup(&self, transactions: Vec<SignedTransaction>) -> Vec<SignedTransaction> {
        let _timer = TXN_DEDUP_SECONDS.start_timer();
        let mut seen = HashMap::new();
        let mut is_possible_duplicate = false;
        let mut possible_duplicates = vec![false; transactions.len()];
        for (i, txn) in transactions.iter().enumerate() {
            match seen.get(&(txn.sender(), txn.replay_protector())) {
                None => {
                    seen.insert((txn.sender(), txn.replay_protector()), i);
                },
                Some(first_index) => {
                    is_possible_duplicate = true;
                    possible_duplicates[*first_index] = true;
                    possible_duplicates[i] = true;
                },
            }
        }
        if !is_possible_duplicate {
            TXN_DEDUP_FILTERED.observe(0 as f64);
            return transactions;
        }

        let num_txns = transactions.len();

        let hash_and_authenticators: Vec<_> = possible_duplicates
            .into_par_iter()
            .zip(&transactions)
            .with_min_len(optimal_min_len(num_txns, 48))
            .map(|(need_hash, txn)| match need_hash {
                true => Some((txn.committed_hash(), txn.authenticator())),
                false => None,
            })
            .collect();

        // TODO: Possibly parallelize. See struct comment.
        let mut seen_hashes = HashSet::new();
        let mut num_duplicates: usize = 0;
        let filtered: Vec<_> = hash_and_authenticators
            .into_iter()
            .zip(transactions)
            .filter_map(|(maybe_hash, txn)| match maybe_hash {
                None => Some(txn),
                Some(hash_and_authenticator) => {
                    if seen_hashes.insert(hash_and_authenticator) {
                        Some(txn)
                    } else {
                        num_duplicates += 1;
                        None
                    }
                },
            })
            .collect();

        TXN_DEDUP_FILTERED.observe(num_duplicates as f64);
        filtered
    }
}

impl TxnHashAndAuthenticatorDeduper {
    pub fn new() -> Self {
        Self {}
    }
}

// TODO[Orderless]: Update these tests also when generating transactions with nonce.
#[cfg(test)]
mod tests {
    use crate::{
        transaction_deduper::TransactionDeduper,
        txn_hash_and_authenticator_deduper::TxnHashAndAuthenticatorDeduper,
    };
    use aptos_crypto::ed25519::{Ed25519PrivateKey, Ed25519PublicKey};
    use aptos_keygen::KeyGen;
    use aptos_types::{
        chain_id::ChainId,
        transaction::{
            EntryFunction, RawTransaction, ReplayProtector, Script, SignedTransaction,
            TransactionExecutable,
        },
        utility_coin::AptosCoinType,
        CoinType,
    };
    use move_core_types::{account_address::AccountAddress, ident_str, language_storage::ModuleId};
    use std::time::Instant;

    struct Account {
        addr: AccountAddress,
        /// The current private key for this account.
        pub privkey: Ed25519PrivateKey,
        /// The current public key for this account.
        pub pubkey: Ed25519PublicKey,
    }

    impl Account {
        pub fn new() -> Self {
            let (privkey, pubkey) = KeyGen::from_os_rng().generate_ed25519_keypair();
            Self::with_keypair(privkey, pubkey)
        }

        pub fn with_keypair(privkey: Ed25519PrivateKey, pubkey: Ed25519PublicKey) -> Self {
            let addr = aptos_types::account_address::from_public_key(&pubkey);
            Account {
                addr,
                privkey,
                pubkey,
            }
        }
    }

    fn raw_txn(
        executable: TransactionExecutable,
        sender: AccountAddress,
        replay_protector: ReplayProtector,
        gas_unit_price: u64,
    ) -> RawTransaction {
        RawTransaction::new_txn(
            sender,
            replay_protector,
            executable,
            None,
            500_000,
            gas_unit_price,
            0,
            ChainId::new(10),
        )
    }

    fn empty_txn(sender: AccountAddress, seq_num: u64, gas_unit_price: u64) -> RawTransaction {
        let executable = TransactionExecutable::Script(Script::new(vec![], vec![], vec![]));
        raw_txn(
            executable,
            sender,
            ReplayProtector::SequenceNumber(seq_num),
            gas_unit_price,
        )
    }

    fn peer_to_peer_txn(
        sender: AccountAddress,
        receiver: AccountAddress,
        seq_num: u64,
        gas_unit_price: u64,
    ) -> RawTransaction {
        let entry_func = EntryFunction::new(
            ModuleId::new(AccountAddress::ONE, ident_str!("coin").to_owned()),
            ident_str!("transfer").to_owned(),
            vec![AptosCoinType::type_tag()],
            vec![
                bcs::to_bytes(&receiver).unwrap(),
                bcs::to_bytes(&1).unwrap(),
            ],
        );
        let executable = TransactionExecutable::EntryFunction(entry_func);
        raw_txn(
            executable,
            sender,
            ReplayProtector::SequenceNumber(seq_num),
            gas_unit_price,
        )
    }

    fn block(refs: Vec<&SignedTransaction>) -> Vec<SignedTransaction> {
        refs.into_iter().cloned().collect()
    }

    #[test]
    fn test_single_txn() {
        let deduper = TxnHashAndAuthenticatorDeduper::new();

        let sender = Account::new();
        let txn = empty_txn(sender.addr, 0, 100)
            .sign(&sender.privkey, sender.pubkey)
            .unwrap()
            .into_inner();
        let txns = vec![txn];
        let deduped_txns = deduper.dedup(txns.clone());
        assert_eq!(txns.len(), deduped_txns.len());
        assert_eq!(txns, deduped_txns);
    }

    #[test]
    fn test_single_duplicate() {
        let deduper = TxnHashAndAuthenticatorDeduper::new();

        let sender = Account::new();
        let txn = empty_txn(sender.addr, 0, 100)
            .sign(&sender.privkey, sender.pubkey)
            .unwrap()
            .into_inner();
        let txns = block(vec![&txn, &txn]);
        let expected = block(vec![&txn]);
        let deduped_txns = deduper.dedup(txns);
        assert_eq!(expected.len(), deduped_txns.len());
        assert_eq!(expected, deduped_txns);
    }

    #[test]
    fn test_repeated_sequence_number() {
        let deduper = TxnHashAndAuthenticatorDeduper::new();

        let sender = Account::new();
        let receiver = Account::new();

        let txn_0a = empty_txn(sender.addr, 0, 100)
            .sign(&sender.privkey, sender.pubkey.clone())
            .unwrap()
            .into_inner();
        // Different txn, same sender and sequence number. Should not be filtered.
        let txn_0b = peer_to_peer_txn(sender.addr, receiver.addr, 0, 100)
            .sign(&sender.privkey, sender.pubkey)
            .unwrap()
            .into_inner();
        let txns = block(vec![&txn_0a, &txn_0b, &txn_0a]);
        let expected = block(vec![&txn_0a, &txn_0b]);
        let deduped_txns = deduper.dedup(txns);
        assert_eq!(expected.len(), deduped_txns.len());
        assert_eq!(expected, deduped_txns);
    }

    #[test]
    fn test_bad_signer() {
        let deduper = TxnHashAndAuthenticatorDeduper::new();

        let sender = Account::new();
        let bad_signer = Account::new();

        // Txn signed by a bad signer (not the sender)
        let txn_0a = empty_txn(sender.addr, 0, 100)
            .sign(&bad_signer.privkey, bad_signer.pubkey.clone())
            .unwrap()
            .into_inner();
        // Same txn, but signed by the correct signer (sender). Should not be filtered.
        let txn_0b = empty_txn(sender.addr, 0, 100)
            .sign(&sender.privkey, sender.pubkey.clone())
            .unwrap()
            .into_inner();
        let txns = block(vec![&txn_0a, &txn_0b]);
        let deduped_txns = deduper.dedup(txns.clone());
        assert_eq!(txns.len(), deduped_txns.len());
        assert_eq!(txns, deduped_txns);
    }

    // The perf tests are simple micro-benchmarks and just output results without checking for regressions
    static PERF_TXN_PER_BLOCK: usize = 10_000;

    fn measure_dedup_time(
        deduper: TxnHashAndAuthenticatorDeduper,
        txns: Vec<SignedTransaction>,
    ) -> f64 {
        let start = Instant::now();
        let mut iterations = 0;
        loop {
            deduper.dedup(txns.clone());
            iterations += 1;
            if iterations % 100 == 0 && start.elapsed().as_millis() > 2000 {
                break;
            }
        }
        let elapsed = start.elapsed();
        println!(
            "elapsed: {}, iterations: {}, time per iteration: {}",
            elapsed.as_secs_f64(),
            iterations,
            elapsed.as_secs_f64() / iterations as f64,
        );
        elapsed.as_secs_f64() / iterations as f64
    }

    #[test]
    fn test_performance_unique_empty_txns() {
        let deduper = TxnHashAndAuthenticatorDeduper::new();

        let sender = Account::new();
        let txns: Vec<_> = (0..PERF_TXN_PER_BLOCK)
            .map(|i| {
                empty_txn(sender.addr, i as u64, 100)
                    .sign(&sender.privkey, sender.pubkey.clone())
                    .unwrap()
                    .into_inner()
            })
            .collect();
        let deduped_txns = deduper.dedup(txns.clone());
        assert_eq!(txns.len(), deduped_txns.len());
        assert_eq!(txns, deduped_txns);

        measure_dedup_time(deduper, txns);
    }

    #[test]
    fn test_performance_duplicate_empty_txns() {
        let deduper = TxnHashAndAuthenticatorDeduper::new();

        let sender = Account::new();
        let txn = empty_txn(sender.addr, 0, 100)
            .sign(&sender.privkey, sender.pubkey)
            .unwrap()
            .into_inner();
        let txns: Vec<_> = std::iter::repeat(txn.clone())
            .take(PERF_TXN_PER_BLOCK)
            .collect();
        let expected = block(vec![&txn]);
        let deduped_txns = deduper.dedup(txns.clone());
        assert_eq!(expected.len(), deduped_txns.len());
        assert_eq!(expected, deduped_txns);

        measure_dedup_time(deduper, txns);
    }

    #[test]
    fn test_performance_unique_p2p_txns() {
        let deduper = TxnHashAndAuthenticatorDeduper::new();

        let sender = Account::new();
        let receiver = Account::new();
        let txns: Vec<_> = (0..PERF_TXN_PER_BLOCK)
            .map(|i| {
                peer_to_peer_txn(sender.addr, receiver.addr, i as u64, 100)
                    .sign(&sender.privkey, sender.pubkey.clone())
                    .unwrap()
                    .into_inner()
            })
            .collect();
        let deduped_txns = deduper.dedup(txns.clone());
        assert_eq!(txns.len(), deduped_txns.len());
        assert_eq!(txns, deduped_txns);

        measure_dedup_time(deduper, txns);
    }

    #[test]
    fn test_performance_duplicate_p2p_txns() {
        let deduper = TxnHashAndAuthenticatorDeduper::new();

        let sender = Account::new();
        let receiver = Account::new();
        let txn = peer_to_peer_txn(sender.addr, receiver.addr, 0, 100)
            .sign(&sender.privkey, sender.pubkey)
            .unwrap()
            .into_inner();
        let txns: Vec<_> = std::iter::repeat(txn.clone())
            .take(PERF_TXN_PER_BLOCK)
            .collect();
        let expected = block(vec![&txn]);
        let deduped_txns = deduper.dedup(txns.clone());
        assert_eq!(expected.len(), deduped_txns.len());
        assert_eq!(expected, deduped_txns);

        measure_dedup_time(deduper, txns);
    }
}
