// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::{
    config::VMConfig,
    native_functions::{NativeFunction, NativeFunctions},
    storage::{
        ty_tag_converter::{TypeTagCache, TypeTagConverter},
        verified_module_cache::VERIFIED_MODULES_CACHE,
    },
    Module, Script,
};
use ambassador::delegatable_trait;
use bytes::Bytes;
use move_binary_format::{
    access::{ModuleAccess, ScriptAccess},
    errors::{verification_error, Location, PartialVMError, PartialVMResult, VMResult},
    file_format::{CompiledScript, StructFieldInformation, TableIndex},
    CompiledModule, IndexKind,
};
use move_bytecode_verifier::dependencies;
use move_core_types::{
    account_address::AccountAddress,
    identifier::{IdentStr, Identifier},
    language_storage::{ModuleId, TypeTag, MEM_MODULE_ID, OPTION_MODULE_ID},
    value::MoveTypeLayout,
    vm_status::{sub_status::unknown_invariant_violation::EPARANOID_FAILURE, StatusCode},
};
use move_vm_metrics::{Timer, VERIFIED_MODULE_CACHE_SIZE, VM_TIMER};
#[cfg(any(test, feature = "testing"))]
use move_vm_types::loaded_data::{
    runtime_types::StructIdentifier, struct_name_indexing::StructNameIndex,
};
use move_vm_types::{
    loaded_data::{runtime_types::Type, struct_name_indexing::StructNameIndexMap},
    module_id_interner::InternedModuleIdPool,
    ty_interner::InternedTypePool,
    values::{CopyMode, Value},
};
use once_cell::sync::OnceCell;
use parking_lot::Mutex;
use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
};

const OPTION_MODULE_BYTES: &[u8] = include_bytes!("option.mv");
const MEM_MODULE_BYTES: &[u8] = include_bytes!("mem.mv");

/// Holds size estimations for different value representations.
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct SizeEstimates {
    pub variant_and_struct_pointers: Vec<usize>,
    pub variant_pointers_structs_inline: Vec<usize>,
    pub variant_and_structs_inline: Vec<usize>,
}

impl SizeEstimates {
    /// References are just pointer-sized values in any representation.
    pub fn reference() -> Self {
        Self {
            variant_and_struct_pointers: vec![8],
            variant_pointers_structs_inline: vec![8],
            variant_and_structs_inline: vec![8],
        }
    }

    /// Used for non-reference values.
    pub fn new(value: &Value, layout: &MoveTypeLayout) -> Self {
        let estimate_size = |mode: CopyMode| -> Vec<usize> {
            let mut x = value.estimate_copy_size(layout, mode);
            // Vectors may have zero data, so we can skip them here?
            x.retain(|&x| x != 0);
            x
        };
        Self {
            variant_and_struct_pointers: estimate_size(CopyMode::AllPointers),
            variant_pointers_structs_inline: estimate_size(CopyMode::VariantPointers),
            variant_and_structs_inline: estimate_size(CopyMode::AllInline),
        }
    }
}

/// Holds function call statistics
#[derive(Debug, Clone)]
pub struct FunctionCallStats {
    pub call_count: usize,
    pub num_locals: usize,
    pub num_instructions: usize,
}

impl FunctionCallStats {
    pub fn new(call_count: usize, num_locals: usize, num_instructions: usize) -> Self {
        Self {
            call_count,
            num_locals,
            num_instructions,
        }
    }
}

#[derive(Debug, Clone, Default)]
struct SizeStatistics {
    pub variant_and_struct_pointers: HashMap<Vec<usize>, usize>,
    pub variant_pointers_structs_inline: HashMap<Vec<usize>, usize>,
    pub variant_and_structs_inline: HashMap<Vec<usize>, usize>,
    pub all: Vec<SizeEstimates>,
}

impl SizeStatistics {
    fn total(&self) -> usize {
        self.all.len()
    }

    fn record(&mut self, sizes: SizeEstimates) {
        *self
            .variant_and_struct_pointers
            .entry(sizes.variant_and_struct_pointers.clone())
            .or_insert(0) += 1;
        *self
            .variant_pointers_structs_inline
            .entry(sizes.variant_pointers_structs_inline.clone())
            .or_insert(0) += 1;
        *self
            .variant_and_structs_inline
            .entry(sizes.variant_and_structs_inline.clone())
            .or_insert(0) += 1;
        self.all.push(sizes)
    }

    fn display(&self) {
        let display_map = |data: &HashMap<Vec<usize>, usize>| {
            let total_count = data.values().sum::<usize>();
            let mut data = data.clone().into_iter().collect::<Vec<_>>();
            data.sort_by(|a, b| b.1.cmp(&a.1));
            for (size, count) in data {
                let percentage = (count as f64) / total_count as f64 * 100.0;
                let sum = size.iter().copied().sum::<usize>();
                let max = size.iter().copied().max().unwrap_or(0);
                println!("  {} ({:.1}%): [size[0] = {}, len(size)={}, sum(size)={}, max(size)={}, {:?}]", count, percentage, size[0], size.len(), sum, max, size);
            }
        };

        println!("variant_and_structs_inline:");
        display_map(&self.variant_and_structs_inline);

        println!("variant_pointers_structs_inline:");
        display_map(&self.variant_pointers_structs_inline);

        println!("variant_and_struct_pointers:");
        display_map(&self.variant_and_struct_pointers);
    }
}

