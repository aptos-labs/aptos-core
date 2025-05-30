// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_crypto::{ed25519::Ed25519PublicKey, HashValue};
use aptos_types::{
    account_address::AccountAddress,
    transaction::{
        authenticator::{AccountAuthenticator, AnyPublicKey, TransactionAuthenticator},
        EntryFunction, MultisigTransactionPayload, Script, SignedTransaction, TransactionArgument,
        TransactionExecutableRef, TransactionExtraConfig, TransactionPayload,
        TransactionPayloadInner,
    },
};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum Matcher {
    All,                                           // Matches any transactions
    BlockId(HashValue), // Matches transactions in a specific block (identified by block ID)
    BlockTimeStampGreaterThan(u64), // Matches transactions in blocks with timestamps greater than the specified value
    BlockTimeStampLessThan(u64), // Matches transactions in blocks with timestamps less than the specified value
    TransactionId(HashValue),    // Matches a specific transaction by its ID
    Sender(AccountAddress),      // Matches transactions sent by a specific account address
    ModuleAddress(AccountAddress), // Matches transactions that call a module at a specific address
    EntryFunction(AccountAddress, String, String), // Matches transactions that call a specific entry function in a module
    BlockEpochGreaterThan(u64), // Matches transactions in blocks with epochs greater than the specified value
    BlockEpochLessThan(u64), // Matches transactions in blocks with epochs less than the specified value
    MatchesAllOf(Vec<Matcher>), // Matches transactions that satisfy all the provided conditions (i.e., logical AND)
    AccountAddress(AccountAddress), // Matches transactions that involve a specific account address
    PublicKey(AnyPublicKey),    // Matches transactions that involve a specific public key
}

impl Matcher {
    fn matches(
        &self,
        block_id: HashValue,
        block_epoch: u64,
        block_timestamp: u64,
        txn: &SignedTransaction,
    ) -> bool {
        match self {
            Matcher::All => true,
            Matcher::BlockId(id) => block_id == *id,
            Matcher::BlockTimeStampGreaterThan(timestamp) => block_timestamp > *timestamp,
            Matcher::BlockTimeStampLessThan(timestamp) => block_timestamp < *timestamp,
            Matcher::TransactionId(id) => txn.committed_hash() == *id,
            Matcher::Sender(sender) => matches_sender_address(txn, sender),
            Matcher::ModuleAddress(address) => matches_entry_function_module_address(txn, address),
            Matcher::EntryFunction(address, module_name, function) => {
                matches_entry_function(txn, address, module_name, function)
            },
            Matcher::BlockEpochGreaterThan(epoch) => block_epoch > *epoch,
            Matcher::BlockEpochLessThan(epoch) => block_epoch < *epoch,
            Matcher::MatchesAllOf(matchers) => matchers
                .iter()
                .all(|matcher| matcher.matches(block_id, block_epoch, block_timestamp, txn)),
            Matcher::AccountAddress(address) => {
                matches_sender_address(txn, address)
                    || matches_entry_function_module_address(txn, address)
                    || matches_multisig_address(txn, address)
                    || matches_script_argument_address(txn, address)
                    || matches_transaction_authenticator_address(txn, address)
            },
            Matcher::PublicKey(public_key) => {
                matches_transaction_authenticator_public_key(txn, public_key)
            },
        }
    }
}

/// Returns true iff the Ed25519 public key matches the given AnyPublicKey
fn compare_ed25519_public_key(
    ed25519_public_key: &Ed25519PublicKey,
    any_public_key: &AnyPublicKey,
) -> bool {
    if let AnyPublicKey::Ed25519 { public_key } = any_public_key {
        ed25519_public_key == public_key
    } else {
        false
    }
}

/// Returns true iff the entry function's module address, name, and function name
/// match the given account address, module name, and function name.
fn compare_entry_function(
    entry_function: &EntryFunction,
    address: &AccountAddress,
    module_name: &String,
    function_name: &String,
) -> bool {
    entry_function.module().address() == address
        && entry_function.module().name().to_string() == *module_name
        && entry_function.function().to_string() == *function_name
}

/// Returns true iff the entry function's module address matches the given account address
fn compare_entry_function_module_address(
    entry_function: &EntryFunction,
    address: &AccountAddress,
) -> bool {
    entry_function.module().address() == address
}

/// Returns true iff the script's arguments contain the given account address
fn compare_script_argument_address(script: &Script, address: &AccountAddress) -> bool {
    script.args().iter().any(|transaction_argument| {
        if let TransactionArgument::Address(argument_address) = transaction_argument {
            argument_address == address
        } else {
            false
        }
    })
}

