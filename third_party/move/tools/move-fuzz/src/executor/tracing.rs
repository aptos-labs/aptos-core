// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    account::{AddressKind, AddressRegistry, NamedAddressKind},
    deps::{PkgDefinition, PkgKind},
    package::FuzzPackage,
};
use anyhow::{bail, Result};
use aptos_cached_packages::aptos_stdlib;
use aptos_framework::natives::code::PackageMetadata;
use aptos_gas_meter::{StandardGasAlgebra, StandardGasMeter};
use aptos_gas_schedule::{
    AptosGasParameters, FromOnChainGasSchedule, InstructionGasParameters, MiscGasParameters,
    ToOnChainGasSchedule, VMGasParameters,
};
use aptos_language_e2e_tests::executor::FakeExecutor;
use aptos_transaction_simulation::{Account, SimulationStateStore};
use aptos_types::{
    access_path::Path,
    account_address::AccountAddress,
    on_chain_config::{GasScheduleV2, OnChainConfig},
    state_store::{
        state_key::{inner::StateKeyInner, StateKey},
        state_storage_usage::StateStorageUsage,
        state_value::StateValue,
        StateViewId, StateViewResult, TStateView,
    },
    transaction::{
        AuxiliaryInfo, ExecutionStatus, TransactionOutput, TransactionPayload, TransactionStatus,
    },
    vm_status::VMStatus,
    write_set::TransactionWrite,
};
use aptos_vm::{data_cache::AsMoveResolver, AptosVM};
use aptos_vm_environment::environment::AptosEnvironment;
use aptos_vm_logging::log_schema::AdapterLogSchema;
use aptos_vm_types::{
    module_and_script_storage::AsAptosCodeStorage, storage::StorageGasParameters,
};
use legacy_move_compiler::compiled_unit::CompiledUnit;
use move_core_types::{identifier::Identifier, language_storage::StructTag};
use move_package::compilation::compiled_package::CompiledUnitWithSource;
use std::{
    cell::RefCell,
    collections::{BTreeMap, BTreeSet},
    hash::{Hash, Hasher},
    path::Path as StdPath,
};

/// Default APT fund per each new account (10M, with 8 decimals)
const INITIAL_APT_BALANCE: u64 = 1_000_000_000_000_000;

/// Max transaction size in bytes (1MB)
const MAX_TRANSACTION_SIZE_IN_BYTES: u64 = 1024 * 1024;

/// Gas consumption profile
#[derive(Debug, Clone)]
enum GasProfile {
    Constant {
        price_per_gas_unit: u64,
        max_gas_units_per_txn: u64,
    },
}

impl GasProfile {
    /// Return gas information needed for transaction
    pub fn get_config_for_txn(&self) -> (u64, u64) {
        match self {
            GasProfile::Constant {
                price_per_gas_unit,
                max_gas_units_per_txn,
            } => (*price_per_gas_unit, *max_gas_units_per_txn),
        }
    }
}

/// A resource write extracted from transaction output
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResourceWrite {
    pub address: AccountAddress,
    pub struct_tag: StructTag,
    pub is_resource_group: bool,
}

/// A resource read extracted from state view access
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ResourceRead {
    pub address: AccountAddress,
    pub struct_tag: StructTag,
    pub is_resource_group: bool,
}

/// Convert non-resource state keys (table item / raw) into synthetic StructTags
/// so they can still participate in def-use tracking.
fn synthetic_struct_tag(prefix: &str, state_key: &StateKey) -> StructTag {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    format!("{state_key:?}").hash(&mut hasher);
    let suffix = hasher.finish();
    StructTag {
        address: AccountAddress::ONE,
        module: Identifier::new("global_state").expect("valid synthetic module identifier"),
        name: Identifier::new(format!("{prefix}{suffix:016x}"))
            .expect("valid synthetic struct identifier"),
        type_args: vec![],
    }
}

