// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    data_cache::get_resource_group_member_from_metadata,
    move_vm_ext::{resource_state_key, SessionId},
};
use aptos_framework::natives::{
    aggregator_natives::NativeAggregatorContext,
    code::{NativeCodeContext, PublishRequest},
    cryptography::{algebra::AlgebraContext, ristretto255_point::NativeRistrettoPointContext},
    event::NativeEventContext,
    object::NativeObjectContext,
    randomness::RandomnessContext,
    state_storage::NativeStateStorageContext,
    transaction_context::NativeTransactionContext,
};
use aptos_table_natives::NativeTableContext;
use aptos_types::{
    state_store::state_key::StateKey, transaction::user_transaction_context::UserTransactionContext,
};
use aptos_vm_environment::environment::AptosEnvironment;
use aptos_vm_types::{
    change_set::VMChangeSet, module_and_script_storage::code_storage::AptosCodeStorage,
    resolver::ExecutorView, storage::change_set_configs::ChangeSetConfigs,
};
use bytes::Bytes;
use move_binary_format::errors::{Location, PartialVMError, PartialVMResult, VMResult};
use move_core_types::{
    account_address::AccountAddress,
    gas_algebra::NumBytes,
    identifier::IdentStr,
    language_storage::{ModuleId, StructTag, TypeTag},
    value::MoveTypeLayout,
    vm_status::StatusCode,
};
use move_vm_runtime::{
    data_cache::TransactionDataCache,
    module_traversal::TraversalContext,
    move_vm::{MoveVM, SerializedReturnValues},
    native_extensions::NativeContextExtensions,
    FunctionValueExtensionAdapter, LayoutConverter, LoadedFunction, ModuleStorage,
    StorageLayoutConverter,
};
use move_vm_types::{
    gas::GasMeter,
    loaded_data::runtime_types::Type,
    resolver::{resource_size, ResourceResolver},
    value_serde::ValueSerDeContext,
    values::GlobalValue,
};
use std::{
    borrow::Borrow,
    cell::RefCell,
    collections::{BTreeMap, HashSet},
};

struct LoadedData {
    value: GlobalValue,
    layout: MoveTypeLayout,
    has_identifier_mappings: bool,
    // pointer to write op information, if exists. Info can be invalidated.
    partial_write_set: Option<usize>,
}

#[derive(Default)]
struct VersionedLoadedData {
    versions: BTreeMap<u8, LoadedData>,
}

pub struct AptosTransactionDataCache {
    current_session_id: u8,
    data_cache: BTreeMap<AccountAddress, BTreeMap<Type, VersionedLoadedData>>,
}

impl AptosTransactionDataCache {
    pub fn empty() -> Self {
        Self {
            current_session_id: 0,
            data_cache: BTreeMap::new(),
        }
    }
}

pub struct Session<'ctx, C, T> {
    data_cache: AptosTransactionDataCache,
    resolver: Resolver<'ctx, T, C>,
    #[allow(dead_code)]
    env: AptosEnvironment,
    extensions: NativeContextExtensions<'ctx>,
    #[allow(dead_code)]
    change_set_configs: ChangeSetConfigs,
}

struct Resolver<'ctx, T, C> {
    executor_view: &'ctx T,
    module_storage: &'ctx C,
    accessed_groups: RefCell<HashSet<StateKey>>,
}

impl<'ctx, T, C> ResourceResolver for Resolver<'ctx, T, C>
where
    T: ExecutorView,
    C: AptosCodeStorage,
{
    fn get_resource_bytes_with_metadata_and_layout(
        &self,
        address: &AccountAddress,
        struct_tag: &StructTag,
        layout: Option<&MoveTypeLayout>,
    ) -> PartialVMResult<(Option<Bytes>, usize)> {
        let metadata = self
            .module_storage
            .fetch_existing_module_metadata(&struct_tag.address, struct_tag.module.as_ident_str())
            .map_err(|err| err.to_partial())?;
        let resource_group = get_resource_group_member_from_metadata(struct_tag, &metadata);

        Ok(if let Some(resource_group) = resource_group {
            let key = StateKey::resource_group(address, &resource_group);
            let buf = self
                .executor_view
                .get_resource_from_group(&key, struct_tag, layout)?;

            let first_access = self.accessed_groups.borrow_mut().insert(key.clone());
            let group_size = if first_access {
                self.executor_view.resource_group_size(&key)?.get()
            } else {
                0
            };

            let buf_size = resource_size(&buf);
            (buf, buf_size + group_size as usize)
        } else {
            let state_key = resource_state_key(address, struct_tag)?;
            let buf = self.executor_view.get_resource_bytes(&state_key, layout)?;
            let buf_size = resource_size(&buf);
            (buf, buf_size)
        })
    }
}