/// Returns true iff the account authenticator contains the given account address
fn matches_account_authenticator_address(
    account_authenticator: &AccountAuthenticator,
    address: &AccountAddress,
) -> bool {
    // Match all variants explicitly to ensure future enum changes are caught during compilation
    match account_authenticator {
        AccountAuthenticator::Ed25519 { .. }
        | AccountAuthenticator::MultiEd25519 { .. }
        | AccountAuthenticator::NoAccountAuthenticator => false,
        AccountAuthenticator::SingleKey { authenticator } => {
            matches_any_public_key_address(authenticator.public_key(), address)
        },
        AccountAuthenticator::MultiKey { authenticator } => authenticator
            .public_keys()
            .public_keys()
            .iter()
            .any(|any_public_key| matches_any_public_key_address(any_public_key, address)),
        AccountAuthenticator::Abstraction { function_info, .. } => {
            function_info.module_address == *address
        },
    }
}

/// Returns true iff the account authenticator contains the given public key
fn matches_account_authenticator_public_key(
    account_authenticator: &AccountAuthenticator,
    any_public_key: &AnyPublicKey,
) -> bool {
    // Match all variants explicitly to ensure future enum changes are caught during compilation
    match account_authenticator {
        AccountAuthenticator::NoAccountAuthenticator | AccountAuthenticator::Abstraction { .. } => {
            false
        },
        AccountAuthenticator::Ed25519 { public_key, .. } => {
            compare_ed25519_public_key(public_key, any_public_key)
        },
        AccountAuthenticator::MultiEd25519 { public_key, .. } => {
            public_key.public_keys().iter().any(|ed25519_public_key| {
                compare_ed25519_public_key(ed25519_public_key, any_public_key)
            })
        },
        AccountAuthenticator::SingleKey { authenticator } => {
            authenticator.public_key() == any_public_key
        },
        AccountAuthenticator::MultiKey { authenticator } => authenticator
            .public_keys()
            .public_keys()
            .iter()
            .any(|key| key == any_public_key),
    }
}

/// Returns true iff the public key contains the given account address
fn matches_any_public_key_address(any_public_key: &AnyPublicKey, address: &AccountAddress) -> bool {
    // Match all variants explicitly to ensure future enum changes are caught during compilation
    match any_public_key {
        AnyPublicKey::Ed25519 { .. }
        | AnyPublicKey::Secp256k1Ecdsa { .. }
        | AnyPublicKey::Secp256r1Ecdsa { .. }
        | AnyPublicKey::Keyless { .. } => false,
        AnyPublicKey::FederatedKeyless { public_key } => {
            // Check if the public key's JWK address matches the given address
            public_key.jwk_addr == *address
        },
    }
}

/// Returns true iff the transaction's entry function matches the given account address, module name, and function name
fn matches_entry_function(
    signed_transaction: &SignedTransaction,
    address: &AccountAddress,
    module_name: &String,
    function: &String,
) -> bool {
    // Match all variants explicitly to ensure future enum changes are caught during compilation
    match signed_transaction.payload() {
        TransactionPayload::Script(_) | TransactionPayload::ModuleBundle(_) => false,
        TransactionPayload::Multisig(multisig) => multisig
            .transaction_payload
            .as_ref()
            .map(|payload| match payload {
                MultisigTransactionPayload::EntryFunction(entry_function) => {
                    compare_entry_function(entry_function, address, module_name, function)
                },
            })
            .unwrap_or(false),
        TransactionPayload::EntryFunction(entry_function) => {
            compare_entry_function(entry_function, address, module_name, function)
        },
        TransactionPayload::Payload(TransactionPayloadInner::V1 { executable, .. }) => {
            match executable.as_ref() {
                TransactionExecutableRef::Script(_) | TransactionExecutableRef::Empty => false,
                TransactionExecutableRef::EntryFunction(entry_function) => {
                    compare_entry_function(entry_function, address, module_name, function)
                },
            }
        },
    }
}

/// Returns true iff the transaction's module address matches the given account address
fn matches_entry_function_module_address(
    signed_transaction: &SignedTransaction,
    module_address: &AccountAddress,
) -> bool {
    // Match all variants explicitly to ensure future enum changes are caught during compilation
    match signed_transaction.payload() {
        TransactionPayload::Script(_) | TransactionPayload::ModuleBundle(_) => false,
        TransactionPayload::Multisig(multisig) => multisig
            .transaction_payload
            .as_ref()
            .map(|payload| match payload {
                MultisigTransactionPayload::EntryFunction(entry_function) => {
                    compare_entry_function_module_address(entry_function, module_address)
                },
            })
            .unwrap_or(false),
        TransactionPayload::EntryFunction(entry_function) => {
            compare_entry_function_module_address(entry_function, module_address)
        },
        TransactionPayload::Payload(TransactionPayloadInner::V1 { executable, .. }) => {
            match executable.as_ref() {
                TransactionExecutableRef::Script(_) | TransactionExecutableRef::Empty => false,
                TransactionExecutableRef::EntryFunction(entry_function) => {
                    compare_entry_function_module_address(entry_function, module_address)
                },
            }
        },
    }
}

