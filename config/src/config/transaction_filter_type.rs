// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_crypto::HashValue;
use aptos_types::{
    account_address::AccountAddress,
    transaction::{SignedTransaction, TransactionExecutableRef},
};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum Matcher {
    AllOf(Vec<Matcher>), // Matches against all the provided matchers (i.e., conjunction of all conditions)
    Any,                 // Matches any transaction
    BlockId(HashValue),  // Matches transactions in a specific block (identified by the block ID)
    BlockTimeStampGreaterThan(u64), // Matches transactions in blocks with a timestamp greater than the specified value
    BlockTimeStampLessThan(u64), // Matches transactions in blocks with a timestamp less than the specified value
    EpochGreaterThan(u64), // Matches transactions in blocks with an epoch greater than the specified value
    EpochLessThan(u64), // Matches transactions in blocks with an epoch less than the specified value
    TransactionId(HashValue), // Matches a specific transaction ID
    Sender(AccountAddress), // Matches transactions sent by a specific account address
    ModuleAddress(AccountAddress), // Matches transactions from a specific module address
    EntryFunction(AccountAddress, String, String), // Matches transactions that call a specific entry function in a module
}

impl Matcher {
    fn new_all_of(matchers: Vec<Matcher>) -> Self {
        Matcher::AllOf(matchers)
    }

    fn new_any() -> Self {
        Matcher::Any
    }

    fn new_block_id(id: HashValue) -> Self {
        Matcher::BlockId(id)
    }

    fn new_block_timestamp_greater_than(timestamp: u64) -> Self {
        Matcher::BlockTimeStampGreaterThan(timestamp)
    }

    fn new_block_timestamp_less_than(timestamp: u64) -> Self {
        Matcher::BlockTimeStampLessThan(timestamp)
    }

    fn new_epoch_greater_than(epoch: u64) -> Self {
        Matcher::EpochGreaterThan(epoch)
    }

    fn new_epoch_less_than(epoch: u64) -> Self {
        Matcher::EpochLessThan(epoch)
    }

    fn new_transaction_id(id: HashValue) -> Self {
        Matcher::TransactionId(id)
    }

    fn new_sender(sender: AccountAddress) -> Self {
        Matcher::Sender(sender)
    }

    fn new_module_address(address: AccountAddress) -> Self {
        Matcher::ModuleAddress(address)
    }

    fn new_entry_function(address: AccountAddress, module_name: String, function: String) -> Self {
        Matcher::EntryFunction(address, module_name, function)
    }