/// A state view wrapper that records all state key accesses
struct RecordingStateView<'a, S: ?Sized> {
    inner: &'a S,
    reads: RefCell<BTreeSet<StateKey>>,
}

impl<'a, S: TStateView<Key = StateKey> + ?Sized> RecordingStateView<'a, S> {
    fn new(inner: &'a S) -> Self {
        Self {
            inner,
            reads: RefCell::new(BTreeSet::new()),
        }
    }

    /// Extract resource reads from the recorded state key accesses
    fn extract_resource_reads(&self) -> Vec<ResourceRead> {
        let mut result = Vec::new();
        for key in self.reads.borrow().iter() {
            let read = match key.inner() {
                StateKeyInner::AccessPath(ap) => match ap.get_path() {
                    Path::Resource(struct_tag) => ResourceRead {
                        struct_tag,
                        address: ap.address,
                        is_resource_group: false,
                    },
                    Path::ResourceGroup(struct_tag) => ResourceRead {
                        struct_tag,
                        address: ap.address,
                        is_resource_group: true,
                    },
                    Path::Code(..) => continue,
                },
                StateKeyInner::TableItem { .. } => ResourceRead {
                    struct_tag: synthetic_struct_tag("table_", key),
                    address: AccountAddress::ONE,
                    is_resource_group: false,
                },
                StateKeyInner::Raw(..) => ResourceRead {
                    struct_tag: synthetic_struct_tag("raw_", key),
                    address: AccountAddress::ONE,
                    is_resource_group: false,
                },
            };
            result.push(read);
        }
        result
    }
}

impl<S: TStateView<Key = StateKey> + ?Sized> TStateView for RecordingStateView<'_, S> {
    type Key = StateKey;

    fn id(&self) -> StateViewId {
        self.inner.id()
    }

    fn get_usage(&self) -> StateViewResult<StateStorageUsage> {
        self.inner.get_usage()
    }

    fn get_state_value(&self, state_key: &StateKey) -> StateViewResult<Option<StateValue>> {
        self.reads.borrow_mut().insert(state_key.clone());
        self.inner.get_state_value(state_key)
    }
}

/// A stateful executor
pub struct TracingExecutor {
    /// backend executor
    executor: FakeExecutor,

    /// address registry
    address_registry: AddressRegistry,

    /// gas profile we are following now
    gas_profile: GasProfile,
}

impl TracingExecutor {
    /// Create a new tracing executor
    pub fn new() -> Self {
        let executor = FakeExecutor::from_head_genesis().set_not_parallel();

        // acquire gas config
        let mut gas_schedule = GasScheduleV2::fetch_config(executor.get_state_view())
            .expect("expect genesis to have a gas schedule");
        let mut gas_params = AptosGasParameters::from_on_chain_gas_schedule(
            &gas_schedule.entries.into_iter().collect(),
            gas_schedule.feature_version,
        )
        .unwrap_or_else(|why| panic!("malformed gas schedule: {why}"));

        // actual gas config tweaks
        gas_params.vm.txn.max_transaction_size_in_bytes = MAX_TRANSACTION_SIZE_IN_BYTES.into();

        // update gas config back into storage
        gas_schedule.entries = gas_params.to_on_chain_gas_schedule(gas_schedule.feature_version);
        executor
            .state_store()
            .set_state_value(
                StateKey::on_chain_config::<GasScheduleV2>()
                    .expect("expect a valid resource tag for gas schedule"),
                StateValue::from(
                    bcs::to_bytes(&gas_schedule)
                        .expect("expect serialization of gas schedule resource to succeed"),
                ),
            )
            .expect("write-back gas configuration");

        // derive the gas profile
        let gas_profile = GasProfile::Constant {
            price_per_gas_unit: gas_params.vm.txn.min_price_per_gas_unit.into(),
            max_gas_units_per_txn: gas_params.vm.txn.maximum_number_of_gas_units.into(),
        };

        // done with the tweaks
        Self {
            executor,
            address_registry: AddressRegistry::new(),
            gas_profile,
        }
    }

