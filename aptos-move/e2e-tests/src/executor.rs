// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

//! Support for running the VM to execute and verify transactions.

use serde::Serialize;
use std::{
    env,
    fs::{self, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
};

use crate::{
    account::{Account, AccountData},
    data_store::{
        FakeDataStore, GENESIS_CHANGE_SET_HEAD, GENESIS_CHANGE_SET_MAINNET,
        GENESIS_CHANGE_SET_TESTNET,
    },
    golden_outputs::GoldenOutputs,
};
use aptos_bitvec::BitVec;
use aptos_crypto::HashValue;
use aptos_gas::{AbstractValueSizeGasParameters, NativeGasParameters};
use aptos_keygen::KeyGen;
use aptos_state_view::StateView;
use aptos_types::on_chain_config::{FeatureFlag, Features};
use aptos_types::{
    access_path::AccessPath,
    account_config::{
        new_block_event_key, AccountResource, CoinInfoResource, CoinStoreResource, NewBlockEvent,
        CORE_CODE_ADDRESS,
    },
    block_metadata::BlockMetadata,
    on_chain_config::{OnChainConfig, ValidatorSet, Version},
    state_store::state_key::StateKey,
    transaction::{
        ExecutionStatus, SignedTransaction, Transaction, TransactionOutput, TransactionStatus,
        VMValidatorResult,
    },
    vm_status::VMStatus,
    write_set::WriteSet,
};
use aptos_vm::{
    data_cache::{AsMoveResolver, StorageAdapter},
    move_vm_ext::{MoveVmExt, SessionId},
    parallel_executor::ParallelAptosVM,
    AptosVM, VMExecutor, VMValidator,
};
use framework::ReleaseBundle;
use move_deps::{
    move_core_types::{
        account_address::AccountAddress,
        identifier::Identifier,
        language_storage::{ModuleId, ResourceKey, TypeTag},
        move_resource::MoveResource,
    },
    move_vm_types::gas::UnmeteredGasMeter,
};
use num_cpus;

static RNG_SEED: [u8; 32] = [9u8; 32];

const ENV_TRACE_DIR: &str = "TRACE";

/// Directory structure of the trace dir
pub const TRACE_FILE_NAME: &str = "name";
pub const TRACE_FILE_ERROR: &str = "error";
pub const TRACE_DIR_META: &str = "meta";
pub const TRACE_DIR_DATA: &str = "data";
pub const TRACE_DIR_INPUT: &str = "input";
pub const TRACE_DIR_OUTPUT: &str = "output";

/// Maps block number N to the index of the input and output transactions
pub type TraceSeqMapping = (usize, Vec<usize>, Vec<usize>);

/// Provides an environment to run a VM instance.
///
/// This struct is a mock in-memory implementation of the Aptos executor.
#[derive(Debug)]
pub struct FakeExecutor {
    data_store: FakeDataStore,
    block_time: u64,
    executed_output: Option<GoldenOutputs>,
    trace_dir: Option<PathBuf>,
    rng: KeyGen,
    no_parallel_exec: bool,
    features: Features,
}

impl FakeExecutor {
    /// Creates an executor from a genesis [`WriteSet`].
    pub fn from_genesis(write_set: &WriteSet) -> Self {
        let mut executor = FakeExecutor {
            data_store: FakeDataStore::default(),
            block_time: 0,
            executed_output: None,
            trace_dir: None,
            rng: KeyGen::from_seed(RNG_SEED),
            no_parallel_exec: false,
            features: Features::default(),
        };
        executor.apply_write_set(write_set);
        // As a set effect, also allow module bundle txns. TODO: Remove
        aptos_vm::aptos_vm::allow_module_bundle_for_test();
        executor
    }

    /// Configure this executor to not use parallel execution.
    pub fn set_not_parallel(mut self) -> Self {
        self.no_parallel_exec = true;
        self
    }

    /// Creates an executor from the genesis file GENESIS_FILE_LOCATION
    pub fn from_head_genesis() -> Self {
        Self::from_genesis(GENESIS_CHANGE_SET_HEAD.clone().write_set())
    }

    /// Creates an executor using the standard genesis.
    pub fn from_testnet_genesis() -> Self {
        Self::from_genesis(GENESIS_CHANGE_SET_TESTNET.clone().write_set())
    }

    /// Creates an executor using the mainnet genesis.
    pub fn from_mainnet_genesis() -> Self {
        Self::from_genesis(GENESIS_CHANGE_SET_MAINNET.clone().write_set())
    }

    /// Creates an executor in which no genesis state has been applied yet.
    pub fn no_genesis() -> Self {
        FakeExecutor {
            data_store: FakeDataStore::default(),
            block_time: 0,
            executed_output: None,
            trace_dir: None,
            rng: KeyGen::from_seed(RNG_SEED),
            no_parallel_exec: false,
            features: Features::default(),
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
                Version::fetch_config(&self.data_store.as_move_resolver()).map_or(0, |v| v.major);

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
        for (bytes, module) in cached_packages::head_release_bundle().code_and_compiled_modules() {
            let id = module.self_id();
            genesis.add_module(&id, bytes.to_vec());
        }
        genesis
    }

    /// Creates fresh genesis from the framework passed in.
    pub fn custom_genesis(framework: &ReleaseBundle, validator_accounts: Option<usize>) -> Self {
        let genesis = vm_genesis::generate_test_genesis(framework, validator_accounts);
        Self::from_genesis(genesis.0.write_set())
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
        // The below will use the genesis keypair but that should be fine.
        let acc = Account::new_genesis_account(addr);
        // Mint the account 10M Aptos coins (with 8 decimals).
        let data = AccountData::with_account(acc, 1_000_000_000_000_000, 0);
        self.add_account_data(&data);
        data.account().clone()
    }

    /// Applies a [`WriteSet`] to this executor's data store.
    pub fn apply_write_set(&mut self, write_set: &WriteSet) {
        self.data_store.add_write_set(write_set);
    }

    /// Adds an account to this executor's data store.
    pub fn add_account_data(&mut self, account_data: &AccountData) {
        self.data_store.add_account_data(account_data)
    }

    /// Adds coin info to this executor's data store.
    pub fn add_coin_info(&mut self) {
        self.data_store.add_coin_info()
    }

    /// Adds a module to this executor's data store.
    ///
    /// Does not do any sort of verification on the module.
    pub fn add_module(&mut self, module_id: &ModuleId, module_blob: Vec<u8>) {
        self.data_store.add_module(module_id, module_blob)
    }

    /// Reads the resource [`Value`] for an account from this executor's data store.
    pub fn read_account_resource(&self, account: &Account) -> Option<AccountResource> {
        self.read_account_resource_at_address(account.address())
    }

    pub fn read_resource<T: MoveResource>(&self, addr: &AccountAddress) -> Option<T> {
        let ap = AccessPath::resource_access_path(ResourceKey::new(*addr, T::struct_tag()));
        let data_blob = StateView::get_state_value(&self.data_store, &StateKey::AccessPath(ap))
            .expect("account must exist in data store")
            .unwrap_or_else(|| panic!("Can't fetch {} resource for {}", T::STRUCT_NAME, addr));
        bcs::from_bytes(data_blob.as_slice()).ok()
    }

    /// Reads the resource [`Value`] for an account under the given address from
    /// this executor's data store.
    pub fn read_account_resource_at_address(
        &self,
        addr: &AccountAddress,
    ) -> Option<AccountResource> {
        self.read_resource(addr)
    }

    /// Reads the CoinStore resource value for an account from this executor's data store.
    pub fn read_coin_store_resource(&self, account: &Account) -> Option<CoinStoreResource> {
        self.read_coin_store_resource_at_address(account.address())
    }

    /// Reads supply from CoinInfo resource value from this executor's data store.
    pub fn read_coin_supply(&self) -> Option<u128> {
        self.read_coin_info_resource()
            .expect("coin info must exist in data store")
            .supply()
            .as_ref()
            .map(|o| match o.aggregator.as_ref() {
                Some(aggregator) => {
                    let state_key = aggregator.state_key();
                    let value_bytes = self
                        .read_state_value(&state_key)
                        .expect("aggregator value must exist in data store");
                    bcs::from_bytes(&value_bytes).unwrap()
                }
                None => o.integer.as_ref().unwrap().value,
            })
    }

    /// Reads the CoinInfo resource value from this executor's data store.
    pub fn read_coin_info_resource(&self) -> Option<CoinInfoResource> {
        self.read_resource(&AccountAddress::ONE)
    }

    /// Reads the CoinStore resource value for an account under the given address from this executor's
    /// data store.
    pub fn read_coin_store_resource_at_address(
        &self,
        addr: &AccountAddress,
    ) -> Option<CoinStoreResource> {
        self.read_resource(addr)
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

    /// Alternate form of 'execute_block' that keeps the vm_status before it goes into the
    /// `TransactionOutput`
    pub fn execute_block_and_keep_vm_status(
        &self,
        txn_block: Vec<SignedTransaction>,
    ) -> Result<Vec<(VMStatus, TransactionOutput)>, VMStatus> {
        AptosVM::execute_block_and_keep_vm_status(
            txn_block
                .into_iter()
                .map(Transaction::UserTransaction)
                .collect(),
            &self.data_store,
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
            }
            TransactionStatus::Discard(status) => panic!("transaction discarded with {:?}", status),
            TransactionStatus::Retry => panic!("transaction status is retry"),
        }
    }

    pub fn execute_transaction_block_parallel(
        &self,
        txn_block: Vec<Transaction>,
    ) -> Result<Vec<TransactionOutput>, VMStatus> {
        let (result, _) =
            ParallelAptosVM::execute_block(txn_block, &self.data_store, num_cpus::get())?;

        Ok(result)
    }

    pub fn execute_transaction_block(
        &self,
        txn_block: Vec<Transaction>,
    ) -> Result<Vec<TransactionOutput>, VMStatus> {
        let mut trace_map = TraceSeqMapping::default();

        // dump serialized transaction details before execution, if tracing
        if let Some(trace_dir) = &self.trace_dir {
            let trace_data_dir = trace_dir.join(TRACE_DIR_DATA);
            trace_map.0 = Self::trace(trace_data_dir.as_path(), self.get_state_view());
            let trace_input_dir = trace_dir.join(TRACE_DIR_INPUT);
            for txn in &txn_block {
                let input_seq = Self::trace(trace_input_dir.as_path(), txn);
                trace_map.1.push(input_seq);
            }
        }

        let output = AptosVM::execute_block(txn_block.clone(), &self.data_store);
        if !self.no_parallel_exec {
            let parallel_output = self.execute_transaction_block_parallel(txn_block);
            assert_eq!(output, parallel_output);
        }

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
                }
                Err(e) => {
                    let mut error_file = OpenOptions::new()
                        .write(true)
                        .create_new(true)
                        .open(trace_dir.join(TRACE_FILE_ERROR))
                        .unwrap();
                    error_file.write_all(e.to_string().as_bytes()).unwrap();
                }
            }
            let trace_meta_dir = trace_dir.join(TRACE_DIR_META);
            Self::trace(trace_meta_dir.as_path(), &trace_map);
        }
        output
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

    /// Get the blob for the associated AccessPath
    pub fn read_state_value(&self, state_key: &StateKey) -> Option<Vec<u8>> {
        StateView::get_state_value(&self.data_store, state_key).unwrap()
    }

    /// Verifies the given transaction by running it through the VM verifier.
    pub fn verify_transaction(&self, txn: SignedTransaction) -> VMValidatorResult {
        let vm = AptosVM::new(self.get_state_view());
        vm.validate_transaction(txn, &self.data_store)
    }

    pub fn get_state_view(&self) -> &FakeDataStore {
        &self.data_store
    }

    pub fn new_block(&mut self) {
        self.new_block_with_timestamp(self.block_time + 1);
    }

    pub fn new_block_with_timestamp(&mut self, time_microseconds: u64) {
        self.block_time = time_microseconds;

        let validator_set = ValidatorSet::fetch_config(&self.data_store.as_move_resolver())
            .expect("Unable to retrieve the validator set from storage");
        let proposer = *validator_set.payload().next().unwrap().account_address();
        // when updating time, proposer cannot be ZERO.
        self.new_block_with_metadata(proposer, vec![])
    }

    pub fn new_block_with_metadata(
        &mut self,
        proposer: AccountAddress,
        failed_proposer_indices: Vec<u32>,
    ) {
        let validator_set = ValidatorSet::fetch_config(&self.data_store.as_move_resolver())
            .expect("Unable to retrieve the validator set from storage");
        let new_block = BlockMetadata::new(
            HashValue::zero(),
            0,
            0,
            proposer,
            BitVec::with_num_bits(validator_set.num_validators() as u16).into(),
            failed_proposer_indices,
            self.block_time,
        );
        let output = self
            .execute_transaction_block(vec![Transaction::BlockMetadata(new_block)])
            .expect("Executing block prologue should succeed")
            .pop()
            .expect("Failed to get the execution result for Block Prologue");
        // check if we emit the expected event, there might be more events for transaction fees
        let event = output.events()[0].clone();
        assert_eq!(event.key(), &new_block_event_key());
        assert!(bcs::from_bytes::<NewBlockEvent>(event.event_data()).is_ok());
        self.apply_write_set(output.write_set());
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

    pub fn exec(
        &mut self,
        module_name: &str,
        function_name: &str,
        type_params: Vec<TypeTag>,
        args: Vec<Vec<u8>>,
    ) {
        let write_set = {
            // TODO(Gas): we probably want to switch to non-zero costs in the future
            let vm = MoveVmExt::new(
                NativeGasParameters::zeros(),
                AbstractValueSizeGasParameters::zeros(),
                self.features
                    .is_enabled(FeatureFlag::TREAT_FRIEND_AS_PRIVATE),
            )
            .unwrap();
            let remote_view = StorageAdapter::new(&self.data_store);
            let mut session = vm.new_session(&remote_view, SessionId::void());
            session
                .execute_function_bypass_visibility(
                    &Self::module(module_name),
                    &Self::name(function_name),
                    type_params,
                    args,
                    &mut UnmeteredGasMeter,
                )
                .unwrap_or_else(|e| {
                    panic!(
                        "Error calling {}.{}: {}",
                        module_name,
                        function_name,
                        e.into_vm_status()
                    )
                });
            let session_out = session.finish().expect("Failed to generate txn effects");
            // TODO: Support deltas in fake executor.
            let (_, change_set) = session_out
                .into_change_set(&mut ())
                .expect("Failed to generate writeset")
                .into_inner();
            let (write_set, _events) = change_set.into_inner();
            write_set
        };
        self.data_store.add_write_set(&write_set);
    }

    pub fn try_exec(
        &mut self,
        module_name: &str,
        function_name: &str,
        type_params: Vec<TypeTag>,
        args: Vec<Vec<u8>>,
    ) -> Result<WriteSet, VMStatus> {
        // TODO(Gas): we probably want to switch to non-zero costs in the future
        let vm = MoveVmExt::new(
            NativeGasParameters::zeros(),
            AbstractValueSizeGasParameters::zeros(),
            self.features
                .is_enabled(FeatureFlag::TREAT_FRIEND_AS_PRIVATE),
        )
        .unwrap();
        let remote_view = StorageAdapter::new(&self.data_store);
        let mut session = vm.new_session(&remote_view, SessionId::void());
        session
            .execute_function_bypass_visibility(
                &Self::module(module_name),
                &Self::name(function_name),
                type_params,
                args,
                &mut UnmeteredGasMeter,
            )
            .map_err(|e| e.into_vm_status())?;
        let session_out = session.finish().expect("Failed to generate txn effects");
        // TODO: Support deltas in fake executor.
        let (_, change_set) = session_out
            .into_change_set(&mut ())
            .expect("Failed to generate writeset")
            .into_inner();
        let (writeset, _events) = change_set.into_inner();
        Ok(writeset)
    }
}
