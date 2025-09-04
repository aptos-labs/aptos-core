// Copyright (c) Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::transaction_shuffler::use_case_aware::{
    types::{InputIdx, OutputIdx},
    utils::StrictMap,
    Config,
};
use velor_types::transaction::use_case::{UseCaseAwareTransaction, UseCaseKey};
use move_core_types::account_address::AccountAddress;
use std::{
    collections::{hash_map, BTreeMap, HashMap, VecDeque},
    fmt::Debug,
};

/// Key used in priority queues.
/// Part of the key is a txn's input index which guarantees in any priority queue, of use cases or
/// accounts, there are not two entries that share the same delay key. Also, when `try_delay_till`
/// is identical, an entry relating to a earlier txn is prioritized.
#[derive(Clone, Copy, Eq, Ord, PartialEq, PartialOrd)]
struct DelayKey {
    try_delay_till: OutputIdx,
    input_idx: InputIdx,
}

impl DelayKey {
    fn new(try_delay_till: OutputIdx, input_idx: InputIdx) -> Self {
        Self {
            try_delay_till,
            input_idx,
        }
    }
}

impl Debug for DelayKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "DelayKey({}, {})", self.try_delay_till, self.input_idx)
    }
}

struct TxnWithInputIdx<Txn> {
    input_idx: InputIdx,
    txn: Txn,
}

impl<Txn> Debug for TxnWithInputIdx<Txn>
where
    Txn: Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Txn({}: {:?})", self.input_idx, self.txn)
    }
}

#[derive(Debug)]
struct Account<Txn> {
    try_delay_till: OutputIdx,
    /// Head txn input_idx, tracked for use when the txns queue is empty, in which case
    /// it keeps the value before the last txn was dequeued.
    input_idx: InputIdx,
    txns: VecDeque<TxnWithInputIdx<Txn>>,
}

impl<Txn> Account<Txn>
where
    Txn: UseCaseAwareTransaction,
{
    fn new_with_txn(try_delay_till: OutputIdx, input_idx: InputIdx, txn: Txn) -> Self {
        let txns = vec![TxnWithInputIdx { input_idx, txn }].into();
        Self {
            try_delay_till,
            input_idx,
            txns,
        }
    }

    fn new_empty(try_delay_till: OutputIdx, input_idx: InputIdx) -> Self {
        Self {
            try_delay_till,
            input_idx,
            txns: VecDeque::new(),
        }
    }

    fn is_empty(&self) -> bool {
        self.txns.is_empty()
    }

    fn delay_key(&self) -> DelayKey {
        DelayKey {
            try_delay_till: self.try_delay_till,
            input_idx: self.input_idx,
        }
    }

    fn expect_first_txn(&self) -> &TxnWithInputIdx<Txn> {
        self.txns.front().expect("Must exist.")
    }

    fn expect_use_case_key(&self) -> UseCaseKey {
        self.expect_first_txn().txn.parse_use_case()
    }

    fn queue_txn(&mut self, input_idx: InputIdx, txn: Txn) {
        if let Some(last_txn) = self.txns.back() {
            assert!(last_txn.input_idx < input_idx);
        } else {
            self.input_idx = input_idx;
        }
        self.txns.push_back(TxnWithInputIdx { input_idx, txn });
    }

    fn expect_dequeue_txn(&mut self) -> TxnWithInputIdx<Txn> {
        let txn = self.txns.pop_front().expect("Must exist.");
        if let Some(next_txn) = self.txns.front() {
            self.input_idx = next_txn.input_idx;
        }
        txn
    }

    fn update_try_delay_till(&mut self, try_delay_till: OutputIdx) {
        self.try_delay_till = try_delay_till;
    }
}

#[derive(Debug)]
struct UseCase {
    try_delay_till: OutputIdx,
    /// Head account input_idx, tracked for use when the accounts queue is empty, in which case
    /// it keeps the value before the last account was removed.
    input_idx: InputIdx,
    account_by_delay: BTreeMap<DelayKey, AccountAddress>,
}

impl UseCase {
    fn new_empty(try_delay_till: OutputIdx, input_idx: InputIdx) -> Self {
        Self {
            try_delay_till,
            input_idx,
            account_by_delay: BTreeMap::new(),
        }
    }

    fn new_with_account<Txn>(
        try_delay_till: OutputIdx,
        address: AccountAddress,
        account: &Account<Txn>,
    ) -> Self
    where
        Txn: UseCaseAwareTransaction,
    {
        let mut account_by_delay = BTreeMap::new();
        account_by_delay.strict_insert(account.delay_key(), address);
        Self {
            try_delay_till,
            input_idx: account.input_idx,
            account_by_delay,
        }
    }

    fn is_empty(&self) -> bool {
        self.account_by_delay.is_empty()
    }

