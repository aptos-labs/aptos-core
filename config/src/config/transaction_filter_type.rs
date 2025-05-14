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
    All,
    BlockId(HashValue),
    BlockTimeStampGreaterThan(u64),
    BlockTimeStampLessThan(u64),
    TransactionId(HashValue),
    Sender(AccountAddress),
    ModuleAddress(AccountAddress),
    EntryFunction(AccountAddress, String, String),
}

impl Matcher {
    fn matches(&self, block_id: HashValue, timestamp: u64, txn: &SignedTransaction) -> bool {
        match self {
            Matcher::All => true,
            Matcher::BlockId(id) => block_id == *id,
            Matcher::BlockTimeStampGreaterThan(ts) => timestamp > *ts,
            Matcher::BlockTimeStampLessThan(ts) => timestamp < *ts,
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
    fn eval(&self, block_id: HashValue, timestamp: u64, txn: &SignedTransaction) -> EvalResult {
        match self {
            Rule::Allow(matcher) => {
                if matcher.matches(block_id, timestamp, txn) {
                    EvalResult::Allow
                } else {
                    EvalResult::NoMatch
                }
            },
            Rule::Deny(matcher) => {
                if matcher.matches(block_id, timestamp, txn) {
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
/// For example a rules might look like this:
///             rules:
///                 - Allow:
///                     Sender: f8871acf2c827d40e23b71f6ff2b9accef8dbb17709b88bd9eb95e6bb748c25a
///                 - Allow:
///                     ModuleAddress: "0000000000000000000000000000000000000000000000000000000000000001"
///                 - Allow:
///                     EntryFunction:
///                         - "0000000000000000000000000000000000000000000000000000000000000001"
///                         - test
///                         - check
///                 - Allow:
///                     EntryFunction:
///                         - "0000000000000000000000000000000000000000000000000000000000000001"
///                         - test
///                         - new
///                 - Deny: All
/// This filter allows transactions from the sender with address f8871acf2c827d40e23b71f6ff2b9accef8dbb17709b88bd9eb95e6bb748c25a or
/// from the module with address 0000000000000000000000000000000000000000000000000000000000000001 or entry functions
/// test::check and test::new from the module 0000000000000000000000000000000000000000000000000000000000000001. All other transactions are denied.
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

    pub fn add_deny_all(mut self) -> Self {
        self.rules.push(Rule::Deny(Matcher::All));
        self
    }

    pub fn add_deny_block_id(mut self, block_id: HashValue) -> Self {
        self.rules.push(Rule::Deny(Matcher::BlockId(block_id)));
        self
    }

    pub fn add_allow_block_timestamp_greater_than(mut self, timestamp: u64) -> Self {
        self.rules
            .push(Rule::Allow(Matcher::BlockTimeStampGreaterThan(timestamp)));
        self
    }

    pub fn add_deny_transaction_id(mut self, txn_id: HashValue) -> Self {
        self.rules.push(Rule::Deny(Matcher::TransactionId(txn_id)));
        self
    }

    pub fn add_allow_sender(mut self, sender: AccountAddress) -> Self {
        self.rules.push(Rule::Allow(Matcher::Sender(sender)));
        self
    }

    pub fn add_deny_sender(mut self, sender: AccountAddress) -> Self {
        self.rules.push(Rule::Deny(Matcher::Sender(sender)));
        self
    }

    pub fn add_allow_module_address(mut self, address: AccountAddress) -> Self {
        self.rules
            .push(Rule::Allow(Matcher::ModuleAddress(address)));
        self
    }

    pub fn add_deny_module_address(mut self, address: AccountAddress) -> Self {
        self.rules.push(Rule::Deny(Matcher::ModuleAddress(address)));
        self
    }

    pub fn add_deny_entry_function(
        mut self,
        address: AccountAddress,
        module_name: String,
        function: String,
    ) -> Self {
        self.rules.push(Rule::Deny(Matcher::EntryFunction(
            address,
            module_name,
            function,
        )));
        self
    }

    pub fn add_allow_entry_function(
        mut self,
        address: AccountAddress,
        module_name: String,
        function: String,
    ) -> Self {
        self.rules.push(Rule::Allow(Matcher::EntryFunction(
            address,
            module_name,
            function,
        )));
        self
    }

    pub fn rules(&self) -> &[Rule] {
        &self.rules
    }

    pub fn allows(&self, block_id: HashValue, timestamp: u64, txn: &SignedTransaction) -> bool {
        for rule in &self.rules {
            // Rules are evaluated in the order and the first rule that matches is used. If no rule
            // matches, the transaction is allowed.
            match rule.eval(block_id, timestamp, txn) {
                EvalResult::Allow => return true,
                EvalResult::Deny => return false,
                EvalResult::NoMatch => continue,
            }
        }
        true
    }
}