    /// Create an account, and fund it with an initial balance if needed
    fn create_account(&mut self, account: Account) {
        self.executor
            .store_and_fund_account(account, INITIAL_APT_BALANCE, 0);
    }

    /// Retrieve the account sequence number
    fn get_account_sequence_number(&self, account: &Account) -> u64 {
        let resource = self
            .executor
            .read_account_resource(account)
            .expect("provisioned account should have a sequence number");
        resource.sequence_number()
    }

    /// Execute a transaction without committing its output
    fn execute_transaction(
        &mut self,
        sender: AccountAddress,
        payload: TransactionPayload,
    ) -> Result<(VMStatus, TransactionOutput)> {
        // retrieve sender account from the address
        let account = self
            .address_registry
            .lookup_account(sender)
            .unwrap_or_else(|| {
                panic!(
                    "[invariant] unable to find the account \
                     associated with the sender address {sender}"
                )
            });

        // construct the transaction
        let (gas_unit_price, max_gas_amount) = self.gas_profile.get_config_for_txn();
        let signed_txn = account
            .transaction()
            .sequence_number(self.get_account_sequence_number(account))
            .gas_unit_price(gas_unit_price)
            .max_gas_amount(max_gas_amount)
            .payload(payload)
            .sign();

        // execute the transaction using our own config of the VM
        let state_view = self.executor.get_state_view();
        let env = AptosEnvironment::new(state_view);
        let vm = AptosVM::new(&env);
        let resolver = state_view.as_move_resolver();
        let code_storage = state_view.as_aptos_code_storage(&env);
        let log_context = AdapterLogSchema::new(state_view.id(), 0);

        let vm_result = vm.execute_user_transaction_with_custom_gas_meter(
            &resolver,
            &code_storage,
            &signed_txn,
            &log_context,
            |gas_feature_version,
             vm_gas_params,
             _,
             is_approved_gov_script,
             meter_balance,
             kill_switch| {
                StandardGasMeter::new(StandardGasAlgebra::new(
                    gas_feature_version,
                    VMGasParameters {
                        misc: MiscGasParameters::zeros(),
                        instr: InstructionGasParameters::zeros(),
                        txn: vm_gas_params.txn,
                    },
                    StorageGasParameters::unlimited(),
                    is_approved_gov_script,
                    meter_balance,
                    kill_switch,
                ))
            },
            &AuxiliaryInfo::default(),
        );
        match vm_result {
            Ok((status, output, _gas_meter)) => {
                match output.try_materialize_into_transaction_output(&resolver) {
                    Ok(txn_output) => Ok((status, txn_output)),
                    Err(error_status) => {
                        bail!("AptosVM failed unexpectedly with status: {error_status}")
                    },
                }
            },
            Err(error_status) => {
                bail!("AptosVM failed unexpectedly with status: {error_status}");
            },
        }
    }

    /// Extract resource writes from a write set.
    ///
    /// Returns tuples of `(struct_tag, address, is_resource_group)` for each
    /// non-deletion resource/resource-group/table-item/raw write.
    fn extract_resource_writes(output: &TransactionOutput) -> Vec<ResourceWrite> {
        let mut result = Vec::new();
        for (state_key, write_op) in output.write_set().write_op_iter() {
            if write_op.is_deletion() {
                continue;
            }
            let write = match state_key.inner() {
                StateKeyInner::AccessPath(ap) => match ap.get_path() {
                    Path::Resource(struct_tag) => ResourceWrite {
                        struct_tag,
                        address: ap.address,
                        is_resource_group: false,
                    },
                    Path::ResourceGroup(struct_tag) => ResourceWrite {
                        struct_tag,
                        address: ap.address,
                        is_resource_group: true,
                    },
                    Path::Code(..) => {
                        // we don't care about code publishing
                        continue;
                    },
                },
                StateKeyInner::TableItem { .. } => ResourceWrite {
                    struct_tag: synthetic_struct_tag("table_", state_key),
                    address: AccountAddress::ONE,
                    is_resource_group: true,
                },
                StateKeyInner::Raw(..) => ResourceWrite {
                    struct_tag: synthetic_struct_tag("raw_", state_key),
                    address: AccountAddress::ONE,
                    is_resource_group: true,
                },
            };
            result.push(write);
        }
        result
    }