/// Returns true iff the transaction's multisig address matches the given account address
fn matches_multisig_address(
    signed_transaction: &SignedTransaction,
    address: &AccountAddress,
) -> bool {
    // Match all variants explicitly to ensure future enum changes are caught during compilation
    match signed_transaction.payload() {
        TransactionPayload::EntryFunction(_)
        | TransactionPayload::Script(_)
        | TransactionPayload::ModuleBundle(_) => false,
        TransactionPayload::Multisig(multisig) => multisig.multisig_address == *address,
        TransactionPayload::Payload(TransactionPayloadInner::V1 { extra_config, .. }) => {
            match extra_config {
                TransactionExtraConfig::V1 {
                    multisig_address, ..
                } => multisig_address
                    .map(|multisig_address| multisig_address == *address)
                    .unwrap_or(false),
            }
        },
    }
}

/// Returns true iff a script argument matches the given account address
fn matches_script_argument_address(
    signed_transaction: &SignedTransaction,
    address: &AccountAddress,
) -> bool {
    // Match all variants explicitly to ensure future enum changes are caught during compilation
    match signed_transaction.payload() {
        TransactionPayload::EntryFunction(_)
        | TransactionPayload::Multisig(_)
        | TransactionPayload::ModuleBundle(_) => false,
        TransactionPayload::Script(script) => compare_script_argument_address(script, address),
        TransactionPayload::Payload(TransactionPayloadInner::V1 { executable, .. }) => {
            match executable.as_ref() {
                TransactionExecutableRef::EntryFunction(_) | TransactionExecutableRef::Empty => {
                    false
                },
                TransactionExecutableRef::Script(script) => {
                    compare_script_argument_address(script, address)
                },
            }
        },
    }
}

/// Returns true iff the transaction's sender matches the given account address
fn matches_sender_address(signed_transaction: &SignedTransaction, sender: &AccountAddress) -> bool {
    signed_transaction.sender() == *sender
}

/// Returns true iff the transaction's authenticator contains the given account address
fn matches_transaction_authenticator_address(
    signed_transaction: &SignedTransaction,
    address: &AccountAddress,
) -> bool {
    // Match all variants explicitly to ensure future enum changes are caught during compilation
    match signed_transaction.authenticator_ref() {
        TransactionAuthenticator::Ed25519 { .. }
        | TransactionAuthenticator::MultiEd25519 { .. } => false,
        TransactionAuthenticator::MultiAgent {
            sender,
            secondary_signer_addresses,
            secondary_signers,
        } => {
            matches_account_authenticator_address(sender, address)
                || secondary_signer_addresses.contains(address)
                || secondary_signers
                    .iter()
                    .any(|signer| matches_account_authenticator_address(signer, address))
        },
        TransactionAuthenticator::FeePayer {
            sender,
            secondary_signer_addresses,
            secondary_signers,
            fee_payer_address,
            fee_payer_signer,
        } => {
            matches_account_authenticator_address(sender, address)
                || secondary_signer_addresses.contains(address)
                || secondary_signers
                    .iter()
                    .any(|signer| matches_account_authenticator_address(signer, address))
                || fee_payer_address == address
                || matches_account_authenticator_address(fee_payer_signer, address)
        },
        TransactionAuthenticator::SingleSender { sender } => {
            matches_account_authenticator_address(sender, address)
        },
    }
}

