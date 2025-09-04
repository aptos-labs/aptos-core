// Copyright (c) Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use velor_logger::info;
use velor_mempool_notifications::CommittedTransaction;
use velor_types::transaction::use_case::UseCaseKey;
use std::{
    cmp::Ordering,
    collections::{BinaryHeap, HashMap, VecDeque},
};

pub(crate) struct UseCaseHistory {
    window_size: usize,
    num_top_to_track: usize,
    recent: VecDeque<HashMap<UseCaseKey, usize>>,
    total: HashMap<UseCaseKey, usize>,
}

impl UseCaseHistory {
    pub(crate) fn new(window_size: usize, num_top_to_track: usize) -> Self {
        Self {
            window_size,
            num_top_to_track,
            recent: VecDeque::with_capacity(window_size + 1),
            total: HashMap::new(),
        }
    }

    fn add_to_recent(&mut self, count_by_usecase: HashMap<UseCaseKey, usize>) {
        for (use_case, count) in &count_by_usecase {
            assert!(*count > 0);
            *self.total.entry(use_case.clone()).or_insert(0) += *count;
        }
        self.recent.push_back(count_by_usecase);

        while self.recent.len() > self.window_size {
            let to_remove = self.recent.pop_front().expect("non-empty after size check");
            for (use_case, count) in to_remove {
                assert!(count > 0);

                use std::collections::hash_map::Entry;
                match self.total.entry(use_case) {
                    Entry::Occupied(mut o) => {
                        assert!(*o.get() >= count);
                        *o.get_mut() -= count;
                        if *o.get() == 0 {
                            o.remove_entry();
                        }
                    },
                    Entry::Vacant(e) => {
                        panic!("Entry present in recent cannot be missing in total {:?}", e)
                    },
                }
            }
        }
    }

    pub fn compute_tracking_set(&self) -> HashMap<UseCaseKey, String> {
        let mut result = HashMap::new();
        result.insert(UseCaseKey::Platform, "entry_platform".to_string());
        result.insert(UseCaseKey::Others, "non_entry".to_string());

        let mut max_heap: BinaryHeap<UseCaseByCount> = self
            .total
            .iter()
            .filter(|(use_case, _)| {
                *use_case != &UseCaseKey::Platform && *use_case != &UseCaseKey::Others
            })
            .map(|(use_case, count)| UseCaseByCount {
                use_case: use_case.clone(),
                count: *count,
            })
            .collect::<BinaryHeap<_>>();

        let mut list_to_print = Vec::new();

        for i in 0..self.num_top_to_track {
            if let Some(UseCaseByCount { use_case, count }) = max_heap.pop() {
                let name = format!("entry_user_top_{}", i + 1);

                list_to_print.push((use_case.clone(), name.clone(), count));
                result.insert(use_case, name);
            }
        }
        info!("Computed new use case tracking set: {:?}", list_to_print);

        result
    }

    pub(crate) fn update_usecases(&mut self, transactions: &[CommittedTransaction]) {
        let mut count_by_usecase = HashMap::new();
        for transaction in transactions {
            *count_by_usecase
                .entry(transaction.use_case.clone())
                .or_insert(0) += 1;
        }

        self.add_to_recent(count_by_usecase);
    }
}

#[derive(Eq, PartialEq)]
struct UseCaseByCount {
    use_case: UseCaseKey,
    count: usize,
}

impl Ord for UseCaseByCount {
    fn cmp(&self, other: &Self) -> Ordering {
        self.count.cmp(&other.count)
    }
}

impl PartialOrd for UseCaseByCount {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

#[test]
fn test_use_case_history() {
    use velor_types::{account_address::AccountAddress, transaction::ReplayProtector};
    let ct = |use_case: UseCaseKey| -> CommittedTransaction {
        CommittedTransaction {
            sender: AccountAddress::ONE,
            replay_protector: ReplayProtector::SequenceNumber(0),
            use_case,
        }
    };

    let expected_from_top2 =
        |top_1: UseCaseKey, top_2: UseCaseKey| -> HashMap<UseCaseKey, String> {
            HashMap::from([
                (Platform, "entry_platform".into()),
                (Others, "non_entry".into()),
                (top_1, "entry_user_top_1".into()),
                (top_2, "entry_user_top_2".into()),
            ])
        };

    let usecase1 = ContractAddress(AccountAddress::random());
    let usecase2 = ContractAddress(AccountAddress::random());
    let usecase3 = ContractAddress(AccountAddress::random());

    use UseCaseKey::*;

    let mut history = UseCaseHistory::new(2, 2);

    history.update_usecases(&[ct(Platform)]);
    assert_eq!(
        history.compute_tracking_set(),
        HashMap::from([
            (Platform, "entry_platform".into()),
            (Others, "non_entry".into())
        ])
    );

    history.update_usecases(&[
        ct(Platform),
        ct(Platform),
        ct(usecase1.clone()),
        ct(usecase1.clone()),
        ct(usecase2.clone()),
    ]);
    assert_eq!(
        history.compute_tracking_set(),
        expected_from_top2(usecase1.clone(), usecase2.clone()),
    );

    history.update_usecases(&[
        ct(usecase1.clone()),
        ct(usecase2.clone()),
        ct(usecase3.clone()),
    ]);
    assert_eq!(
        history.compute_tracking_set(),
        expected_from_top2(usecase1.clone(), usecase2.clone()),
    );

    history.update_usecases(&[
        ct(usecase2.clone()),
        ct(usecase2.clone()),
        ct(usecase2.clone()),
        ct(usecase3.clone()),
    ]);
    assert_eq!(
        history.compute_tracking_set(),
        expected_from_top2(usecase2.clone(), usecase3.clone()),
    );

    history.update_usecases(&[
        ct(usecase2.clone()),
        ct(usecase2.clone()),
        ct(usecase3.clone()),
        ct(usecase3.clone()),
        ct(usecase3.clone()),
    ]);
    assert_eq!(
        history.compute_tracking_set(),
        expected_from_top2(usecase2.clone(), usecase3.clone()),
    );

    history.update_usecases(&[
        ct(ContractAddress(AccountAddress::random())),
        ct(ContractAddress(AccountAddress::random())),
        ct(ContractAddress(AccountAddress::random())),
        ct(ContractAddress(AccountAddress::random())),
        ct(ContractAddress(AccountAddress::random())),
    ]);
    assert_eq!(
        history.compute_tracking_set(),
        expected_from_top2(usecase3.clone(), usecase2.clone()),
    );
}