    fn matches(
        &self,
        block_id: HashValue,
        block_epoch: u64,
        block_timestamp: u64,
        txn: &SignedTransaction,
    ) -> bool {
        match self {
            Matcher::AllOf(matchers) => {
                // All conditions must match for this to be true
                matchers
                    .iter()
                    .all(|matcher| matcher.matches(block_id, block_epoch, block_timestamp, txn))
            },
            Matcher::Any => true,
            Matcher::BlockId(id) => block_id == *id,
            Matcher::BlockTimeStampGreaterThan(timestamp) => block_timestamp > *timestamp,
            Matcher::BlockTimeStampLessThan(timestamp) => block_timestamp < *timestamp,
            Matcher::EpochGreaterThan(epoch) => block_epoch > *epoch,
            Matcher::EpochLessThan(epoch) => block_epoch < *epoch,
            Matcher::TransactionId(id) => txn.committed_hash() == *id,
            Matcher::Sender(sender) => txn.sender() == *sender,
            Matcher::ModuleAddress(address) => match txn.payload().executable_ref() {
                Ok(TransactionExecutableRef::EntryFunction(entry_function))
                    if !txn.payload().is_multisig() =>
                {
                    *entry_function.module().address() == *address
                },
                _ => false,
            },
            Matcher::EntryFunction(address, module_name, function) => {
                match txn.payload().executable_ref() {
                    Ok(TransactionExecutableRef::EntryFunction(entry_function))
                        if !txn.payload().is_multisig() =>
                    {
                        *entry_function.module().address() == *address
                            && entry_function.module().name().to_string() == *module_name
                            && entry_function.function().to_string() == *function
                    },
                    _ => false,
                }
            },
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum Rule {
    Allow(Matcher),
    Deny(Matcher),
}

impl Rule {
    pub fn matcher(&self) -> &Matcher {
        match self {
            Rule::Allow(matcher) => matcher,
            Rule::Deny(matcher) => matcher,
        }
    }
}

enum EvalResult {
    Allow,
    Deny,
    NoMatch,
}

impl Rule {
    fn eval(
        &self,
        block_id: HashValue,
        block_epoch: u64,
        block_timestamp: u64,
        txn: &SignedTransaction,
    ) -> EvalResult {
        match self {
            Rule::Allow(matcher) => {
                if matcher.matches(block_id, block_epoch, block_timestamp, txn) {
                    EvalResult::Allow
                } else {
                    EvalResult::NoMatch
                }
            },
            Rule::Deny(matcher) => {
                if matcher.matches(block_id, block_epoch, block_timestamp, txn) {
                    EvalResult::Deny
                } else {
                    EvalResult::NoMatch
                }
            },
        }
    }
}

/// A filter that can be used to allow or deny transactions from being executed. It contains a set
/// of rules that are evaluated one by one in the order of declaration.
/// If a rule matches, the transaction is either allowed or
/// denied depending on the rule. If no rule matches, the transaction is allowed.
/// For example, rules might look like this:
///             rules:
///                 - Allow:
///                     Sender: f8871acf2c827d40e23b71f6ff2b9accef8dbb17709b88bd9eb95e6bb748c25a
///                 - Allow:
///                     - Sender: b6c09a85c2edd80bd76ef7e071dc52f1e04f3d23a3f486e223e3a12b5c1a07f1
///                     - BlockTimeStampGreaterThan: 1000
///                 - Allow:
///                     ModuleAddress: "0000000000000000000000000000000000000000000000000000000000000001"
///                 - Allow:
///                     EntryFunction:
///                         - "0000000000000000000000000000000000000000000000000000000000000002"
///                         - test
///                         - check
///                 - Allow:
///                     EntryFunction:
///                         - "0000000000000000000000000000000000000000000000000000000000000002"
///                         - test
///                         - new
///                 - Deny: Any
/// This filter allows the following transactions:
/// - Transactions from the sender with address f8871acf2c827d40e23b71f6ff2b9accef8dbb17709b88bd9eb95e6bb748c25a.
/// - Transactions from the sender with address b6c09a85c2edd80bd76ef7e071dc52f1e04f3d23a3f486e223e3a12b5c1a07f1
///   and with a block timestamp greater than 1000.
/// - Transactions from the module with address 0000000000000000000000000000000000000000000000000000000000000001.
/// - Transactions that call the entry function test::check or test::new from the module
///   with address 0000000000000000000000000000000000000000000000000000000000000002.
/// Any other transactions are denied.
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct Filter {
    rules: Vec<Rule>,
}

impl Filter {
    pub fn empty() -> Self {
        Self { rules: vec![] }
    }

    pub fn is_empty(&self) -> bool {
        self.rules.is_empty()
    }

    fn add_matcher_rule(&mut self, allow: bool, matcher: Matcher) {
        if allow {
            self.rules.push(Rule::Allow(matcher));
        } else {
            self.rules.push(Rule::Deny(matcher));
        }
    }

    pub fn add_all_of_filter(mut self, allow: bool, matchers: Vec<Matcher>) -> Self {
        let matcher = Matcher::new_all_of(matchers);
        self.add_matcher_rule(allow, matcher);
        self
    }

    pub fn add_any_filter(mut self, allow: bool) -> Self {
        let matcher = Matcher::new_any();
        self.add_matcher_rule(allow, matcher);
        self
    }

    pub fn add_block_id_filter(mut self, allow: bool, block_id: HashValue) -> Self {
        let matcher = Matcher::new_block_id(block_id);
        self.add_matcher_rule(allow, matcher);
        self
    }

    pub fn add_block_timestamp_greater_than_filter(mut self, allow: bool, timestamp: u64) -> Self {
        let matcher = Matcher::new_block_timestamp_greater_than(timestamp);
        self.add_matcher_rule(allow, matcher);
        self
    }

    pub fn add_block_timestamp_less_than_filter(mut self, allow: bool, timestamp: u64) -> Self {
        let matcher = Matcher::new_block_timestamp_less_than(timestamp);
        self.add_matcher_rule(allow, matcher);
        self
    }

    pub fn add_epoch_greater_than_filter(mut self, allow: bool, epoch: u64) -> Self {
        let matcher = Matcher::new_epoch_greater_than(epoch);
        self.add_matcher_rule(allow, matcher);
        self
    }

    pub fn add_epoch_less_than_filter(mut self, allow: bool, epoch: u64) -> Self {
        let matcher = Matcher::new_epoch_less_than(epoch);
        self.add_matcher_rule(allow, matcher);
        self
    }

    pub fn add_transaction_id_filter(mut self, allow: bool, txn_id: HashValue) -> Self {
        let matcher = Matcher::new_transaction_id(txn_id);
        self.add_matcher_rule(allow, matcher);
        self
    }

    pub fn add_sender_filter(mut self, allow: bool, sender: AccountAddress) -> Self {
        let matcher = Matcher::new_sender(sender);
        self.add_matcher_rule(allow, matcher);
        self
    }

    pub fn add_module_address_filter(mut self, allow: bool, address: AccountAddress) -> Self {
        let matcher = Matcher::new_module_address(address);
        self.add_matcher_rule(allow, matcher);
        self
    }

    pub fn add_entry_function_filter(
        mut self,
        allow: bool,
        address: AccountAddress,
        module_name: String,
        function: String,
    ) -> Self {
        let matcher = Matcher::new_entry_function(address, module_name, function);
        self.add_matcher_rule(allow, matcher);
        self
    }

    pub fn rules(&self) -> &[Rule] {
        &self.rules
    }

    pub fn allows(
        &self,
        block_id: HashValue,
        block_epoch: u64,
        block_timestamp: u64,
        txn: &SignedTransaction,
    ) -> bool {
        for rule in &self.rules {
            // Rules are evaluated in the order and the first rule that matches is used. If no rule
            // matches, the transaction is allowed.
            match rule.eval(block_id, block_epoch, block_timestamp, txn) {
                EvalResult::Allow => return true,
                EvalResult::Deny => return false,
                EvalResult::NoMatch => continue,
            }
        }
        true
    }
}