/// Statistics about execution.
#[derive(Debug, Default)]
pub struct Statistics {
    instruction_counts: Mutex<HashMap<String, usize>>,

    primitive_write_refs: AtomicUsize,
    write_ref_sizes: Mutex<SizeStatistics>,

    primitive_read_refs: AtomicUsize,
    read_ref_sizes: Mutex<SizeStatistics>,

    primitive_copy_locs: AtomicUsize,
    reference_copy_locs: AtomicUsize,
    copy_loc_sizes: Mutex<SizeStatistics>,

    primitive_move_locs: AtomicUsize,
    reference_move_locs: AtomicUsize,
    move_loc_sizes: Mutex<SizeStatistics>,

    primitive_st_locs: AtomicUsize,
    reference_st_locs: AtomicUsize,
    st_loc_sizes: Mutex<SizeStatistics>,

    packed_structs: Mutex<HashMap<String, usize>>,
    pack_sizes: Mutex<SizeStatistics>,

    unpacked_structs: Mutex<HashMap<String, usize>>,
    unpack_sizes: Mutex<SizeStatistics>,

    packed_variants: Mutex<HashMap<String, usize>>,
    pack_variant_sizes: Mutex<SizeStatistics>,

    unpacked_variants: Mutex<HashMap<String, usize>>,
    unpack_variant_sizes: Mutex<SizeStatistics>,

    total_packed_unpacked_structs: Mutex<HashMap<String, usize>>,
    total_packed_unpacked_structs_sizes: Mutex<SizeStatistics>,

    moved_to: Mutex<HashMap<String, usize>>,
    move_to_sizes: Mutex<SizeStatistics>,

    moved_from: Mutex<HashMap<String, usize>>,
    move_from_sizes: Mutex<SizeStatistics>,

    borrowed_global_imm: Mutex<HashMap<String, usize>>,
    borrowed_global_sizes_imm: Mutex<SizeStatistics>,

    borrowed_global_mut: Mutex<HashMap<String, usize>>,
    borrowed_global_sizes_mut: Mutex<SizeStatistics>,

    borrowed_global: Mutex<HashMap<String, usize>>,
    borrowed_global_sizes: Mutex<SizeStatistics>,
}

