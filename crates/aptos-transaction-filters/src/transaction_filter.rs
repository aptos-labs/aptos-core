// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_crypto::{ed25519::Ed25519PublicKey, HashValue};
use aptos_types::transaction::{
    authenticator::{AccountAuthenticator, AnyPublicKey, TransactionAuthenticator},
    EntryFunction, MultisigTransactionPayload, Script, SignedTransaction, TransactionExecutableRef,
    TransactionExtraConfig, TransactionPayload, TransactionPayloadInner,
};
use move_core_types::{account_address::AccountAddress, transaction_argument::TransactionArgument};
use serde::{Deserialize, Serialize};

/// A transaction filter that applies a set of rules to determine
/// if a transaction should be allowed or denied.
///
/// Rules are applied in the order they are defined, and the first
/// matching rule determines the outcome for the transaction.
/// If no rules match, the transaction is allowed by default.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct TransactionFilter {
    transaction_rules: Vec<TransactionRule>,
}

impl TransactionFilter {
    pub fn new(transaction_rules: Vec<TransactionRule>) -> Self {
        Self { transaction_rules }
    }

    /// Returns true iff the filter allows the transaction
    pub fn allows_transaction(&self, signed_transaction: &SignedTransaction) -> bool {
        // If the filter is empty, allow the transaction by default
        if self.is_empty() {
            return true;
        }

        // Check if any rule matches the transaction
        for transaction_rule in &self.transaction_rules {
            if transaction_rule.matches(signed_transaction) {
                return match transaction_rule {
                    TransactionRule::Allow(_) => true,
                    TransactionRule::Deny(_) => false,
                };
            }
        }

        true // No rules match (allow the transaction by default)
    }

    /// Returns an empty transaction filter with no rules
    pub fn empty() -> Self {
        Self {
            transaction_rules: Vec::new(),
        }
    }

    /// Filters the given transactions and returns only those that are allowed
    pub fn filter_transactions(
        &self,
        transactions: Vec<SignedTransaction>,
    ) -> Vec<SignedTransaction> {
        transactions
            .into_iter()
            .filter(|txn| self.allows_transaction(txn))
            .collect()
    }

    /// Returns true iff the filter is empty (i.e., has no rules)
    pub fn is_empty(&self) -> bool {
        self.transaction_rules.is_empty()
    }
}

// These are useful test-only methods for creating and testing filters
#[cfg(any(test, feature = "fuzzing"))]
impl TransactionFilter {
    /// Adds an account address matcher to the filter
    pub fn add_account_address_filter(self, allow: bool, account_address: AccountAddress) -> Self {
        let transaction_matcher = TransactionMatcher::AccountAddress(account_address);
        self.add_multiple_matchers_filter(allow, vec![transaction_matcher])
    }

    /// Adds an all matcher to the filter (matching all transactions)
    pub fn add_all_filter(self, allow: bool) -> Self {
        let transaction_matcher = TransactionMatcher::All;
        self.add_multiple_matchers_filter(allow, vec![transaction_matcher])
    }

    /// Adds an entry function matcher to the filter
    pub fn add_entry_function_filter(
        self,
        allow: bool,
        address: AccountAddress,
        module_name: String,
        function: String,
    ) -> Self {
        let transaction_matcher = TransactionMatcher::EntryFunction(address, module_name, function);
        self.add_multiple_matchers_filter(allow, vec![transaction_matcher])
    }

    /// Adds a module address matcher to the filter
    pub fn add_module_address_filter(self, allow: bool, address: AccountAddress) -> Self {
        let transaction_matcher = TransactionMatcher::ModuleAddress(address);
        self.add_multiple_matchers_filter(allow, vec![transaction_matcher])
    }

    /// Adds a filter rule containing multiple matchers
    pub fn add_multiple_matchers_filter(
        mut self,
        allow: bool,
        transaction_matchers: Vec<TransactionMatcher>,
    ) -> Self {
        let transaction_rule = if allow {
            TransactionRule::Allow(transaction_matchers)
        } else {
            TransactionRule::Deny(transaction_matchers)
        };
        self.transaction_rules.push(transaction_rule);

        self
    }

    /// Adds a public key matcher to the filter
    pub fn add_public_key_filter(self, allow: bool, public_key: AnyPublicKey) -> Self {
        let transaction_matcher = TransactionMatcher::PublicKey(public_key);
        self.add_multiple_matchers_filter(allow, vec![transaction_matcher])
    }

    /// Adds a sender address matcher to the filter
    pub fn add_sender_filter(self, allow: bool, sender: AccountAddress) -> Self {
        let transaction_matcher = TransactionMatcher::Sender(sender);
        self.add_multiple_matchers_filter(allow, vec![transaction_matcher])
    }

    /// Adds a transaction ID matcher to the filter
    pub fn add_transaction_id_filter(self, allow: bool, txn_id: HashValue) -> Self {
        let transaction_matcher = TransactionMatcher::TransactionId(txn_id);
        self.add_multiple_matchers_filter(allow, vec![transaction_matcher])
    }
}

/// A transaction rule that defines whether to allow or deny transactions
/// based on a set of matchers. All matchers must match for the rule to apply.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum TransactionRule {
    Allow(Vec<TransactionMatcher>),
    Deny(Vec<TransactionMatcher>),
}

impl TransactionRule {
    /// Returns true iff the rule matches the given transaction. This
    /// requires that all matchers in the rule match the transaction.
    fn matches(&self, signed_transaction: &SignedTransaction) -> bool {
        let transaction_matchers = match self {
            TransactionRule::Allow(matchers) => matchers,
            TransactionRule::Deny(matchers) => matchers,
        };
        transaction_matchers
            .iter()
            .all(|matcher| matcher.matches(signed_transaction))
    }
}

/// A matcher that defines the criteria for matching transactions
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum TransactionMatcher {
    All,                                           // Matches any transaction
    TransactionId(HashValue),                      // Matches a specific transaction by its ID
    Sender(AccountAddress), // Matches any transaction sent by a specific account address
    ModuleAddress(AccountAddress), // Matches any transaction that calls a module at a specific address
    EntryFunction(AccountAddress, String, String), // Matches any transaction that calls a specific entry function in a module
    AccountAddress(AccountAddress), // Matches any transaction that involves a specific account address
    PublicKey(AnyPublicKey),        // Matches any transaction that involves a specific public key
}

impl TransactionMatcher {
    /// Returns true iff the matcher matches the given transaction
    pub(crate) fn matches(&self, signed_transaction: &SignedTransaction) -> bool {
        match self {
            TransactionMatcher::All => true,
            TransactionMatcher::TransactionId(id) => signed_transaction.committed_hash() == *id,
            TransactionMatcher::Sender(sender) => {
                matches_sender_address(signed_transaction, sender)
            },
            TransactionMatcher::ModuleAddress(address) => {
                matches_entry_function_module_address(signed_transaction, address)
            },
            TransactionMatcher::EntryFunction(address, module_name, function) => {
                matches_entry_function(signed_transaction, address, module_name, function)
            },
            TransactionMatcher::AccountAddress(address) => {
                matches_sender_address(signed_transaction, address)
                    || matches_entry_function_module_address(signed_transaction, address)
                    || matches_multisig_address(signed_transaction, address)
                    || matches_script_argument_address(signed_transaction, address)
                    || matches_transaction_authenticator_address(signed_transaction, address)
            },
            TransactionMatcher::PublicKey(public_key) => {
                matches_transaction_authenticator_public_key(signed_transaction, public_key)
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
