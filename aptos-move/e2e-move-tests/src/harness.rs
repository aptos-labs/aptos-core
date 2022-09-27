// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::AptosPackageHooks;
use aptos::move_tool::MemberId;
use aptos_crypto::ed25519::Ed25519PrivateKey;
use aptos_crypto::{PrivateKey, Uniform};
use aptos_gas::{AptosGasParameters, InitialGasSchedule, ToOnChainGasSchedule};
use aptos_types::on_chain_config::{FeatureFlag, GasScheduleV2};
use aptos_types::{
    access_path::AccessPath,
    account_address::AccountAddress,
    state_store::state_key::StateKey,
    transaction::{EntryFunction, SignedTransaction, TransactionPayload, TransactionStatus},
};
use cached_packages::aptos_stdlib;
use framework::natives::code::PackageMetadata;
use framework::{BuildOptions, BuiltPackage};
use language_e2e_tests::{
    account::{Account, AccountData},
    executor::FakeExecutor,
};
use move_deps::move_core_types::language_storage::{ResourceKey, StructTag, TypeTag};
use move_deps::move_core_types::value::MoveValue;
use move_deps::move_package::package_hooks::register_package_hooks;
use project_root::get_project_root;
use rand::{
    rngs::{OsRng, StdRng},
    Rng, SeedableRng,
};
use serde::de::DeserializeOwned;
use std::collections::BTreeMap;
use std::path::Path;

/// A simple test harness for defining Move e2e tests.
///
/// Tests defined via this harness typically live in the `<crate>/tests` directory, the standard
/// Rust place for defining integration tests.
///
/// For defining a set of new tests around a specific area, you add a new Rust source
/// `tested_area.rs` to the `tests` directory of your crate. You also will create a directory
/// `tested_area.data` which lives side-by-side with the Rust source. In this directory, you
/// place any number of Move packages you need for running the tests. In addition, the test
/// infrastructure will place baseline (golden) files in the `tested_area.data` using the `.exp`
/// (expected) ending.  For examples, see e.g. the `tests/code_publishing.rs` test in this crate.
///
/// NOTE: This harness currently is a wrapper around existing legacy e2e testing infra. We
/// eventually plan to retire the legacy code, and are rather keen to know what of the legacy
/// test infra we want to maintain and also which existing tests to preserve.
pub struct MoveHarness {
    /// The executor being used.
    pub executor: FakeExecutor,
    /// The current transaction sequence number, by account address.
    txn_seq_no: BTreeMap<AccountAddress, u64>,
}

impl MoveHarness {
    /// Creates a new harness.
    pub fn new() -> Self {
        register_package_hooks(Box::new(AptosPackageHooks {}));
        Self {
            executor: FakeExecutor::from_head_genesis(),
            txn_seq_no: BTreeMap::default(),
        }
    }

    pub fn new_testnet() -> Self {
        register_package_hooks(Box::new(AptosPackageHooks {}));
        Self {
            executor: FakeExecutor::from_testnet_genesis(),
            txn_seq_no: BTreeMap::default(),
        }
    }

    pub fn new_with_features(features: Vec<FeatureFlag>) -> Self {
        let mut h = Self::new();
        if !features.is_empty() {
            h.enable_features(features);
        }
        h
    }

    pub fn new_mainnet() -> Self {
        register_package_hooks(Box::new(AptosPackageHooks {}));
        Self {
            executor: FakeExecutor::from_mainnet_genesis(),
            txn_seq_no: BTreeMap::default(),
        }
    }

    /// Creates an account for the given static address. This address needs to be static so
    /// we can load regular Move code to there without need to rewrite code addresses.
    pub fn new_account_at(&mut self, addr: AccountAddress) -> Account {
        // The below will use the genesis keypair but that should be fine.
        let acc = Account::new_genesis_account(addr);
        // Mint the account 10M Aptos coins (with 8 decimals).
        let data = AccountData::with_account(acc, 1_000_000_000_000_000, 10);
        self.executor.add_account_data(&data);
        self.txn_seq_no.insert(addr, 10);
        data.account().clone()
    }

    // Creates an account with a randomly generated address and key pair
    pub fn new_account_with_key_pair(&mut self) -> Account {
        let mut rng = StdRng::from_seed(OsRng.gen());

        let privkey = Ed25519PrivateKey::generate(&mut rng);
        let pubkey = privkey.public_key();
        let acc = Account::with_keypair(privkey, pubkey);
        let data = AccountData::with_account(acc.clone(), 1_000_000_000_000_000, 10);
        self.executor.add_account_data(&data);
        self.txn_seq_no.insert(*acc.address(), 10);
        data.account().clone()
    }

    /// Gets the account where the Aptos framework is installed (0x1).
    pub fn aptos_framework_account(&mut self) -> Account {
        self.new_account_at(AccountAddress::ONE)
    }