impl Statistics {
    pub fn display(&self) {
        {
            let mut instr_counts = self
                .instruction_counts
                .lock()
                .clone()
                .into_iter()
                .collect::<Vec<_>>();
            instr_counts.sort_by(|a, b| b.1.cmp(&a.1));
            let total_instr_counts = instr_counts.iter().map(|x| x.1).sum::<usize>();
            println!("instruction frequencies ({})", total_instr_counts);
            for (instr, count) in instr_counts {
                let percentage = (count as f64 / total_instr_counts as f64) * 100.0;
                println!("{} ({:.1}%): {}", count, percentage, instr);
            }
            println!();
        }

        {
            let primitive_write_refs = self.primitive_write_refs.load(Ordering::SeqCst);
            let total_write_refs = self.write_ref_sizes.lock().total();
            let other_write_refs = total_write_refs - primitive_write_refs;
            let primitive_write_refs_percentage =
                (primitive_write_refs as f64 / total_write_refs as f64) * 100.0;
            let other_write_refs_percentage =
                (other_write_refs as f64 / total_write_refs as f64) * 100.0;
            println!("write_ref frequencies");
            println!(
                "{} ({:.1}%): primitives",
                primitive_write_refs, primitive_write_refs_percentage
            );
            println!(
                "{} ({:.1}%): other",
                other_write_refs, other_write_refs_percentage
            );

            println!("write_ref sizes");
            self.write_ref_sizes.lock().display();
            println!();
        }

        {
            let primitive_read_refs = self.primitive_read_refs.load(Ordering::SeqCst);
            let total_read_refs = self.read_ref_sizes.lock().total();
            let other_read_refs = total_read_refs - primitive_read_refs;
            let primitive_read_refs_percentage =
                (primitive_read_refs as f64 / total_read_refs as f64) * 100.0;
            let other_read_refs_percentage =
                (other_read_refs as f64 / total_read_refs as f64) * 100.0;
            println!("read_ref frequencies");
            println!(
                "{} ({:.1}%): primitives",
                primitive_read_refs, primitive_read_refs_percentage
            );
            println!(
                "{} ({:.1}%): other",
                other_read_refs, other_read_refs_percentage
            );

            println!("read_ref sizes");
            self.read_ref_sizes.lock().display();
            println!();
        }

        {
            let primitive_copy_locs = self.primitive_copy_locs.load(Ordering::SeqCst);
            let reference_copy_locs = self.reference_copy_locs.load(Ordering::SeqCst);
            let total_copy_locs = self.copy_loc_sizes.lock().total();
            let other_copy_locs = total_copy_locs - reference_copy_locs - primitive_copy_locs;
            let primitive_copy_locs_percentage =
                (primitive_copy_locs as f64 / total_copy_locs as f64) * 100.0;
            let reference_copy_locs_percentage =
                (reference_copy_locs as f64 / total_copy_locs as f64) * 100.0;
            let other_copy_locs_percentage =
                (other_copy_locs as f64 / total_copy_locs as f64) * 100.0;
            println!("copy_loc frequencies");
            println!(
                "{} ({:.1}%): primitives",
                primitive_copy_locs, primitive_copy_locs_percentage
            );
            println!(
                "{} ({:.1}%): references",
                reference_copy_locs, reference_copy_locs_percentage
            );
            println!(
                "{} ({:.1}%): other",
                other_copy_locs, other_copy_locs_percentage
            );

            println!("copy_loc sizes");
            self.copy_loc_sizes.lock().display();
            println!();
        }

        {
            let primitive_move_locs = self.primitive_move_locs.load(Ordering::SeqCst);
            let reference_move_locs = self.reference_move_locs.load(Ordering::SeqCst);
            let total_move_locs = self.move_loc_sizes.lock().total();
            let other_move_locs = total_move_locs - reference_move_locs - primitive_move_locs;
            let primitive_move_locs_percentage =
                (primitive_move_locs as f64 / total_move_locs as f64) * 100.0;
            let reference_move_locs_percentage =
                (reference_move_locs as f64 / total_move_locs as f64) * 100.0;
            let other_move_locs_percentage =
                (other_move_locs as f64 / total_move_locs as f64) * 100.0;
            println!("move_loc frequencies");
            println!(
                "{} ({:.1}%): primitives",
                primitive_move_locs, primitive_move_locs_percentage
            );
            println!(
                "{} ({:.1}%): references",
                reference_move_locs, reference_move_locs_percentage
            );
            println!(
                "{} ({:.1}%): other",
                other_move_locs, other_move_locs_percentage
            );

            println!("move_loc sizes");
            self.move_loc_sizes.lock().display();
            println!();
        }

        {
            let primitive_st_locs = self.primitive_st_locs.load(Ordering::SeqCst);
            let reference_st_locs = self.reference_st_locs.load(Ordering::SeqCst);
            let total_st_locs = self.st_loc_sizes.lock().total();
            let other_st_locs = total_st_locs - reference_st_locs - primitive_st_locs;
            let primitive_st_locs_percentage =
                (primitive_st_locs as f64 / total_st_locs as f64) * 100.0;
            let reference_st_locs_percentage =
                (reference_st_locs as f64 / total_st_locs as f64) * 100.0;
            let other_st_locs_percentage = (other_st_locs as f64 / total_st_locs as f64) * 100.0;
            println!("st_loc frequencies");
            println!(
                "{} ({:.1}%): primitives",
                primitive_st_locs, primitive_st_locs_percentage
            );
            println!(
                "{} ({:.1}%): references",
                reference_st_locs, reference_st_locs_percentage
            );
            println!(
                "{} ({:.1}%): other",
                other_st_locs, other_st_locs_percentage
            );

            println!("st_loc sizes");
            self.st_loc_sizes.lock().display();
            println!();
        }

        {
            let total_moved_to = self.move_to_sizes.lock().total();
            let total_moved_to_structs = self.moved_to.lock().clone().len();

            println!("move_to frequencies");
            println!("{} instructions", total_moved_to);
            println!("{} structs/enums", total_moved_to_structs);

            println!("move_to sizes");
            self.move_to_sizes.lock().display();
            println!();
        }

        {
            let total_moved_from = self.move_from_sizes.lock().total();
            let total_moved_from_structs = self.moved_from.lock().clone().len();

            println!("move_from frequencies");
            println!("{} instructions", total_moved_from);
            println!("{} structs/enums", total_moved_from_structs);

            println!("move_from sizes");
            self.move_from_sizes.lock().display();
            println!();
        }

        {
            let total_structs = self.borrowed_global.lock().len();
            let total_imm_structs = self.borrowed_global_imm.lock().len();
            let total_mut_structs = self.borrowed_global_mut.lock().len();

            println!("borrow_global frequencies");
            println!("{} structs (total)", total_structs);
            println!("{} structs (imm)", total_imm_structs);
            println!("{} structs (mut)", total_mut_structs);
            println!("------------------");

            let num_global_borrows_mut = self.borrowed_global_sizes_mut.lock().total();
            let num_global_borrows_imm = self.borrowed_global_sizes_imm.lock().total();
            let num_global_borrows = self.borrowed_global_sizes.lock().total();

            let num_global_borrows_mut_percentage =
                (num_global_borrows_mut as f64 / num_global_borrows as f64) * 100.0;
            let num_global_borrows_imm_percentage =
                (num_global_borrows_imm as f64 / num_global_borrows as f64) * 100.0;

            println!(
                "{} ({:.1}%): borrow_global_mut",
                num_global_borrows_mut, num_global_borrows_mut_percentage
            );
            println!(
                "{} ({:.1}%): borrow_global",
                num_global_borrows_imm, num_global_borrows_imm_percentage
            );
            println!("------------------");

            println!("borrow_global (total) sizes");
            self.borrowed_global_sizes.lock().display();
            println!("------------------");
            println!("borrow_global (mut) sizes");
            self.borrowed_global_sizes_mut.lock().display();
            println!("------------------");
            println!("borrow_global (imm) sizes");
            self.borrowed_global_sizes_imm.lock().display();
            println!();
        }

        {
            let num_packs = self.pack_sizes.lock().total();
            let num_pack_variants = self.pack_variant_sizes.lock().total();
            let num_unpacks = self.unpack_sizes.lock().total();
            let num_unpack_variants = self.unpack_variant_sizes.lock().total();
            let total_packs_unpacks = self.total_packed_unpacked_structs_sizes.lock().total();

            let num_packs_percentage = (num_packs as f64 / total_packs_unpacks as f64) * 100.0;
            let num_pack_variants_percentage =
                (num_pack_variants as f64 / total_packs_unpacks as f64) * 100.0;
            let num_unpacks_percentage = (num_unpacks as f64 / total_packs_unpacks as f64) * 100.0;
            let num_unpack_variants_percentage =
                (num_unpack_variants as f64 / total_packs_unpacks as f64) * 100.0;

            let num_packed_structs = self.packed_structs.lock().len();
            let num_packed_variants = self.packed_variants.lock().len();
            let num_unpacked_structs = self.unpacked_structs.lock().len();
            let num_unpacked_variants = self.unpacked_variants.lock().len();
            let total = self.total_packed_unpacked_structs.lock().len();

            println!("pack/unpack frequencies");
            println!("{} total structs/variants", total);
            println!("{} packed structs", num_packed_structs);
            println!("{} packed variants", num_packed_variants);
            println!("{} unpacked structs", num_unpacked_structs);
            println!("{} unpacked variants", num_unpacked_variants);
            println!("------------------");
            println!("{} ({:.1}%) pack", num_packs, num_packs_percentage);
            println!(
                "{} ({:.1}%) pack_variant",
                num_pack_variants, num_pack_variants_percentage
            );
            println!("{} ({:.1}%) unpack", num_unpacks, num_unpacks_percentage);
            println!(
                "{} ({:.1}%) unpack_variant",
                num_unpack_variants, num_unpack_variants_percentage
            );
            println!("------------------");

            println!("total sizes");
            self.total_packed_unpacked_structs_sizes.lock().display();
            println!("------------------");
            println!("pack sizes");
            self.pack_sizes.lock().display();
            println!("------------------");
            println!("pack_variant sizes");
            self.pack_variant_sizes.lock().display();
            println!("------------------");
            println!("unpack sizes");
            self.unpack_sizes.lock().display();
            println!("------------------");
            println!("unpack_variant sizes");
            self.unpack_variant_sizes.lock().display();
            println!();
        }
    }