impl<'ctx, C, T> Session<'ctx, C, T>
where
    C: AptosCodeStorage,
    T: ExecutorView,
{
    pub fn new(
        executor_view: &'ctx T,
        module_storage: &'ctx C,
        env: AptosEnvironment,
        session_id: SessionId,
        maybe_user_transaction_context: Option<UserTransactionContext>,
        change_set_configs: ChangeSetConfigs,
    ) -> Self {
        let mut extensions = NativeContextExtensions::default();
        let txn_hash: [u8; 32] = session_id
            .as_uuid()
            .to_vec()
            .try_into()
            .expect("HashValue should convert to [u8; 32]");

        extensions.add(NativeTableContext::new(txn_hash, executor_view));
        extensions.add(NativeRistrettoPointContext::new());
        extensions.add(AlgebraContext::new());
        extensions.add(NativeAggregatorContext::new(
            txn_hash,
            executor_view,
            env.vm_config().delayed_field_optimization_enabled,
            executor_view,
        ));
        extensions.add(RandomnessContext::new());
        extensions.add(NativeTransactionContext::new(
            txn_hash.to_vec(),
            session_id.into_script_hash(),
            env.chain_id().id(),
            maybe_user_transaction_context,
        ));
        extensions.add(NativeCodeContext::new());
        extensions.add(NativeStateStorageContext::new(executor_view));
        extensions.add(NativeEventContext::default());
        extensions.add(NativeObjectContext::default());

        Self {
            data_cache: AptosTransactionDataCache::empty(),
            resolver: Resolver {
                accessed_groups: RefCell::new(HashSet::new()),
                executor_view,
                module_storage,
            },
            env,
            extensions,
            change_set_configs,
        }
    }

    pub fn code_storage(&self) -> &'ctx C {
        self.resolver.module_storage
    }

    pub fn executor_view(&self) -> &'ctx T {
        self.resolver.executor_view
    }

    pub fn execute_entry_function(
        &mut self,
        func: LoadedFunction,
        args: Vec<impl Borrow<[u8]>>,
        gas_meter: &mut impl GasMeter,
        traversal_context: &mut TraversalContext,
    ) -> VMResult<()> {
        if !func.is_entry() {
            let module_id = func
                .module_id()
                .ok_or_else(|| {
                    let msg = "Entry function always has module id".to_string();
                    PartialVMError::new(StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR)
                        .with_message(msg)
                        .finish(Location::Undefined)
                })?
                .clone();
            return Err(PartialVMError::new(
                StatusCode::EXECUTE_ENTRY_FUNCTION_CALLED_ON_NON_ENTRY_FUNCTION,
            )
            .finish(Location::Module(module_id)));
        }

        MoveVM::execute_loaded_function(
            func,
            args,
            &mut self.data_cache,
            gas_meter,
            traversal_context,
            &mut self.extensions,
            self.resolver.module_storage,
            &self.resolver,
        )?;
        Ok(())
    }

    pub fn execute_function_bypass_visibility(
        &mut self,
        module_id: &ModuleId,
        function_name: &IdentStr,
        ty_args: Vec<TypeTag>,
        args: Vec<impl Borrow<[u8]>>,
        gas_meter: &mut impl GasMeter,
        traversal_context: &mut TraversalContext,
    ) -> VMResult<SerializedReturnValues> {
        let func =
            self.resolver
                .module_storage
                .load_function(module_id, function_name, &ty_args)?;
        MoveVM::execute_loaded_function(
            func,
            args,
            &mut self.data_cache,
            gas_meter,
            traversal_context,
            &mut self.extensions,
            self.resolver.module_storage,
            &self.resolver,
        )
    }

    pub(crate) fn execute_init_hack(
        &mut self,
        module_id: &ModuleId,
        function_name: &IdentStr,
        ty_args: Vec<TypeTag>,
        args: Vec<impl Borrow<[u8]>>,
        gas_meter: &mut impl GasMeter,
        traversal_context: &mut TraversalContext,
        module_storage: &impl ModuleStorage,
    ) -> VMResult<SerializedReturnValues> {
        let func =
            self.resolver
                .module_storage
                .load_function(module_id, function_name, &ty_args)?;
        MoveVM::execute_loaded_function(
            func,
            args,
            &mut self.data_cache,
            gas_meter,
            traversal_context,
            &mut self.extensions,
            module_storage,
            &self.resolver,
        )
    }

    pub fn execute_loaded_function(
        &mut self,
        func: LoadedFunction,
        args: Vec<impl Borrow<[u8]>>,
        gas_meter: &mut impl GasMeter,
        traversal_context: &mut TraversalContext,
    ) -> VMResult<SerializedReturnValues> {
        MoveVM::execute_loaded_function(
            func,
            args,
            &mut self.data_cache,
            gas_meter,
            traversal_context,
            &mut self.extensions,
            self.resolver.module_storage,
            &self.resolver,
        )
    }

    pub fn finish(&mut self) -> VMResult<VMChangeSet> {
        // same as before, but we need to use mut self! Should it be immutable reference?
        unimplemented!()
    }

    pub fn release_resource_group_cache(&self) {
        self.resolver.executor_view.release_group_cache();
    }

    pub fn change_set_configs(&self) -> &ChangeSetConfigs {
        &self.change_set_configs
    }

    /// Returns the publish request if it exists. If the provided flag is set to true, disables any
    /// subsequent module publish requests.
    #[allow(dead_code)]
    pub(crate) fn extract_publish_request(&mut self) -> Option<PublishRequest> {
        let ctx = self.extensions.get_mut::<NativeCodeContext>();
        ctx.extract_publish_request()
    }

    pub(crate) fn mark_unbiasable(&mut self) {
        let txn_context = self.extensions.get_mut::<RandomnessContext>();
        txn_context.mark_unbiasable();
    }

    // this works for revert as well!
    // we should encapsulate this, probably be encapsulating sessions because we can have same id
    // type but different inner types which need to map to different ids?
    pub fn reset(&mut self, id: u8, session_id: SessionId) {
        // anything to assert? we should clean up if we have max M, but we go from S < M to T > M.
        // lazy cleanup
        self.data_cache.current_session_id = id;

        // Need to update extensions that use transaction and script hashes.
        let txn_hash: [u8; 32] = session_id
            .as_uuid()
            .to_vec()
            .try_into()
            .expect("HashValue should convert to [u8; 32]");
        let script_hash = session_id.into_script_hash();

        let table_context = self.extensions.get_mut::<NativeTableContext>();
        table_context.reset_txn_hash(txn_hash);

        let aggregator_context = self.extensions.get_mut::<NativeAggregatorContext>();
        aggregator_context.reset_txn_hash(txn_hash);

        let txn_context = self.extensions.get_mut::<NativeTransactionContext>();
        txn_context.reset_txn_and_script_hashes(txn_hash.to_vec(), script_hash);

        // TODO: do we need to reset "stateless" extensions????
    }

    pub fn snapshot_and_reset_session_id(&mut self, session_id: SessionId) {
        self.data_cache.current_session_id += 1;

        // Need to update extensions that use transaction and script hashes.
        let txn_hash: [u8; 32] = session_id
            .as_uuid()
            .to_vec()
            .try_into()
            .expect("HashValue should convert to [u8; 32]");
        let script_hash = session_id.into_script_hash();

        let table_context = self.extensions.get_mut::<NativeTableContext>();
        table_context.reset_txn_hash(txn_hash);

        let aggregator_context = self.extensions.get_mut::<NativeAggregatorContext>();
        aggregator_context.reset_txn_hash(txn_hash);

        let txn_context = self.extensions.get_mut::<NativeTransactionContext>();
        txn_context.reset_txn_and_script_hashes(txn_hash.to_vec(), script_hash);
    }

    #[allow(dead_code)]
    pub fn view_latest() {}

    // for init module: enable checks or clear cache or similar.
}

