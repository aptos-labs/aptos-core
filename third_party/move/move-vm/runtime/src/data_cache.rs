// Copyright (c) The Diem Core Contributors
// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::{
    module_traversal::TraversalContext,
    native_functions::DependencyGasMeterWrapper,
    storage::{
        loader::traits::{ModuleMetadataLoader, StructDefinitionLoader},
        module_storage::FunctionValueExtensionAdapter,
        ty_layout_converter::LayoutConverter,
    },
    Loader, ModuleStorage,
};
use bytes::Bytes;
use std::sync::atomic::{AtomicU64, Ordering as AtomicOrd};

/// Reset resource load stats counters. Call at start of benchmark phase to exclude setup.
pub fn reset_resource_load_stats() {
    RESET_RESOURCE_LOAD_FLAG.store(1, AtomicOrd::Relaxed);
    eprintln!("[RL] Reset requested");
}

static RESET_RESOURCE_LOAD_FLAG: AtomicU64 = AtomicU64::new(0);
use move_binary_format::errors::*;
use move_core_types::{
    account_address::AccountAddress,
    effects::{AccountChanges, ChangeSet, Changes},
    gas_algebra::NumBytes,
    language_storage::{StructTag, TypeTag},
    value::MoveTypeLayout,
    vm_status::StatusCode,
};
use move_vm_types::{
    gas::DependencyGasMeter,
    loaded_data::runtime_types::Type,
    resolver::ResourceResolver,
    value_serde::{FunctionValueExtension, ValueSerDeContext},
    values::{GlobalValue, Value},
};
use std::collections::btree_map::{BTreeMap, Entry};
use triomphe::Arc as TriompheArc;

/// A hack to be able to use [MoveVmDataCache] in native context where there is no access to
/// static gas meter.
pub trait NativeContextMoveVmDataCache {
    /// Used by native context only! Returns true if resource exists in global storage, and false
    /// otherwise. Also, returns the number of bytes loaded (if any, otherwise [None]).
    fn native_check_resource_exists(
        &mut self,
        gas_meter: &mut dyn DependencyGasMeter,
        traversal_context: &mut TraversalContext,
        addr: &AccountAddress,
        ty: &Type,
    ) -> PartialVMResult<(bool, Option<NumBytes>)>;
}

/// Provides access to global storage for Move VM.
pub trait MoveVmDataCache: NativeContextMoveVmDataCache {
    /// Loads resource from global storage. Returns the immutable reference to it, along with the
    /// number of bytes loaded (if any, otherwise [None]).
    ///
    /// Note: default implementation loads the resource for mutation, casting the mutable reference
    /// to immutable.
    fn load_resource(
        &mut self,
        gas_meter: &mut impl DependencyGasMeter,
        traversal_context: &mut TraversalContext,
        addr: &AccountAddress,
        ty: &Type,
    ) -> PartialVMResult<(&GlobalValue, Option<NumBytes>)> {
        let (gv, bytes_loaded) = self.load_resource_mut(gas_meter, traversal_context, addr, ty)?;
        Ok((gv, bytes_loaded))
    }

    /// Loads resource from global storage. Returns the mutable reference to it, along with the
    /// number of bytes loaded (if any, otherwise [None]).
    fn load_resource_mut(
        &mut self,
        gas_meter: &mut impl DependencyGasMeter,
        traversal_context: &mut TraversalContext,
        addr: &AccountAddress,
        ty: &Type,
    ) -> PartialVMResult<(&mut GlobalValue, Option<NumBytes>)>;
}

/// Adapter for data cache that also stores references to code and data global storages. In case
/// resource is not yet in data cache, global storage is used to add it there.
pub struct MoveVmDataCacheAdapter<'a, LoaderImpl> {
    data_cache: &'a mut TransactionDataCache,
    resource_resolver: &'a dyn ResourceResolver,
    loader: &'a LoaderImpl,
}