    fn root_module_publish_chunks(
        built_package: &FuzzPackage,
        manifest_path: &StdPath,
    ) -> Result<Vec<(AccountAddress, PackageMetadata, Vec<Vec<u8>>)>> {
        let full_code = built_package.extract_code();
        let full_metadata = built_package.extract_metadata(manifest_path)?;
        let mut grouped: BTreeMap<AccountAddress, (Vec<_>, Vec<_>)> = BTreeMap::new();

        let mut module_idx = 0;
        for CompiledUnitWithSource {
            unit,
            source_path: _,
        } in built_package.root_compiled_units()
        {
            let CompiledUnit::Module(module) = unit else {
                continue;
            };
            let entry = grouped.entry(module.address.into_inner()).or_default();
            entry.0.push(full_metadata.modules[module_idx].clone());
            entry.1.push(full_code[module_idx].clone());
            module_idx += 1;
        }

        assert_eq!(module_idx, full_code.len());
        assert_eq!(module_idx, full_metadata.modules.len());

        let mut chunks = Vec::with_capacity(grouped.len());
        for (sender_addr, (modules, code)) in grouped {
            let mut metadata = full_metadata.clone();
            metadata.modules = modules;
            chunks.push((sender_addr, metadata, code));
        }
        Ok(chunks)
    }

    fn ensure_publish_account(
        &mut self,
        sender_addr: AccountAddress,
        address_kind: NamedAddressKind,
        chunk_idx: usize,
    ) -> Result<()> {
        if self.address_registry.lookup_account(sender_addr).is_some() {
            return Ok(());
        }

        let synthetic_name = format!("publish_addr_{chunk_idx}");
        let new_account = self.address_registry.sync_named_address(
            synthetic_name.into(),
            sender_addr,
            None,
            address_kind,
        )?;
        if let Some(account) = new_account {
            self.create_account(account);
        }
        Ok(())
    }

    /// Execute a transaction with output (if any) committed
    fn execute_transaction_and_commit_output(
        &mut self,
        sender: AccountAddress,
        payload: TransactionPayload,
    ) -> Result<(VMStatus, TransactionStatus, Vec<ResourceWrite>)> {
        let (vm_status, output) = self.execute_transaction(sender, payload)?;
        let resource_writes = Self::extract_resource_writes(&output);
        let (write_set, events, _gas_used, txn_status, _txn_misc) = output.unpack();
        match txn_status {
            TransactionStatus::Keep(_) => {
                self.executor.apply_write_set(&write_set);
                self.executor.append_events(events);
            },
            TransactionStatus::Discard(_) => {},
            TransactionStatus::Retry => {
                bail!("unexpected retry status for transaction execution");
            },
        }
        Ok((vm_status, txn_status, resource_writes))
    }

    /// Execute a transaction with output (if any) committed, expect a success
    fn execute_transaction_and_commit_output_expect_success(
        &mut self,
        sender: AccountAddress,
        payload: TransactionPayload,
    ) -> Result<()> {
        let (vm_status, txn_status, _resource_writes) =
            self.execute_transaction_and_commit_output(sender, payload)?;
        match txn_status {
            TransactionStatus::Keep(ExecutionStatus::Success) => {
                assert!(matches!(vm_status, VMStatus::Executed));
                Ok(())
            },
            _ => bail!(
                "transaction failed unexpectedly with status: {:?}",
                txn_status
            ),
        }
    }

