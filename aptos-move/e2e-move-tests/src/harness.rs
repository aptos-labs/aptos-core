// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{assert_success, AptosPackageHooks};
use anyhow::Error;
use aptos::move_tool::MemberId;
use aptos_cached_packages::aptos_stdlib;
use aptos_crypto::{ed25519::Ed25519PrivateKey, PrivateKey, Uniform};
use aptos_framework::{natives::code::PackageMetadata, BuildOptions, BuiltPackage};
use aptos_gas_profiling::TransactionGasLog;
use aptos_gas_schedule::{
    AptosGasParameters, FromOnChainGasSchedule, InitialGasSchedule, ToOnChainGasSchedule,
};
use aptos_language_e2e_tests::{
    account::{Account, AccountData},
    executor::FakeExecutor,
};
use aptos_types::{
    access_path::AccessPath,
    account_address::AccountAddress,
    account_config::{AccountResource, CORE_CODE_ADDRESS},
    contract_event::ContractEvent,
    on_chain_config::{FeatureFlag, GasScheduleV2, OnChainConfig},
    state_store::{
        state_key::StateKey,
        state_value::{StateValue, StateValueMetadata},
    },
    transaction::{
        EntryFunction, Script, SignedTransaction, TransactionArgument, TransactionOutput,
        TransactionPayload, TransactionStatus,
    },
};
use move_core_types::{
    language_storage::{StructTag, TypeTag},
    move_resource::MoveStructType,
    value::MoveValue,
};
use move_package::package_hooks::register_package_hooks;
use project_root::get_project_root;
use rand::{
    rngs::{OsRng, StdRng},
    Rng, SeedableRng,
};
use serde::{de::DeserializeOwned, Serialize};
use std::{collections::BTreeMap, path::Path};

const DEFAULT_GAS_UNIT_PRICE: u64 = 100;

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
    /// The last counted transaction sequence number, by account address.
    txn_seq_no: BTreeMap<AccountAddress, u64>,

    default_gas_unit_price: u64,
}

impl MoveHarness {
    /// Creates a new harness.
    pub fn new() -> Self {
        register_package_hooks(Box::new(AptosPackageHooks {}));
        Self {
            executor: FakeExecutor::from_head_genesis(),
            txn_seq_no: BTreeMap::default(),
            default_gas_unit_price: DEFAULT_GAS_UNIT_PRICE,
        }
    }

    pub fn new_with_validators(count: u64) -> Self {
        register_package_hooks(Box::new(AptosPackageHooks {}));
        Self {
            executor: FakeExecutor::from_head_genesis_with_count(count),
            txn_seq_no: BTreeMap::default(),
            default_gas_unit_price: DEFAULT_GAS_UNIT_PRICE,
        }
    }

    pub fn new_testnet() -> Self {
        register_package_hooks(Box::new(AptosPackageHooks {}));
        Self {
            executor: FakeExecutor::from_testnet_genesis(),
            txn_seq_no: BTreeMap::default(),
            default_gas_unit_price: DEFAULT_GAS_UNIT_PRICE,
        }
    }

    pub fn new_with_features(
        enabled_features: Vec<FeatureFlag>,
        disabled_features: Vec<FeatureFlag>,
    ) -> Self {
        let mut h = Self::new();
        h.enable_features(enabled_features, disabled_features);
        h
    }