impl<'a, LoaderImpl> NativeContextMoveVmDataCache for MoveVmDataCacheAdapter<'a, LoaderImpl>
where
    LoaderImpl: Loader,
{
    fn native_check_resource_exists(
        &mut self,
        gas_meter: &mut dyn DependencyGasMeter,
        traversal_context: &mut TraversalContext,
        addr: &AccountAddress,
        ty: &Type,
    ) -> PartialVMResult<(bool, Option<NumBytes>)> {
        let mut gas_meter = DependencyGasMeterWrapper::new(gas_meter);
        let (gv, bytes_loaded) = self.load_resource(&mut gas_meter, traversal_context, addr, ty)?;
        let exists = gv.exists();
        Ok((exists, bytes_loaded))
    }
}

impl<'a, LoaderImpl> MoveVmDataCacheAdapter<'a, LoaderImpl>
where
    LoaderImpl: Loader,
{
    pub fn new(
        data_cache: &'a mut TransactionDataCache,
        resource_resolver: &'a dyn ResourceResolver,
        loader: &'a LoaderImpl,
    ) -> Self {
        Self {
            data_cache,
            resource_resolver,
            loader,
        }
    }
}

impl<'a, LoaderImpl> MoveVmDataCache for MoveVmDataCacheAdapter<'a, LoaderImpl>
where
    LoaderImpl: Loader,
{
    fn load_resource_mut(
        &mut self,
        gas_meter: &mut impl DependencyGasMeter,
        traversal_context: &mut TraversalContext,
        addr: &AccountAddress,
        ty: &Type,
    ) -> PartialVMResult<(&mut GlobalValue, Option<NumBytes>)> {
        let bytes_loaded = if !self.data_cache.contains_resource(addr, ty) {
            let (entry, bytes_loaded) = TransactionDataCache::create_data_cache_entry(
                self.loader,
                &LayoutConverter::new(self.loader),
                gas_meter,
                traversal_context,
                self.loader.unmetered_module_storage(),
                self.resource_resolver,
                addr,
                ty,
            )?;
            self.data_cache.insert_resource(*addr, ty.clone(), entry)?;
            Some(bytes_loaded)
        } else {
            None
        };

        let gv = self.data_cache.get_resource_mut(addr, ty)?;
        Ok((gv, bytes_loaded))
    }
}

/// An entry in the data cache, containing resource's [GlobalValue] as well as additional cached
/// information such as tag, layout, and a flag whether there are any delayed fields inside the
/// resource.
struct DataCacheEntry {
    struct_tag: StructTag,
    layout: TriompheArc<MoveTypeLayout>,
    contains_delayed_fields: bool,
    value: GlobalValue,
}

/// Transaction data cache. Keep updates within a transaction so they can all be published at
/// once when the transaction succeeds.
///
/// It also provides an implementation for the opcodes that refer to storage and gives the
/// proper guarantees of reference lifetime.
///
/// Dirty objects are serialized and returned in make_write_set.
///
/// It is a responsibility of the client to publish changes once the transaction is executed.
///
/// The Move VM takes a `DataStore` in input and this is the default and correct implementation
/// for a data store related to a transaction. Clients should create an instance of this type
/// and pass it to the Move VM.
pub struct TransactionDataCache {
    account_map: BTreeMap<AccountAddress, BTreeMap<Type, DataCacheEntry>>,
}

impl TransactionDataCache {
    /// Create a `TransactionDataCache` with a `RemoteCache` that provides access to data
    /// not updated in the transaction.
    pub fn empty() -> Self {
        TransactionDataCache {
            account_map: BTreeMap::new(),
        }
    }

    /// Make a write set from the updated (dirty, deleted) global resources along with
    /// published modules.
    ///
    /// Gives all proper guarantees on lifetime of global data as well.
    pub fn into_effects(self, module_storage: &dyn ModuleStorage) -> PartialVMResult<ChangeSet> {
        let resource_converter = |value: Value,
                                  layout: TriompheArc<MoveTypeLayout>,
                                  _: bool|
         -> PartialVMResult<Bytes> {
            let function_value_extension = FunctionValueExtensionAdapter { module_storage };
            let max_value_nest_depth = function_value_extension.max_value_nest_depth();
            ValueSerDeContext::new(max_value_nest_depth)
                .with_func_args_deserialization(&function_value_extension)
                .serialize(&value, &layout)?
                .map(Into::into)
                .ok_or_else(|| {
                    // Note: When enable_closure_depth_check is enabled, do not format
                    // `value` here - deeply nested closures can cause stack overflow
                    // during Display formatting.
                    let enable_closure_depth_check = module_storage
                        .runtime_environment()
                        .vm_config()
                        .enable_closure_depth_check;
                    let message = if enable_closure_depth_check {
                        "Error when serializing resource.".to_string()
                    } else {
                        format!("Error when serializing resource {}.", value)
                    };
                    PartialVMError::new(StatusCode::INTERNAL_TYPE_ERROR).with_message(message)
                })
        };
        self.into_custom_effects(&resource_converter)
    }