    /// Provision a framework package (should already be included in genesis)
    fn provision_framework_package(&mut self, built_package: &FuzzPackage) -> Result<()> {
        // every named address in the framework package will be marked and
        // should remain as a framework address
        for (&name, &addr) in &built_package
            .compiled_package_info()
            .address_alias_instantiation
        {
            let new_account = self.address_registry.sync_named_address(
                name,
                addr,
                Some(NamedAddressKind::Framework),
                NamedAddressKind::Framework,
            )?;
            if let Some(account) = new_account {
                self.create_account(account);
            }
        }

        // we don't need to publish the framework package, so nothing to do
        Ok(())
    }

    /// Provision a regular package
    fn provision_regular_package(
        &mut self,
        address_kind: NamedAddressKind,
        built_package: &FuzzPackage,
        manifest_path: &StdPath,
    ) -> Result<()> {
        log::debug!("provision package: {}", built_package.name());

        // collect addresses and create accounts
        for (&name, &addr) in &built_package
            .compiled_package_info()
            .address_alias_instantiation
        {
            // - if we have already seen the (name, addr) pair in dictionary,
            //   do nothing, otherwise,
            // - create an account and register the (name, addr) pair with the
            //   designated kind
            let new_account =
                self.address_registry
                    .sync_named_address(name, addr, None, address_kind)?;
            if let Some(account) = new_account {
                self.create_account(account);
            }
        }

        let publish_chunks = Self::root_module_publish_chunks(built_package, manifest_path)?;
        if publish_chunks.is_empty() {
            return Ok(());
        }

        for (chunk_idx, (sender_addr, metadata, code)) in publish_chunks.into_iter().enumerate() {
            self.ensure_publish_account(sender_addr, address_kind, chunk_idx)?;
            let payload = aptos_stdlib::code_publish_package_txn(
                bcs::to_bytes(&metadata)
                    .expect("bcs serialization of package metadata must succeed"),
                code,
            );
            self.execute_transaction_and_commit_output_expect_success(sender_addr, payload)?;
        }
        log::debug!("package published: {}", built_package.name());

        // done
        Ok(())
    }

    /// Provision the executor with a pre-compiled package
    pub fn add_new_package(&mut self, pkg: &PkgDefinition) -> Result<()> {
        match &pkg.kind {
            PkgKind::Framework => self.provision_framework_package(&pkg.package),
            PkgKind::Dependency => self.provision_regular_package(
                NamedAddressKind::Dependency,
                &pkg.package,
                &pkg.manifest_path,
            ),
            PkgKind::Primary => self.provision_regular_package(
                NamedAddressKind::Primary,
                &pkg.package,
                &pkg.manifest_path,
            ),
        }
    }

    /// Create a new user account in the executor
    pub fn add_new_user(&mut self) {
        let account = self.address_registry.make_user_account();
        self.create_account(account);
    }

    /// Return all addresses known to the executor, sorted by kind
    pub fn all_addresses_by_kind(&self) -> BTreeMap<AddressKind, BTreeSet<AccountAddress>> {
        self.address_registry.all_addresses_by_kind()
    }

    /// Extract all resource writes from the full state store.
    ///
    /// Returns every resource/resource-group/table-item/raw entry as a `ResourceWrite`.
    /// The caller (e.g. `Mutator::update_object_dict`) is responsible for
    /// the two-pass ObjectGroup filtering to identify which addresses are
    /// objects and which resources belong to them.
    pub fn scan_all_resource_writes(&self) -> Vec<ResourceWrite> {
        let delta = self.executor.get_state_delta();
        let mut result = Vec::new();
        for (state_key, value_opt) in &delta {
            if value_opt.is_none() {
                continue;
            }
            match state_key.inner() {
                StateKeyInner::AccessPath(ap) => match ap.get_path() {
                    Path::Resource(struct_tag) => {
                        result.push(ResourceWrite {
                            address: ap.address,
                            struct_tag,
                            is_resource_group: false,
                        });
                    },
                    Path::ResourceGroup(struct_tag) => {
                        result.push(ResourceWrite {
                            address: ap.address,
                            struct_tag,
                            is_resource_group: true,
                        });
                    },
                    Path::Code(..) => {},
                },
                StateKeyInner::TableItem { .. } => {
                    result.push(ResourceWrite {
                        address: AccountAddress::ONE,
                        struct_tag: synthetic_struct_tag("table_", state_key),
                        is_resource_group: true,
                    });
                },
                StateKeyInner::Raw(..) => {
                    result.push(ResourceWrite {
                        address: AccountAddress::ONE,
                        struct_tag: synthetic_struct_tag("raw_", state_key),
                        is_resource_group: true,
                    });
                },
            }
        }
        result
    }

