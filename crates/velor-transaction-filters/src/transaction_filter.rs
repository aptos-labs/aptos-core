// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use velor_crypto::{ed25519::Ed25519PublicKey, HashValue};
use velor_types::transaction::{
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

    /// Adds an all matcher to the filter (matching all transactions)
    pub fn add_all_filter(self, allow: bool) -> Self {
        let transaction_matcher = TransactionMatcher::All;
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
}

// These are useful test-only methods for creating and testing filters
#[cfg(any(test, feature = "fuzzing"))]
impl TransactionFilter {
    /// Adds an account address matcher to the filter
    pub fn add_account_address_filter(self, allow: bool, account_address: AccountAddress) -> Self {
        let transaction_matcher = TransactionMatcher::AccountAddress(account_address);
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

#[cfg(test)]
mod test {
    use super::*;
    use velor_crypto::{
        ed25519::Ed25519PrivateKey,
        multi_ed25519::{MultiEd25519PublicKey, MultiEd25519Signature},
        secp256k1_ecdsa, secp256r1_ecdsa, PrivateKey, SigningKey, Uniform,
    };
    use velor_types::{
        chain_id::ChainId,
        function_info::FunctionInfo,
        keyless::test_utils::get_sample_groth16_sig_and_pk,
        transaction::{
            authenticator::{AnySignature, SingleKeyAuthenticator},
            RawTransaction,
        },
    };
    use rand::thread_rng;

    #[test]
    fn test_matches_account_authenticator_address() {
        // Create an empty account authenticator
        let account_authenticator = AccountAuthenticator::NoAccountAuthenticator;

        // Verify that the authenticator doesn't match the target address
        let target_address = AccountAddress::random();
        verify_matches_account_auth_address(&account_authenticator, &target_address, false);

        // Create an Ed25519 account authenticator
        let raw_transaction = create_raw_transaction();
        let private_key = Ed25519PrivateKey::generate_for_testing();
        let public_key = private_key.public_key();
        let signature = private_key.sign(&raw_transaction).unwrap();
        let account_authenticator = AccountAuthenticator::Ed25519 {
            public_key: public_key.clone(),
            signature: signature.clone(),
        };

        // Verify that the authenticator doesn't match the target address
        verify_matches_account_auth_address(&account_authenticator, &target_address, false);

        // Create a MultiEd25519 account authenticator
        let multi_public_key = MultiEd25519PublicKey::new(vec![public_key], 1).unwrap();
        let multi_signature = MultiEd25519Signature::from(signature);
        let account_authenticator = AccountAuthenticator::MultiEd25519 {
            public_key: multi_public_key,
            signature: multi_signature,
        };

        // Verify that the authenticator doesn't match the target address
        verify_matches_account_auth_address(&account_authenticator, &target_address, false);

        // Create an Abstraction account authenticator (with the target address as the module address)
        let function_info = FunctionInfo::new(target_address, "".into(), "".into());
        let account_authenticator =
            AccountAuthenticator::abstraction(function_info, vec![], vec![]);

        // Verify that the authenticator matches the target address
        verify_matches_account_auth_address(&account_authenticator, &target_address, true);
    }

    #[test]
    fn test_matches_any_public_key_address() {
        // Create an Ed25519 public key
        let private_key = Ed25519PrivateKey::generate_for_testing();
        let public_key = AnyPublicKey::ed25519(private_key.public_key());

        // Verify that the public key doesn't match the target address
        let target_address = AccountAddress::random();
        verify_matches_public_key_address(&public_key, &target_address, false);

        // Create a Secp256k1Ecdsa public key
        let private_key = secp256k1_ecdsa::PrivateKey::generate_for_testing();
        let public_key = AnyPublicKey::secp256k1_ecdsa(private_key.public_key());

        // Verify that the public key doesn't match the target address
        verify_matches_public_key_address(&public_key, &target_address, false);

        // Create a Secp256r1Ecdsa public key
        let private_key = secp256r1_ecdsa::PrivateKey::generate_for_testing();
        let public_key = AnyPublicKey::secp256r1_ecdsa(private_key.public_key());

        // Verify that the public key doesn't match the target address
        verify_matches_public_key_address(&public_key, &target_address, false);

        // Create a Keyless public key
        let (_, keyless_public_key) = get_sample_groth16_sig_and_pk();
        let public_key = AnyPublicKey::keyless(keyless_public_key.clone());

        // Verify that the public key doesn't match the target address
        verify_matches_public_key_address(&public_key, &target_address, false);

        // Create a FederatedKeyless public key with the target address as the JWK address
        let federated_keyless_public_key = velor_types::keyless::FederatedKeylessPublicKey {
            jwk_addr: target_address,
            pk: keyless_public_key,
        };
        let public_key = AnyPublicKey::federated_keyless(federated_keyless_public_key);

        // Verify that the public key matches the target address
        verify_matches_public_key_address(&public_key, &target_address, true);
    }

    #[test]
    fn test_matches_transaction_authenticator_address() {
        // Create an Ed25519 transaction authenticator
        let raw_transaction = create_raw_transaction();
        let private_key = Ed25519PrivateKey::generate_for_testing();
        let signature = private_key.sign(&raw_transaction).unwrap();
        let signed_transaction = SignedTransaction::new(
            raw_transaction.clone(),
            private_key.public_key(),
            signature.clone(),
        );

        // Verify that the authenticator doesn't match the target address
        let target_address = AccountAddress::random();
        verify_matches_transaction_auth_address(&signed_transaction, &target_address, false);

        // Create a MultiEd25519 transaction authenticator
        let multi_public_key =
            MultiEd25519PublicKey::new(vec![private_key.public_key()], 1).unwrap();
        let multi_signature = MultiEd25519Signature::from(signature);
        let signed_transaction = SignedTransaction::new_multisig(
            raw_transaction.clone(),
            multi_public_key,
            multi_signature,
        );

        // Verify that the authenticator doesn't match the target address
        verify_matches_transaction_auth_address(&signed_transaction, &target_address, false);

        // Create a multi-agent transaction authenticator with the target secondary signer
        let signed_transaction = SignedTransaction::new_multi_agent(
            raw_transaction.clone(),
            AccountAuthenticator::NoAccountAuthenticator,
            vec![
                AccountAddress::random(),
                target_address,
                AccountAddress::random(),
            ],
            vec![AccountAuthenticator::NoAccountAuthenticator],
        );

        // Verify that the authenticator matches the target address
        verify_matches_transaction_auth_address(&signed_transaction, &target_address, true);

        // Create a fee payer transaction authenticator
        let fee_payer_address = AccountAddress::random();
        let secondary_signer_address = AccountAddress::random();
        let signed_transaction = SignedTransaction::new_fee_payer(
            raw_transaction.clone(),
            AccountAuthenticator::NoAccountAuthenticator,
            vec![secondary_signer_address],
            vec![AccountAuthenticator::NoAccountAuthenticator],
            fee_payer_address,
            AccountAuthenticator::NoAccountAuthenticator,
        );

        // Verify that the authenticator matches the fee payer and secondary signer addresses
        for address in [&fee_payer_address, &secondary_signer_address] {
            verify_matches_transaction_auth_address(&signed_transaction, address, true);
        }

        // Verify that the authenticator doesn't match the target address
        verify_matches_transaction_auth_address(&signed_transaction, &target_address, false);
    }

    #[test]
    fn test_matches_account_authenticator_public_key() {
        // Create an empty account authenticator
        let account_authenticator = AccountAuthenticator::NoAccountAuthenticator;

        // Verify that the authenticator doesn't match the public key
        let private_key_1 = get_random_private_key();
        verify_matches_account_auth_public_key(
            &account_authenticator,
            &AnyPublicKey::ed25519(private_key_1.public_key()),
            false,
        );

        // Create an abstraction account authenticator
        let function_info = FunctionInfo::new(
            AccountAddress::random(),
            "test_module".to_string(),
            "test_function".to_string(),
        );
        let account_authenticator =
            AccountAuthenticator::abstraction(function_info, vec![], vec![]);

        // Verify that the authenticator doesn't match the public key
        verify_matches_account_auth_public_key(
            &account_authenticator,
            &AnyPublicKey::ed25519(private_key_1.public_key()),
            false,
        );

        // Create an Ed25519 account authenticator (using the private key)
        let account_authenticator = AccountAuthenticator::Ed25519 {
            public_key: private_key_1.public_key(),
            signature: private_key_1.sign(&create_raw_transaction()).unwrap(),
        };

        // Verify that the authenticator matches the expected public key
        verify_matches_account_auth_public_key(
            &account_authenticator,
            &AnyPublicKey::ed25519(private_key_1.public_key()),
            true,
        );
        verify_matches_account_auth_public_key(
            &account_authenticator,
            &AnyPublicKey::ed25519(get_random_private_key().public_key()),
            false,
        );

        // Create a MultiEd25519 account authenticator
        let private_key_2 = get_random_private_key();
        let multi_public_key = MultiEd25519PublicKey::new(
            vec![private_key_1.public_key(), private_key_2.public_key()],
            1,
        )
        .unwrap();
        let multi_signature =
            MultiEd25519Signature::from(private_key_1.sign(&create_raw_transaction()).unwrap());
        let account_authenticator = AccountAuthenticator::MultiEd25519 {
            public_key: multi_public_key,
            signature: multi_signature,
        };

        // Verify that the authenticator matches the expected public key
        verify_matches_account_auth_public_key(
            &account_authenticator,
            &AnyPublicKey::ed25519(private_key_1.public_key()),
            true,
        );
        verify_matches_account_auth_public_key(
            &account_authenticator,
            &AnyPublicKey::ed25519(private_key_2.public_key()),
            true,
        );
        verify_matches_account_auth_public_key(
            &account_authenticator,
            &AnyPublicKey::ed25519(get_random_private_key().public_key()),
            false,
        );

        // Create a SingleKey account authenticator
        let account_authenticator = AccountAuthenticator::SingleKey {
            authenticator: SingleKeyAuthenticator::new(
                AnyPublicKey::Ed25519 {
                    public_key: private_key_1.public_key(),
                },
                AnySignature::Ed25519 {
                    signature: private_key_1.sign(&create_raw_transaction()).unwrap(),
                },
            ),
        };

        // Verify that the authenticator matches the expected public key
        verify_matches_account_auth_public_key(
            &account_authenticator,
            &AnyPublicKey::ed25519(private_key_1.public_key()),
            true,
        );
        verify_matches_account_auth_public_key(
            &account_authenticator,
            &AnyPublicKey::ed25519(private_key_2.public_key()),
            false,
        );
    }

    #[test]
    fn test_matches_transaction_authenticator_public_key() {
        // Create an Ed25519 transaction authenticator
        let raw_transaction = create_raw_transaction();
        let private_key_1 = Ed25519PrivateKey::generate_for_testing();
        let signature = private_key_1.sign(&raw_transaction).unwrap();
        let signed_transaction = SignedTransaction::new(
            raw_transaction.clone(),
            private_key_1.public_key(),
            signature.clone(),
        );

        // Verify that the authenticator matches the expected public key
        let private_key_2 = get_random_private_key();
        verify_matches_transaction_auth_public_key(
            &signed_transaction,
            &AnyPublicKey::ed25519(private_key_1.public_key()),
            true,
        );
        verify_matches_transaction_auth_public_key(
            &signed_transaction,
            &AnyPublicKey::ed25519(private_key_2.public_key()),
            false,
        );

        // Create a MultiEd25519 transaction authenticator
        let multi_public_key = MultiEd25519PublicKey::new(
            vec![private_key_1.public_key(), private_key_2.public_key()],
            1,
        )
        .unwrap();
        let multi_signature = MultiEd25519Signature::from(signature.clone());
        let signed_transaction = SignedTransaction::new_multisig(
            raw_transaction.clone(),
            multi_public_key,
            multi_signature,
        );

        // Verify that the authenticator matches the expected public key
        verify_matches_transaction_auth_public_key(
            &signed_transaction,
            &AnyPublicKey::ed25519(private_key_1.public_key()),
            true,
        );
        verify_matches_transaction_auth_public_key(
            &signed_transaction,
            &AnyPublicKey::ed25519(private_key_2.public_key()),
            true,
        );
        verify_matches_transaction_auth_public_key(
            &signed_transaction,
            &AnyPublicKey::ed25519(get_random_private_key().public_key()),
            false,
        );

        // Create a multi-agent transaction authenticator
        let signed_transaction = SignedTransaction::new_multi_agent(
            raw_transaction.clone(),
            AccountAuthenticator::Ed25519 {
                public_key: private_key_1.public_key(),
                signature: signature.clone(),
            },
            vec![],
            vec![AccountAuthenticator::Ed25519 {
                public_key: private_key_2.public_key(),
                signature: signature.clone(),
            }],
        );

        // Verify that the authenticator matches the expected public key
        verify_matches_transaction_auth_public_key(
            &signed_transaction,
            &AnyPublicKey::ed25519(private_key_1.public_key()),
            true,
        );
        verify_matches_transaction_auth_public_key(
            &signed_transaction,
            &AnyPublicKey::ed25519(private_key_2.public_key()),
            true,
        );
        verify_matches_transaction_auth_public_key(
            &signed_transaction,
            &AnyPublicKey::ed25519(get_random_private_key().public_key()),
            false,
        );
    }

    /// Creates and returns a raw transaction
    fn create_raw_transaction() -> RawTransaction {
        RawTransaction::new(
            AccountAddress::random(),
            0,
            TransactionPayload::Script(Script::new(vec![], vec![], vec![])),
            0,
            0,
            0,
            ChainId::new(10),
        )
    }

    /// Generates and returns a random Ed25519 private key
    fn get_random_private_key() -> Ed25519PrivateKey {
        Ed25519PrivateKey::generate(&mut thread_rng())
    }

    /// Verifies that the given account authenticator contains the expected address
    fn verify_matches_account_auth_address(
        account_authenticator: &AccountAuthenticator,
        address: &AccountAddress,
        matches: bool,
    ) {
        let result = matches_account_authenticator_address(account_authenticator, address);
        assert_eq!(matches, result);
    }

    /// Verifies that the given account authenticator contains the expected public key
    fn verify_matches_account_auth_public_key(
        account_authenticator: &AccountAuthenticator,
        any_public_key: &AnyPublicKey,
        matches: bool,
    ) {
        let result =
            matches_account_authenticator_public_key(account_authenticator, any_public_key);
        assert_eq!(matches, result);
    }

    /// Verifies that the given public key contains the target address
    fn verify_matches_public_key_address(
        any_public_key: &AnyPublicKey,
        address: &AccountAddress,
        matches: bool,
    ) {
        let result = matches_any_public_key_address(any_public_key, address);
        assert_eq!(matches, result);
    }

    /// Verifies that the given transaction authenticator contains the expected address
    fn verify_matches_transaction_auth_address(
        signed_transaction: &SignedTransaction,
        address: &AccountAddress,
        matches: bool,
    ) {
        let result = matches_transaction_authenticator_address(signed_transaction, address);
        assert_eq!(matches, result);
    }

    /// Verifies that the given transaction authenticator contains the expected public key
    fn verify_matches_transaction_auth_public_key(
        signed_transaction: &SignedTransaction,
        any_public_key: &AnyPublicKey,
        matches: bool,
    ) {
        let result =
            matches_transaction_authenticator_public_key(signed_transaction, any_public_key);
        assert_eq!(matches, result);
    }
}