    pub fn record_instruction(&self, name: &str) {
        let mut counts = self.instruction_counts.lock();
        *counts.entry(name.to_string()).or_insert(0) += 1;
    }

    pub fn record_write_ref(&self, size: SizeEstimates, is_primitive: bool) {
        self.write_ref_sizes.lock().record(size);
        if is_primitive {
            self.primitive_write_refs.fetch_add(1, Ordering::SeqCst);
        }
    }

    pub fn record_read_ref(&self, size: SizeEstimates, is_primitive: bool) {
        self.read_ref_sizes.lock().record(size);
        if is_primitive {
            self.primitive_read_refs.fetch_add(1, Ordering::SeqCst);
        }
    }

    pub fn record_copy_loc(&self, size: SizeEstimates, is_primitive: bool, is_reference: bool) {
        self.copy_loc_sizes.lock().record(size);
        if is_primitive {
            self.primitive_copy_locs.fetch_add(1, Ordering::SeqCst);
        }
        if is_reference {
            self.reference_copy_locs.fetch_add(1, Ordering::SeqCst);
        }
    }

    pub fn record_move_loc(&self, size: SizeEstimates, is_primitive: bool, is_reference: bool) {
        self.move_loc_sizes.lock().record(size);
        if is_primitive {
            self.primitive_move_locs.fetch_add(1, Ordering::SeqCst);
        }
        if is_reference {
            self.reference_move_locs.fetch_add(1, Ordering::SeqCst);
        }
    }

    pub fn record_st_loc(&self, size: SizeEstimates, is_primitive: bool, is_reference: bool) {
        self.st_loc_sizes.lock().record(size);
        if is_primitive {
            self.primitive_st_locs.fetch_add(1, Ordering::SeqCst);
        }
        if is_reference {
            self.reference_st_locs.fetch_add(1, Ordering::SeqCst);
        }
    }