    pub fn new_mainnet() -> Self {
        register_package_hooks(Box::new(AptosPackageHooks {}));
        Self {
            executor: FakeExecutor::from_mainnet_genesis(),
            txn_seq_no: BTreeMap::default(),
            default_gas_unit_price: DEFAULT_GAS_UNIT_PRICE,
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
        let data = AccountData::with_account(acc.clone(), 1_000_000_000_000_000, 0);
        self.executor.add_account_data(&data);
        self.txn_seq_no.insert(*acc.address(), 0);
        data.account().clone()
    }

    pub fn new_account_with_balance_and_sequence_number(
        &mut self,
        balance: u64,
        sequence_number: u64,
    ) -> Account {
        let mut rng = StdRng::from_seed(OsRng.gen());

        let privkey = Ed25519PrivateKey::generate(&mut rng);
        let pubkey = privkey.public_key();
        let acc = Account::with_keypair(privkey, pubkey);
        let data = AccountData::with_account(acc.clone(), balance, sequence_number);
        self.executor.add_account_data(&data);
        self.txn_seq_no.insert(*acc.address(), sequence_number);
        data.account().clone()
    }

    /// Gets the account where the Aptos framework is installed (0x1).
    pub fn aptos_framework_account(&mut self) -> Account {
        self.new_account_at(AccountAddress::ONE)
    }

    /// Runs a signed transaction. On success, applies the write set.
    pub fn run_raw(&mut self, txn: SignedTransaction) -> TransactionOutput {
        let output = self.executor.execute_transaction(txn);
        if matches!(output.status(), TransactionStatus::Keep(_)) {
            self.executor.apply_write_set(output.write_set());
        }
        output
    }

    /// Runs a signed transaction. On success, applies the write set.
    pub fn run(&mut self, txn: SignedTransaction) -> TransactionStatus {
        self.run_raw(txn).status().to_owned()
    }

    /// Runs a signed transaction. On success, applies the write set and return events
    pub fn run_with_events(
        &mut self,
        txn: SignedTransaction,
    ) -> (TransactionStatus, Vec<ContractEvent>) {
        let output = self.executor.execute_transaction(txn);
        if matches!(output.status(), TransactionStatus::Keep(_)) {
            self.executor.apply_write_set(output.write_set());
        }
        (output.status().to_owned(), output.events().to_owned())
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
        let on_chain_seq_no = self.sequence_number(account.address());
        let seq_no_ref = self.txn_seq_no.get_mut(account.address()).unwrap();
        let seq_no = std::cmp::max(on_chain_seq_no, *seq_no_ref);
        *seq_no_ref = seq_no + 1;
        account
            .transaction()
            .sequence_number(seq_no)
            .max_gas_amount(2_000_000)
            .gas_unit_price(self.default_gas_unit_price)
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

    /// Runs a transaction and return gas used.
    pub fn evaluate_gas(&mut self, account: &Account, payload: TransactionPayload) -> u64 {
        let txn = self.create_transaction_payload(account, payload);
        let output = self.run_raw(txn);
        assert_success!(output.status().to_owned());
        output.gas_used()
    }

    /// Runs a transaction with the gas profiler.
    pub fn evaluate_gas_with_profiler(
        &mut self,
        account: &Account,
        payload: TransactionPayload,
    ) -> (TransactionGasLog, u64) {
        let txn = self.create_transaction_payload(account, payload);
        let (output, gas_log) = self
            .executor
            .execute_transaction_with_gas_profiler(txn)
            .unwrap();
        if matches!(output.status(), TransactionStatus::Keep(_)) {
            self.executor.apply_write_set(output.write_set());
        }
        (gas_log, output.gas_used())
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

    pub fn create_script(
        &mut self,
        account: &Account,
        code: Vec<u8>,
        ty_args: Vec<TypeTag>,
        args: Vec<TransactionArgument>,
    ) -> SignedTransaction {
        self.create_transaction_payload(
            account,
            TransactionPayload::Script(Script::new(code, ty_args, args)),
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

    /// Run the specified entry point `fun` and return the gas used.
    pub fn evaluate_entry_function_gas(
        &mut self,
        account: &Account,
        fun: MemberId,
        ty_args: Vec<TypeTag>,
        args: Vec<Vec<u8>>,
    ) -> u64 {
        let txn = self.create_entry_function(account, fun, ty_args, args);
        let output = self.run_raw(txn);
        assert_success!(output.status().to_owned());
        output.gas_used()
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

    pub fn evaluate_publish_gas(&mut self, account: &Account, path: &Path) -> u64 {
        let txn = self.create_publish_package(account, path, None, |_| {});
        let output = self.run_raw(txn);
        assert_success!(output.status().to_owned());
        output.gas_used()
    }

    pub fn evaluate_publish_gas_with_profiler(
        &mut self,
        account: &Account,
        path: &Path,
    ) -> (TransactionGasLog, u64) {
        let txn = self.create_publish_package(account, path, None, |_| {});
        let (output, gas_log) = self
            .executor
            .execute_transaction_with_gas_profiler(txn)
            .unwrap();
        if matches!(output.status(), TransactionStatus::Keep(_)) {
            self.executor.apply_write_set(output.write_set());
        }
        (gas_log, output.gas_used())
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

    // Executes the block of transactions inserting metadata at the start of the
    // block. Returns a vector of transaction statuses and the gas they used.
    pub fn run_block_with_metadata(
        &mut self,
        proposer: AccountAddress,
        failed_proposer_indices: Vec<u32>,
        txns: Vec<SignedTransaction>,
    ) -> Vec<(TransactionStatus, u64)> {
        self.fast_forward(1);
        self.executor
            .run_block_with_metadata(proposer, failed_proposer_indices, txns)
    }

    pub fn read_state_value(&self, state_key: &StateKey) -> Option<StateValue> {
        self.executor.read_state_value(state_key)
    }

    pub fn read_state_value_bytes(&self, state_key: &StateKey) -> Option<Vec<u8>> {
        self.read_state_value(state_key).map(StateValue::into_bytes)
    }

    /// Reads the raw, serialized data of a resource.
    pub fn read_resource_raw(
        &self,
        addr: &AccountAddress,
        struct_tag: StructTag,
    ) -> Option<Vec<u8>> {
        let path =
            AccessPath::resource_access_path(*addr, struct_tag).expect("access path in test");
        self.read_state_value_bytes(&StateKey::access_path(path))
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

    pub fn read_resource_metadata(
        &self,
        addr: &AccountAddress,
        struct_tag: StructTag,
    ) -> Option<Option<StateValueMetadata>> {
        self.read_state_value(&StateKey::access_path(
            AccessPath::resource_access_path(*addr, struct_tag).expect("access path in test"),
        ))
        .map(StateValue::into_metadata)
    }

    pub fn read_resource_group(
        &self,
        addr: &AccountAddress,
        struct_tag: StructTag,
    ) -> Option<BTreeMap<StructTag, Vec<u8>>> {
        let path = AccessPath::resource_group_access_path(*addr, struct_tag);
        self.read_state_value_bytes(&StateKey::access_path(path))
            .map(|data| bcs::from_bytes(&data).unwrap())
    }

    pub fn read_resource_from_resource_group<T: DeserializeOwned>(
        &self,
        addr: &AccountAddress,
        resource_group: StructTag,
        struct_tag: StructTag,
    ) -> Option<T> {
        if let Some(group) = self.read_resource_group(addr, resource_group) {
            if let Some(data) = group.get(&struct_tag) {
                return Some(bcs::from_bytes::<T>(data).unwrap());
            }
        }
        None
    }

    /// Checks whether resource exists.
    pub fn exists_resource(&self, addr: &AccountAddress, struct_tag: StructTag) -> bool {
        self.read_resource_raw(addr, struct_tag).is_some()
    }

    /// Write the resource data `T`.
    pub fn set_resource<T: Serialize>(
        &mut self,
        addr: AccountAddress,
        struct_tag: StructTag,
        data: &T,
    ) {
        let path = AccessPath::resource_access_path(addr, struct_tag).expect("access path in test");
        let state_key = StateKey::access_path(path);
        self.executor
            .write_state_value(state_key, bcs::to_bytes(data).unwrap());
    }

    /// Enables features
    pub fn enable_features(&mut self, enabled: Vec<FeatureFlag>, disabled: Vec<FeatureFlag>) {
        let acc = self.aptos_framework_account();
        let enabled = enabled.into_iter().map(|f| f as u64).collect::<Vec<_>>();
        let disabled = disabled.into_iter().map(|f| f as u64).collect::<Vec<_>>();
        self.executor
            .exec("features", "change_feature_flags", vec![], vec![
                MoveValue::Signer(*acc.address())
                    .simple_serialize()
                    .unwrap(),
                bcs::to_bytes(&enabled).unwrap(),
                bcs::to_bytes(&disabled).unwrap(),
            ]);
    }

    /// Increase maximal transaction size.
    pub fn increase_transaction_size(&mut self) {
        // TODO: The AptosGasParameters::zeros() schedule doesn't do what we want, so
        // explicitly manipulating gas entries. Wasn't obvious from the gas code how to
        // do this differently then below, so perhaps improve this...
        let entries = AptosGasParameters::initial()
            .to_on_chain_gas_schedule(aptos_gas_schedule::LATEST_GAS_FEATURE_VERSION);
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
            feature_version: aptos_gas_schedule::LATEST_GAS_FEATURE_VERSION,
            entries,
        };
        let schedule_bytes = bcs::to_bytes(&gas_schedule).expect("bcs");
        self.executor
            .exec("gas_schedule", "set_gas_schedule", vec![], vec![
                MoveValue::Signer(AccountAddress::ONE)
                    .simple_serialize()
                    .unwrap(),
                MoveValue::vector_u8(schedule_bytes)
                    .simple_serialize()
                    .unwrap(),
            ]);
    }

    pub fn sequence_number(&self, addr: &AccountAddress) -> u64 {
        self.read_resource::<AccountResource>(addr, AccountResource::struct_tag())
            .unwrap()
            .sequence_number()
    }

    pub fn modify_gas_schedule_raw(&mut self, modify: impl FnOnce(&mut GasScheduleV2)) {
        let mut gas_schedule: GasScheduleV2 = self
            .read_resource(&CORE_CODE_ADDRESS, GasScheduleV2::struct_tag())
            .unwrap();
        modify(&mut gas_schedule);
        self.set_resource(
            CORE_CODE_ADDRESS,
            GasScheduleV2::struct_tag(),
            &gas_schedule,
        )
    }

    pub fn modify_gas_schedule(&mut self, modify: impl FnOnce(&mut AptosGasParameters)) {
        let gas_schedule: GasScheduleV2 = self
            .read_resource(&CORE_CODE_ADDRESS, GasScheduleV2::struct_tag())
            .unwrap();
        let feature_version = gas_schedule.feature_version;
        let mut gas_params = AptosGasParameters::from_on_chain_gas_schedule(
            &gas_schedule.to_btree_map(),
            feature_version,
        )
        .unwrap();
        modify(&mut gas_params);
        self.set_resource(
            CORE_CODE_ADDRESS,
            GasScheduleV2::struct_tag(),
            &GasScheduleV2 {
                feature_version,
                entries: gas_params.to_on_chain_gas_schedule(feature_version),
            },
        );
    }

    pub fn set_default_gas_unit_price(&mut self, gas_unit_price: u64) {
        self.default_gas_unit_price = gas_unit_price;
    }

    pub fn execute_view_function(
        &mut self,
        fun: MemberId,
        type_args: Vec<TypeTag>,
        arguments: Vec<Vec<u8>>,
    ) -> Result<Vec<Vec<u8>>, Error> {
        self.executor
            .execute_view_function(fun.module_id, fun.member_id, type_args, arguments)
    }
}

impl Default for MoveHarness {
    fn default() -> Self {
        Self::new()
    }
}

/// Enables golden files for the given harness. The golden file will be stored side-by-side
/// with the data directory of a Rust source, named after the test function.
#[macro_export]
macro_rules! enable_golden {
    ($h:expr) => {
        $h.internal_set_golden(
            std::file!(),
            aptos_language_e2e_tests::current_function_name!(),
        )
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
    ($s:expr, $c:expr) => {{
        use aptos_types::transaction::*;
        assert_eq!(
            $s,
            TransactionStatus::Keep(ExecutionStatus::MiscellaneousError(Some($c)))
        );
    }};
}

#[macro_export]
macro_rules! assert_move_abort {
    ($s:expr, $c:ident) => {{
        use aptos_types::transaction::*;
        assert!(match $s {
            TransactionStatus::Keep(ExecutionStatus::MoveAbort {
                location: _,
                code: _,
                info,
            }) => info == $c,
            _ => false,
        });
    }};
}
