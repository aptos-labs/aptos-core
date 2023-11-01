// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use aptos_crypto::HashValue;
use aptos_types::{
    account_address::AccountAddress,
    transaction::{SignedTransaction, TransactionPayload},
};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
enum Matcher {
    All,
    BlockId(HashValue),
    TransactionId(HashValue),
    Sender(AccountAddress),
    ModuleAddress(AccountAddress),
    EntryFunction(AccountAddress, String, String),
}

impl Matcher {
    fn matches(&self, block_id: HashValue, txn: &SignedTransaction) -> bool {
        match self {
            Matcher::All => true,
            Matcher::BlockId(id) => block_id == *id,
            Matcher::TransactionId(id) => txn.clone().committed_hash() == *id,
            Matcher::Sender(sender) => txn.sender() == *sender,
            Matcher::ModuleAddress(address) => match txn.payload() {
                TransactionPayload::EntryFunction(entry_function) => {
                    *entry_function.module().address() == *address
                },
                _ => false,
            },
            Matcher::EntryFunction(address, module_name, function) => match txn.payload() {
                TransactionPayload::EntryFunction(entry_function) => {
                    *entry_function.module().address() == *address
                        && entry_function.module().name().to_string() == *module_name
                        && entry_function.function().to_string() == *function
                },
                _ => false,
            },
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
enum Rule {
    Allow(Matcher),
    Deny(Matcher),
}

enum MatchResult {
    Allow,
    Deny,
    NoMatch,
}

impl Rule {
    fn matches(&self, block_id: HashValue, txn: &SignedTransaction) -> MatchResult {
        match self {
            Rule::Allow(matcher) => {
                if matcher.matches(block_id, txn) {
                    MatchResult::Allow
                } else {
                    MatchResult::NoMatch
                }
            },
            Rule::Deny(matcher) => {
                if matcher.matches(block_id, txn) {
                    MatchResult::Deny
                } else {
                    MatchResult::NoMatch
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

    pub fn matches(&self, block_id: HashValue, txn: &SignedTransaction) -> bool {
        for rule in &self.rules {
            match rule.matches(block_id, txn) {
                MatchResult::Allow => return true,
                MatchResult::Deny => return false,
                MatchResult::NoMatch => continue,
            }
        }
        true
    }
}