    /// Execute a transaction without committing, tracking state reads
    fn execute_transaction_tracking_reads(
        &mut self,
        sender: AccountAddress,
        payload: TransactionPayload,
    ) -> Result<(VMStatus, TransactionOutput, Vec<ResourceRead>)> {
        let account = self
            .address_registry
            .lookup_account(sender)
            .unwrap_or_else(|| {
                panic!(
                    "[invariant] unable to find the account \
                     associated with the sender address {sender}"
                )
            });

        let (gas_unit_price, max_gas_amount) = self.gas_profile.get_config_for_txn();
        let signed_txn = account
            .transaction()
            .sequence_number(self.get_account_sequence_number(account))
            .gas_unit_price(gas_unit_price)
            .max_gas_amount(max_gas_amount)
            .payload(payload)
            .sign();

        let state_view = self.executor.get_state_view();
        let recording_view = RecordingStateView::new(state_view);
        let env = AptosEnvironment::new(&recording_view);
        let vm = AptosVM::new(&env);
        let resolver = recording_view.as_move_resolver();
        let code_storage = recording_view.as_aptos_code_storage(&env);
        let log_context = AdapterLogSchema::new(recording_view.id(), 0);

        let vm_result = vm.execute_user_transaction_with_custom_gas_meter(
            &resolver,
            &code_storage,
            &signed_txn,
            &log_context,
            |gas_feature_version,
             vm_gas_params,
             _,
             is_approved_gov_script,
             meter_balance,
             kill_switch| {
                StandardGasMeter::new(StandardGasAlgebra::new(
                    gas_feature_version,
                    VMGasParameters {
                        misc: MiscGasParameters::zeros(),
                        instr: InstructionGasParameters::zeros(),
                        txn: vm_gas_params.txn,
                    },
                    StorageGasParameters::unlimited(),
                    is_approved_gov_script,
                    meter_balance,
                    kill_switch,
                ))
            },
            &AuxiliaryInfo::default(),
        );
        let resource_reads = recording_view.extract_resource_reads();
        match vm_result {
            Ok((status, output, _gas_meter)) => {
                match output.try_materialize_into_transaction_output(&resolver) {
                    Ok(txn_output) => Ok((status, txn_output, resource_reads)),
                    Err(error_status) => {
                        bail!("AptosVM failed unexpectedly with status: {error_status}")
                    },
                }
            },
            Err(error_status) => {
                bail!("AptosVM failed unexpectedly with status: {error_status}");
            },
        }
    }

    /// Run a transaction with a sender, tracking resource reads and writes
    pub fn run_payload_with_sender_tracking(
        &mut self,
        sender: AccountAddress,
        payload: TransactionPayload,
    ) -> Result<(
        VMStatus,
        TransactionStatus,
        Vec<ResourceWrite>,
        Vec<ResourceRead>,
    )> {
        let (vm_status, output, resource_reads) =
            self.execute_transaction_tracking_reads(sender, payload)?;
        let resource_writes = Self::extract_resource_writes(&output);
        let (write_set, events, _gas_used, txn_status, _txn_misc) = output.unpack();
        match txn_status {
            TransactionStatus::Keep(_) => {
                self.executor.apply_write_set(&write_set);
                self.executor.append_events(events);
            },
            TransactionStatus::Discard(_) => {},
            TransactionStatus::Retry => {
                bail!("unexpected retry status for transaction execution");
            },
        }
        Ok((vm_status, txn_status, resource_writes, resource_reads))
    }