    /// Runs a signed transaction. On success, applies the write set.
    pub fn run(&mut self, txn: SignedTransaction) -> TransactionStatus {
        let output = self.executor.execute_transaction(txn);
        if matches!(output.status(), TransactionStatus::Keep(_)) {
            self.executor.apply_write_set(output.write_set());
        }
        output.status().to_owned()
    }

    /// Runs a block of signed transactions. On success, applies the write set.
    pub fn run_block(&mut self, txn_block: Vec<SignedTransaction>) -> Vec<TransactionStatus> {
        let mut result = vec![];
        for output in self.executor.execute_block(txn_block).unwrap() {
            if matches!(output.status(), TransactionStatus::Keep(_)) {
                self.executor.apply_write_set(output.write_set());
            }
            result.push(output.status().to_owned())
        }
        result
    }

    /// Creates a transaction, based on provided payload.
    pub fn create_transaction_payload(
        &mut self,
        account: &Account,
        payload: TransactionPayload,
    ) -> SignedTransaction {
        let seq_no_ref = self.txn_seq_no.get_mut(account.address()).unwrap();
        let seq_no = *seq_no_ref;
        *seq_no_ref += 1;
        account
            .transaction()
            .sequence_number(seq_no)
            .max_gas_amount(1_000_000)
            .gas_unit_price(1)
            .payload(payload)
            .sign()
    }

    /// Runs a transaction, based on provided payload. If the transaction succeeds, any generated
    /// writeset will be applied to storage.
    pub fn run_transaction_payload(
        &mut self,
        account: &Account,
        payload: TransactionPayload,
    ) -> TransactionStatus {
        let txn = self.create_transaction_payload(account, payload);
        self.run(txn)
    }

    /// Creates a transaction which runs the specified entry point `fun`. Arguments need to be
    /// provided in bcs-serialized form.
    pub fn create_entry_function(
        &mut self,
        account: &Account,
        fun: MemberId,
        ty_args: Vec<TypeTag>,
        args: Vec<Vec<u8>>,
    ) -> SignedTransaction {
        let MemberId {
            module_id,
            member_id: function_id,
        } = fun;
        self.create_transaction_payload(
            account,
            TransactionPayload::EntryFunction(EntryFunction::new(
                module_id,
                function_id,
                ty_args,
                args,
            )),
        )
    }

    /// Run the specified entry point `fun`. Arguments need to be provided in bcs-serialized form.
    pub fn run_entry_function(
        &mut self,
        account: &Account,
        fun: MemberId,
        ty_args: Vec<TypeTag>,
        args: Vec<Vec<u8>>,
    ) -> TransactionStatus {
        let txn = self.create_entry_function(account, fun, ty_args, args);
        self.run(txn)
    }

    /// Creates a transaction which publishes the Move Package found at the given path on behalf
    /// of the given account.
    ///
    /// The passed function allows to manipulate the generated metadata for testing purposes.
    pub fn create_publish_package(
        &mut self,
        account: &Account,
        path: &Path,
        options: Option<BuildOptions>,
        mut patch_metadata: impl FnMut(&mut PackageMetadata),
    ) -> SignedTransaction {
        let package = BuiltPackage::build(path.to_owned(), options.unwrap_or_default())
            .expect("building package must succeed");
        let code = package.extract_code();
        let mut metadata = package
            .extract_metadata()
            .expect("extracting package metadata must succeed");
        patch_metadata(&mut metadata);
        self.create_transaction_payload(
            account,
            aptos_stdlib::code_publish_package_txn(
                bcs::to_bytes(&metadata).expect("PackageMetadata has BCS"),
                code,
            ),
        )
    }

    /// Runs transaction which publishes the Move Package.
    pub fn publish_package(&mut self, account: &Account, path: &Path) -> TransactionStatus {
        let txn = self.create_publish_package(account, path, None, |_| {});
        self.run(txn)
    }

    /// Runs transaction which publishes the Move Package.
    pub fn publish_package_with_options(
        &mut self,
        account: &Account,
        path: &Path,
        options: BuildOptions,
    ) -> TransactionStatus {
        let txn = self.create_publish_package(account, path, Some(options), |_| {});
        self.run(txn)
    }

    /// Runs transaction which publishes the Move Package, and alllows to patch the metadata
    pub fn publish_package_with_patcher(
        &mut self,
        account: &Account,
        path: &Path,
        metadata_patcher: impl FnMut(&mut PackageMetadata),
    ) -> TransactionStatus {
        let txn = self.create_publish_package(account, path, None, metadata_patcher);
        self.run(txn)
    }

    pub fn fast_forward(&mut self, seconds: u64) {
        let current_time = self.executor.get_block_time();
        self.executor
            .set_block_time(current_time + seconds * 1_000_000)
    }

    pub fn new_epoch(&mut self) {
        self.fast_forward(7200);
        self.executor.new_block()
    }

    pub fn new_block_with_metadata(
        &mut self,
        proposer: AccountAddress,
        failed_proposer_indices: Vec<u32>,
    ) {
        self.fast_forward(1);
        self.executor
            .new_block_with_metadata(proposer, failed_proposer_indices);
    }

