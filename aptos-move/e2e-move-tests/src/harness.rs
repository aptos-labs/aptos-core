// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{assert_success, build_package, AptosPackageHooks};
use aptos_cached_packages::aptos_stdlib;
use aptos_framework::{natives::code::PackageMetadata, BuildOptions, BuiltPackage};
use aptos_gas_profiling::TransactionGasLog;
use aptos_gas_schedule::{
    AptosGasParameters, FromOnChainGasSchedule, InitialGasSchedule, ToOnChainGasSchedule,
};
use aptos_language_e2e_tests::{
    account::{Account, TransactionBuilder},
    executor::FakeExecutor,
};
use aptos_rest_client::AptosBaseUrl;
use aptos_transaction_simulation::SimulationStateStore;
use aptos_types::{
    account_address::AccountAddress,
    account_config::{
        fungible_store::FungibleStoreResource, object::ObjectGroupResource, AccountResource,
        CoinStoreResource, CORE_CODE_ADDRESS,
    },
    chain_id::ChainId,
    contract_event::ContractEvent,
    fee_statement::FeeStatement,
    move_utils::MemberId,
    on_chain_config::{FeatureFlag, GasScheduleV2, OnChainConfig},
    state_store::{
        state_key::StateKey,
        state_value::{StateValue, StateValueMetadata},
    },
    transaction::{
        EntryFunction, Multisig, MultisigTransactionPayload, Script, SignedTransaction,
        TransactionArgument, TransactionOutput, TransactionPayload, TransactionStatus,
        ViewFunctionOutput,
    },
    AptosCoinType,
};
use claims::assert_ok;
use move_core_types::{
    language_storage::{StructTag, TypeTag},
    move_resource::MoveStructType,
    value::MoveValue,
};
use move_package::package_hooks::register_package_hooks;
use once_cell::sync::Lazy;
use project_root::get_project_root;
use proptest::strategy::{BoxedStrategy, Just, Strategy};
use serde::{de::DeserializeOwned, Serialize};
use std::{
    collections::{BTreeMap, HashMap},
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

// Code representing successful transaction, used for run_block_in_parts_and_check
pub const SUCCESS: u64 = 0;

const DEFAULT_GAS_UNIT_PRICE: u64 = 100;

static CACHED_BUILT_PACKAGES: Lazy<Mutex<HashMap<PathBuf, Arc<anyhow::Result<BuiltPackage>>>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

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

    pub default_gas_unit_price: u64,
    pub max_gas_per_txn: u64,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum BlockSplit {
    Whole,
    SingleTxnPerBlock,
    SplitIntoThree { first_len: usize, second_len: usize },
}

impl MoveHarness {
    const DEFAULT_MAX_GAS_PER_TXN: u64 = 2_000_000;

    /// Creates a new harness.
    pub fn new() -> Self {
        register_package_hooks(Box::new(AptosPackageHooks {}));
        Self {
            executor: FakeExecutor::from_head_genesis(),
            txn_seq_no: BTreeMap::default(),
            default_gas_unit_price: DEFAULT_GAS_UNIT_PRICE,
            max_gas_per_txn: Self::DEFAULT_MAX_GAS_PER_TXN,
        }
    }

    pub fn new_with_executor(executor: FakeExecutor) -> Self {
        register_package_hooks(Box::new(AptosPackageHooks {}));
        Self {
            executor,
            txn_seq_no: BTreeMap::default(),
            default_gas_unit_price: DEFAULT_GAS_UNIT_PRICE,
            max_gas_per_txn: Self::DEFAULT_MAX_GAS_PER_TXN,
        }
    }

    pub fn new_with_validators(count: u64) -> Self {
        register_package_hooks(Box::new(AptosPackageHooks {}));
        Self {
            executor: FakeExecutor::from_head_genesis_with_count(count),
            txn_seq_no: BTreeMap::default(),
            default_gas_unit_price: DEFAULT_GAS_UNIT_PRICE,
            max_gas_per_txn: Self::DEFAULT_MAX_GAS_PER_TXN,
        }
    }

    pub fn new_testnet() -> Self {
        register_package_hooks(Box::new(AptosPackageHooks {}));
        Self {
            executor: FakeExecutor::from_testnet_genesis(),
            txn_seq_no: BTreeMap::default(),
            default_gas_unit_price: DEFAULT_GAS_UNIT_PRICE,
            max_gas_per_txn: Self::DEFAULT_MAX_GAS_PER_TXN,
        }
    }

    pub fn new_with_remote_state(network_url: AptosBaseUrl, txn_id: u64) -> Self {
        register_package_hooks(Box::new(AptosPackageHooks {}));

        let executor = FakeExecutor::from_remote_state(network_url, txn_id);

        let gas_schedule: GasScheduleV2 = executor.state_store().get_on_chain_config().unwrap();
        let feature_version = gas_schedule.feature_version;
        let gas_params = AptosGasParameters::from_on_chain_gas_schedule(
            &gas_schedule.into_btree_map(),
            feature_version,
        )
        .unwrap();

        Self {
            executor,
            txn_seq_no: BTreeMap::default(),
            default_gas_unit_price: gas_params.vm.txn.min_price_per_gas_unit.into(),
            max_gas_per_txn: Self::DEFAULT_MAX_GAS_PER_TXN,
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
            max_gas_per_txn: Self::DEFAULT_MAX_GAS_PER_TXN,
        }
    }

    pub fn store_and_fund_account(&mut self, acc: &Account, balance: u64, seq_num: u64) -> Account {
        let data = self
            .executor
            .store_and_fund_account(acc.clone(), balance, seq_num);
        self.txn_seq_no.insert(*acc.address(), seq_num);
        data.account().clone()
    }

    /// Creates an account for the given static address. This address needs to be static so
    /// we can load regular Move code to there without need to rewrite code addresses.
    pub fn new_account_at(&mut self, addr: AccountAddress) -> Account {
        self.new_account_with_balance_at(addr, 1_000_000_000_000_000)
    }

    pub fn new_account_with_balance_at(&mut self, addr: AccountAddress, balance: u64) -> Account {
        // The below will use the genesis keypair but that should be fine.
        let acc = Account::new_genesis_account(addr);
        // Mint the account 10M Aptos coins (with 8 decimals).
        self.store_and_fund_account(&acc, balance, 10)
    }

    // Creates an account with a randomly generated address and key pair
    pub fn new_account_with_key_pair(&mut self) -> Account {
        // Mint the account 10M Aptos coins (with 8 decimals).
        self.store_and_fund_account(&Account::new(), 1_000_000_000_000_000, 0)
    }

    pub fn new_account_with_balance_and_sequence_number(
        &mut self,
        balance: u64,
        sequence_number: u64,
    ) -> Account {
        self.store_and_fund_account(&Account::new(), balance, sequence_number)
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
            self.executor.append_events(output.events().to_vec());
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

    /// Runs a block of signed transactions. On success, applies the write set.
    pub fn run_block_get_output(
        &mut self,
        txn_block: Vec<SignedTransaction>,
    ) -> Vec<TransactionOutput> {
        let result = assert_ok!(self.executor.execute_block(txn_block));
        for output in &result {
            if matches!(output.status(), TransactionStatus::Keep(_)) {
                self.executor.apply_write_set(output.write_set());
            }
        }
        result
    }

    /// Creates a transaction without signing it
    pub fn create_transaction_without_sign(
        &mut self,
        account: &Account,
        payload: TransactionPayload,
    ) -> TransactionBuilder {
        let on_chain_seq_no = self.sequence_number_opt(account.address()).unwrap_or(0);
        let seq_no_ref = self.txn_seq_no.entry(*account.address()).or_insert(0);
        let seq_no = std::cmp::max(on_chain_seq_no, *seq_no_ref);
        *seq_no_ref = seq_no + 1;
        account
            .transaction()
            .chain_id(self.executor.get_chain_id())
            .ttl(
                self.executor.get_block_time() + 3_600_000_000, /* an hour after the current time */
            )
            .sequence_number(seq_no)
            .max_gas_amount(self.max_gas_per_txn)
            .gas_unit_price(self.default_gas_unit_price)
            .payload(payload)
    }

    /// Creates a transaction, based on provided payload.
    /// The chain_id is by default for test
    pub fn create_transaction_payload(
        &mut self,
        account: &Account,
        payload: TransactionPayload,
    ) -> SignedTransaction {
        self.create_transaction_without_sign(account, payload)
            .sign()
    }

    /// Creates a transaction to be sent to mainnet
    pub fn create_transaction_payload_mainnet(
        &mut self,
        account: &Account,
        payload: TransactionPayload,
    ) -> SignedTransaction {
        self.create_transaction_without_sign(account, payload)
            .chain_id(ChainId::mainnet())
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

    /// Runs a transaction sent to mainnet
    pub fn run_transaction_payload_mainnet(
        &mut self,
        account: &Account,
        payload: TransactionPayload,
    ) -> TransactionStatus {
        let txn = self.create_transaction_payload_mainnet(account, payload);
        assert!(self.chain_id_is_mainnet(&CORE_CODE_ADDRESS));
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
    ) -> (TransactionGasLog, u64, Option<FeeStatement>) {
        let txn = self.create_transaction_payload(account, payload);
        let (output, gas_log) = self
            .executor
            .execute_transaction_with_gas_profiler(txn)
            .unwrap();
        if matches!(output.status(), TransactionStatus::Keep(_)) {
            self.executor.apply_write_set(output.write_set());
        }
        (
            gas_log,
            output.gas_used(),
            output.try_extract_fee_statement().unwrap(),
        )
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

    /// Create a multisig transaction.
    pub fn create_multisig(
        &mut self,
        account: &Account,
        multisig_address: AccountAddress,
        transaction_payload: Option<MultisigTransactionPayload>,
    ) -> SignedTransaction {
        self.create_transaction_payload(
            account,
            TransactionPayload::Multisig(Multisig {
                multisig_address,
                transaction_payload,
            }),
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

    /// Run the multisig transaction.
    pub fn run_multisig(
        &mut self,
        account: &Account,
        multisig_address: AccountAddress,
        transaction_payload: Option<MultisigTransactionPayload>,
    ) -> TransactionStatus {
        let txn = self.create_multisig(account, multisig_address, transaction_payload);
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

    /// Creates a transaction which publishes the passed already-built Move Package on behalf
    /// of the given account.
    ///
    /// The passed function allows to manipulate the generated metadata for testing purposes.
    pub fn create_publish_built_package(
        &mut self,
        account: &Account,
        package: &BuiltPackage,
        mut patch_metadata: impl FnMut(&mut PackageMetadata),
    ) -> SignedTransaction {
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

    /// Creates a transaction which publishes the passed already-built Move Package to an object,
    /// on behalf of the given account.
    ///
    /// The passed function allows to manipulate the generated metadata for testing purposes.
    pub fn create_object_code_deployment_built_package(
        &mut self,
        account: &Account,
        package: &BuiltPackage,
        mut patch_metadata: impl FnMut(&mut PackageMetadata),
    ) -> SignedTransaction {
        let code = package.extract_code();
        let mut metadata = package
            .extract_metadata()
            .expect("extracting package metadata must succeed");
        patch_metadata(&mut metadata);
        self.create_transaction_payload(
            account,
            aptos_stdlib::object_code_deployment_publish(
                bcs::to_bytes(&metadata).expect("PackageMetadata has BCS"),
                code,
            ),
        )
    }

    /// Creates a transaction which upgrades the passed already-built Move Package,
    /// on behalf of the given account.
    ///
    /// The passed function allows to manipulate the generated for testing purposes.
    pub fn create_object_code_upgrade_built_package(
        &mut self,
        account: &Account,
        package: &BuiltPackage,
        mut patch_metadata: impl FnMut(&mut PackageMetadata),
        code_object: AccountAddress,
    ) -> SignedTransaction {
        let code = package.extract_code();
        let mut metadata = package
            .extract_metadata()
            .expect("extracting package metadata must succeed");
        patch_metadata(&mut metadata);
        self.create_transaction_payload(
            account,
            aptos_stdlib::object_code_deployment_upgrade(
                bcs::to_bytes(&metadata).expect("PackageMetadata has BCS"),
                code,
                code_object,
            ),
        )
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
        patch_metadata: impl FnMut(&mut PackageMetadata),
    ) -> SignedTransaction {
        let package = BuiltPackage::build(path.to_owned(), options.unwrap_or_default())
            .expect("building package must succeed");
        self.create_publish_built_package(account, &package, patch_metadata)
    }

    pub fn create_object_code_upgrade_package(
        &mut self,
        account: &Account,
        path: &Path,
        options: BuildOptions,
        patch_metadata: impl FnMut(&mut PackageMetadata),
        code_object: AccountAddress,
    ) -> SignedTransaction {
        let package =
            build_package(path.to_owned(), options).expect("building package must succeed");
        self.create_object_code_upgrade_built_package(
            account,
            &package,
            patch_metadata,
            code_object,
        )
    }

    pub fn create_object_code_deployment_package(
        &mut self,
        account: &Account,
        path: &Path,
        options: BuildOptions,
        patch_metadata: impl FnMut(&mut PackageMetadata),
    ) -> SignedTransaction {
        let package =
            build_package(path.to_owned(), options).expect("building package must succeed");
        self.create_object_code_deployment_built_package(account, &package, patch_metadata)
    }

    pub fn create_publish_package_cache_building(
        &mut self,
        account: &Account,
        path: &Path,
        patch_metadata: impl FnMut(&mut PackageMetadata),
    ) -> SignedTransaction {
        let package_arc = {
            let mut cache = CACHED_BUILT_PACKAGES.lock().unwrap();

            Arc::clone(cache.entry(path.to_owned()).or_insert_with(|| {
                Arc::new(build_package(path.to_owned(), BuildOptions::default()))
            }))
        };
        let package_ref = package_arc
            .as_ref()
            .as_ref()
            .expect("building package must succeed");
        self.create_publish_built_package(account, package_ref, patch_metadata)
    }

    /// Runs transaction which publishes the Move Package.
    pub fn publish_package_cache_building(
        &mut self,
        account: &Account,
        path: &Path,
    ) -> TransactionStatus {
        let txn = self.create_publish_package_cache_building(account, path, |_| {});
        self.run(txn)
    }

    /// Runs transaction which publishes the Move Package.
    pub fn publish_package(&mut self, account: &Account, path: &Path) -> TransactionStatus {
        let txn = self.create_publish_package(account, path, None, |_| {});
        self.run(txn)
    }

    /// Runs the transaction which publishes the Move Package to an object.
    pub fn object_code_deployment_package(
        &mut self,
        account: &Account,
        path: &Path,
        options: BuildOptions,
    ) -> TransactionStatus {
        let txn = self.create_object_code_deployment_package(account, path, options, |_| {});
        self.run(txn)
    }

    /// Creates a transaction which publishes the passed already-built Move Package to an object,
    /// on behalf of the given account.
    ///
    /// The passed function allows to manipulate the generated metadata for testing purposes.
    pub fn object_code_upgrade_package(
        &mut self,
        account: &Account,
        path: &Path,
        options: BuildOptions,
        code_object: AccountAddress,
    ) -> TransactionStatus {
        let txn =
            self.create_object_code_upgrade_package(account, path, options, |_| {}, code_object);
        self.run(txn)
    }

    /// Marks all the packages in the `code_object` as immutable.
    pub fn object_code_freeze_code_object(
        &mut self,
        account: &Account,
        code_object: AccountAddress,
    ) -> TransactionStatus {
        let txn = self.create_transaction_payload(
            account,
            aptos_stdlib::object_code_deployment_freeze_code_object(code_object),
        );
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
    ) -> (TransactionGasLog, u64, Option<FeeStatement>) {
        let txn = self.create_publish_package(account, path, None, |_| {});
        let (output, gas_log) = self
            .executor
            .execute_transaction_with_gas_profiler(txn)
            .unwrap();
        if matches!(output.status(), TransactionStatus::Keep(_)) {
            self.executor.apply_write_set(output.write_set());
        }
        (
            gas_log,
            output.gas_used(),
            output.try_extract_fee_statement().unwrap(),
        )
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

    pub fn get_events(&self) -> &[ContractEvent] {
        self.executor.get_events()
    }

    pub fn read_state_value(&self, state_key: &StateKey) -> Option<StateValue> {
        self.executor.read_state_value(state_key)
    }

    pub fn read_state_value_bytes(&self, state_key: &StateKey) -> Option<Vec<u8>> {
        self.read_state_value(state_key)
            .map(|val| val.bytes().to_vec())
    }

    /// Reads the raw, serialized data of a resource.
    pub fn read_resource_raw(
        &self,
        addr: &AccountAddress,
        struct_tag: StructTag,
    ) -> Option<Vec<u8>> {
        self.read_state_value_bytes(&StateKey::resource(addr, &struct_tag).unwrap())
    }

    /// Reads the resource data `T`.
    /// WARNING: Does not work with resource groups (because set_resource does not work?).
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
    ) -> Option<StateValueMetadata> {
        self.read_state_value(&StateKey::resource(addr, &struct_tag).unwrap())
            .map(StateValue::into_metadata)
    }

    pub fn read_resource_group_metadata(
        &self,
        addr: &AccountAddress,
        struct_tag: StructTag,
    ) -> Option<StateValueMetadata> {
        self.read_state_value(&StateKey::resource_group(addr, &struct_tag))
            .map(StateValue::into_metadata)
    }

    pub fn read_resource_group(
        &self,
        addr: &AccountAddress,
        struct_tag: StructTag,
    ) -> Option<BTreeMap<StructTag, Vec<u8>>> {
        self.read_state_value_bytes(&StateKey::resource_group(addr, &struct_tag))
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

    pub fn read_aptos_balance(&self, addr: &AccountAddress) -> u64 {
        self.read_resource::<CoinStoreResource<AptosCoinType>>(
            addr,
            CoinStoreResource::<AptosCoinType>::struct_tag(),
        )
        .map(|c| c.coin())
        .unwrap_or(0)
            + self
                .read_resource_from_resource_group::<FungibleStoreResource>(
                    &aptos_types::account_config::fungible_store::primary_apt_store(*addr),
                    ObjectGroupResource::struct_tag(),
                    FungibleStoreResource::struct_tag(),
                )
                .map(|c| c.balance())
                .unwrap_or(0)
    }

    /// Write the resource data `T`.
    /// WARNING: Does not work with resource groups.
    pub fn set_resource<T: Serialize>(
        &mut self,
        addr: AccountAddress,
        struct_tag: StructTag,
        data: &T,
    ) {
        let state_key = StateKey::resource(&addr, &struct_tag).unwrap();
        self.executor
            .write_state_value(state_key, bcs::to_bytes(data).unwrap());
    }

    /// Enables features
    pub fn enable_features(&mut self, enabled: Vec<FeatureFlag>, disabled: Vec<FeatureFlag>) {
        let acc = self.aptos_framework_account();
        let enabled = enabled.into_iter().map(|f| f as u64).collect::<Vec<_>>();
        let disabled = disabled.into_iter().map(|f| f as u64).collect::<Vec<_>>();
        self.executor
            .exec("features", "change_feature_flags_internal", vec![], vec![
                MoveValue::Signer(*acc.address())
                    .simple_serialize()
                    .unwrap(),
                bcs::to_bytes(&enabled).unwrap(),
                bcs::to_bytes(&disabled).unwrap(),
            ]);
    }

    fn override_one_gas_param(&mut self, param: &str, param_value: u64) {
        // TODO: The AptosGasParameters::zeros() schedule doesn't do what we want, so
        // explicitly manipulating gas entries. Wasn't obvious from the gas code how to
        // do this differently then below, so perhaps improve this...
        let entries = AptosGasParameters::initial()
            .to_on_chain_gas_schedule(aptos_gas_schedule::LATEST_GAS_FEATURE_VERSION);
        let entries = entries
            .into_iter()
            .map(|(name, val)| {
                if name == param {
                    (name, param_value)
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
        let core_signer_arg = MoveValue::Signer(AccountAddress::ONE)
            .simple_serialize()
            .unwrap();
        self.executor
            .exec("gas_schedule", "set_for_next_epoch", vec![], vec![
                core_signer_arg.clone(),
                MoveValue::vector_u8(schedule_bytes)
                    .simple_serialize()
                    .unwrap(),
            ]);
        self.executor
            .exec("aptos_governance", "force_end_epoch", vec![], vec![
                core_signer_arg,
            ]);
    }

    pub fn modify_gas_scaling(&mut self, gas_scaling_factor: u64) {
        self.override_one_gas_param("txn.gas_unit_scaling_factor", gas_scaling_factor);
    }

    /// Increase maximal transaction size.
    pub fn increase_transaction_size(&mut self) {
        self.override_one_gas_param("txn.max_transaction_size_in_bytes", 1000 * 1024);
    }

    pub fn sequence_number_opt(&self, addr: &AccountAddress) -> Option<u64> {
        self.read_resource::<AccountResource>(addr, AccountResource::struct_tag())
            .as_ref()
            .map(AccountResource::sequence_number)
    }

    pub fn sequence_number(&self, addr: &AccountAddress) -> u64 {
        self.sequence_number_opt(addr).unwrap()
    }

    fn chain_id_is_mainnet(&self, addr: &AccountAddress) -> bool {
        self.read_resource::<ChainId>(addr, ChainId::struct_tag())
            .unwrap()
            .is_mainnet()
    }

    pub fn modify_gas_schedule_raw(&mut self, modify: impl FnOnce(&mut GasScheduleV2)) {
        let mut gas_schedule = self.get_gas_schedule();
        modify(&mut gas_schedule);
        self.set_resource(
            CORE_CODE_ADDRESS,
            GasScheduleV2::struct_tag(),
            &gas_schedule,
        )
    }

    pub fn modify_gas_schedule(&mut self, modify: impl FnOnce(&mut AptosGasParameters)) {
        let (feature_version, mut gas_params) = self.get_gas_params();
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

    pub fn get_gas_params(&self) -> (u64, AptosGasParameters) {
        let gas_schedule: GasScheduleV2 = self.get_gas_schedule();
        let feature_version = gas_schedule.feature_version;
        let params = AptosGasParameters::from_on_chain_gas_schedule(
            &gas_schedule.into_btree_map(),
            feature_version,
        )
        .unwrap();
        (feature_version, params)
    }

    pub fn get_gas_schedule(&self) -> GasScheduleV2 {
        self.read_resource(&CORE_CODE_ADDRESS, GasScheduleV2::struct_tag())
            .unwrap()
    }

    pub fn set_default_gas_unit_price(&mut self, gas_unit_price: u64) {
        self.default_gas_unit_price = gas_unit_price;
    }

    pub fn execute_view_function(
        &mut self,
        fun: MemberId,
        type_args: Vec<TypeTag>,
        arguments: Vec<Vec<u8>>,
    ) -> ViewFunctionOutput {
        self.executor
            .execute_view_function(fun, type_args, arguments)
    }

    /// Splits transactions into blocks based on passed `block_split``, and
    /// checks whether each transaction aborted based on passed in
    /// move abort code (if >0), or succeeded (if ==0).
    /// `txn_block` is vector of (abort code, transaction) tuples.
    ///
    /// This is useful when testing that different block boundaries
    /// work correctly.
    pub fn run_block_in_parts_and_check(
        &mut self,
        block_split: BlockSplit,
        txn_block: Vec<(u64, SignedTransaction)>,
    ) -> Vec<TransactionOutput> {
        fn run_and_check_block(
            harness: &mut MoveHarness,
            txn_block: Vec<(u64, SignedTransaction)>,
            offset: usize,
        ) -> Vec<TransactionOutput> {
            use crate::assert_abort_ref;

            if txn_block.is_empty() {
                return vec![];
            }
            let (errors, txns): (Vec<_>, Vec<_>) = txn_block.into_iter().unzip();
            println!(
                "=== Running block from {} with {} tnx ===",
                offset,
                txns.len()
            );
            let outputs = harness.run_block_get_output(txns);
            for (idx, (error, output)) in errors.into_iter().zip(outputs.iter()).enumerate() {
                if error == SUCCESS {
                    assert_success!(
                        output.status().clone(),
                        "Didn't succeed on txn {}, with block starting at {}",
                        idx + offset,
                        offset,
                    );
                } else {
                    assert_abort_ref!(
                        output.status(),
                        error,
                        "Error code mismatch on txn {} that should've failed, with block starting at {}. Expected {}, got {:?}",
                        idx + offset,
                        offset,
                        error,
                        output.status(),
                    );
                }
            }
            outputs
        }

        match block_split {
            BlockSplit::Whole => run_and_check_block(self, txn_block, 0),
            BlockSplit::SingleTxnPerBlock => {
                let mut outputs = vec![];
                for (idx, (error, status)) in txn_block.into_iter().enumerate() {
                    outputs.append(&mut run_and_check_block(self, vec![(error, status)], idx));
                }
                outputs
            },
            BlockSplit::SplitIntoThree {
                first_len,
                second_len,
            } => {
                assert!(first_len + second_len <= txn_block.len());
                let (left, rest) = txn_block.split_at(first_len);
                let (mid, right) = rest.split_at(second_len);

                let mut outputs = vec![];
                outputs.append(&mut run_and_check_block(self, left.to_vec(), 0));
                outputs.append(&mut run_and_check_block(self, mid.to_vec(), first_len));
                outputs.append(&mut run_and_check_block(
                    self,
                    right.to_vec(),
                    first_len + second_len,
                ));
                outputs
            },
        }
    }

    pub fn set_max_gas_per_txn(&mut self, max_gas_per_txn: u64) {
        self.max_gas_per_txn = max_gas_per_txn
    }
}

impl BlockSplit {
    pub fn arbitrary(len: usize) -> BoxedStrategy<BlockSplit> {
        // skip last choice if length is not big enough for it.
        (0..(if len > 1 { 3 } else { 2 }))
            .prop_flat_map(move |enum_type| {
                // making running a test with a full block likely
                match enum_type {
                    0 => Just(BlockSplit::Whole).boxed(),
                    1 => Just(BlockSplit::SingleTxnPerBlock).boxed(),
                    _ => {
                        // First is non-empty, and not the whole block here: [1, len)
                        (1usize..len)
                            .prop_flat_map(move |first| {
                                // Second is non-empty, but can finish the block: [1, len - first]
                                (Just(first), 1usize..len - first + 1)
                            })
                            .prop_map(|(first, second)| BlockSplit::SplitIntoThree {
                                first_len: first,
                                second_len: second,
                            })
                            .boxed()
                    },
                }
            })
            .boxed()
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
    ($s:expr $(,)?) => {{
        assert_eq!($s, aptos_types::transaction::TransactionStatus::Keep(
            aptos_types::transaction::ExecutionStatus::Success))
    }};
    ($s:expr, $($arg:tt)+) => {{
        assert_eq!(
            $s,
            aptos_types::transaction::TransactionStatus::Keep(
                aptos_types::transaction::ExecutionStatus::Success),
            $($arg)+
        )
    }};
}

/// Helper to assert transaction resulted in OUT_OF_GAS error
#[macro_export]
macro_rules! assert_out_of_gas {
    ($s:expr $(,)?) => {{
        assert_eq!($s, aptos_types::transaction::TransactionStatus::Keep(
            aptos_types::transaction::ExecutionStatus::OutOfGas))
    }};
    ($s:expr, $($arg:tt)+) => {{
        assert_eq!(
            $s,
            aptos_types::transaction::TransactionStatus::Keep(
                aptos_types::transaction::ExecutionStatus::OutOfGas),
            $($arg)+
        )
    }};
}

/// Helper to assert transaction aborts.
/// TODO merge/replace with assert_abort_ref
#[macro_export]
macro_rules! assert_abort {
    // identity needs to be before pattern (both with and without message),
    // as if we pass variable - it matches the pattern arm, but value is not used, but overridden.
    // Opposite order and test_asserts_variable_used / test_asserts_variable_used_with_message tests
    // would fail
    ($s:expr, $c:ident $(,)?) => {{
        assert!(matches!(
            $s,
            aptos_types::transaction::TransactionStatus::Keep(
                aptos_types::transaction::ExecutionStatus::MoveAbort { code, .. }
            )
            if code == $c,
        ));
    }};
    ($s:expr, $c:pat $(,)?) => {{
        assert!(matches!(
            $s,
            aptos_types::transaction::TransactionStatus::Keep(
                aptos_types::transaction::ExecutionStatus::MoveAbort { code: $c, .. }
            ),
        ));
    }};
    ($s:expr, $c:ident, $($arg:tt)+) => {{
        assert!(
            matches!(
                $s,
                aptos_types::transaction::TransactionStatus::Keep(
                    aptos_types::transaction::ExecutionStatus::MoveAbort { code, .. }
                )
                if code == $c,
            ),
            $($arg)+
        );
    }};
    ($s:expr, $c:pat, $($arg:tt)+) => {{
        assert!(
            matches!(
                $s,
                aptos_types::transaction::TransactionStatus::Keep(
                    aptos_types::transaction::ExecutionStatus::MoveAbort { code: $c, .. }
                ),
            ),
            $($arg)+
        );
    }};
}

/// Helper to assert transaction aborts.
/// Takes reference, as then we can get a better error message.
#[macro_export]
macro_rules! assert_abort_ref {
    // identity needs to be before pattern (both with and without message),
    // as if we pass variable - it matches the pattern arm, but value is not used, but overridden.
    // Opposite order and test_asserts_variable_used / test_asserts_variable_used_with_message tests
    // would fail
    ($s:expr, $c:ident $(,)?) => {{
        claims::assert_matches!(
            $s,
            &aptos_types::transaction::TransactionStatus::Keep(
                aptos_types::transaction::ExecutionStatus::MoveAbort { code, .. }
            )
            if code == $c,
        );
    }};
    ($s:expr, $c:pat $(,)?) => {{
        claims::assert_matches!(
            $s,
            &aptos_types::transaction::TransactionStatus::Keep(
                aptos_types::transaction::ExecutionStatus::MoveAbort { code: $c, .. }
            )
        );
    }};
    ($s:expr, $c:ident, $($arg:tt)+) => {{
        claims::assert_matches!(
            $s,
            &aptos_types::transaction::TransactionStatus::Keep(
                aptos_types::transaction::ExecutionStatus::MoveAbort { code, .. }
            )
            if code == $c,
            $($arg)+
        );
    }};
    ($s:expr, $c:pat, $($arg:tt)+) => {{
        claims::assert_matches!(
            $s,
            &aptos_types::transaction::TransactionStatus::Keep(
                aptos_types::transaction::ExecutionStatus::MoveAbort { code: $c, .. }
            ),
            $($arg)+
        );
    }};
}

/// Helper to assert vm status code.
#[macro_export]
macro_rules! assert_vm_status {
    ($s:expr, $c:expr $(,)?) => {{
        use aptos_types::transaction::*;
        assert_eq!(
            $s,
            TransactionStatus::Keep(ExecutionStatus::MiscellaneousError(Some($c)))
        );
    }};
    ($s:expr, $c:expr, $($arg:tt)+) => {{
        use aptos_types::transaction::*;
        assert_eq!(
            $s,
            TransactionStatus::Keep(ExecutionStatus::MiscellaneousError(Some($c))),
            $($arg)+,
        );
    }};
}

#[macro_export]
macro_rules! assert_move_abort {
    ($s:expr, $c:ident $(,)?) => {{
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
    ($s:expr, $c:ident, $($arg:tt)+) => {{
        use aptos_types::transaction::*;
        assert!(
            match $s {
                TransactionStatus::Keep(ExecutionStatus::MoveAbort {
                    location: _,
                    code: _,
                    info,
                }) => info == $c,
                _ => false,
            },
            $($arg)+
        );
    }};
}

#[cfg(test)]
mod tests {
    use aptos_types::transaction::{ExecutionStatus, TransactionStatus};
    use move_core_types::vm_status::AbortLocation;

    #[test]
    fn test_asserts() {
        let success = TransactionStatus::Keep(ExecutionStatus::Success);

        let abort_13 = TransactionStatus::Keep(ExecutionStatus::MoveAbort {
            code: 13,
            location: AbortLocation::Script,
            info: None,
        });

        assert_success!(success);
        assert_success!(success,);
        assert_success!(success, "success");
        assert_success!(success, "message {}", 0);
        assert_success!(success, "message {}", 0,);

        let x = 13;
        assert_abort!(abort_13, 13);
        assert_abort!(abort_13, 13,);
        assert_abort!(abort_13, x);
        assert_abort!(abort_13, _);
        assert_abort!(abort_13, 13 | 14);
        assert_abort!(abort_13, 13, "abort");
        assert_abort!(abort_13, 13, "abort {}", 0);
        assert_abort!(abort_13, x, "abort");
        assert_abort!(abort_13, x, "abort {}", 0);
        assert_abort!(abort_13, _, "abort");
        assert_abort!(abort_13, 13 | 14, "abort");
        assert_abort!(abort_13, 13 | 14, "abort",);
    }

    #[test]
    #[should_panic]
    fn test_asserts_variable_used() {
        let abort_13 = TransactionStatus::Keep(ExecutionStatus::MoveAbort {
            code: 13,
            location: AbortLocation::Script,
            info: None,
        });

        let x = 14;
        assert_abort!(abort_13, x);
    }

    #[test]
    #[should_panic]
    fn test_asserts_variable_used_with_message() {
        let abort_13 = TransactionStatus::Keep(ExecutionStatus::MoveAbort {
            code: 13,
            location: AbortLocation::Script,
            info: None,
        });

        let x = 14;
        assert_abort!(abort_13, x, "abort");
    }
}