    pub fn record_pack(&self, struct_name: String, size: SizeEstimates) {
        let mut counts = self.packed_structs.lock();
        *counts.entry(struct_name.clone()).or_insert(0) += 1;
        self.pack_sizes.lock().record(size.clone());

        let mut counts = self.total_packed_unpacked_structs.lock();
        *counts.entry(struct_name.clone()).or_insert(0) += 1;
        self.total_packed_unpacked_structs_sizes.lock().record(size);
    }

    pub fn record_pack_variant(&self, struct_name: String, size: SizeEstimates) {
        let mut counts = self.packed_variants.lock();
        *counts.entry(struct_name.clone()).or_insert(0) += 1;
        self.pack_variant_sizes.lock().record(size.clone());

        let mut counts = self.total_packed_unpacked_structs.lock();
        *counts.entry(struct_name.clone()).or_insert(0) += 1;
        self.total_packed_unpacked_structs_sizes.lock().record(size);
    }

    pub fn record_unpack(&self, struct_name: String, size: SizeEstimates) {
        let mut counts = self.unpacked_structs.lock();
        *counts.entry(struct_name.clone()).or_insert(0) += 1;
        self.unpack_sizes.lock().record(size.clone());

        let mut counts = self.total_packed_unpacked_structs.lock();
        *counts.entry(struct_name.clone()).or_insert(0) += 1;
        self.total_packed_unpacked_structs_sizes.lock().record(size);
    }

    pub fn record_unpack_variant(&self, struct_name: String, size: SizeEstimates) {
        let mut counts = self.unpacked_variants.lock();
        *counts.entry(struct_name.clone()).or_insert(0) += 1;
        self.unpack_variant_sizes.lock().record(size.clone());

        let mut counts = self.total_packed_unpacked_structs.lock();
        *counts.entry(struct_name.clone()).or_insert(0) += 1;
        self.total_packed_unpacked_structs_sizes.lock().record(size);
    }

    pub fn record_borrow_global(&self, struct_name: String, size: SizeEstimates, is_mutable: bool) {
        if is_mutable {
            let mut counts = self.borrowed_global_mut.lock();
            *counts.entry(struct_name.clone()).or_insert(0) += 1;
            self.borrowed_global_sizes_mut.lock().record(size.clone());
        } else {
            let mut counts = self.borrowed_global_imm.lock();
            *counts.entry(struct_name.clone()).or_insert(0) += 1;
            self.borrowed_global_sizes_imm.lock().record(size.clone());
        }
        let mut counts = self.borrowed_global.lock();
        *counts.entry(struct_name).or_insert(0) += 1;
        self.borrowed_global_sizes.lock().record(size);
    }

    pub fn record_move_to(&self, struct_name: String, size: SizeEstimates) {
        let mut counts = self.moved_to.lock();
        *counts.entry(struct_name).or_insert(0) += 1;
        self.move_to_sizes.lock().record(size);
    }

    pub fn record_move_from(&self, struct_name: String, size: SizeEstimates) {
        let mut counts = self.moved_from.lock();
        *counts.entry(struct_name).or_insert(0) += 1;
        self.move_from_sizes.lock().record(size);
    }
}

/// Reference wrapper for Statistics that can only be set once.
/// Uses OnceCell to ensure statistics are set exactly once during execution.
#[derive(Debug)]
pub struct StatisticsRef {
    inner: OnceCell<Statistics>,
}

impl StatisticsRef {
    pub fn new() -> Self {
        Self {
            inner: OnceCell::with_value(Statistics::default()),
        }
    }

    /// Gets the statistics if they have been set.
    pub fn get(&self) -> &Statistics {
        self.inner.get().expect("Statistics must be set")
    }
}

/// [MoveVM] runtime environment encapsulating different configurations. Shared between the VM and
/// the code cache, possibly across multiple threads.
pub struct RuntimeEnvironment {
    /// Configuration for the VM. Contains information about enabled checks, verification,
    /// deserialization, etc.
    vm_config: VMConfig,
    /// All registered native functions in the current context (binary). When a verified [Module]
    /// is constructed, existing native functions are inlined in the module representation, so that
    /// the interpreter can call them directly.
    natives: NativeFunctions,

    /// Map from struct names to indices, to save on unnecessary cloning and reduce memory
    /// consumption. Used by all struct type creations in the VM and in code cache.
    ///
    /// SAFETY:
    ///   By itself, it is fine to index struct names even of non-successful module publishes. If
    ///   we cached some name, which was not published, it will stay in cache and will be used by
    ///   another republish. Since there is no other information other than index, even for structs
    ///   with different layouts it is fine to re-use the index.
    ///   We wrap the index map into an [Arc] so that on republishing these clones are cheap.
    struct_name_index_map: Arc<StructNameIndexMap>,

    /// Caches struct tags for instantiated types. This cache can be used concurrently and
    /// speculatively because type tag information does not change with module publishes.
    ty_tag_cache: Arc<TypeTagCache>,

    /// Pool of interned type representations. Same lifetime as struct index map.
    interned_ty_pool: Arc<InternedTypePool>,

    /// Pool of interned module ids.
    interned_module_id_pool: Arc<InternedModuleIdPool>,

