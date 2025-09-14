// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Support for running the VM to execute and verify transactions.

use crate::{
    account::{Account, AccountData},
    golden_outputs::GoldenOutputs,
};
use aptos_abstract_gas_usage::CalibrationAlgebra;
use aptos_bitvec::BitVec;
use aptos_block_executor::{
    code_cache_global_manager::AptosModuleCacheManager, txn_commit_hook::NoOpTransactionCommitHook,
    txn_provider::default::DefaultTxnProvider,
};
use aptos_crypto::HashValue;
use aptos_framework::ReleaseBundle;
use aptos_gas_algebra::DynamicExpression;
use aptos_gas_meter::{AptosGasMeter, GasAlgebra, StandardGasAlgebra, StandardGasMeter};
use aptos_gas_profiling::{GasProfiler, TransactionGasLog};
use aptos_keygen::KeyGen;
use aptos_rest_client::AptosBaseUrl;
use aptos_transaction_simulation::{
    DeltaStateStore, EitherStateView, EmptyStateView, SimulationStateStore,
    GENESIS_CHANGE_SET_HEAD, GENESIS_CHANGE_SET_MAINNET, GENESIS_CHANGE_SET_TESTNET,
};
use aptos_types::{
    account_config::{
        new_block_event_key, primary_apt_store, AccountResource, CoinInfoResource,
        ConcurrentSupplyResource, FungibleStoreResource, NewBlockEvent, ObjectGroupResource,
        CORE_CODE_ADDRESS,
    },
    block_executor::{
        config::{
            BlockExecutorConfig, BlockExecutorConfigFromOnchain, BlockExecutorLocalConfig,
            BlockExecutorModuleCacheLocalConfig,
        },
        transaction_slice_metadata::TransactionSliceMetadata,
    },
    block_metadata::BlockMetadata,
    chain_id::ChainId,
    contract_event::ContractEvent,
    move_utils::MemberId,
    on_chain_config::{
        AptosVersion, CurrentTimeMicroseconds, FeatureFlag, Features, OnChainConfig, ValidatorSet,
    },
    state_store::{state_key::StateKey, state_value::StateValue, StateView, TStateView},
    transaction::{
        signature_verified_transaction::{
            into_signature_verified_block, SignatureVerifiedTransaction,
        },
        AuxiliaryInfo, BlockOutput, ExecutionStatus, SignedTransaction, Transaction,
        TransactionExecutableRef, TransactionOutput, TransactionStatus, VMValidatorResult,
        ViewFunctionOutput,
    },
    vm_status::VMStatus,
    write_set::{WriteOp, WriteSet, WriteSetMut},
    AptosCoinType, CoinType,
};
use aptos_validator_interface::{DebuggerStateView, RestDebuggerInterface};
use aptos_vm::{
    block_executor::{AptosTransactionOutput, AptosVMBlockExecutorWrapper},
    data_cache::AsMoveResolver,
    gas::make_prod_gas_meter,
    move_vm_ext::{AptosMoveResolver, MoveVmExt, SessionExt, SessionId},
    AptosVM, VMValidator,
};
use aptos_vm_environment::environment::AptosEnvironment;
use aptos_vm_genesis::{generate_genesis_change_set_for_testing_with_count, GenesisOptions};
use aptos_vm_logging::log_schema::AdapterLogSchema;
use aptos_vm_types::{
    module_and_script_storage::{module_storage::AptosModuleStorage, AsAptosCodeStorage},
    module_write_set::ModuleWriteSet,
    resolver::NoopBlockSynchronizationKillSwitch,
    storage::change_set_configs::ChangeSetConfigs,
};
use bytes::Bytes;
use move_core_types::{
    account_address::AccountAddress,
    identifier::Identifier,
    language_storage::{ModuleId, StructTag, TypeTag},
    move_resource::{MoveResource, MoveStructType},
    value::MoveValue,
};
use move_vm_runtime::module_traversal::{TraversalContext, TraversalStorage};
use move_vm_types::gas::UnmeteredGasMeter;
use serde::Serialize;
use std::{
    collections::{BTreeMap, BTreeSet},
    env,
    fs::{self, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
    str::FromStr,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

static RNG_SEED: [u8; 32] = [9u8; 32];

const ENV_TRACE_DIR: &str = "TRACE";

// Enables running parallel, in addition to sequential, in a
// BothComparison mode.
const ENV_ENABLE_PARALLEL: &str = "E2E_PARALLEL_EXEC";

/// Directory structure of the trace dir
pub const TRACE_FILE_NAME: &str = "name";
pub const TRACE_FILE_ERROR: &str = "error";
pub const TRACE_DIR_META: &str = "meta";
pub const TRACE_DIR_DATA: &str = "data";
pub const TRACE_DIR_INPUT: &str = "input";
pub const TRACE_DIR_OUTPUT: &str = "output";

const POSTFIX: &str = "_should_error";

/// Maps block number N to the index of the input and output transactions
pub type TraceSeqMapping = (usize, Vec<usize>, Vec<usize>);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ExecutorMode {
    SequentialOnly,
    ParallelOnly,
    // Runs sequential, then parallel, and compares outputs.
    BothComparison,
}

type FakeExecutorStateStore = DeltaStateStore<EitherStateView<EmptyStateView, DebuggerStateView>>;

fn empty_in_memory_state_store() -> FakeExecutorStateStore {
    DeltaStateStore::new_with_base(EitherStateView::Left(EmptyStateView))
}

/// Provides an environment to run a VM instance.
///
/// This struct is a mock in-memory implementation of the Aptos executor.
pub struct FakeExecutor {
    state_store: FakeExecutorStateStore,
    event_store: Vec<ContractEvent>,
    executor_thread_pool: Arc<rayon::ThreadPool>,
    block_time: u64,
    executed_output: Option<GoldenOutputs>,
    trace_dir: Option<PathBuf>,
    rng: KeyGen,
    /// If set, determines whether to execute a comparison test with the parallel block executor.
    /// If not set, environment variable E2E_PARALLEL_EXEC must be set
    /// s.t. the comparison test is executed (BothComparison).
    executor_mode: Option<ExecutorMode>,
    allow_block_executor_fallback: bool,
}

pub enum GasMeterType {
    RegularGasMeter,
    UnmeteredGasMeter,
}

#[derive(Clone)]
pub struct Measurement {
    elapsed: Duration,
    /// In internal gas units
    execution_gas: u64,
    /// In internal gas units
    io_gas: u64,
}

const GAS_SCALING_FACTOR: f64 = 1_000_000.0;

impl Measurement {
    pub fn elapsed_micros(&self) -> u128 {
        self.elapsed.as_micros()
    }

    pub fn elapsed_secs_f64(&self) -> f64 {
        self.elapsed.as_secs_f64()
    }

    pub fn elapsed_micros_f64(&self) -> f64 {
        self.elapsed.as_secs_f64() * 1_000_000.0
    }

    pub fn execution_gas_units(&self) -> f64 {
        self.execution_gas as f64 / GAS_SCALING_FACTOR
    }

    pub fn io_gas_units(&self) -> f64 {
        self.io_gas as f64 / GAS_SCALING_FACTOR
    }
}

pub enum ExecFuncTimerDynamicArgs {
    NoArgs,
    DistinctSigners,
    DistinctSignersAndFixed(Vec<AccountAddress>),
}

impl FakeExecutor {
    /// Creates an executor from a genesis [`WriteSet`].
    pub fn from_genesis(write_set: &WriteSet, chain_id: ChainId) -> Self {
        let executor_thread_pool = Arc::new(
            rayon::ThreadPoolBuilder::new()
                .num_threads(num_cpus::get())
                .build()
                .unwrap(),
        );

        let state_store = empty_in_memory_state_store();
        state_store.set_chain_id(chain_id).unwrap();

        let mut executor = FakeExecutor {
            state_store,
            event_store: Vec::new(),
            executor_thread_pool,
            block_time: 0,
            executed_output: None,
            trace_dir: None,
            rng: KeyGen::from_seed(RNG_SEED),
            executor_mode: None,
            allow_block_executor_fallback: true,
        };
        executor.apply_write_set(write_set);
        executor
    }

    #[cfg(any(test, feature = "fuzzing"))]
    pub fn from_genesis_with_existing_thread_pool(
        write_set: &WriteSet,
        chain_id: ChainId,
        executor_thread_pool: Arc<rayon::ThreadPool>,
    ) -> Self {
        let state_store = empty_in_memory_state_store();
        state_store.set_chain_id(chain_id).unwrap();

        let mut executor = FakeExecutor {
            state_store,
            event_store: Vec::new(),
            executor_thread_pool,
            block_time: 0,
            executed_output: None,
            trace_dir: None,
            rng: KeyGen::from_seed(RNG_SEED),
            executor_mode: None,
            allow_block_executor_fallback: true,
        };
        executor.apply_write_set(write_set);
        executor
    }

    fn from_remote_state_impl(
        network_url: AptosBaseUrl,
        txn_id: u64,
        api_key: Option<&str>,
    ) -> Self {
        let mut builder = aptos_rest_client::Client::builder(network_url);
        if let Some(api_key) = api_key {
            builder = builder
                .api_key(api_key)
                .expect("failed to configure API key")
        }
        let rest_client = builder.build();

        let debugger = Arc::new(RestDebuggerInterface::new(rest_client));
        let debugger_state_view = DebuggerStateView::new(debugger, txn_id);
        let state_store =
            DeltaStateStore::new_with_base(EitherStateView::Right(debugger_state_view));

        let timestamp = state_store
            .get_on_chain_config::<CurrentTimeMicroseconds>()
            .expect("failed to get block time from remote");

        let executor_thread_pool = Arc::new(
            rayon::ThreadPoolBuilder::new()
                .num_threads(num_cpus::get())
                .build()
                .unwrap(),
        );

        Self {
            state_store,
            event_store: Vec::new(),
            executor_thread_pool,
            block_time: timestamp.microseconds,
            executed_output: None,
            trace_dir: None,
            rng: KeyGen::from_seed(RNG_SEED),
            executor_mode: None,
            allow_block_executor_fallback: true,
        }
    }

    /// Creates a [`FakeExecutor`] from a remote network state at the version specified by the
    /// transaction id, with support for a custom API key to access node APIs.
    ///
    /// Simulations based on remote states rely heavily on API calls, which can easily run into
    /// rate limits if executed repeatedly or in parallel.
    /// Providing an API key raises these limits significantly.
    ///
    /// If you hit rate limits, you can create a free Aptos Build account and generate an API key:
    /// - https://build.aptoslabs.com/docs/start#api-quick-start
    pub fn from_remote_state_with_api_key(
        network_url: AptosBaseUrl,
        txn_id: u64,
        api_key: &str,
    ) -> Self {
        Self::from_remote_state_impl(network_url, txn_id, Some(api_key))
    }

    /// Creates a [`FakeExecutor`] from a remote network state at the version specified by the
    /// transaction id.
    pub fn from_remote_state(network_url: AptosBaseUrl, txn_id: u64) -> Self {
        Self::from_remote_state_impl(network_url, txn_id, None)
    }

    pub fn set_executor_mode(mut self, mode: ExecutorMode) -> Self {
        self.executor_mode = Some(mode);
        self
    }

    /// Configure this executor to not use parallel execution. By default, parallel execution is
    /// enabled if E2E_PARALLEL_EXEC is set. This overrides the default.
    pub fn set_not_parallel(self) -> Self {
        self.set_executor_mode(ExecutorMode::SequentialOnly)
    }

    /// Configure this executor to use parallel execution. By default, parallel execution is
    /// enabled if E2E_PARALLEL_EXEC is set. This overrides the default.
    pub fn set_parallel(self) -> Self {
        self.set_executor_mode(ExecutorMode::BothComparison)
    }

    pub fn disable_block_executor_fallback(&mut self) {
        self.allow_block_executor_fallback = false;
    }

    /// Creates an executor from the genesis file GENESIS_FILE_LOCATION
    pub fn from_head_genesis() -> Self {
        Self::from_genesis(GENESIS_CHANGE_SET_HEAD.clone().write_set(), ChainId::test())
    }

    /// Creates an executor from the genesis file GENESIS_FILE_LOCATION
    pub fn from_head_genesis_with_count(count: u64) -> Self {
        Self::from_genesis(
            generate_genesis_change_set_for_testing_with_count(GenesisOptions::Head, count)
                .write_set(),
            ChainId::test(),
        )
    }

    /// Creates an executor using the standard genesis.
    pub fn from_testnet_genesis() -> Self {
        Self::from_genesis(
            GENESIS_CHANGE_SET_TESTNET.clone().write_set(),
            ChainId::testnet(),
        )
    }

    /// Creates an executor using the mainnet genesis.
    pub fn from_mainnet_genesis() -> Self {
        Self::from_genesis(
            GENESIS_CHANGE_SET_MAINNET.clone().write_set(),
            ChainId::mainnet(),
        )
    }

    pub fn state_store(&self) -> &(impl SimulationStateStore + use<>) {
        &self.state_store
    }

    /// Creates an executor in which no genesis state has been applied yet.
    pub fn no_genesis() -> Self {
        let executor_thread_pool = Arc::new(
            rayon::ThreadPoolBuilder::new()
                .num_threads(num_cpus::get())
                .build()
                .unwrap(),
        );
        FakeExecutor {
            state_store: empty_in_memory_state_store(),
            event_store: Vec::new(),
            executor_thread_pool,
            block_time: 0,
            executed_output: None,
            trace_dir: None,
            rng: KeyGen::from_seed(RNG_SEED),
            executor_mode: None,
            allow_block_executor_fallback: true,
        }
    }

    pub fn set_golden_file(&mut self, test_name: &str) {
        // 'test_name' includes ':' in the names, lets re-write these to be '_'s so that these
        // files can persist on windows machines.
        let file_name = test_name.replace(':', "_");
        self.executed_output = Some(GoldenOutputs::new(&file_name));
        self.set_tracing(test_name, file_name)
    }

    pub fn set_golden_file_at(&mut self, path: &str, test_name: &str) {
        // 'test_name' includes ':' in the names, lets re-write these to be '_'s so that these
        // files can persist on windows machines.
        let file_name = test_name.replace(':', "_");
        self.executed_output = Some(GoldenOutputs::new_at_path(PathBuf::from(path), &file_name));
        self.set_tracing(test_name, file_name)
    }

    fn set_tracing(&mut self, test_name: &str, file_name: String) {
        // NOTE: tracing is only available when
        //  - the e2e test outputs a golden file, and
        //  - the environment variable is properly set
        if let Some(env_trace_dir) = env::var_os(ENV_TRACE_DIR) {
            let aptos_version =
                AptosVersion::fetch_config(&self.state_store).map_or(0, |v| v.major);

            let trace_dir = Path::new(&env_trace_dir).join(file_name);
            if trace_dir.exists() {
                fs::remove_dir_all(&trace_dir).expect("Failed to clean up the trace directory");
            }
            fs::create_dir_all(&trace_dir).expect("Failed to create the trace directory");
            let mut name_file = OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(trace_dir.join(TRACE_FILE_NAME))
                .unwrap();
            write!(name_file, "{}::{}", test_name, aptos_version).unwrap();
            for sub_dir in &[
                TRACE_DIR_META,
                TRACE_DIR_DATA,
                TRACE_DIR_INPUT,
                TRACE_DIR_OUTPUT,
            ] {
                fs::create_dir(trace_dir.join(sub_dir)).unwrap_or_else(|err| {
                    panic!("Failed to create <trace>/{} directory: {}", sub_dir, err)
                });
            }
            self.trace_dir = Some(trace_dir);
        }
    }

    /// Creates an executor with only the standard library Move modules published and not other
    /// initialization done.
    pub fn stdlib_only_genesis() -> Self {
        let mut genesis = Self::no_genesis();
        for (bytes, module) in
            aptos_cached_packages::head_release_bundle().code_and_compiled_modules()
        {
            let id = module.self_id();
            genesis.add_module(&id, bytes.to_vec());
        }
        genesis
    }

    /// Creates fresh genesis from the framework passed in.
    pub fn custom_genesis(framework: &ReleaseBundle, validator_accounts: Option<usize>) -> Self {
        let genesis = aptos_vm_genesis::generate_test_genesis(framework, validator_accounts);
        Self::from_genesis(genesis.0.write_set(), ChainId::test())
    }

    /// Create one instance of [`AccountData`] without saving it to data store.
    pub fn create_raw_account(&mut self) -> Account {
        Account::new_from_seed(&mut self.rng)
    }

    /// Create one instance of [`AccountData`] without saving it to data store.
    pub fn create_raw_account_data(&mut self, balance: u64, seq_num: u64) -> AccountData {
        AccountData::new_from_seed(&mut self.rng, balance, seq_num)
    }

    /// Creates a number of [`Account`] instances all with the same balance and sequence number,
    /// and publishes them to this executor's data store.
    pub fn create_accounts(&mut self, size: usize, balance: u64, seq_num: u64) -> Vec<Account> {
        let mut accounts: Vec<Account> = Vec::with_capacity(size);
        for _i in 0..size {
            let account_data = AccountData::new_from_seed(&mut self.rng, balance, seq_num);
            self.add_account_data(&account_data);
            accounts.push(account_data.into_account());
        }
        accounts
    }

    /// Creates an account for the given static address. This address needs to be static so
    /// we can load regular Move code to there without need to rewrite code addresses.
    pub fn new_account_at(&mut self, addr: AccountAddress) -> Account {
        let data = self.new_account_data_at(addr);
        data.account().clone()
    }

    pub fn new_account_data_at(&mut self, addr: AccountAddress) -> AccountData {
        // The below will use the genesis keypair but that should be fine.
        let acc = Account::new_genesis_account(addr);

        // Mint the account 10M Aptos coins (with 8 decimals).
        self.store_and_fund_account(acc, 1_000_000_000_000_000, 0)
    }

    pub fn store_and_fund_account(
        &mut self,
        account: Account,
        balance: u64,
        seq_num: u64,
    ) -> AccountData {
        let features = Features::fetch_config(&self.state_store).unwrap_or_default();
        let use_fa_balance = features.is_enabled(FeatureFlag::NEW_ACCOUNTS_DEFAULT_TO_FA_APT_STORE);
        let use_concurrent_balance =
            features.is_enabled(FeatureFlag::DEFAULT_TO_CONCURRENT_FUNGIBLE_BALANCE);

        // Mint the account 10M Aptos coins (with 8 decimals).
        let data = AccountData::with_account(
            account,
            balance,
            seq_num,
            use_fa_balance,
            use_concurrent_balance,
        );
        self.add_account_data(&data);
        data
    }

    /// Applies a [`WriteSet`] to this executor's data store.
    pub fn apply_write_set(&mut self, write_set: &WriteSet) {
        self.state_store.apply_write_set(write_set).unwrap();
    }

    pub fn append_events(&mut self, events: Vec<ContractEvent>) {
        self.event_store.extend(events);
    }

    /// Adds an account to this executor's data store.
    pub fn add_account_data(&mut self, account_data: &AccountData) {
        self.state_store.add_account_data(account_data).unwrap();
        // When a new account data with balance is initialized. The total_supply should be updated
        // correspondingly to be consistent with the global state.
        // if new_added_supply = 0, it is a noop.

        if let Some(new_added_supply) = account_data.coin_balance() {
            if new_added_supply != 0 {
                let coin_info_resource = self
                    .read_apt_coin_info_resource()
                    .expect("coin info must exist in data store");
                let old_supply = self.read_coin_supply().unwrap();
                self.state_store
                    .apply_write_set(
                        &coin_info_resource
                            .to_writeset(old_supply + (new_added_supply as u128))
                            .unwrap(),
                    )
                    .unwrap();
            }
        }

        if let Some(new_added_supply) = account_data.fungible_balance() {
            if new_added_supply != 0 {
                let mut fa_resource_group = self
                    .read_resource_group::<ObjectGroupResource>(&AccountAddress::TEN)
                    .expect("resource group must exist in data store");
                let mut supply = bcs::from_bytes::<ConcurrentSupplyResource>(
                    fa_resource_group
                        .group
                        .get(&ConcurrentSupplyResource::struct_tag())
                        .unwrap(),
                )
                .unwrap();
                supply
                    .current
                    .set(supply.current.get() + new_added_supply as u128);
                fa_resource_group
                    .group
                    .insert(
                        ConcurrentSupplyResource::struct_tag(),
                        bcs::to_bytes(&supply).unwrap(),
                    )
                    .unwrap();
                self.state_store
                    .apply_write_set(
                        &WriteSetMut::new(vec![(
                            StateKey::resource_group(
                                &AccountAddress::TEN,
                                &ObjectGroupResource::struct_tag(),
                            ),
                            WriteOp::legacy_modification(
                                bcs::to_bytes(&fa_resource_group).unwrap().into(),
                            ),
                        )])
                        .freeze()
                        .unwrap(),
                    )
                    .unwrap();
            }
        }
    }

    /// Adds a module to this executor's data store.
    ///
    /// Does not do any sort of verification on the module.
    pub fn add_module(&mut self, module_id: &ModuleId, module_blob: Vec<u8>) {
        self.state_store
            .add_module_blob(module_id, module_blob)
            .unwrap()
    }

    /// Reads the resource `Value` for an account from this executor's data store.
    pub fn read_account_resource(&self, account: &Account) -> Option<AccountResource> {
        self.read_account_resource_at_address(account.address())
    }

    pub fn read_resource<T: MoveResource>(&self, addr: &AccountAddress) -> Option<T> {
        let data_blob = TStateView::get_state_value_bytes(
            &self.state_store,
            &StateKey::resource_typed::<T>(addr).expect("failed to create StateKey"),
        )
        .expect("account must exist in data store")
        .unwrap_or_else(|| panic!("Can't fetch {} resource for {}", T::STRUCT_NAME, addr));
        bcs::from_bytes(&data_blob).ok()
    }

    pub fn read_resource_group<T: MoveResource>(&self, addr: &AccountAddress) -> Option<T> {
        let data_blob = TStateView::get_state_value_bytes(
            &self.state_store,
            &StateKey::resource_group(addr, &T::struct_tag()),
        )
        .expect("account must exist in data store")
        .unwrap_or_else(|| panic!("Can't fetch {} resource group for {}", T::STRUCT_NAME, addr));
        bcs::from_bytes(&data_blob).ok()
    }

    pub fn read_resource_from_group<T: MoveResource>(
        &self,
        addr: &AccountAddress,
        resource_group_tag: &StructTag,
    ) -> Option<T> {
        let bytes_opt = TStateView::get_state_value_bytes(
            &self.state_store,
            &StateKey::resource_group(addr, resource_group_tag),
        )
        .expect("account must exist in data store");

        let group: Option<BTreeMap<StructTag, Vec<u8>>> = bytes_opt
            .map(|bytes| bcs::from_bytes(&bytes))
            .transpose()
            .unwrap();
        group
            .and_then(|g| g.get(&T::struct_tag()).map(|b| bcs::from_bytes(b)))
            .transpose()
            .unwrap()
    }

    /// Reads the resource `Value` for an account under the given address from
    /// this executor's data store.
    pub fn read_account_resource_at_address(
        &self,
        addr: &AccountAddress,
    ) -> Option<AccountResource> {
        self.read_resource(addr)
    }

    /// Reads the CoinStore resource value for an account from this executor's data store.
    pub fn read_apt_fungible_store_resource(
        &self,
        account: &Account,
    ) -> Option<FungibleStoreResource> {
        self.read_resource_from_group(
            &primary_apt_store(*account.address()),
            &ObjectGroupResource::struct_tag(),
        )
    }

    /// Reads supply from CoinInfo resource value from this executor's data store.
    pub fn read_coin_supply(&mut self) -> Option<u128> {
        let bytes = self
            .execute_view_function(
                str::parse("0x1::coin::supply").unwrap(),
                vec![move_core_types::language_storage::TypeTag::from_str(
                    "0x1::aptos_coin::AptosCoin",
                )
                .unwrap()],
                vec![],
            )
            .values
            .unwrap()
            .pop()
            .unwrap();
        bcs::from_bytes::<Option<u128>>(bytes.as_slice()).unwrap()
    }

    /// Reads the CoinInfo resource value from this executor's data store.
    pub fn read_apt_coin_info_resource(&self) -> Option<CoinInfoResource<AptosCoinType>> {
        self.read_resource(&AptosCoinType::coin_info_address())
    }

    /// Executes the given block of transactions.
    ///
    /// Typical tests will call this method and check that the output matches what was expected.
    /// However, this doesn't apply the results of successful transactions to the data store.
    pub fn execute_block(
        &self,
        txn_block: Vec<SignedTransaction>,
    ) -> Result<Vec<TransactionOutput>, VMStatus> {
        self.execute_transaction_block(
            txn_block
                .into_iter()
                .map(Transaction::UserTransaction)
                .collect(),
        )
    }

    /// Executes the transaction as a singleton block and applies the resulting write set to the
    /// data store. Panics if execution fails
    pub fn execute_and_apply(&mut self, transaction: SignedTransaction) -> TransactionOutput {
        let mut outputs = self.execute_block(vec![transaction]).unwrap();
        assert!(outputs.len() == 1, "transaction outputs size mismatch");
        let output = outputs.pop().unwrap();
        match output.status() {
            TransactionStatus::Keep(status) => {
                self.apply_write_set(output.write_set());
                assert_eq!(
                    status,
                    &ExecutionStatus::Success,
                    "transaction failed with {:?}",
                    status
                );
                output
            },
            TransactionStatus::Discard(status) => panic!("transaction discarded with {:?}", status),
            TransactionStatus::Retry => panic!("transaction status is retry"),
        }
    }

    fn execute_transaction_block_impl_with_state_view(
        &self,
        txn_block: Vec<SignatureVerifiedTransaction>,
        onchain_config: BlockExecutorConfigFromOnchain,
        sequential: bool,
        state_view: &(impl StateView + Sync),
    ) -> Result<Vec<TransactionOutput>, VMStatus> {
        let config = BlockExecutorConfig {
            local: BlockExecutorLocalConfig {
                blockstm_v2: false,
                concurrency_level: if sequential {
                    1
                } else {
                    usize::min(4, num_cpus::get())
                },
                allow_fallback: self.allow_block_executor_fallback,
                discard_failed_blocks: false,
                module_cache_config: BlockExecutorModuleCacheLocalConfig::default(),
            },
            onchain: onchain_config,
        };
        let txn_provider = DefaultTxnProvider::new_without_info(txn_block);
        AptosVMBlockExecutorWrapper::execute_block_on_thread_pool::<
            _,
            NoOpTransactionCommitHook<AptosTransactionOutput, VMStatus>,
            _,
        >(
            self.executor_thread_pool.clone(),
            &txn_provider,
            &state_view,
            // Do not use shared module caches in tests.
            &AptosModuleCacheManager::new(),
            config,
            TransactionSliceMetadata::unknown(),
            None,
        )
        .map(BlockOutput::into_transaction_outputs_forced)
    }

    pub fn execute_transaction_block_with_state_view(
        &self,
        txn_block: Vec<Transaction>,
        state_view: &(impl StateView + Sync),
    ) -> Result<Vec<TransactionOutput>, VMStatus> {
        let mut trace_map: (usize, Vec<usize>, Vec<usize>) = TraceSeqMapping::default();

        // dump serialized transaction details before execution, if tracing
        /*
        if let Some(trace_dir) = &self.trace_dir {
            let trace_data_dir = trace_dir.join(TRACE_DIR_DATA);
            trace_map.0 = Self::trace(trace_data_dir.as_path(), self.get_state_view());
            let trace_input_dir = trace_dir.join(TRACE_DIR_INPUT);
            for txn in &txn_block {
                let input_seq = Self::trace(trace_input_dir.as_path(), txn);
                trace_map.1.push(input_seq);
            }
        }
        */

        let sig_verified_block = into_signature_verified_block(txn_block);

        let mode = self.executor_mode.unwrap_or_else(|| {
            if env::var(ENV_ENABLE_PARALLEL).is_ok() {
                ExecutorMode::BothComparison
            } else {
                ExecutorMode::SequentialOnly
            }
        });

        // TODO fetch values from state?
        let onchain_config = BlockExecutorConfigFromOnchain::on_but_large_for_test();

        let sequential_output = if mode != ExecutorMode::ParallelOnly {
            Some(self.execute_transaction_block_impl_with_state_view(
                sig_verified_block.clone(),
                onchain_config.clone(),
                true,
                state_view,
            ))
        } else {
            None
        };

        let parallel_output = if mode != ExecutorMode::SequentialOnly {
            Some(self.execute_transaction_block_impl_with_state_view(
                sig_verified_block,
                onchain_config,
                false,
                state_view,
            ))
        } else {
            None
        };

        if mode == ExecutorMode::BothComparison {
            let sequential_output = sequential_output.as_ref().unwrap();
            let parallel_output = parallel_output.as_ref().unwrap();

            // make more granular comparison, to be able to understand test failures better
            if sequential_output.is_ok() && parallel_output.is_ok() {
                let txns_output_1 = sequential_output.as_ref().unwrap();
                let txns_output_2 = parallel_output.as_ref().unwrap();
                assert_outputs_equal(txns_output_1, "sequential", txns_output_2, "parallel");
            } else {
                assert_eq!(sequential_output, parallel_output, "Output mismatch");
            }
        }

        let output = sequential_output.or(parallel_output).unwrap();

        if let Some(logger) = &self.executed_output {
            logger.log(format!("{:#?}\n", output).as_str());
        }

        // dump serialized transaction output after execution, if tracing
        if let Some(trace_dir) = &self.trace_dir {
            match &output {
                Ok(results) => {
                    let trace_output_dir = trace_dir.join(TRACE_DIR_OUTPUT);
                    for res in results {
                        let output_seq = Self::trace(trace_output_dir.as_path(), res);
                        trace_map.2.push(output_seq);
                    }
                },
                Err(e) => {
                    let mut error_file = OpenOptions::new()
                        .write(true)
                        .create_new(true)
                        .open(trace_dir.join(TRACE_FILE_ERROR))
                        .unwrap();
                    error_file.write_all(e.to_string().as_bytes()).unwrap();
                },
            }
            let trace_meta_dir = trace_dir.join(TRACE_DIR_META);
            Self::trace(trace_meta_dir.as_path(), &trace_map);
        }
        output
    }

    pub fn execute_transaction_block(
        &self,
        txn_block: Vec<Transaction>,
    ) -> Result<Vec<TransactionOutput>, VMStatus> {
        self.execute_transaction_block_with_state_view(txn_block, &self.state_store)
    }

    pub fn execute_transaction(&self, txn: SignedTransaction) -> TransactionOutput {
        let txn_block = vec![txn];
        let mut outputs = self
            .execute_block(txn_block)
            .expect("The VM should not fail to startup");
        outputs
            .pop()
            .expect("A block with one transaction should have one output")
    }

    pub fn execute_transaction_with_gas_profiler(
        &self,
        txn: SignedTransaction,
        auxiliary_info: &AuxiliaryInfo,
    ) -> anyhow::Result<(TransactionOutput, TransactionGasLog)> {
        let txn = txn
            .check_signature()
            .expect("invalid signature for transaction");

        let log_context = AdapterLogSchema::new(self.state_store.id(), 0);

        // TODO(Gas): revisit this.
        let env = AptosEnvironment::new(&self.state_store);
        let vm = AptosVM::new(&env, self.get_state_view());

        let resolver = self.state_store.as_move_resolver();
        let code_storage = self.get_state_view().as_aptos_code_storage(&env);

        let (_status, output, gas_profiler) = vm.execute_user_transaction_with_modified_gas_meter(
            &resolver,
            &code_storage,
            &txn,
            &log_context,
            |gas_meter| {
                let gas_profiler = match txn.payload().executable_ref() {
                    Ok(TransactionExecutableRef::Script(_)) => GasProfiler::new_script(gas_meter),
                    Ok(TransactionExecutableRef::EntryFunction(entry_func))
                        if !txn.payload().is_multisig() =>
                    {
                        GasProfiler::new_function(
                            gas_meter,
                            entry_func.module().clone(),
                            entry_func.function().to_owned(),
                            entry_func.ty_args().to_vec(),
                        )
                    },
                    Ok(_) => unimplemented!("multisig or empty payload not supported yet"),
                    Err(_) => unimplemented!("payload type is deprecated"),
                };
                gas_profiler
            },
            auxiliary_info,
        )?;

        Ok((
            output.try_materialize_into_transaction_output(&resolver)?,
            gas_profiler.finish(),
        ))
    }

    fn trace<P: AsRef<Path>, T: Serialize>(dir: P, item: &T) -> usize {
        let dir = dir.as_ref();
        let seq = fs::read_dir(dir).expect("Unable to read trace dir").count();
        let bytes = bcs::to_bytes(item)
            .unwrap_or_else(|err| panic!("Failed to serialize the trace item: {:?}", err));
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(dir.join(seq.to_string()))
            .expect("Unable to create a trace file");
        file.write_all(&bytes)
            .expect("Failed to write to the trace file");
        seq
    }

    pub fn get_events(&self) -> &[ContractEvent] {
        self.event_store.as_slice()
    }

    pub fn read_state_value(&self, state_key: &StateKey) -> Option<StateValue> {
        TStateView::get_state_value(&self.state_store, state_key).unwrap()
    }

    /// Get the blob for the associated AccessPath
    pub fn read_state_value_bytes(&self, state_key: &StateKey) -> Option<Bytes> {
        TStateView::get_state_value_bytes(&self.state_store, state_key).unwrap()
    }

    /// Set the blob for the associated AccessPath
    pub fn write_state_value(&mut self, state_key: StateKey, data_blob: Vec<u8>) {
        self.state_store
            .set_state_value(state_key, StateValue::new_legacy(data_blob.into()))
            .unwrap();
    }

    /// Validates the given transaction by running it through the VM validator.
    pub fn validate_transaction(&self, txn: SignedTransaction) -> VMValidatorResult {
        let env = AptosEnvironment::new(&self.state_store);
        let vm = AptosVM::new(&env, self.get_state_view());
        vm.validate_transaction(
            txn,
            &self.state_store,
            &self.state_store.as_aptos_code_storage(&env),
        )
    }

    pub fn get_state_view(&self) -> &(impl StateView + use<>) {
        &self.state_store
    }

    pub fn new_block(&mut self) {
        self.new_block_with_timestamp(self.block_time + 1);
    }

    pub fn new_block_with_timestamp(&mut self, time_microseconds: u64) {
        self.block_time = time_microseconds;

        let validator_set = ValidatorSet::fetch_config(&self.state_store)
            .expect("Unable to retrieve the validator set from storage");
        let proposer = *validator_set.payload().next().unwrap().account_address();
        // when updating time, proposer cannot be ZERO.
        self.new_block_with_metadata(proposer, vec![])
    }

    pub fn run_block_with_metadata(
        &mut self,
        proposer: AccountAddress,
        failed_proposer_indices: Vec<u32>,
        txns: Vec<SignedTransaction>,
    ) -> Vec<(TransactionStatus, u64)> {
        let mut txn_block: Vec<Transaction> =
            txns.into_iter().map(Transaction::UserTransaction).collect();
        let validator_set = ValidatorSet::fetch_config(&self.state_store)
            .expect("Unable to retrieve the validator set from storage");
        let new_block_metadata = BlockMetadata::new(
            HashValue::zero(),
            0,
            0,
            proposer,
            BitVec::with_num_bits(validator_set.num_validators() as u16).into(),
            failed_proposer_indices,
            self.block_time,
        );
        txn_block.insert(0, Transaction::BlockMetadata(new_block_metadata));

        let outputs = self
            .execute_transaction_block(txn_block)
            .expect("Must execute transactions");

        // Check if we emit the expected event for block metadata, there might be more events for transaction fees.
        let event = outputs[0].events()[0]
            .v1()
            .expect("The first event must be a block metadata v0 event")
            .clone();
        assert_eq!(event.key(), &new_block_event_key());
        assert!(bcs::from_bytes::<NewBlockEvent>(event.event_data()).is_ok());

        let mut results = vec![];
        for output in outputs {
            if !output.status().is_discarded() {
                self.apply_write_set(output.write_set());
            }
            results.push((output.status().clone(), output.gas_used()));
        }
        results
    }

    pub fn new_block_with_metadata(
        &mut self,
        proposer: AccountAddress,
        failed_proposer_indices: Vec<u32>,
    ) {
        self.run_block_with_metadata(proposer, failed_proposer_indices, vec![]);
    }

    fn module(name: &str) -> ModuleId {
        ModuleId::new(CORE_CODE_ADDRESS, Identifier::new(name).unwrap())
    }

    fn name(name: &str) -> Identifier {
        Identifier::new(name).unwrap()
    }

    pub fn set_block_time(&mut self, new_block_time: u64) {
        self.block_time = new_block_time;
    }

    pub fn get_block_time(&mut self) -> u64 {
        self.block_time
    }

    pub fn get_block_time_seconds(&mut self) -> u64 {
        self.block_time / 1_000_000
    }

    pub fn get_chain_id(&self) -> ChainId {
        self.state_store.get_chain_id().unwrap()
    }

    /// exec_func_record_running_time is like exec(), however, we can run a Module published under
    /// the creator address instead of 0x1, as what is currently done in exec.
    /// Additionally we have dynamic_args and gas_meter_type to configure it further.
    pub fn exec_func_record_running_time(
        &mut self,
        module: &ModuleId,
        function_name: &str,
        type_params: Vec<TypeTag>,
        args: Vec<Vec<u8>>,
        num_measured_iterations: u64,
        dynamic_args: ExecFuncTimerDynamicArgs,
        gas_meter_type: GasMeterType,
    ) -> Measurement {
        // First few runs will not be recorded: this ensures modules used for execution are cached.
        const NUM_WARM_UP_RUNS: u64 = 1;

        let mut extra_accounts = match &dynamic_args {
            ExecFuncTimerDynamicArgs::DistinctSigners
            | ExecFuncTimerDynamicArgs::DistinctSignersAndFixed(_) => (0..num_measured_iterations
                + NUM_WARM_UP_RUNS)
                .map(|_| *self.new_account_at(AccountAddress::random()).address())
                .collect::<Vec<_>>(),
            _ => vec![],
        };

        let env = AptosEnvironment::new(&self.state_store);
        let resolver = self.state_store.as_move_resolver();
        let vm = MoveVmExt::new(&env);
        let module_storage = self.state_store.as_aptos_code_storage(&env);

        let mut i = 0;
        let mut measurements = Vec::new();

        while i < num_measured_iterations + NUM_WARM_UP_RUNS {
            let mut session = vm.new_session(&resolver, SessionId::void(), None);

            let fun_name = Self::name(function_name);
            let should_error = fun_name.clone().into_string().ends_with(POSTFIX);
            let mut arg = args.clone();
            match &dynamic_args {
                ExecFuncTimerDynamicArgs::DistinctSigners => {
                    arg.insert(
                        0,
                        MoveValue::Signer(extra_accounts.pop().unwrap())
                            .simple_serialize()
                            .unwrap(),
                    );
                },
                ExecFuncTimerDynamicArgs::DistinctSignersAndFixed(signers) => {
                    for signer in signers.iter().rev() {
                        arg.insert(0, MoveValue::Signer(*signer).simple_serialize().unwrap());
                    }
                    arg.insert(
                        0,
                        MoveValue::Signer(extra_accounts.pop().unwrap())
                            .simple_serialize()
                            .unwrap(),
                    );
                },
                _ => {},
            }

            let (mut regular, mut unmetered) = match gas_meter_type {
                GasMeterType::RegularGasMeter => (
                    Some(make_prod_gas_meter(
                        env.gas_feature_version(),
                        env.gas_params().as_ref().unwrap().vm.clone(),
                        env.storage_gas_params().as_ref().unwrap().clone(),
                        false,
                        1_000_000_000_000_000.into(),
                        &NoopBlockSynchronizationKillSwitch {},
                    )),
                    None,
                ),
                GasMeterType::UnmeteredGasMeter => (None, Some(UnmeteredGasMeter)),
            };

            let start = Instant::now();

            let traversal_storage = TraversalStorage::new();
            // Not sure how to create a common type for both. Box<dyn GasMeter> doesn't work for some reason.
            let result = match gas_meter_type {
                GasMeterType::RegularGasMeter => session.execute_function_bypass_visibility(
                    module,
                    &fun_name,
                    type_params.clone(),
                    arg,
                    regular.as_mut().unwrap(),
                    &mut TraversalContext::new(&traversal_storage),
                    &module_storage,
                ),
                GasMeterType::UnmeteredGasMeter => session.execute_function_bypass_visibility(
                    module,
                    &fun_name,
                    type_params.clone(),
                    arg,
                    unmetered.as_mut().unwrap(),
                    &mut TraversalContext::new(&traversal_storage),
                    &module_storage,
                ),
            };
            let elapsed = start.elapsed();
            if let Err(err) = result {
                if !should_error {
                    println!(
                        "Entry function under measurement failed with an error. Continuing, but measurements are probably not what is expected. Error: {}",
                        err
                    );
                }
            }

            if i > NUM_WARM_UP_RUNS {
                measurements.push(Measurement {
                    elapsed,
                    execution_gas: regular
                        .as_ref()
                        .map_or(0, |gas| gas.algebra().execution_gas_used().into()),
                    io_gas: regular
                        .as_ref()
                        .map_or(0, |gas| gas.algebra().io_gas_used().into()),
                });
            }
            i += 1;
        }

        // take median of all running time iterations as a more robust measurement
        measurements.sort_by_key(|v| v.elapsed);
        let length = measurements.len();
        let mid = length / 2;
        let mut measurement = measurements[mid].clone();

        if length % 2 == 0 {
            measurement = Measurement {
                elapsed: (measurements[mid - 1].elapsed + measurements[mid].elapsed) / 2,
                execution_gas: (measurements[mid - 1].execution_gas
                    + measurements[mid].execution_gas)
                    / 2,
                io_gas: (measurements[mid - 1].io_gas + measurements[mid].io_gas) / 2,
            };
        }

        measurement
    }

    /// record abstract usage using a modified gas meter
    pub fn exec_abstract_usage(
        &mut self,
        module: &ModuleId,
        function_name: &str,
        type_params: Vec<TypeTag>,
        args: Vec<Vec<u8>>,
    ) -> Vec<DynamicExpression> {
        // Define the shared buffers
        let a1 = Arc::new(Mutex::new(Vec::<DynamicExpression>::new()));
        let a2 = Arc::clone(&a1);

        let (write_set, _events) = {
            let env = AptosEnvironment::new_with_gas_hook(
                &self.state_store,
                Arc::new(move |expression| {
                    a2.lock().unwrap().push(expression);
                }),
            );
            let resolver = self.state_store.as_move_resolver();
            let vm = MoveVmExt::new(&env);

            let module_storage = self.state_store.as_aptos_code_storage(&env);
            let mut session = vm.new_session(&resolver, SessionId::void(), None);

            let fun_name = Self::name(function_name);
            let should_error = fun_name.clone().into_string().ends_with(POSTFIX);

            let traversal_storage = TraversalStorage::new();
            let mut traversal_context = TraversalContext::new(&traversal_storage);

            let result = session.execute_function_bypass_visibility(
                module,
                &fun_name,
                type_params,
                args,
                &mut StandardGasMeter::new(CalibrationAlgebra {
                    base: StandardGasAlgebra::new(
                        env.gas_feature_version(),
                        env.gas_params().as_ref().unwrap().vm.clone(),
                        env.storage_gas_params().as_ref().unwrap().clone(),
                        false,
                        10_000_000_000_000,
                        &NoopBlockSynchronizationKillSwitch {},
                    ),
                    shared_buffer: Arc::clone(&a1),
                }),
                &mut traversal_context,
                &module_storage,
            );
            if let Err(err) = result {
                if !should_error {
                    println!("Should error, but ignoring for now... {}", err);
                }
            }
            let change_set_configs = &env
                .storage_gas_params()
                .as_ref()
                .unwrap()
                .change_set_configs;
            finish_session_assert_no_modules(session, &module_storage, change_set_configs)
        };
        self.state_store.apply_write_set(&write_set).unwrap();

        let a1_result = Arc::into_inner(a1);
        a1_result
            .expect("Failed to get a1 arc result")
            .lock()
            .unwrap()
            .to_vec()
    }

    pub fn exec(
        &mut self,
        module_name: &str,
        function_name: &str,
        type_params: Vec<TypeTag>,
        args: Vec<Vec<u8>>,
    ) {
        let module_id = Self::module(module_name);
        let (write_set, events) = {
            let env = AptosEnvironment::new(&self.state_store);
            let resolver = self.state_store.as_move_resolver();
            let vm = MoveVmExt::new(&env);

            let module_storage = self.state_store.as_aptos_code_storage(&env);
            let mut session = vm.new_session(&resolver, SessionId::void(), None);

            let traversal_storage = TraversalStorage::new();
            let mut traversal_context = TraversalContext::new(&traversal_storage);

            session
                .execute_function_bypass_visibility(
                    &module_id,
                    &Self::name(function_name),
                    type_params,
                    args,
                    // TODO(Gas): we probably want to switch to metered execution in the future
                    &mut UnmeteredGasMeter,
                    &mut traversal_context,
                    &module_storage,
                )
                .unwrap_or_else(|e| {
                    panic!(
                        "Error calling {}.{}: {}",
                        &module_id,
                        function_name,
                        e.into_vm_status()
                    )
                });
            finish_session_assert_no_modules(
                session,
                &module_storage,
                &ChangeSetConfigs::unlimited_at_gas_feature_version(env.gas_feature_version()),
            )
        };
        self.state_store.apply_write_set(&write_set).unwrap();
        self.event_store.extend(events);
    }

    pub fn try_exec(
        &mut self,
        module_name: &str,
        function_name: &str,
        type_params: Vec<TypeTag>,
        args: Vec<Vec<u8>>,
    ) -> Result<(WriteSet, Vec<ContractEvent>), VMStatus> {
        let env = AptosEnvironment::new(&self.state_store);
        let resolver = self.state_store.as_move_resolver();
        let vm = MoveVmExt::new(&env);

        let module_storage = self.state_store.as_aptos_code_storage(&env);

        let mut session = vm.new_session(&resolver, SessionId::void(), None);
        let traversal_storage = TraversalStorage::new();
        session
            .execute_function_bypass_visibility(
                &Self::module(module_name),
                &Self::name(function_name),
                type_params,
                args,
                // TODO(Gas): we probably want to switch to metered execution in the future
                &mut UnmeteredGasMeter,
                &mut TraversalContext::new(&traversal_storage),
                &module_storage,
            )
            .map_err(|e| e.into_vm_status())?;
        Ok(finish_session_assert_no_modules(
            session,
            &module_storage,
            &ChangeSetConfigs::unlimited_at_gas_feature_version(env.gas_feature_version()),
        ))
    }

    pub fn execute_view_function(
        &mut self,
        fun: MemberId,
        type_args: Vec<TypeTag>,
        arguments: Vec<Vec<u8>>,
    ) -> ViewFunctionOutput {
        let max_gas_amount = u64::MAX;
        AptosVM::execute_view_function(
            self.get_state_view(),
            fun.module_id,
            fun.member_id,
            type_args,
            arguments,
            max_gas_amount,
        )
    }

    /// Force-rotates the authentication key of the account at the given address.
    ///
    /// Returns a new [`Account`] struct that contains the newly generated key pair, which you
    /// can use to sign transactions.
    pub fn rotate_account_authentication_key(&mut self, addr: AccountAddress) -> Account {
        let account = Account::new_from_addr_with_new_keypair_from_seed(addr, &mut self.rng);

        // Note: This does not update the mapping of originating addresses but it is probably fine
        //       for testing purposes.
        self.exec("account", "rotate_authentication_key_call", vec![], vec![
            MoveValue::Signer(addr).simple_serialize().unwrap(),
            MoveValue::vector_u8(account.auth_key())
                .simple_serialize()
                .unwrap(),
        ]);

        account
    }

    /// Enables and disables specified features, committing the result to the state.
    pub fn enable_features(
        &mut self,
        signer: &AccountAddress,
        enabled: Vec<FeatureFlag>,
        disabled: Vec<FeatureFlag>,
    ) {
        let enabled = enabled.into_iter().map(|f| f as u64).collect::<Vec<_>>();
        let disabled = disabled.into_iter().map(|f| f as u64).collect::<Vec<_>>();
        self.exec("features", "change_feature_flags_internal", vec![], vec![
            MoveValue::Signer(*signer).simple_serialize().unwrap(),
            bcs::to_bytes(&enabled).unwrap(),
            bcs::to_bytes(&disabled).unwrap(),
        ]);
    }
}

/// Finishes the session, and asserts there has been no modules published (publishing is the
/// responsibility of the adapter, i.e., [AptosVM]).
fn finish_session_assert_no_modules(
    session: SessionExt<impl AptosMoveResolver>,
    module_storage: &impl AptosModuleStorage,
    change_set_configs: &ChangeSetConfigs,
) -> (WriteSet, Vec<ContractEvent>) {
    let change_set = session
        .finish(change_set_configs, module_storage)
        .expect("Failed to finish the session");

    change_set
        .try_combine_into_storage_change_set(ModuleWriteSet::empty())
        .expect("Failed to convert to storage ChangeSet")
        .into_inner()
}

pub fn assert_outputs_equal(
    txns_output_1: &[TransactionOutput],
    name1: &str,
    txns_output_2: &[TransactionOutput],
    name2: &str,
) {
    assert_eq!(
        txns_output_1.len(),
        txns_output_2.len(),
        "Transaction outputs size mismatch: in {:?} and in {:?}",
        name1,
        name2,
    );

    for (idx, (txn_output_1, txn_output_2)) in
        txns_output_1.iter().zip(txns_output_2.iter()).enumerate()
    {
        // Gas is usually the problem, so check it separately to
        // have a concise error message.
        assert_eq!(
            txn_output_1.try_extract_fee_statement().unwrap_or_default(),
            txn_output_2.try_extract_fee_statement().unwrap_or_default(),
            "Different gas used for {:?} and {:?} for transaction outputs at index {}",
            name1,
            name2,
            idx,
        );

        // Identify differences in write sets, if any.

        let keys = txn_output_1
            .write_set()
            .write_op_iter()
            .chain(txn_output_2.write_set().write_op_iter())
            .map(|(k, _)| k)
            .collect::<BTreeSet<_>>();
        let mut differences = vec![];
        for key in keys {
            let write1 = txn_output_1.write_set().get_write_op(key);
            let write2 = txn_output_2.write_set().get_write_op(key);

            if write1 != write2 {
                differences.push(format!(
                    "Write for {:?} differs: {:?} vs {:?}",
                    key, write1, write2
                ));
            }
        }
        if !differences.is_empty() {
            println!("Differences:\n{}", differences.join("\n"));
        }
        assert!(
            differences.is_empty(),
            "First write op mismatch for transaction output at index {}, between {} and {}",
            idx,
            name1,
            name2,
        );

        // Still perform comparison on all fields in transaction
        // outputs to catch other inconsistencies.
        assert_eq!(
            txn_output_1, txn_output_2,
            "first transaction output mismatch at index {}, for {} and {}",
            idx, name1, name2,
        );
    }
}