    /// Run a transaction with a sender
    pub fn run_payload_with_sender(
        &mut self,
        sender: AccountAddress,
        payload: TransactionPayload,
    ) -> Result<(VMStatus, TransactionStatus, Vec<ResourceWrite>)> {
        self.execute_transaction_and_commit_output(sender, payload)
    }
}

impl Default for TracingExecutor {
    fn default() -> Self {
        Self::new()
    }
}

impl Clone for TracingExecutor {
    fn clone(&self) -> Self {
        Self {
            executor: self.executor.duplicate_with_assumption(),
            address_registry: self.address_registry.clone(),
            gas_profile: self.gas_profile.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{synthetic_struct_tag, RecordingStateView};
    use aptos_types::state_store::{
        state_key::StateKey, state_storage_usage::StateStorageUsage, table::TableHandle, TStateView,
    };
    use move_core_types::{
        account_address::AccountAddress, identifier::Identifier, language_storage::StructTag,
    };

    struct DummyStateView;

    impl TStateView for DummyStateView {
        type Key = StateKey;

        fn get_usage(&self) -> aptos_types::state_store::StateViewResult<StateStorageUsage> {
            Ok(StateStorageUsage::zero())
        }

        fn get_state_value(
            &self,
            _state_key: &Self::Key,
        ) -> aptos_types::state_store::StateViewResult<
            Option<aptos_types::state_store::state_value::StateValue>,
        > {
            Ok(None)
        }
    }

    fn struct_tag(name: &str) -> StructTag {
        StructTag {
            address: AccountAddress::ONE,
            module: Identifier::new("m").unwrap(),
            name: Identifier::new(name).unwrap(),
            type_args: vec![],
        }
    }

    #[test]
    fn test_synthetic_struct_tag_is_stable_and_prefix_sensitive() {
        let key = StateKey::raw(b"abc");

        let table_tag = synthetic_struct_tag("table_", &key);
        let table_tag_again = synthetic_struct_tag("table_", &key);
        let raw_tag = synthetic_struct_tag("raw_", &key);

        assert_eq!(table_tag, table_tag_again);
        assert_ne!(table_tag, raw_tag);
        assert_eq!(table_tag.module.as_str(), "global_state");
        assert!(table_tag.name.as_str().starts_with("table_"));
        assert!(raw_tag.name.as_str().starts_with("raw_"));
    }

    #[test]
    fn test_recording_state_view_extracts_and_deduplicates_resource_reads() {
        let inner = DummyStateView;
        let view = RecordingStateView::new(&inner);

        let resource_key = StateKey::resource(
            &AccountAddress::from_hex_literal("0xcafe").unwrap(),
            &struct_tag("Coin"),
        )
        .unwrap();
        let table_key = StateKey::table_item(
            &TableHandle(AccountAddress::from_hex_literal("0xbeef").unwrap()),
            b"k",
        );
        let raw_key = StateKey::raw(b"raw-state");
        let module_name = Identifier::new("mod").unwrap();
        let code_key = StateKey::module(&AccountAddress::ONE, module_name.as_ident_str());

        view.get_state_value(&resource_key).unwrap();
        view.get_state_value(&resource_key).unwrap();
        view.get_state_value(&table_key).unwrap();
        view.get_state_value(&raw_key).unwrap();
        view.get_state_value(&code_key).unwrap();

        let reads = view.extract_resource_reads();
        assert_eq!(reads.len(), 3);
        assert!(reads.iter().any(|read| {
            read.address == AccountAddress::from_hex_literal("0xcafe").unwrap()
                && read.struct_tag == struct_tag("Coin")
                && !read.is_resource_group
        }));
        assert!(reads
            .iter()
            .any(|read| read.struct_tag.name.as_str().starts_with("table_")));
        assert!(reads
            .iter()
            .any(|read| read.struct_tag.name.as_str().starts_with("raw_")));
    }
}