    /// Statistics collector for execution analysis! DO NOT USE ACROSS THREADS!
    pub statistics: Arc<StatisticsRef>,
}

impl RuntimeEnvironment {
    /// Creates a new runtime environment with native functions and default VM configurations. If
    /// there are duplicated natives, panics.
    pub fn new(
        natives: impl IntoIterator<Item = (AccountAddress, Identifier, Identifier, NativeFunction)>,
    ) -> Self {
        Self::new_for_move_third_party_tests(natives, true, true)
    }

    /// API to control the enum option feature flag depending on whether the caller is from aptos or not
    pub fn new_for_move_third_party_tests(
        natives: impl IntoIterator<Item = (AccountAddress, Identifier, Identifier, NativeFunction)>,
        enable_enum_option: bool,
        enable_framework_for_option: bool,
    ) -> Self {
        let vm_config = VMConfig {
            // Keep the paranoid mode on as we most likely want this for tests.
            paranoid_type_checks: true,
            enable_enum_option,
            enable_framework_for_option,
            ..VMConfig::default_for_test()
        };
        Self::new_with_config(natives, vm_config)
    }

    /// Creates a new runtime environment with native functions and VM configurations. If there are
    /// duplicated natives, panics.
    pub fn new_with_config(
        natives: impl IntoIterator<Item = (AccountAddress, Identifier, Identifier, NativeFunction)>,
        vm_config: VMConfig,
    ) -> Self {
        let natives = NativeFunctions::new(natives)
            .unwrap_or_else(|e| panic!("Failed to create native functions: {}", e));
        Self {
            vm_config,
            natives,
            struct_name_index_map: Arc::new(StructNameIndexMap::empty()),
            ty_tag_cache: Arc::new(TypeTagCache::empty()),
            interned_ty_pool: Arc::new(InternedTypePool::new()),
            interned_module_id_pool: Arc::new(InternedModuleIdPool::new()),
            statistics: Arc::new(StatisticsRef::new()),
        }
    }

    /// Returns the config currently used by this runtime environment.
    pub fn vm_config(&self) -> &VMConfig {
        &self.vm_config
    }

    /// Returns the type pool for interning that is currently used by this runtime environment.
    pub fn ty_pool(&self) -> &InternedTypePool {
        &self.interned_ty_pool
    }

    pub fn module_id_pool(&self) -> &InternedModuleIdPool {
        &self.interned_module_id_pool
    }

    /// Enables delayed field optimization for this environment.
    pub fn enable_delayed_field_optimization(&mut self) {
        self.vm_config.delayed_field_optimization_enabled = true;
    }

    /// Creates a locally verified compiled script by running:
    ///   1. Move bytecode verifier,
    ///   2. Verifier extension, if provided.
    pub fn build_locally_verified_script(
        &self,
        compiled_script: Arc<CompiledScript>,
    ) -> VMResult<LocallyVerifiedScript> {
        move_bytecode_verifier::verify_script_with_config(
            &self.vm_config().verifier_config,
            compiled_script.as_ref(),
        )?;
        Ok(LocallyVerifiedScript(compiled_script))
    }

    /// Creates a verified script by running dependency verification pass over locally verified
    /// script. The caller must provide verified module dependencies.
    pub fn build_verified_script(
        &self,
        locally_verified_script: LocallyVerifiedScript,
        immediate_dependencies: &[Arc<Module>],
    ) -> VMResult<Script> {
        dependencies::verify_script(
            &self.vm_config.verifier_config,
            locally_verified_script.0.as_ref(),
            immediate_dependencies
                .iter()
                .map(|module| module.as_ref().as_ref()),
        )?;
        Script::new(
            locally_verified_script.0,
            self.struct_name_index_map(),
            self.ty_pool(),
            self.module_id_pool(),
        )
        .map_err(|err| err.finish(Location::Script))
    }

    /// Creates a locally verified compiled module by running:
    ///   1. Move bytecode verifier,
    ///   2. Verifier extension, if provided.
    pub fn build_locally_verified_module(
        &self,
        compiled_module: Arc<CompiledModule>,
        module_size: usize,
        module_hash: &[u8; 32],
    ) -> VMResult<LocallyVerifiedModule> {
        if !VERIFIED_MODULES_CACHE.contains(module_hash) {
            let _timer =
                VM_TIMER.timer_with_label("move_bytecode_verifier::verify_module_with_config");

            // For regular execution, we cache already verified modules. Note that this even caches
            // verification for the published modules. This should be ok because as long as the
            // hash is the same, the deployed bytecode and any dependencies are the same, and so
            // the cached verification result can be used.
            move_bytecode_verifier::verify_module_with_config(
                &self.vm_config().verifier_config,
                compiled_module.as_ref(),
            )?;
            check_natives(compiled_module.as_ref())?;
            VERIFIED_MODULES_CACHE.put(*module_hash);
        }

        Ok(LocallyVerifiedModule(compiled_module, module_size))
    }