/// Returns true iff the transaction's authenticator contains the given public key
fn matches_transaction_authenticator_public_key(
    signed_transaction: &SignedTransaction,
    any_public_key: &AnyPublicKey,
) -> bool {
    // Match all variants explicitly to ensure future enum changes are caught during compilation
    match signed_transaction.authenticator_ref() {
        TransactionAuthenticator::Ed25519 { public_key, .. } => {
            compare_ed25519_public_key(public_key, any_public_key)
        },
        TransactionAuthenticator::MultiEd25519 { public_key, .. } => {
            public_key.public_keys().iter().any(|ed25519_public_key| {
                compare_ed25519_public_key(ed25519_public_key, any_public_key)
            })
        },
        TransactionAuthenticator::MultiAgent {
            sender,
            secondary_signers,
            ..
        } => {
            matches_account_authenticator_public_key(sender, any_public_key)
                || secondary_signers
                    .iter()
                    .any(|signer| matches_account_authenticator_public_key(signer, any_public_key))
        },
        TransactionAuthenticator::FeePayer {
            sender,
            secondary_signers,
            fee_payer_signer,
            ..
        } => {
            matches_account_authenticator_public_key(sender, any_public_key)
                || secondary_signers
                    .iter()
                    .any(|signer| matches_account_authenticator_public_key(signer, any_public_key))
                || matches_account_authenticator_public_key(fee_payer_signer, any_public_key)
        },
        TransactionAuthenticator::SingleSender { sender } => {
            matches_account_authenticator_public_key(sender, any_public_key)
        },
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

/// A filter that can be used to allow or deny transactions from being executed. It contains a
/// set of rules that are evaluated one by one in the order of declaration. If a rule matches,
/// the transaction is either allowed or denied depending on the rule. If no rule matches,
/// the transaction is allowed.
///
/// For example, a filter might look like this:
///             rules:
///                 - Allow:
///                     Sender: f8871acf2c827d40e23b71f6ff2b9accef8dbb17709b88bd9eb95e6bb748c25a
///                 - Allow:
///                     MatchesAllOf:
///                         - Sender: 0xcd3357a925307983f7fbf1a433e87e49eda93fbb94d0d31974e68b5d60e09f3a
///                         - BlockEpochGreaterThan: 10
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
///                 - Deny: All
/// This filter allows transactions with the following properties:
/// - Sender with address f8871acf2c827d40e23b71f6ff2b9accef8dbb17709b88bd9eb95e6bb748c25a.
/// - Sender with address cd3357a925307983f7fbf1a433e87e49eda93fbb94d0d31974e68b5d60e09f3a, and
///   block epoch greater than 10.
/// - Transactions for the module with address 0000000000000000000000000000000000000000000000000000000000000001.
/// - Transactions that call the entry function test::check or test::new from the module with
///   address 0000000000000000000000000000000000000000000000000000000000000002.
/// All other transactions are denied.
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

    fn add_match_rule(mut self, allow: bool, matcher: Matcher) -> Self {
        if allow {
            self.rules.push(Rule::Allow(matcher));
        } else {
            self.rules.push(Rule::Deny(matcher));
        }
        self
    }

    pub fn add_all_filter(self, allow: bool) -> Self {
        let matcher = Matcher::All;
        self.add_match_rule(allow, matcher)
    }

    pub fn add_block_id_filter(self, allow: bool, block_id: HashValue) -> Self {
        let matcher = Matcher::BlockId(block_id);
        self.add_match_rule(allow, matcher)
    }

    pub fn add_block_timestamp_greater_than_filter(self, allow: bool, timestamp: u64) -> Self {
        let matcher = Matcher::BlockTimeStampGreaterThan(timestamp);
        self.add_match_rule(allow, matcher)
    }

    pub fn add_block_timestamp_less_than_filter(self, allow: bool, timestamp: u64) -> Self {
        let matcher = Matcher::BlockTimeStampLessThan(timestamp);
        self.add_match_rule(allow, matcher)
    }

    pub fn add_transaction_id_filter(self, allow: bool, txn_id: HashValue) -> Self {
        let matcher = Matcher::TransactionId(txn_id);
        self.add_match_rule(allow, matcher)
    }

    pub fn add_sender_filter(self, allow: bool, sender: AccountAddress) -> Self {
        let matcher = Matcher::Sender(sender);
        self.add_match_rule(allow, matcher)
    }

    pub fn add_module_address_filter(self, allow: bool, address: AccountAddress) -> Self {
        let matcher = Matcher::ModuleAddress(address);
        self.add_match_rule(allow, matcher)
    }

    pub fn add_entry_function_filter(
        self,
        allow: bool,
        address: AccountAddress,
        module_name: String,
        function: String,
    ) -> Self {
        let matcher = Matcher::EntryFunction(address, module_name, function);
        self.add_match_rule(allow, matcher)
    }

    pub fn add_block_epoch_greater_than_filter(self, allow: bool, epoch: u64) -> Self {
        let matcher = Matcher::BlockEpochGreaterThan(epoch);
        self.add_match_rule(allow, matcher)
    }

    pub fn add_block_epoch_less_than_filter(self, allow: bool, epoch: u64) -> Self {
        let matcher = Matcher::BlockEpochLessThan(epoch);
        self.add_match_rule(allow, matcher)
    }

    pub fn add_matches_all_of_filter(self, allow: bool, matchers: Vec<Matcher>) -> Self {
        let matcher = Matcher::MatchesAllOf(matchers);
        self.add_match_rule(allow, matcher)
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