    pub fn read_state_value(&self, state_key: &StateKey) -> Option<Vec<u8>> {
        self.executor.read_state_value(state_key).and_then(|bytes| {
            if bytes.is_empty() {
                None
            } else {
                Some(bytes)
            }
        })
    }

    /// Reads the raw, serialized data of a resource.
    pub fn read_resource_raw(
        &self,
        addr: &AccountAddress,
        struct_tag: StructTag,
    ) -> Option<Vec<u8>> {
        let path = AccessPath::resource_access_path(ResourceKey::new(*addr, struct_tag));
        self.read_state_value(&StateKey::AccessPath(path))
    }

    /// Reads the resource data `T`.
    pub fn read_resource<T: DeserializeOwned>(
        &self,
        addr: &AccountAddress,
        struct_tag: StructTag,
    ) -> Option<T> {
        Some(
            bcs::from_bytes::<T>(&self.read_resource_raw(addr, struct_tag)?).expect(
                "serialization expected to succeed (Rust type incompatible with Move type?)",
            ),
        )
    }

    /// Checks whether resource exists.
    pub fn exists_resource(&self, addr: &AccountAddress, struct_tag: StructTag) -> bool {
        self.read_resource_raw(addr, struct_tag).is_some()
    }

    /// Enables features
    pub fn enable_features(&mut self, features: Vec<FeatureFlag>) {
        let acc = self.aptos_framework_account();
        let enable = features.into_iter().map(|f| f as u64).collect::<Vec<_>>();
        self.executor.exec(
            "features",
            "change_feature_flags",
            vec![],
            vec![
                MoveValue::Signer(*acc.address())
                    .simple_serialize()
                    .unwrap(),
                bcs::to_bytes(&enable).unwrap(),
                bcs::to_bytes(&Vec::<u64>::new()).unwrap(),
            ],
        );
    }

    /// Increase maximal transaction size.
    pub fn increase_transaction_size(&mut self) {
        // TODO: The AptosGasParameters::zeros() schedule doesn't do what we want, so
        // explicitly manipulating gas entries. Wasn't obvious from the gas code how to
        // do this differently then below, so perhaps improve this...
        let entries = AptosGasParameters::initial().to_on_chain_gas_schedule();
        let entries = entries
            .into_iter()
            .map(|(name, val)| {
                if name == "txn.max_transaction_size_in_bytes" {
                    (name, 1000 * 1024)
                } else {
                    (name, val)
                }
            })
            .collect::<Vec<_>>();
        let gas_schedule = GasScheduleV2 {
            feature_version: aptos_gas::LATEST_GAS_FEATURE_VERSION,
            entries,
        };
        let schedule_bytes = bcs::to_bytes(&gas_schedule).expect("bcs");
        self.executor.exec(
            "gas_schedule",
            "set_gas_schedule",
            vec![],
            vec![
                MoveValue::Signer(AccountAddress::ONE)
                    .simple_serialize()
                    .unwrap(),
                MoveValue::vector_u8(schedule_bytes)
                    .simple_serialize()
                    .unwrap(),
            ],
        );
    }
}

/// Enables golden files for the given harness. The golden file will be stored side-by-side
/// with the data directory of a Rust source, named after the test function.
#[macro_export]
macro_rules! enable_golden {
    ($h:expr) => {
        $h.internal_set_golden(std::file!(), language_e2e_tests::current_function_name!())
    };
}

impl MoveHarness {
    /// Internal function to support the `enable_golden` macro.
    pub fn internal_set_golden(&mut self, file_macro_value: &str, function_macro_value: &str) {
        // The result of `std::file!` gives us a name relative to the project root,
        // so we need to add that to it. We also want to replace the extension `.rs` with `.data`.
        let mut path = get_project_root().unwrap().join(file_macro_value);
        path.set_extension("data");
        // The result of the `current_function` macro gives us the fully qualified
        // We only want the trailing simple name.
        let fun = function_macro_value.split("::").last().unwrap();
        self.executor
            .set_golden_file_at(&path.display().to_string(), fun)
    }
}

/// Helper to assert transaction is successful
#[macro_export]
macro_rules! assert_success {
    ($s:expr) => {{
        use aptos_types::transaction::*;
        assert_eq!($s, TransactionStatus::Keep(ExecutionStatus::Success))
    }};
}

/// Helper to assert transaction aborts.
#[macro_export]
macro_rules! assert_abort {
    ($s:expr, $c:pat) => {{
        assert!(matches!(
            $s,
            aptos_types::transaction::TransactionStatus::Keep(
                aptos_types::transaction::ExecutionStatus::MoveAbort { code: $c, .. }
            ),
        ));
    }};
}

/// Helper to assert vm status code.
#[macro_export]
macro_rules! assert_vm_status {
    ($s:expr, $c:pat) => {{
        use aptos_types::transaction::*;
        assert!(matches!(
            $s,
            TransactionStatus::Keep(ExecutionStatus::MiscellaneousError(Some($c)))
        ));
    }};
}