    /// Creates a verified module by running dependency verification pass for a locally verified
    /// module. The caller must provide verified module dependencies.
    pub(crate) fn build_verified_module_with_linking_checks(
        &self,
        locally_verified_module: LocallyVerifiedModule,
        immediate_dependencies: &[Arc<Module>],
    ) -> VMResult<Module> {
        dependencies::verify_module(
            &self.vm_config.verifier_config,
            locally_verified_module.0.as_ref(),
            immediate_dependencies
                .iter()
                .map(|module| module.as_ref().as_ref()),
        )?;
        let result = Module::new(
            &self.natives,
            locally_verified_module.1,
            locally_verified_module.0,
            self.struct_name_index_map(),
            self.ty_pool(),
            self.module_id_pool(),
        );

        // Note: loader V1 implementation does not set locations for this error.
        result.map_err(|e| e.finish(Location::Undefined))
    }

    /// Creates a verified module for a locally verified module. Does not perform linking checks
    /// for module's verified dependencies.
    pub(crate) fn build_verified_module_skip_linking_checks(
        &self,
        locally_verified_module: LocallyVerifiedModule,
    ) -> VMResult<Module> {
        Module::new(
            &self.natives,
            locally_verified_module.1,
            locally_verified_module.0,
            self.struct_name_index_map(),
            self.ty_pool(),
            self.module_id_pool(),
        )
        .map_err(|err| err.finish(Location::Undefined))
    }

    /// Deserializes bytes into a compiled module.
    pub fn deserialize_into_compiled_module(&self, bytes: &Bytes) -> VMResult<CompiledModule> {
        CompiledModule::deserialize_with_config(bytes, &self.vm_config().deserializer_config)
            .map_err(|err| {
                let msg = format!("Deserialization error: {:?}", err);
                PartialVMError::new(StatusCode::CODE_DESERIALIZATION_ERROR)
                    .with_message(msg)
                    .finish(Location::Undefined)
            })
    }

    /// Deserializes bytes into a compiled script.
    pub fn deserialize_into_script(&self, serialized_script: &[u8]) -> VMResult<CompiledScript> {
        CompiledScript::deserialize_with_config(
            serialized_script,
            &self.vm_config().deserializer_config,
        )
        .map_err(|err| {
            let msg = format!("[VM] deserializer for script returned error: {:?}", err);
            PartialVMError::new(StatusCode::CODE_DESERIALIZATION_ERROR)
                .with_message(msg)
                .finish(Location::Script)
        })
    }

    /// Returns an error is module's address and name do not match the expected values.
    #[cfg_attr(feature = "force-inline", inline(always))]
    pub fn paranoid_check_module_address_and_name(
        &self,
        module: &CompiledModule,
        expected_address: &AccountAddress,
        expected_module_name: &IdentStr,
    ) -> VMResult<()> {
        if self.vm_config().paranoid_type_checks {
            let actual_address = module.self_addr();
            let actual_module_name = module.self_name();
            if expected_address != actual_address || expected_module_name != actual_module_name {
                let msg = format!(
                    "Expected module {}::{}, but got {}::{}",
                    expected_address, expected_module_name, actual_address, actual_module_name
                );
                return Err(
                    PartialVMError::new(StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR)
                        .with_message(msg)
                        .with_sub_status(EPARANOID_FAILURE)
                        .finish(Location::Undefined),
                );
            }
        }
        Ok(())
    }

    /// Returns native functions available to this runtime.
    #[allow(dead_code)]
    pub(crate) fn natives(&self) -> &NativeFunctions {
        &self.natives
    }

    /// Returns the re-indexing map currently used by this runtime environment to remap struct
    /// identifiers into indices.
    pub fn struct_name_index_map(&self) -> &StructNameIndexMap {
        &self.struct_name_index_map
    }

    /// Returns the type tag cache used by this environment to store already constructed struct
    /// tags.
    pub(crate) fn ty_tag_cache(&self) -> &TypeTagCache {
        &self.ty_tag_cache
    }

    /// Returns the type tag for the given type. Construction of the tag can fail if it is too
    /// "complex": i.e., too deeply nested, or has large struct identifiers.
    pub fn ty_to_ty_tag(&self, ty: &Type) -> PartialVMResult<TypeTag> {
        let ty_tag_builder = TypeTagConverter::new(self);
        ty_tag_builder.ty_to_ty_tag(ty)
    }

    /// If type is a (generic or non-generic) struct or enum, returns its name. Otherwise, returns
    /// [None].
    pub fn get_struct_name(&self, ty: &Type) -> PartialVMResult<Option<(ModuleId, Identifier)>> {
        use Type::*;

        Ok(match ty {
            Struct { idx, .. } | StructInstantiation { idx, .. } => {
                let struct_identifier = self.struct_name_index_map().idx_to_struct_name(*idx)?;
                let (module, name) = struct_identifier.into_module_and_name();
                Some((module, name))
            },
            Bool
            | U8
            | U16
            | U32
            | U64
            | U128
            | U256
            | I8
            | I16
            | I32
            | I64
            | I128
            | I256
            | Address
            | Signer
            | TyParam(_)
            | Vector(_)
            | Reference(_)
            | MutableReference(_)
            | Function { .. } => None,
        })
    }