    /// Same like `into_effects`, but also allows clients to select the format of
    /// produced effects for resources.
    pub fn into_custom_effects<Resource>(
        self,
        resource_converter: &dyn Fn(
            Value,
            TriompheArc<MoveTypeLayout>,
            bool,
        ) -> PartialVMResult<Resource>,
    ) -> PartialVMResult<Changes<Resource>> {
        let mut change_set = Changes::<Resource>::new();
        for (addr, account_data_cache) in self.account_map.into_iter() {
            let mut resources = BTreeMap::new();
            for entry in account_data_cache.into_values() {
                let DataCacheEntry {
                    struct_tag,
                    layout,
                    contains_delayed_fields,
                    value,
                } = entry;
                if let Some(op) = value.into_effect_with_layout(layout) {
                    resources.insert(
                        struct_tag,
                        op.and_then(|(value, layout)| {
                            resource_converter(value, layout, contains_delayed_fields)
                        })?,
                    );
                }
            }
            if !resources.is_empty() {
                change_set
                    .add_account_changeset(addr, AccountChanges::from_resources(resources))
                    .expect("accounts should be unique");
            }
        }

        Ok(change_set)
    }

    /// Retrieves data from the remote on-chain storage and converts it into a [DataCacheEntry].
    /// Also returns the size of the loaded resource in bytes. This method does not add the entry
    /// to the cache - it is the caller's responsibility to add it there.
    fn create_data_cache_entry(
        metadata_loader: &impl ModuleMetadataLoader,
        layout_converter: &LayoutConverter<impl StructDefinitionLoader>,
        gas_meter: &mut impl DependencyGasMeter,
        traversal_context: &mut TraversalContext,
        module_storage: &dyn ModuleStorage,
        resource_resolver: &dyn ResourceResolver,
        addr: &AccountAddress,
        ty: &Type,
    ) -> PartialVMResult<(DataCacheEntry, NumBytes)> {
        use std::sync::atomic::{AtomicU64, Ordering as AO};
        static RL_CALLS: AtomicU64 = AtomicU64::new(0);
        static RL_FETCH_NANOS: AtomicU64 = AtomicU64::new(0);
        static RL_DESER_NANOS: AtomicU64 = AtomicU64::new(0);
        static RL_TOTAL_NANOS: AtomicU64 = AtomicU64::new(0);
        static RL_BYTES: AtomicU64 = AtomicU64::new(0);
        // Size buckets: 0=empty, 1=1-100B, 2=101-500B, 3=501B-2KB, 4=2-10KB, 5=10KB+
        const NB: usize = 6;
        static B_CALLS: [AtomicU64; NB] = { const Z: AtomicU64 = AtomicU64::new(0); [Z; NB] };
        static B_FETCH: [AtomicU64; NB] = { const Z: AtomicU64 = AtomicU64::new(0); [Z; NB] };
        static B_DESER: [AtomicU64; NB] = { const Z: AtomicU64 = AtomicU64::new(0); [Z; NB] };
        static B_BYTES: [AtomicU64; NB] = { const Z: AtomicU64 = AtomicU64::new(0); [Z; NB] };

        // Check if counters should be reset (set by reset_resource_load_stats)
        if RESET_RESOURCE_LOAD_FLAG.compare_exchange(1, 0, AO::Relaxed, AO::Relaxed).is_ok() {
            RL_CALLS.store(0, AO::Relaxed);
            RL_FETCH_NANOS.store(0, AO::Relaxed);
            RL_DESER_NANOS.store(0, AO::Relaxed);
            RL_TOTAL_NANOS.store(0, AO::Relaxed);
            RL_BYTES.store(0, AO::Relaxed);
            for i in 0..NB {
                B_CALLS[i].store(0, AO::Relaxed);
                B_FETCH[i].store(0, AO::Relaxed);
                B_DESER[i].store(0, AO::Relaxed);
                B_BYTES[i].store(0, AO::Relaxed);
            }
            eprintln!("[RL] Counters reset for benchmark phase");
        }

        let t0 = std::time::Instant::now();

        let struct_tag = match module_storage.runtime_environment().ty_to_ty_tag(ty)? {
            TypeTag::Struct(struct_tag) => *struct_tag,
            _ => {
                return Err(PartialVMError::new(StatusCode::INTERNAL_TYPE_ERROR));
            },
        };

        let layout_with_delayed_fields = layout_converter.type_to_type_layout_with_delayed_fields(
            gas_meter,
            traversal_context,
            ty,
            false,
        )?;

        let (data, bytes_loaded) = {
            let module = metadata_loader.load_module_for_metadata(
                gas_meter,
                traversal_context,
                &struct_tag.module_id(),
            )?;

            resource_resolver.get_resource_bytes_with_metadata_and_layout(
                addr,
                &struct_tag,
                &module.metadata,
                layout_with_delayed_fields.layout_when_contains_delayed_fields(),
            )?
        };
        let t1 = std::time::Instant::now();

        let function_value_extension = FunctionValueExtensionAdapter { module_storage };
        let (layout, contains_delayed_fields) = layout_with_delayed_fields.unpack();
        let value = match data {
            Some(ref blob) => {
                let max_value_nest_depth = function_value_extension.max_value_nest_depth();
                let val = ValueSerDeContext::new(max_value_nest_depth)
                    .with_func_args_deserialization(&function_value_extension)
                    .with_delayed_fields_serde()
                    .deserialize(blob, &layout)
                    .ok_or_else(|| {
                        let msg = format!(
                            "Failed to deserialize resource {} at {}!",
                            struct_tag.to_canonical_string(),
                            addr
                        );
                        PartialVMError::new(StatusCode::FAILED_TO_DESERIALIZE_RESOURCE)
                            .with_message(msg)
                    })?;
                GlobalValue::cached(val)?
            },
            None => GlobalValue::none(),
        };
        let t2 = std::time::Instant::now();

        let sz = data.as_ref().map_or(0u64, |b| b.len() as u64);
        let fetch_ns = (t1 - t0).as_nanos() as u64;
        let deser_ns = (t2 - t1).as_nanos() as u64;
        let total_ns = (t2 - t0).as_nanos() as u64;
        let calls = RL_CALLS.fetch_add(1, AO::Relaxed) + 1;
        RL_FETCH_NANOS.fetch_add(fetch_ns, AO::Relaxed);
        RL_DESER_NANOS.fetch_add(deser_ns, AO::Relaxed);
        RL_TOTAL_NANOS.fetch_add(total_ns, AO::Relaxed);
        RL_BYTES.fetch_add(sz, AO::Relaxed);

        let b = match sz { 0 => 0, 1..=100 => 1, 101..=500 => 2, 501..=2048 => 3, 2049..=10240 => 4, _ => 5 };
        B_CALLS[b].fetch_add(1, AO::Relaxed);
        B_FETCH[b].fetch_add(fetch_ns, AO::Relaxed);
        B_DESER[b].fetch_add(deser_ns, AO::Relaxed);
        B_BYTES[b].fetch_add(sz, AO::Relaxed);

        // Per-resource-type tracking
        {
            use std::sync::Mutex;
            use std::collections::HashMap;
            static TYPE_STATS: std::sync::LazyLock<Mutex<HashMap<String, (u64, u64, u64, u64)>>> =
                std::sync::LazyLock::new(|| Mutex::new(HashMap::new()));

            let name = struct_tag.name.as_str().to_string();
            if let Ok(mut map) = TYPE_STATS.try_lock() {
                let entry = map.entry(name).or_insert((0, 0, 0, 0));
                entry.0 += 1;        // calls
                entry.1 += total_ns; // total_nanos
                entry.2 += sz;       // bytes
                entry.3 += fetch_ns; // fetch_nanos
            }

            if calls % 200000 == 0 {
                if let Ok(map) = TYPE_STATS.lock() {
                    let mut entries: Vec<_> = map.iter().collect();
                    entries.sort_by(|a, b| b.1.1.cmp(&a.1.1)); // sort by total time desc
                    eprintln!("\n[RL-TYPE] Per-resource-type stats (top 20):");
                    eprintln!("  {:40} {:>8} {:>10} {:>8} {:>8} {:>8}",
                        "Resource", "calls", "total_ms", "avg_us", "fetch_us", "bytes");
                    for (name, (c, t, bytes, f)) in entries.iter().take(20) {
                        eprintln!("  {:40} {:>8} {:>10.1} {:>8.1} {:>8.1} {:>8}",
                            name, c, *t as f64/1e6, *t as f64 / *c as f64 / 1e3,
                            *f as f64 / *c as f64 / 1e3, bytes / c);
                    }
                }
            }
        }

        if calls % 200000 == 0 {
            let f = RL_FETCH_NANOS.load(AO::Relaxed);
            let d = RL_DESER_NANOS.load(AO::Relaxed);
            let t = RL_TOTAL_NANOS.load(AO::Relaxed);
            eprintln!(
                "[RL] calls={} total={:.1}ms avg={:.0}ns fetch={:.0}ns deser={:.0}ns avg_bytes={}",
                calls, t as f64/1e6, t as f64/calls as f64,
                f as f64/calls as f64, d as f64/calls as f64,
                RL_BYTES.load(AO::Relaxed)/calls
            );
            let labels = ["empty", "1-100B", "101-500B", "501-2KB", "2-10KB", "10KB+"];
            for i in 0..NB {
                let c = B_CALLS[i].load(AO::Relaxed);
                if c > 0 {
                    eprintln!(
                        "  [{:8}] calls={:>8} avg_bytes={:>5} fetch={:>6.0}ns deser={:>6.0}ns",
                        labels[i], c, B_BYTES[i].load(AO::Relaxed)/c,
                        B_FETCH[i].load(AO::Relaxed) as f64/c as f64,
                        B_DESER[i].load(AO::Relaxed) as f64/c as f64
                    );
                }
            }
        }

        let entry = DataCacheEntry {
            struct_tag,
            layout,
            contains_delayed_fields,
            value,
        };
        Ok((entry, NumBytes::new(bytes_loaded as u64)))
    }