#[allow(dead_code)]
pub struct PrologueSession<'ctx, C, T> {
    session: Session<'ctx, C, T>,
}

impl<'ctx, C, T> PrologueSession<'ctx, C, T>
where
    C: AptosCodeStorage,
    T: ExecutorView,
{
    // pub fn new()
}

// let session = Session::new();
// session.run(); --> prologue
// session.snapshot();
//
// session.run() --> user
// session.view() --> to charge gas.

// view:
//  for each (addr, tag)
//     create a state key
//     create a write op
//  convert
//  map (addr, tag) -> write op map

// finish()
// for all (addr, tag) that have been updated {
//   if their write exists, use it (once)
//   if their write does not exist, recompute write op for this group.
// }

impl TransactionDataCache for AptosTransactionDataCache {
    fn load_resource(
        &mut self,
        module_storage: &dyn ModuleStorage,
        resource_resolver: &dyn ResourceResolver,
        addr: AccountAddress,
        ty: &Type,
        is_mut: bool,
    ) -> PartialVMResult<(&mut GlobalValue, Option<NumBytes>)> {
        let data_cache = self.data_cache.entry(addr).or_default();
        let versions = &mut data_cache.entry(ty.clone()).or_default().versions;

        let mut loaded_size = None;

        let prev = versions.range(0..self.current_session_id + 1).next_back();

        // TODO: should we charge for cow?

        // if id is the old one, and we borrow_mut, create a new entry ( ~ clone on write) or if it
        // did not exist
        if let Some((id, old_data)) = prev {
            if is_mut && id != &self.current_session_id {
                if let Some(_idx) = &old_data.partial_write_set {
                    // assert dirty flag is set for global value
                    // todo: invalidate cached write op.
                }
                versions.insert(self.current_session_id + 1, LoadedData {
                    value: old_data.value.deep_copy()?,
                    // todo: layout check?
                    layout: old_data.layout.clone(),
                    has_identifier_mappings: old_data.has_identifier_mappings,
                    partial_write_set: None,
                });
            }
        } else {
            // The tag must be a struct for the type to be a resource.
            let ty_tag = module_storage.runtime_environment().ty_to_ty_tag(ty)?;
            let struct_tag = match ty_tag {
                TypeTag::Struct(struct_tag) => struct_tag,
                _ => return Err(PartialVMError::new(StatusCode::INTERNAL_TYPE_ERROR)),
            };

            let (layout, has_identifier_mappings) = StorageLayoutConverter::new(module_storage)
                .type_to_type_layout_with_identifier_mappings(ty)?;

            let (data, bytes_loaded) = resource_resolver
                .get_resource_bytes_with_metadata_and_layout(
                    &addr,
                    &struct_tag,
                    if has_identifier_mappings {
                        Some(&layout)
                    } else {
                        None
                    },
                )?;
            loaded_size = Some(NumBytes::new(bytes_loaded as u64));

            let function_value_extension = FunctionValueExtensionAdapter { module_storage };
            let value = match data {
                Some(blob) => {
                    let val = match ValueSerDeContext::new()
                        .with_func_args_deserialization(&function_value_extension)
                        .with_delayed_fields_serde()
                        .deserialize(&blob, &layout)
                    {
                        Some(val) => val,
                        None => {
                            let msg = format!(
                                "Failed to deserialize resource {} at {}!",
                                struct_tag, addr
                            );
                            return Err(PartialVMError::new(
                                StatusCode::FAILED_TO_DESERIALIZE_RESOURCE,
                            )
                            .with_message(msg));
                        },
                    };

                    GlobalValue::cached(val)?
                },
                None => GlobalValue::none(),
            };

            versions.insert(self.current_session_id + 1, LoadedData {
                value,
                layout,
                has_identifier_mappings,
                partial_write_set: None,
            });
        }

        let value = &mut versions
            .range_mut(0..self.current_session_id)
            .next_back()
            .expect("Value have just been inserted")
            .1
            .value;
        Ok((value, loaded_size))
    }
}