    /// Returns the size of the struct name re-indexing cache. Can be used to bound the size of the
    /// cache at block boundaries.
    pub fn struct_name_index_map_size(&self) -> PartialVMResult<usize> {
        self.struct_name_index_map.checked_len()
    }

    /// Flushes the global caches with struct name indices and struct tags. Note that when calling
    /// this function, modules that still store indices into struct name cache must also be flushed.
    pub fn flush_all_caches(&self) {
        self.ty_tag_cache.flush();
        self.struct_name_index_map.flush();
        self.interned_ty_pool.flush();
        self.interned_module_id_pool.flush();
    }

    /// Flushes the global verified module cache. Should be used when verifier configuration has
    /// changed.
    pub fn flush_verified_module_cache() {
        VERIFIED_MODULES_CACHE.flush();
    }

    /// Logs the size of the verified module cache.
    pub fn log_verified_cache_size() {
        let size = VERIFIED_MODULES_CACHE.size();
        VERIFIED_MODULE_CACHE_SIZE.set(size as i64);
    }

    /// Test-only function to be able to populate [StructNameIndexMap] outside of this crate.
    #[cfg(any(test, feature = "testing"))]
    pub fn struct_name_to_idx_for_test(
        &self,
        struct_name: StructIdentifier,
    ) -> PartialVMResult<StructNameIndex> {
        self.struct_name_index_map.struct_name_to_idx(&struct_name)
    }

    /// Test-only function to be able to check cached struct names.
    #[cfg(any(test, feature = "testing"))]
    pub fn idx_to_struct_name_for_test(
        &self,
        idx: StructNameIndex,
    ) -> PartialVMResult<StructIdentifier> {
        self.struct_name_index_map.idx_to_struct_name(idx)
    }

    pub fn get_option_module_bytes(&self) -> Bytes {
        Bytes::from(OPTION_MODULE_BYTES.to_vec())
    }

    pub fn get_mem_module_bytes(&self) -> Bytes {
        Bytes::from(MEM_MODULE_BYTES.to_vec())
    }

    pub fn get_module_bytes_override(
        &self,
        addr: &AccountAddress,
        name: &IdentStr,
    ) -> Option<Bytes> {
        let enable_enum_option = self.vm_config().enable_enum_option;
        let enable_framework_for_option = self.vm_config().enable_framework_for_option;
        if !enable_framework_for_option && enable_enum_option {
            if addr == OPTION_MODULE_ID.address() && *name == *OPTION_MODULE_ID.name() {
                return Some(self.get_option_module_bytes());
            }
            if addr == MEM_MODULE_ID.address() && *name == *MEM_MODULE_ID.name() {
                return Some(self.get_mem_module_bytes());
            }
        }
        None
    }
}

impl Clone for RuntimeEnvironment {
    fn clone(&self) -> Self {
        Self {
            vm_config: self.vm_config.clone(),
            natives: self.natives.clone(),
            struct_name_index_map: Arc::clone(&self.struct_name_index_map),
            ty_tag_cache: Arc::clone(&self.ty_tag_cache),
            interned_ty_pool: Arc::clone(&self.interned_ty_pool),
            interned_module_id_pool: Arc::clone(&self.interned_module_id_pool),
            statistics: Arc::clone(&self.statistics),
        }
    }
}

/// Represents any type that contains a [RuntimeEnvironment].
#[delegatable_trait]
pub trait WithRuntimeEnvironment {
    fn runtime_environment(&self) -> &RuntimeEnvironment;
}

impl WithRuntimeEnvironment for RuntimeEnvironment {
    fn runtime_environment(&self) -> &RuntimeEnvironment {
        self
    }
}

///Compiled module that passed local bytecode verification, but not the linking checks yet for its
/// dependencies. Also carries module size in bytes.
pub struct LocallyVerifiedModule(Arc<CompiledModule>, usize);

impl LocallyVerifiedModule {
    pub fn immediate_dependencies_iter(
        &self,
    ) -> impl DoubleEndedIterator<Item = (&AccountAddress, &IdentStr)> {
        self.0.immediate_dependencies_iter()
    }
}

/// Compiled script that passed local bytecode verification, but not the linking checks.
pub struct LocallyVerifiedScript(Arc<CompiledScript>);

impl LocallyVerifiedScript {
    pub fn immediate_dependencies_iter(
        &self,
    ) -> impl DoubleEndedIterator<Item = (&AccountAddress, &IdentStr)> {
        self.0.immediate_dependencies_iter()
    }
}

fn check_natives(module: &CompiledModule) -> VMResult<()> {
    // TODO: fix check and error code if we leave something around for native structs.
    // For now this generates the only error test cases care about...
    for (idx, struct_def) in module.struct_defs().iter().enumerate() {
        if struct_def.field_information == StructFieldInformation::Native {
            return Err(verification_error(
                StatusCode::MISSING_DEPENDENCY,
                IndexKind::FunctionHandle,
                idx as TableIndex,
            )
            .finish(Location::Module(module.self_id())));
        }
    }
    Ok(())
}