    /// Returns true if resource has been inserted into the cache. Otherwise, returns false. The
    /// state of the cache does not chang when calling this function.
    fn contains_resource(&self, addr: &AccountAddress, ty: &Type) -> bool {
        self.account_map
            .get(addr)
            .is_some_and(|account_cache| account_cache.contains_key(ty))
    }

    /// Stores a new entry for loaded resource into the data cache. Returns an error if there is an
    /// entry already for the specified address-type pair.
    fn insert_resource(
        &mut self,
        addr: AccountAddress,
        ty: Type,
        data_cache_entry: DataCacheEntry,
    ) -> PartialVMResult<()> {
        match self.account_map.entry(addr).or_default().entry(ty.clone()) {
            Entry::Vacant(entry) => entry.insert(data_cache_entry),
            Entry::Occupied(_) => {
                let msg = format!("Entry for {:?} at {} already exists", ty, addr);
                let err = PartialVMError::new(StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR)
                    .with_message(msg);
                return Err(err);
            },
        };
        Ok(())
    }

    /// Returns the resource from the data cache. If resource has not been inserted (i.e., it does
    /// not exist in cache), an error is returned.
    fn get_resource_mut(
        &mut self,
        addr: &AccountAddress,
        ty: &Type,
    ) -> PartialVMResult<&mut GlobalValue> {
        if let Some(account_cache) = self.account_map.get_mut(addr) {
            if let Some(entry) = account_cache.get_mut(ty) {
                return Ok(&mut entry.value);
            }
        }

        let msg = format!("Resource for {:?} at {} must exist", ty, addr);
        let err =
            PartialVMError::new(StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR).with_message(msg);
        Err(err)
    }
}