    fn delay_key(&self) -> DelayKey {
        // If head account will be ready later than the use case itself, respect that.
        let try_delay_till = std::cmp::max(
            self.try_delay_till,
            self.account_by_delay
                .first_key_value()
                .map_or(0, |(k, _)| k.try_delay_till),
        );

        DelayKey {
            try_delay_till,
            input_idx: self.input_idx,
        }
    }

    /// Expects head account to exist (otherwise panic) and return both the DelayKey and the
    /// account address for the entry.
    fn expect_pop_head_account(&mut self) -> (DelayKey, AccountAddress) {
        let (account_delay_key, address) = self.account_by_delay.pop_first().expect("Must exist.");
        if let Some((next_account_delay_key, _)) = self.account_by_delay.first_key_value() {
            self.input_idx = next_account_delay_key.input_idx;
        }
        (account_delay_key, address)
    }

    fn update_try_delay_till(&mut self, try_delay_till: OutputIdx) {
        self.try_delay_till = try_delay_till;
    }

    fn add_account<Txn>(&mut self, address: AccountAddress, account: &Account<Txn>)
    where
        Txn: UseCaseAwareTransaction,
    {
        let account_delay_key = account.delay_key();
        self.account_by_delay
            .strict_insert(account_delay_key, address);
        let (_, head_address) = self
            .account_by_delay
            .first_key_value()
            .expect("Must exist.");
        if head_address == &address {
            self.input_idx = account_delay_key.input_idx;
        }
    }
}

/// Structure to track:
///     1. all use cases and accounts that are subject to delaying, no matter they have pending txns
/// associated or not.
///     2. all txns that are examined and delayed previously.
///
/// Note:
///     A delayed txn is attached to an account and the account is attached to a priority queue in a use
/// case, which has an entry in the main priority queue.
///     Empty accounts and use cases are still tracked for the delay so that a next txn in the
/// input stream is properly delayed if associated with such an account or use case.
#[derive(Debug, Default)]
pub(crate) struct DelayedQueue<Txn> {
    /// Registry of all accounts, each of which includes the expected output_idx to delay until and
    /// a queue (might be empty) of txns by that sender.
    ///
    /// An empty account address is tracked in `account_placeholders_by_delay` while a non-empty
    /// account address is tracked under `use_cases`.
    accounts: HashMap<AccountAddress, Account<Txn>>,
    /// Registry of all use cases, each of which includes the expected output_idx to delay until and
    /// a priority queue (might be empty) of non-empty accounts whose head txn belongs to that use case.
    ///
    /// An empty use case is tracked in `use_case_placeholders_by_delay` while a non-empty use case
    /// is tracked in the top level `use_cases_by_delay`.
    use_cases: HashMap<UseCaseKey, UseCase>,

    /// Main delay queue of txns. All use cases are non-empty of non-empty accounts.
    /// All pending txns are reachable from this nested structure.
    ///
    /// The DelayKey is derived from the head account's DelayKey combined with the use case's own
    /// DelayKey.
    ///
    /// The head txn of the head account of the head use case in this nested structure is the
    /// next txn to be possibly ready.
    use_cases_by_delay: BTreeMap<DelayKey, UseCaseKey>,
    /// Empty account addresses by the DelayKey (those w/o known delayed txns), kept to track the delay.
    account_placeholders_by_delay: BTreeMap<DelayKey, AccountAddress>,
    /// Empty UseCaseKeys by the DelayKey (those w/o known delayed txns), kept to track the delay.
    use_case_placeholders_by_delay: BTreeMap<DelayKey, UseCaseKey>,

    /// Externally set output index; when an item has try_delay_till <= output_idx, it's deemed ready
    output_idx: OutputIdx,

    config: Config,
}

impl<Txn> DelayedQueue<Txn>
where
    Txn: UseCaseAwareTransaction,
{
    pub fn new(config: Config) -> Self {
        Self {
            accounts: HashMap::new(),
            use_cases: HashMap::new(),

            account_placeholders_by_delay: BTreeMap::new(),
            use_case_placeholders_by_delay: BTreeMap::new(),

            use_cases_by_delay: BTreeMap::new(),

            output_idx: 0,

            config,
        }
    }

    /// Remove stale (empty use cases and accounts with try_delay_till <= self.output_idx) placeholders.
    fn drain_placeholders(&mut self) {
        let least_to_keep = DelayKey::new(self.output_idx + 1, 0);

        let remaining_use_case_placeholders = self
            .use_case_placeholders_by_delay
            .split_off(&least_to_keep);
        let remaining_account_placeholders =
            self.account_placeholders_by_delay.split_off(&least_to_keep);

        self.use_case_placeholders_by_delay
            .iter()
            .for_each(|(_delay_key, use_case_key)| self.use_cases.strict_remove(use_case_key));
        self.account_placeholders_by_delay
            .iter()
            .for_each(|(_delay_key, address)| self.accounts.strict_remove(address));

        self.use_case_placeholders_by_delay = remaining_use_case_placeholders;
        self.account_placeholders_by_delay = remaining_account_placeholders;
    }

    pub fn bump_output_idx(&mut self, output_idx: OutputIdx) {
        assert!(output_idx >= self.output_idx);
        // It's possible that the queue returned nothing last round hence the output idx didn't move.
        if output_idx > self.output_idx {
            self.output_idx = output_idx;
            self.drain_placeholders();
        }
    }

    pub fn pop_head(&mut self, only_if_ready: bool) -> Option<Txn> {
        // See if any delayed txn exists. If not, return None.
        let use_case_entry = match self.use_cases_by_delay.first_entry() {
            None => {
                return None;
            },
            Some(occupied_entry) => occupied_entry,
        };
        let use_case_delay_key = use_case_entry.key();

        // Check readiness.
        if only_if_ready && use_case_delay_key.try_delay_till > self.output_idx {
            return None;
        }

        // Gonna return the front txn of the front account of the front use case.

        // First, both the use case and account need to be removed from the priority queues.
        let use_case_delay_key = *use_case_delay_key;
        let use_case_key = use_case_entry.remove();
        let use_case = self.use_cases.expect_mut(&use_case_key);
        let (account_delay_key, address) = use_case.expect_pop_head_account();
        assert!(account_delay_key.try_delay_till <= use_case_delay_key.try_delay_till);
        assert_eq!(account_delay_key.input_idx, use_case_delay_key.input_idx);

        // Pop first txn from account (for returning it later).
        let account = self.accounts.expect_mut(&address);
        let txn = account.expect_dequeue_txn();

        // Update priorities.
        account.update_try_delay_till(self.output_idx + 1 + self.config.sender_spread_factor());
        use_case.update_try_delay_till(
            self.output_idx + 1 + self.config.use_case_spread_factor(&use_case_key),
        );

        // Add account and original use case back to delay queues.

        if account.is_empty() {
            self.account_placeholders_by_delay
                .strict_insert(account.delay_key(), address);
            if use_case.is_empty() {
                self.use_case_placeholders_by_delay
                    .strict_insert(use_case.delay_key(), use_case_key.clone());
            } else {
                self.use_cases_by_delay
                    .strict_insert(use_case.delay_key(), use_case_key.clone());
            }
        } else {
            // See if account now belongs to a different use case.
            let new_use_case_key = account.expect_use_case_key();
            if new_use_case_key == use_case_key {
                use_case.add_account(address, account);
                self.use_cases_by_delay
                    .strict_insert(use_case.delay_key(), use_case_key.clone());
            } else {
                // Account now belongs to a different use case.

                // Add original use case back to delay queue.
                if use_case.is_empty() {
                    self.use_case_placeholders_by_delay
                        .strict_insert(use_case.delay_key(), use_case_key.clone());
                } else {
                    self.use_cases_by_delay
                        .strict_insert(use_case.delay_key(), use_case_key.clone());
                }

                // Add the account to the new use case.
                match self.use_cases.entry(new_use_case_key.clone()) {
                    hash_map::Entry::Occupied(mut occupied_entry) => {
                        // Existing use case, remove from priority queues.
                        let new_use_case = occupied_entry.get_mut();
                        if new_use_case.is_empty() {
                            self.use_case_placeholders_by_delay
                                .strict_remove(&new_use_case.delay_key());
                        } else {
                            self.use_cases_by_delay
                                .strict_remove(&new_use_case.delay_key());
                        }
                        // Add account to use case.
                        new_use_case.add_account(address, account);
                        // Add new use case back to delay queue.
                        self.use_cases_by_delay
                            .strict_insert(new_use_case.delay_key(), new_use_case_key.clone());
                    },
                    hash_map::Entry::Vacant(entry) => {
                        // Use case not tracked previously, try_delay_till = output_idx + 1
                        let new_use_case = entry.insert(UseCase::new_with_account(
                            self.output_idx + 1,
                            address,
                            account,
                        ));
                        self.use_cases_by_delay
                            .strict_insert(new_use_case.delay_key(), new_use_case_key.clone());
                    },
                }
            }
        }

        Some(txn.txn)
    }

    /// Txn has to be delayed, attach it to respective account and use case.
    fn queue_txn(
        &mut self,
        input_idx: InputIdx,
        address: AccountAddress,
        use_case_key: UseCaseKey,
        txn: Txn,
    ) {
        match self.accounts.get_mut(&address) {
            Some(account) => {
                if account.is_empty() {
                    // Account placeholder exists, move it from the placeholder queue to the main queue.
                    self.account_placeholders_by_delay
                        .remove(&account.delay_key());
                    account.queue_txn(input_idx, txn);
                    match self.use_cases.entry(use_case_key.clone()) {
                        hash_map::Entry::Occupied(occupied) => {
                            let use_case = occupied.into_mut();
                            if use_case.is_empty() {
                                self.use_case_placeholders_by_delay
                                    .strict_remove(&use_case.delay_key());
                            } else {
                                self.use_cases_by_delay.strict_remove(&use_case.delay_key());
                            }
                            use_case.add_account(address, account);
                            self.use_cases_by_delay
                                .strict_insert(use_case.delay_key(), use_case_key.clone());
                        },
                        hash_map::Entry::Vacant(vacant) => {
                            // Use case not tracked previously, the use case is ready at the current
                            // output_idx, instead of output_idx +1 -- it makes a difference if
                            // a txn later in the input queue that's of the same use case but not
                            // blocked by account delay is tested for readiness.
                            let use_case =
                                UseCase::new_with_account(self.output_idx, address, account);
                            self.use_cases_by_delay
                                .strict_insert(use_case.delay_key(), use_case_key.clone());
                            vacant.insert(use_case);
                        },
                    }
                } else {
                    // Account tracked and not empty, so appending a new txn to it won't affect positions
                    // in delay queues
                    account.queue_txn(input_idx, txn);
                }
            },
            None => {
                // Account not previously tracked.
                let account = Account::new_with_txn(self.output_idx + 1, input_idx, txn);
                // Account didn't exist before, so use case must have been tracked, otherwise the
                // txn whould've been selected for output, bypassing the queue.
                let use_case = self.use_cases.expect_mut(&use_case_key);
                if use_case.is_empty() {
                    self.use_case_placeholders_by_delay
                        .strict_remove(&use_case.delay_key());
                } else {
                    self.use_cases_by_delay.strict_remove(&use_case.delay_key());
                }
                use_case.add_account(address, &account);

                self.accounts.strict_insert(address, account);
                self.use_cases_by_delay
                    .strict_insert(use_case.delay_key(), use_case_key.clone());
            },
        }
    }

    /// Txn from input queue directly selected for output, needs to bump delays for relevant
    /// account and use case.
    fn update_delays_for_selected_txn(
        &mut self,
        input_idx: InputIdx,
        address: AccountAddress,
        use_case_key: UseCaseKey,
    ) {
        let account_try_delay_till = self.output_idx + 1 + self.config.sender_spread_factor();
        let use_case_try_delay_till =
            self.output_idx + 1 + self.config.use_case_spread_factor(&use_case_key);

        match self.use_cases.entry(use_case_key.clone()) {
            hash_map::Entry::Occupied(occupied) => {
                let use_case = occupied.into_mut();
                // Txn wouldn't have been selected for output if the use case is empty (tracking
                // for a try_delay_till > self.output_idx)
                assert!(!use_case.is_empty());

                self.use_cases_by_delay.strict_remove(&use_case.delay_key());
                use_case.update_try_delay_till(use_case_try_delay_till);
                self.use_cases_by_delay
                    .strict_insert(use_case.delay_key(), use_case_key);
            },
            hash_map::Entry::Vacant(vacant) => {
                let use_case = UseCase::new_empty(use_case_try_delay_till, input_idx);
                self.use_case_placeholders_by_delay
                    .strict_insert(use_case.delay_key(), use_case_key);
                vacant.insert(use_case);
            },
        }

        // Notice this function is called after the txn is selected for output due to no delaying
        // needed, so the account must not have been tracked before, otherwise it wouldn't have been
        // selected for output.
        let new_account = Account::new_empty(account_try_delay_till, input_idx);
        let new_account_delay_key = new_account.delay_key();
        self.accounts.strict_insert(address, new_account);
        self.account_placeholders_by_delay
            .strict_insert(new_account_delay_key, address);
    }

    /// Return the txn back if relevant use case and sender are not subject to delaying. Otherwise,
    /// Queue it up.
    pub fn queue_or_return(&mut self, input_idx: InputIdx, txn: Txn) -> Option<Txn> {
        let address = txn.parse_sender();
        let account_opt = self.accounts.get_mut(&address);
        let use_case_key = txn.parse_use_case();
        let use_case_opt = self.use_cases.get_mut(&use_case_key);

        let account_should_delay = account_opt.as_ref().is_some_and(|account| {
            !account.is_empty()  // needs delaying due to queued txns under the same account
                    || account.try_delay_till > self.output_idx
        });
        let use_case_should_delay = use_case_opt
            .as_ref()
            .is_some_and(|use_case| use_case.try_delay_till > self.output_idx);

        if account_should_delay || use_case_should_delay {
            self.queue_txn(input_idx, address, use_case_key, txn);
            None
        } else {
            self.update_delays_for_selected_txn(input_idx, address, use_case_key);
            Some(txn)
        }
    }
}
