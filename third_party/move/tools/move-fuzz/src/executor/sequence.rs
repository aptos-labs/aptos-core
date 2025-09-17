// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    executor::{
        clone_exec_coverage_map, collect_coverage_keys, count_coverage_entries, coverage_delta,
        merge_coverage,
        oneshot::ExecStatus,
        tracing::{ResourceRead, ResourceWrite, TracingExecutor},
    },
    mutate::mutator::{Mutator, TypePool},
    prep::canvas::{BasicInput, ScriptSignature},
    state::{
        PersistedChainFuzzer, PersistedChainSeedRecord, PersistedDefUseGraph,
        PersistedExecCoverageMap, PersistedObjectState, PersistedSeedInput, PersistedSeedNode,
        PersistedSequenceDb, PersistedSequenceEntry,
    },
};
use anyhow::{bail, Result};
use aptos_types::{
    account_address::AccountAddress,
    transaction::{
        ExecutionStatus, Script, TransactionArgument, TransactionPayload, TransactionStatus,
    },
};
use log::debug;
use move_core_types::{
    language_storage::{StructTag, TypeTag as VmTypeTag},
    value::MoveValue,
    vm_status::VMStatus,
};
use move_coverage::coverage_map::{CoverageMap, ExecCoverageMap};
use move_vm_runtime::tracing::{clear_tracing_buffer, flush_tracing_buffer};
use rand::{rngs::StdRng, seq::SliceRandom, Rng};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::{Path, PathBuf},
    time::Instant,
};

/// Maximum number of chain fuzzers to create
pub const MAX_CHAIN_FUZZERS: usize = 50;
const MAX_CHAIN_CORPUS: usize = 160;

/// Number of discovery runs per script during profiling
const NUM_DISCOVERY_RUNS: usize = 10;

// ---------------------------------------------------------------------------
// Resource tagging and script profiling (kept from original)
// ---------------------------------------------------------------------------

/// A global-state identifier for def-use matching.
/// Includes both storage account and resource type.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct ResourceTag {
    pub account: AccountAddress,
    pub struct_tag: StructTag,
}

fn should_track_resource_tag(account: AccountAddress, struct_tag: &StructTag) -> bool {
    // Keep synthetic table/raw keys in the DUG. They are often the only
    // observable cross-transaction state for table-backed protocols.
    // The transaction-context tags are still omitted because they are
    // high-volume per-txn noise rather than durable contract state.
    if account == AccountAddress::ONE {
        let module = struct_tag.module.as_str();
        if module == "transaction_context" {
            return false;
        }
    }
    true
}

fn is_object_group_struct_tag(struct_tag: &StructTag) -> bool {
    struct_tag.address == AccountAddress::ONE
        && struct_tag.module.as_str() == "object"
        && struct_tag.name.as_str() == "ObjectGroup"
}

/// Concrete transaction inputs for one script invocation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SeedInput {
    pub sender: AccountAddress,
    pub ty_args: Vec<VmTypeTag>,
    pub args: Vec<MoveValue>,
}

impl SeedInput {
    pub fn new(sender: AccountAddress, ty_args: Vec<VmTypeTag>, args: Vec<MoveValue>) -> Self {
        Self {
            sender,
            ty_args,
            args,
        }
    }
}

impl From<(Vec<VmTypeTag>, Vec<MoveValue>)> for SeedInput {
    fn from(value: (Vec<VmTypeTag>, Vec<MoveValue>)) -> Self {
        Self {
            sender: AccountAddress::ONE,
            ty_args: value.0,
            args: value.1,
        }
    }
}

/// Per-script resource access profile
pub struct ScriptProfile {
    pub script_index: usize,
    pub reads: BTreeSet<ResourceTag>,
    pub writes: BTreeSet<ResourceTag>,
    pub ever_succeeded: bool,
}

/// Resource profile from a single execution.
/// Used to feed per-seed observations back into the DUG.
#[derive(Clone)]
pub struct ExecResourceProfile {
    pub script_index: usize,
    pub reads: BTreeSet<ResourceTag>,
    pub writes: BTreeSet<ResourceTag>,
    pub succeeded: bool,
}

impl ExecResourceProfile {
    /// Build from raw execution outputs.
    ///
    /// Reads are always recorded.
    /// Writes are only recorded when `succeeded` is true.
    /// This matches the convention in `discover_profiles()`.
    pub fn from_execution(
        script_index: usize,
        resource_writes: &[ResourceWrite],
        resource_reads: &[ResourceRead],
        succeeded: bool,
    ) -> Self {
        let mut reads = BTreeSet::new();
        for read in resource_reads {
            if should_track_resource_tag(read.address, &read.struct_tag) {
                reads.insert(ResourceTag {
                    account: read.address,
                    struct_tag: read.struct_tag.clone(),
                });
            }
        }

        let mut writes = BTreeSet::new();
        if succeeded {
            for write in resource_writes {
                if should_track_resource_tag(write.address, &write.struct_tag) {
                    writes.insert(ResourceTag {
                        account: write.address,
                        struct_tag: write.struct_tag.clone(),
                    });
                }
            }
        }

        Self {
            script_index,
            reads,
            writes,
            succeeded,
        }
    }
}

/// Discover resource access profiles for each script by running them
/// multiple times with random inputs
pub fn discover_profiles(
    base_executor: &TracingExecutor,
    entrypoints: &[(ScriptSignature, Vec<u8>)],
    type_pool: &TypePool,
    dict_string: &[String],
    seed: u64,
    _trace_path: &Path,
) -> Vec<ScriptProfile> {
    let mut profiles = Vec::with_capacity(entrypoints.len());

    for (idx, (sig, code)) in entrypoints.iter().enumerate() {
        let mut executor = base_executor.clone();
        let mut mutator = Mutator::new(
            seed.wrapping_add(idx as u64),
            executor.all_addresses_by_kind(),
            type_pool.clone(),
            dict_string.to_vec(),
        );

        let mut all_reads = BTreeSet::new();
        let mut all_writes = BTreeSet::new();
        let mut ever_succeeded = false;

        for _ in 0..NUM_DISCOVERY_RUNS {
            let sender = mutator.random_signer();

            let non_signer_params: Vec<_> = sig
                .parameters
                .iter()
                .filter(|ty| !matches!(ty, BasicInput::Signer))
                .collect();

            let ty_args = mutator.random_type_args(&sig.generics);
            let args: Vec<MoveValue> = non_signer_params
                .iter()
                .map(|ty| mutator.random_value(ty))
                .collect();

            let payload = TransactionPayload::Script(Script::new(
                code.clone(),
                ty_args,
                args.iter()
                    .map(|arg| {
                        TransactionArgument::Serialized(
                            MoveValue::simple_serialize(arg).expect("arguments must serialize"),
                        )
                    })
                    .collect(),
            ));

            // clear trace buffer before discovery run
            clear_tracing_buffer();

            let result = executor.run_payload_with_sender_tracking(sender, payload);

            // flush trace buffer after discovery run
            flush_tracing_buffer();

            if let Ok((vm_status, txn_status, resource_writes, resource_reads)) = result {
                // collect reads
                for read in &resource_reads {
                    if should_track_resource_tag(read.address, &read.struct_tag) {
                        all_reads.insert(ResourceTag {
                            account: read.address,
                            struct_tag: read.struct_tag.clone(),
                        });
                    }
                }

                // collect writes only from successful executions
                let is_success = matches!(
                    (&vm_status, &txn_status),
                    (
                        VMStatus::Executed,
                        TransactionStatus::Keep(ExecutionStatus::Success)
                    )
                );
                if is_success {
                    ever_succeeded = true;
                    for write in &resource_writes {
                        if should_track_resource_tag(write.address, &write.struct_tag) {
                            all_writes.insert(ResourceTag {
                                account: write.address,
                                struct_tag: write.struct_tag.clone(),
                            });
                        }
                    }
                }
            }
        }

        profiles.push(ScriptProfile {
            script_index: idx,
            reads: all_reads,
            writes: all_writes,
            ever_succeeded,
        });
    }

    profiles
}

// ---------------------------------------------------------------------------
// Def-Use Graph (DUG)
// ---------------------------------------------------------------------------

/// A bipartite Def-Use Graph of global state.
///
/// Nodes are either **type nodes** (resource types) or **script nodes** (by index).
/// Edges are:
/// - **Def** (script → type): the script writes (defines) this resource type
/// - **Use** (type → script): the script reads (uses) this resource type
pub struct DefUseGraph {
    num_scripts: usize,

    /// All distinct ResourceTag values observed across all profiles
    type_nodes: Vec<ResourceTag>,

    /// ResourceTag → type node index (for fast lookup)
    type_index: BTreeMap<ResourceTag, usize>,

    /// script_index → set of type node indices this script writes
    /// (only populated for scripts that ever_succeeded)
    defs: Vec<BTreeSet<usize>>,

    /// script_index → set of type node indices this script reads
    uses: Vec<BTreeSet<usize>>,

    /// Resource types available from the initial provisioned state.
    initial_types: BTreeSet<usize>,

    /// Addresses known to correspond to on-chain objects.
    object_addresses: BTreeSet<AccountAddress>,

    /// type_node_index → set of script indices that produce (write) it
    producers: BTreeMap<usize, BTreeSet<usize>>,

    /// Which scripts ever succeeded during discovery
    ever_succeeded: BTreeSet<usize>,

    /// Monotonic modification counter for change detection
    modification_count: usize,

    /// Monotonic seed-catalog modification counter for concrete seed evolution.
    seed_modification_count: usize,

    /// Per-seed nodes observed during fuzzing.
    seed_nodes: Vec<SeedNode>,

    /// seed_node_index -> set of type node indices this seed writes
    seed_defs: Vec<BTreeSet<usize>>,

    /// seed_node_index -> set of type node indices this seed reads
    seed_uses: Vec<BTreeSet<usize>>,

    /// type_node_index -> set of seed node indices that produce (write) it
    seed_producers: BTreeMap<usize, BTreeSet<usize>>,

    /// Monotonic seed ID allocator
    next_seed_id: u64,
}

/// Seed node in the DUG (one observed execution seed).
#[derive(Clone, Debug)]
pub struct SeedNode {
    pub id: u64,
    pub script_index: usize,
    pub seed: SeedInput,
    pub succeeded: bool,
}

impl DefUseGraph {
    /// Build an empty DUG with no observed types/edges.
    pub fn new(num_scripts: usize) -> Self {
        Self {
            num_scripts,
            type_nodes: Vec::new(),
            type_index: BTreeMap::new(),
            defs: vec![BTreeSet::new(); num_scripts],
            uses: vec![BTreeSet::new(); num_scripts],
            initial_types: BTreeSet::new(),
            object_addresses: BTreeSet::new(),
            producers: BTreeMap::new(),
            ever_succeeded: BTreeSet::new(),
            modification_count: 0,
            seed_modification_count: 0,
            seed_nodes: Vec::new(),
            seed_defs: Vec::new(),
            seed_uses: Vec::new(),
            seed_producers: BTreeMap::new(),
            next_seed_id: 0,
        }
    }

    /// Build a DUG from discovery profiles.
    pub fn from_profiles(profiles: &[ScriptProfile]) -> Self {
        let num_scripts = profiles.len();
        let mut dug = Self::new(num_scripts);
        for profile in profiles {
            let exec_profile = ExecResourceProfile {
                script_index: profile.script_index,
                reads: profile.reads.clone(),
                writes: profile.writes.clone(),
                succeeded: profile.ever_succeeded,
            };
            dug.ingest_profile(&exec_profile);
        }

        // Building from historical profiles is a bootstrap operation.
        // Reset the marker so dynamic updates start from 0.
        dug.modification_count = 0;
        dug
    }

    /// Number of seed nodes in the DUG.
    pub fn num_seeds(&self) -> usize {
        self.seed_nodes.len()
    }

    /// Access a seed node by index.
    pub fn seed_node(&self, seed_node: usize) -> &SeedNode {
        &self.seed_nodes[seed_node]
    }

    /// Type node indices read by a specific seed node.
    pub fn seed_uses_of(&self, seed_node: usize) -> &BTreeSet<usize> {
        &self.seed_uses[seed_node]
    }

    /// Type node indices written by a specific seed node.
    pub fn seed_defs_of(&self, seed_node: usize) -> &BTreeSet<usize> {
        &self.seed_defs[seed_node]
    }

    /// Seed nodes that produce (write) the given type node.
    pub fn seed_producers_of(&self, type_node: usize) -> &BTreeSet<usize> {
        static EMPTY: BTreeSet<usize> = BTreeSet::new();
        self.seed_producers.get(&type_node).unwrap_or(&EMPTY)
    }

    /// Type node indices that a script reads (uses)
    pub fn uses_of(&self, script_index: usize) -> &BTreeSet<usize> {
        &self.uses[script_index]
    }

    /// Type node indices that a script writes (defines)
    pub fn defs_of(&self, script_index: usize) -> &BTreeSet<usize> {
        &self.defs[script_index]
    }

    /// Scripts that produce (write) a given type node
    pub fn producers_of(&self, type_node: usize) -> &BTreeSet<usize> {
        static EMPTY: BTreeSet<usize> = BTreeSet::new();
        self.producers.get(&type_node).unwrap_or(&EMPTY)
    }

    /// Whether a script ever succeeded during discovery profiling
    pub fn script_ever_succeeded(&self, script_index: usize) -> bool {
        self.ever_succeeded.contains(&script_index)
    }

    /// Unmet dependencies: types a script reads and are not available initially.
    pub fn unmet_deps(&self, script_index: usize) -> BTreeSet<usize> {
        self.uses[script_index]
            .iter()
            .filter(|&&type_idx| !self.type_is_available(&self.initial_types, type_idx))
            .copied()
            .collect()
    }

    /// Number of distinct resource type nodes in the DUG
    pub fn num_types(&self) -> usize {
        self.type_nodes.len()
    }

    /// Number of script nodes in the DUG
    pub fn num_scripts(&self) -> usize {
        self.num_scripts
    }

    /// Get the ResourceTag for a type node index
    #[cfg(test)]
    pub fn type_tag(&self, type_node: usize) -> &ResourceTag {
        &self.type_nodes[type_node]
    }

    fn note_object_address(&mut self, tag: &ResourceTag) {
        if is_object_group_struct_tag(&tag.struct_tag) {
            self.object_addresses.insert(tag.account);
        }
    }

    fn is_object_abstractable_tag(&self, tag: &ResourceTag) -> bool {
        self.object_addresses.contains(&tag.account) && !is_object_group_struct_tag(&tag.struct_tag)
    }

    fn equivalent_type_nodes(&self, type_node: usize) -> BTreeSet<usize> {
        let mut equivalent = BTreeSet::from([type_node]);
        let Some(tag) = self.type_nodes.get(type_node) else {
            return equivalent;
        };
        if !self.is_object_abstractable_tag(tag) {
            return equivalent;
        }
        for (idx, other) in self.type_nodes.iter().enumerate() {
            if idx != type_node
                && self.is_object_abstractable_tag(other)
                && other.struct_tag == tag.struct_tag
            {
                equivalent.insert(idx);
            }
        }
        equivalent
    }

    fn type_is_available(&self, available_types: &BTreeSet<usize>, needed_type: usize) -> bool {
        self.equivalent_type_nodes(needed_type)
            .into_iter()
            .any(|type_idx| available_types.contains(&type_idx))
    }

    pub fn approx_producers_of(&self, type_node: usize) -> BTreeSet<usize> {
        self.equivalent_type_nodes(type_node)
            .into_iter()
            .flat_map(|type_idx| self.producers_of(type_idx).iter().copied())
            .collect()
    }

    pub fn approx_seed_producers_of(&self, type_node: usize) -> BTreeSet<usize> {
        self.equivalent_type_nodes(type_node)
            .into_iter()
            .flat_map(|type_idx| self.seed_producers_of(type_idx).iter().copied())
            .collect()
    }

    fn resolved_dependency_count(
        &self,
        produced_types: &BTreeSet<usize>,
        unresolved_types: &BTreeSet<usize>,
    ) -> usize {
        unresolved_types
            .iter()
            .filter(|&&needed_type| self.type_is_available(produced_types, needed_type))
            .count()
    }

    /// Get the exact resource tags available from the initial provisioned state.
    pub fn initial_resource_tags(&self) -> BTreeSet<ResourceTag> {
        self.initial_types
            .iter()
            .map(|&type_idx| self.type_nodes[type_idx].clone())
            .collect()
    }

    fn tags_are_compatible(&self, available: &ResourceTag, needed: &ResourceTag) -> bool {
        available == needed
            || (self.is_object_abstractable_tag(available)
                && self.is_object_abstractable_tag(needed)
                && available.struct_tag == needed.struct_tag)
    }

    fn resource_tag_is_available(
        &self,
        available_tags: &BTreeSet<ResourceTag>,
        needed_tag: &ResourceTag,
    ) -> bool {
        available_tags
            .iter()
            .any(|available| self.tags_are_compatible(available, needed_tag))
    }

    fn compatible_resource_overlap(
        &self,
        available_tags: &BTreeSet<ResourceTag>,
        needed_tags: &BTreeSet<ResourceTag>,
    ) -> usize {
        needed_tags
            .iter()
            .filter(|needed| self.resource_tag_is_available(available_tags, needed))
            .count()
    }

    pub fn compatible_type_overlap_with_tags(
        &self,
        available_types: &BTreeSet<usize>,
        needed_tags: &BTreeSet<ResourceTag>,
    ) -> usize {
        needed_tags
            .iter()
            .filter(|needed| {
                available_types.iter().any(|&type_idx| {
                    self.type_nodes
                        .get(type_idx)
                        .is_some_and(|available| self.tags_are_compatible(available, needed))
                })
            })
            .count()
    }

    pub fn observed_unresolved_dependency_tags(
        &self,
        script_index: usize,
        observed_reads: &BTreeSet<ResourceTag>,
    ) -> BTreeSet<ResourceTag> {
        let unmet_tags = self.exact_unmet_dependency_tags(script_index);
        if unmet_tags.is_empty() {
            return BTreeSet::new();
        }
        observed_reads
            .iter()
            .filter(|read| {
                unmet_tags
                    .iter()
                    .any(|needed| self.tags_are_compatible(needed, read))
            })
            .cloned()
            .collect()
    }

    /// Exact unmet dependency tags for a script after accounting for initial state.
    pub fn exact_unmet_dependency_tags(&self, script_index: usize) -> BTreeSet<ResourceTag> {
        self.unmet_deps(script_index)
            .into_iter()
            .map(|type_idx| self.type_nodes[type_idx].clone())
            .collect()
    }

    // -----------------------------------------------------------------------
    // Mutation methods (for dynamic DUG updates)
    // -----------------------------------------------------------------------

    /// Intern a ResourceTag, returning its type node index.
    /// Creates a new type node if the tag hasn't been seen before.
    fn intern_type(&mut self, tag: &ResourceTag) -> usize {
        if let Some(&idx) = self.type_index.get(tag) {
            idx
        } else {
            let idx = self.type_nodes.len();
            self.type_nodes.push(tag.clone());
            self.type_index.insert(tag.clone(), idx);
            idx
        }
    }

    /// Add a resource type that is already available from the initial state.
    /// Returns true if the initial-state availability changed the DUG.
    pub fn add_initial_tag(&mut self, tag: &ResourceTag) -> bool {
        self.note_object_address(tag);
        let type_idx = self.intern_type(tag);
        let inserted = self.initial_types.insert(type_idx);
        if inserted {
            self.modification_count += 1;
        }
        inserted
    }

    /// Ingest initial-state resource writes discovered during executor provisioning.
    pub fn ingest_initial_writes(&mut self, writes: &[ResourceWrite]) -> bool {
        let mut changed = false;
        for write in writes {
            if should_track_resource_tag(write.address, &write.struct_tag) {
                let tag = ResourceTag {
                    account: write.address,
                    struct_tag: write.struct_tag.clone(),
                };
                changed |= self.add_initial_tag(&tag);
            }
        }
        changed
    }

    /// Add a def edge: script `script_index` writes `tag`.
    /// Returns true if the edge was new (DUG changed).
    pub fn add_def(&mut self, script_index: usize, tag: &ResourceTag) -> bool {
        assert!(script_index < self.num_scripts);
        self.note_object_address(tag);
        let ti = self.intern_type(tag);
        let inserted = self.defs[script_index].insert(ti);
        if inserted {
            self.producers.entry(ti).or_default().insert(script_index);
            self.modification_count += 1;
        }
        inserted
    }

    /// Add a use edge: script `script_index` reads `tag`.
    /// Returns true if the edge was new (DUG changed).
    pub fn add_use(&mut self, script_index: usize, tag: &ResourceTag) -> bool {
        assert!(script_index < self.num_scripts);
        let ti = self.intern_type(tag);
        let inserted = self.uses[script_index].insert(ti);
        if inserted {
            self.modification_count += 1;
        }
        inserted
    }

    /// Mark a script as having succeeded at least once.
    /// Returns true if this is the first time the script succeeded.
    pub fn mark_succeeded(&mut self, script_index: usize) -> bool {
        assert!(script_index < self.num_scripts);
        let inserted = self.ever_succeeded.insert(script_index);
        if inserted {
            self.modification_count += 1;
        }
        inserted
    }

    /// Ingest an ExecResourceProfile into the DUG.
    /// Adds use edges for all reads, and def edges + mark_succeeded for
    /// successful writes.
    /// Returns true when at least one edge/state in the DUG changed.
    pub fn ingest_profile(&mut self, profile: &ExecResourceProfile) -> bool {
        let mut changed = false;
        for tag in &profile.reads {
            changed |= self.add_use(profile.script_index, tag);
        }
        if profile.succeeded {
            changed |= self.mark_succeeded(profile.script_index);
            for tag in &profile.writes {
                changed |= self.add_def(profile.script_index, tag);
            }
        }
        changed
    }

    /// Ingest one concrete seed observation (profile + concrete sender/args).
    /// Returns `(dug_changed, seed_id)`.
    pub fn add_seed_observation(
        &mut self,
        profile: &ExecResourceProfile,
        seed: SeedInput,
    ) -> (bool, u64) {
        let changed = self.ingest_profile(profile);

        let mut seed_use_set = BTreeSet::new();
        for tag in &profile.reads {
            let ti = self.intern_type(tag);
            seed_use_set.insert(ti);
        }

        let mut seed_def_set = BTreeSet::new();
        if profile.succeeded {
            for tag in &profile.writes {
                let ti = self.intern_type(tag);
                seed_def_set.insert(ti);
            }
        }

        let seed_id = self.next_seed_id;
        self.next_seed_id += 1;
        let seed_node_idx = self.seed_nodes.len();
        self.seed_nodes.push(SeedNode {
            id: seed_id,
            script_index: profile.script_index,
            seed,
            succeeded: profile.succeeded,
        });
        self.seed_uses.push(seed_use_set);
        self.seed_defs.push(seed_def_set.clone());
        if profile.succeeded {
            for ti in seed_def_set {
                self.seed_producers
                    .entry(ti)
                    .or_default()
                    .insert(seed_node_idx);
            }
        }
        self.seed_modification_count += 1;

        (changed, seed_id)
    }

    /// Check if the DUG has been modified since a given marker value.
    /// Pass the return value of `modification_marker()` at a previous point.
    pub fn has_changed_since(&self, marker: usize) -> bool {
        self.modification_count > marker
    }

    /// Get the current modification marker (for use with `has_changed_since()`).
    pub fn modification_marker(&self) -> usize {
        self.modification_count
    }

    /// Check if the concrete seed catalog has changed since a given marker.
    pub fn has_seed_catalog_changed_since(&self, marker: usize) -> bool {
        self.seed_modification_count > marker
    }

    /// Get the current concrete seed-catalog modification marker.
    pub fn seed_modification_marker(&self) -> usize {
        self.seed_modification_count
    }

    /// Look up the type node index for a ResourceTag.
    pub fn type_index_of(&self, tag: &ResourceTag) -> Option<&usize> {
        self.type_index.get(tag)
    }

    /// Scripts that consume (read) a given type node.
    ///
    /// Only called during periodic reconstruction, not in the hot loop,
    /// so linear iteration over all scripts is acceptable.
    pub fn consumers_of(&self, type_node: usize) -> BTreeSet<usize> {
        (0..self.num_scripts)
            .filter(|&si| self.uses[si].contains(&type_node))
            .collect()
    }

    fn first_unmet_dependency_in_steps(&self, steps: &[usize]) -> Option<(usize, usize)> {
        let mut available_types = self.initial_types.clone();
        for (insert_pos, &step) in steps.iter().enumerate() {
            if step >= self.num_scripts {
                return Some((insert_pos, usize::MAX));
            }
            for &needed_type in &self.uses[step] {
                if !self.type_is_available(&available_types, needed_type) {
                    return Some((insert_pos, needed_type));
                }
            }
            available_types.extend(self.defs[step].iter());
        }
        None
    }

    fn first_unmet_dependency_in_seed_chain(&self, seed_nodes: &[usize]) -> Option<(usize, usize)> {
        let mut available_types = self.initial_types.clone();
        for (insert_pos, &seed_node) in seed_nodes.iter().enumerate() {
            if seed_node >= self.seed_nodes.len() {
                return Some((insert_pos, usize::MAX));
            }
            for &needed_type in &self.seed_uses[seed_node] {
                if !self.type_is_available(&available_types, needed_type) {
                    return Some((insert_pos, needed_type));
                }
            }
            available_types.extend(self.seed_defs[seed_node].iter());
        }
        None
    }

    /// Check whether a given step sequence has all read dependencies satisfiable
    /// from the initial state and earlier sequence steps.
    pub fn are_dependencies_satisfied(&self, steps: &[usize]) -> bool {
        self.first_unmet_dependency_in_steps(steps).is_none()
    }

    /// Check whether a concrete seed chain has all dependencies satisfiable
    /// from the initial state and earlier seed executions.
    pub fn are_seed_dependencies_satisfied(&self, seed_nodes: &[usize]) -> bool {
        self.first_unmet_dependency_in_seed_chain(seed_nodes)
            .is_none()
    }

    /// Export the DUG into a persisted checkpoint representation.
    pub fn snapshot(&self) -> Result<PersistedDefUseGraph> {
        Ok(PersistedDefUseGraph {
            num_scripts: self.num_scripts,
            type_nodes: self.type_nodes.clone(),
            defs: self.defs.clone(),
            uses: self.uses.clone(),
            initial_types: self.initial_types.clone(),
            producers: self.producers.clone(),
            ever_succeeded: self.ever_succeeded.clone(),
            modification_count: self.modification_count,
            seed_modification_count: self.seed_modification_count,
            seed_nodes: self
                .seed_nodes
                .iter()
                .map(|seed_node| {
                    Ok(PersistedSeedNode {
                        id: seed_node.id,
                        script_index: seed_node.script_index,
                        seed: PersistedSeedInput::try_from_seed(&seed_node.seed)?,
                        succeeded: seed_node.succeeded,
                    })
                })
                .collect::<Result<Vec<_>>>()?,
            seed_defs: self.seed_defs.clone(),
            seed_uses: self.seed_uses.clone(),
            seed_producers: self.seed_producers.clone(),
            next_seed_id: self.next_seed_id,
        })
    }

    /// Restore a DUG from a persisted checkpoint representation.
    pub fn from_persisted(state: PersistedDefUseGraph) -> Result<Self> {
        if state.defs.len() != state.num_scripts || state.uses.len() != state.num_scripts {
            bail!("persisted DUG has mismatched script edge dimensions");
        }
        if state.seed_nodes.len() != state.seed_defs.len()
            || state.seed_nodes.len() != state.seed_uses.len()
        {
            bail!("persisted DUG has mismatched seed dimensions");
        }

        let type_count = state.type_nodes.len();
        let mut type_index = BTreeMap::new();
        for (idx, tag) in state.type_nodes.iter().cloned().enumerate() {
            if type_index.insert(tag, idx).is_some() {
                bail!("persisted DUG contains duplicate type nodes");
            }
        }

        let validate_type_refs = |sets: &[BTreeSet<usize>]| -> Result<()> {
            for refs in sets {
                if refs.iter().any(|idx| *idx >= type_count) {
                    bail!("persisted DUG references out-of-range type node");
                }
            }
            Ok(())
        };
        validate_type_refs(&state.defs)?;
        validate_type_refs(&state.uses)?;
        validate_type_refs(&state.seed_defs)?;
        validate_type_refs(&state.seed_uses)?;
        if state.initial_types.iter().any(|idx| *idx >= type_count) {
            bail!("persisted DUG initial types reference out-of-range type node");
        }
        for (type_idx, scripts) in &state.producers {
            if *type_idx >= type_count || scripts.iter().any(|idx| *idx >= state.num_scripts) {
                bail!("persisted DUG producers contain invalid indices");
            }
        }
        for script_idx in &state.ever_succeeded {
            if *script_idx >= state.num_scripts {
                bail!("persisted DUG ever-succeeded set contains invalid script index");
            }
        }
        for seed_node in &state.seed_nodes {
            if seed_node.script_index >= state.num_scripts {
                bail!("persisted DUG seed node contains invalid script index");
            }
        }
        for (type_idx, seeds) in &state.seed_producers {
            if *type_idx >= type_count || seeds.iter().any(|idx| *idx >= state.seed_nodes.len()) {
                bail!("persisted DUG seed producers contain invalid indices");
            }
        }

        let object_addresses = state
            .type_nodes
            .iter()
            .filter(|tag| is_object_group_struct_tag(&tag.struct_tag))
            .map(|tag| tag.account)
            .collect();

        Ok(Self {
            num_scripts: state.num_scripts,
            type_nodes: state.type_nodes,
            type_index,
            defs: state.defs,
            uses: state.uses,
            initial_types: state.initial_types,
            object_addresses,
            producers: state.producers,
            ever_succeeded: state.ever_succeeded,
            modification_count: state.modification_count,
            seed_modification_count: state.seed_modification_count,
            seed_nodes: state
                .seed_nodes
                .into_iter()
                .map(|seed_node| {
                    Ok(SeedNode {
                        id: seed_node.id,
                        script_index: seed_node.script_index,
                        seed: seed_node.seed.into_seed()?,
                        succeeded: seed_node.succeeded,
                    })
                })
                .collect::<Result<Vec<_>>>()?,
            seed_defs: state.seed_defs,
            seed_uses: state.seed_uses,
            seed_producers: state.seed_producers,
            next_seed_id: state.next_seed_id,
        })
    }
}

// ---------------------------------------------------------------------------
// Chain: an ordered sequence of script indices
// ---------------------------------------------------------------------------

/// A dependency chain: an ordered sequence of scripts to execute.
/// `steps[0]` runs first (deepest dependency), `steps[last]` is the target.
#[derive(Debug, Clone)]
pub struct Chain {
    pub steps: Vec<usize>,
}

impl Chain {
    /// The target script (last in the chain)
    pub fn target(&self) -> usize {
        *self.steps.last().expect("chain must be non-empty")
    }

    /// Number of steps in the chain
    pub fn len(&self) -> usize {
        self.steps.len()
    }

    /// Whether the chain is empty
    pub fn is_empty(&self) -> bool {
        self.steps.is_empty()
    }
}

fn respects_max_repetition(steps: &[usize], max_repetition: usize) -> bool {
    if max_repetition == 0 {
        return steps.is_empty();
    }

    let mut occurrence_count = BTreeMap::new();
    for &step in steps {
        let count = occurrence_count.entry(step).or_insert(0usize);
        *count += 1;
        if *count > max_repetition {
            return false;
        }
    }
    true
}

fn candidate_chain_is_valid(
    dug: &DefUseGraph,
    steps: &[usize],
    max_chain_length: usize,
    max_repetition: usize,
) -> bool {
    steps.len() <= max_chain_length
        && respects_max_repetition(steps, max_repetition)
        && dug.are_dependencies_satisfied(steps)
}

// ---------------------------------------------------------------------------
// Sequence Database
// ---------------------------------------------------------------------------

/// A stored sequence: the chain steps + the seed inputs that produced new coverage.
#[derive(Clone)]
pub struct SequenceEntry {
    /// Unique monotonic identifier
    pub id: u64,
    /// Ordered script indices (same semantics as Chain.steps)
    pub steps: Vec<usize>,
    /// Per-step concrete seed inputs — len == steps.len()
    pub seed: Vec<SeedInput>,
    /// Resource types written by the whole sequence (union of all step writes)
    pub produced_types: BTreeSet<ResourceTag>,
    /// Resource types read by the whole sequence (union of all step reads)
    pub consumed_types: BTreeSet<ResourceTag>,
    /// Per-step exact resource types written by the sequence.
    pub step_produced_types: Vec<BTreeSet<ResourceTag>>,
    /// Per-step exact resource types read by the sequence.
    pub step_consumed_types: Vec<BTreeSet<ResourceTag>>,
    /// Whether all steps succeeded
    pub all_succeeded: bool,
}

#[derive(Clone)]
struct ChainSeedRecord {
    input: Vec<SeedInput>,
    score: u32,
    last_used_at: u64,
}

/// Central store of coverage-producing multi-transaction sequences.
///
/// Provides:
/// 1. Cross-fuzzer seed sharing via prefix matching
/// 2. Sequence extension proposals based on DUG connectivity
pub struct SequenceDb {
    entries: Vec<SequenceEntry>,
    next_id: u64,
}

/// Base probability of drawing a seed from the SequenceDb.
const SEQ_DB_PROB_BASE: u8 = 20;
/// Boosted probability when local corpus is stale.
const SEQ_DB_PROB_STALE: u8 = 50;
/// Boosted probability when local corpus is empty.
const SEQ_DB_PROB_EMPTY_CORPUS: u8 = 80;
/// Duration after which local corpus is considered stale.
const CORPUS_STALE_SECS: u64 = 60;
/// Hard cap on stored sequence entries.
const MAX_SEQUENCE_DB_ENTRIES: usize = 4096;

impl Default for SequenceDb {
    fn default() -> Self {
        Self::new()
    }
}

impl SequenceDb {
    /// Create a new empty sequence database
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
            next_id: 0,
        }
    }

    fn entry_quality(entry: &SequenceEntry) -> (bool, usize, usize, usize, u64) {
        (
            entry.all_succeeded,
            entry.produced_types.len(),
            entry
                .step_produced_types
                .iter()
                .filter(|writes| !writes.is_empty())
                .count(),
            entry.steps.len(),
            entry.id,
        )
    }

    fn prune_entries(&mut self) {
        while self.entries.len() > MAX_SEQUENCE_DB_ENTRIES {
            let remove_idx = self
                .entries
                .iter()
                .enumerate()
                .min_by_key(|(_, entry)| Self::entry_quality(entry))
                .map(|(idx, _)| idx)
                .expect("non-empty entries");
            self.entries.swap_remove(remove_idx);
        }
    }

    /// Add an entry from a ChainFuzzer coverage discovery.
    ///
    /// `steps` and `seed` come from the chain fuzzer, `profiles` are the
    /// per-step ExecResourceProfiles generated when new coverage was found.
    /// Returns the entry's unique ID.
    pub fn add_entry<S: Into<SeedInput>>(
        &mut self,
        steps: Vec<usize>,
        seed: Vec<S>,
        profiles: &[ExecResourceProfile],
    ) -> u64 {
        let seed: Vec<SeedInput> = seed.into_iter().map(Into::into).collect();
        assert_eq!(steps.len(), seed.len());
        assert_eq!(steps.len(), profiles.len());

        let mut produced_types = BTreeSet::new();
        let mut consumed_types = BTreeSet::new();
        let mut step_produced_types = Vec::with_capacity(profiles.len());
        let mut step_consumed_types = Vec::with_capacity(profiles.len());
        let mut all_succeeded = true;

        for p in profiles {
            produced_types.extend(p.writes.iter().cloned());
            consumed_types.extend(p.reads.iter().cloned());
            step_produced_types.push(p.writes.clone());
            step_consumed_types.push(p.reads.clone());
            if !p.succeeded {
                all_succeeded = false;
            }
        }

        if let Some(existing) = self
            .entries
            .iter()
            .find(|entry| entry.steps == steps && entry.seed == seed)
        {
            return existing.id;
        }

        let id = self.next_id;
        self.next_id += 1;

        self.entries.push(SequenceEntry {
            id,
            steps,
            seed,
            produced_types,
            consumed_types,
            step_produced_types,
            step_consumed_types,
            all_succeeded,
        });
        self.prune_entries();

        id
    }

    /// Find all entries whose `steps` are a prefix of (or equal to) `chain_steps`.
    pub fn find_prefix_seeds(&self, chain_steps: &[usize]) -> Vec<&SequenceEntry> {
        self.entries
            .iter()
            .filter(|e| {
                e.all_succeeded
                    && e.steps.len() <= chain_steps.len()
                    && e.steps.as_slice() == &chain_steps[..e.steps.len()]
            })
            .collect()
    }

    /// Count prefix-compatible entries for a given chain.
    pub fn prefix_compatible_count(&self, chain_steps: &[usize]) -> usize {
        self.find_prefix_seeds(chain_steps).len()
    }

    /// Pick a random prefix-compatible entry's seed, truncated to the prefix length.
    pub fn pick_prefix_seed(
        &self,
        chain_steps: &[usize],
        rng: &mut StdRng,
    ) -> Option<Vec<SeedInput>> {
        let compatible: Vec<_> = self.find_prefix_seeds(chain_steps);
        if compatible.is_empty() {
            return None;
        }
        let entry = compatible[rng.gen_range(0, compatible.len())];
        // Return seed truncated to the prefix length
        Some(entry.seed[..entry.steps.len()].to_vec())
    }

    fn entry_prefix_is_state_consistent(dug: &DefUseGraph, entry: &SequenceEntry) -> bool {
        let mut available = dug.initial_resource_tags();
        for (reads, writes) in entry
            .step_consumed_types
            .iter()
            .zip(entry.step_produced_types.iter())
        {
            for tag in reads {
                if !dug.resource_tag_is_available(&available, tag) {
                    return false;
                }
            }
            available.extend(writes.iter().cloned());
        }
        true
    }

    fn entry_prefix_relevance(
        dug: &DefUseGraph,
        chain_steps: &[usize],
        entry: &SequenceEntry,
    ) -> usize {
        if entry.steps.len() >= chain_steps.len() {
            return usize::MAX;
        }

        let next_step = chain_steps[entry.steps.len()];
        let exact_needs = dug.exact_unmet_dependency_tags(next_step);
        if exact_needs.is_empty() {
            return usize::MAX;
        }

        dug.compatible_resource_overlap(&entry.produced_types, &exact_needs)
    }

    /// Find successful prefix entries that are concrete-state compatible with a target chain.
    pub fn find_concrete_prefix_seeds<'a>(
        &'a self,
        dug: &DefUseGraph,
        chain_steps: &[usize],
    ) -> Vec<&'a SequenceEntry> {
        self.entries
            .iter()
            .filter(|entry| {
                entry.all_succeeded
                    && entry.steps.len() <= chain_steps.len()
                    && entry.steps.as_slice() == &chain_steps[..entry.steps.len()]
                    && Self::entry_prefix_is_state_consistent(dug, entry)
                    && Self::entry_prefix_relevance(dug, chain_steps, entry) > 0
            })
            .collect()
    }

    /// Count concrete-state-compatible prefix entries for a given chain.
    pub fn concrete_prefix_compatible_count(
        &self,
        dug: &DefUseGraph,
        chain_steps: &[usize],
    ) -> usize {
        self.find_concrete_prefix_seeds(dug, chain_steps).len()
    }

    /// Pick the strongest concrete-state-compatible prefix seed for a given chain.
    pub fn pick_concrete_prefix_seed(
        &self,
        dug: &DefUseGraph,
        chain_steps: &[usize],
        rng: &mut StdRng,
    ) -> Option<Vec<SeedInput>> {
        let compatible = self.find_concrete_prefix_seeds(dug, chain_steps);
        if compatible.is_empty() {
            return None;
        }

        let mut best_score = None;
        let mut best_entries = Vec::new();
        for entry in compatible {
            let score = (
                Self::entry_prefix_relevance(dug, chain_steps, entry),
                entry.steps.len(),
                entry.produced_types.len(),
            );
            match best_score {
                None => {
                    best_score = Some(score);
                    best_entries.push(entry);
                },
                Some(existing) if score > existing => {
                    best_score = Some(score);
                    best_entries.clear();
                    best_entries.push(entry);
                },
                Some(existing) if score == existing => {
                    best_entries.push(entry);
                },
                Some(_) => {},
            }
        }

        let entry = best_entries[rng.gen_range(0, best_entries.len())];
        Some(entry.seed[..entry.steps.len()].to_vec())
    }

    /// Propose sequence extensions by appending DUG-linked consumers.
    ///
    /// For each all-succeeded entry shorter than `max_chain_length`, finds scripts
    /// that consume types produced by the sequence and creates extended chains.
    /// A consumer may already appear in the chain (enabling recursive sequences
    /// like \<S2, S3, S1, S4, S1\>) as long as it doesn't exceed `max_repetition`.
    /// Returns `(extended_chain, parent_seed)` pairs (max `max_extensions`).
    pub fn propose_extensions(
        &self,
        dug: &DefUseGraph,
        max_chain_length: usize,
        max_repetition: usize,
        max_extensions: usize,
    ) -> Vec<(Chain, Vec<SeedInput>)> {
        let mut extensions = Vec::new();
        let mut seen_steps = BTreeSet::new();

        for entry in &self.entries {
            if !entry.all_succeeded || entry.steps.len() >= max_chain_length {
                continue;
            }

            for tag in &entry.produced_types {
                if let Some(&ti) = dug.type_index_of(tag) {
                    for consumer in dug.consumers_of(ti) {
                        let mut ext_steps = entry.steps.clone();
                        ext_steps.push(consumer);
                        if !candidate_chain_is_valid(
                            dug,
                            &ext_steps,
                            max_chain_length,
                            max_repetition,
                        ) || !seen_steps.insert(ext_steps.clone())
                        {
                            continue;
                        }
                        extensions.push((Chain { steps: ext_steps }, entry.seed.clone()));
                    }
                }
            }
        }

        extensions.sort_by(|(a, _), (b, _)| {
            let a_target = a.target();
            let b_target = b.target();
            let a_score = (
                dug.unmet_deps(a_target).len(),
                dug.defs_of(a_target).len(),
                !dug.script_ever_succeeded(a_target),
                a.len(),
            );
            let b_score = (
                dug.unmet_deps(b_target).len(),
                dug.defs_of(b_target).len(),
                !dug.script_ever_succeeded(b_target),
                b.len(),
            );
            b_score.cmp(&a_score)
        });
        extensions.truncate(max_extensions);
        extensions
    }

    /// Total number of entries in the database
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether the database is empty
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Access all entries for persistence.
    pub fn entries(&self) -> &[SequenceEntry] {
        &self.entries
    }

    /// Export the sequence database into a persisted checkpoint representation.
    pub fn snapshot(&self) -> Result<PersistedSequenceDb> {
        Ok(PersistedSequenceDb {
            entries: self
                .entries
                .iter()
                .map(|entry| {
                    Ok(PersistedSequenceEntry {
                        id: entry.id,
                        steps: entry.steps.clone(),
                        seed: entry
                            .seed
                            .iter()
                            .map(PersistedSeedInput::try_from_seed)
                            .collect::<Result<Vec<_>>>()?,
                        produced_types: entry.produced_types.clone(),
                        consumed_types: entry.consumed_types.clone(),
                        step_produced_types: entry.step_produced_types.clone(),
                        step_consumed_types: entry.step_consumed_types.clone(),
                        all_succeeded: entry.all_succeeded,
                    })
                })
                .collect::<Result<Vec<_>>>()?,
            next_id: self.next_id,
        })
    }

    /// Restore the sequence database from a persisted checkpoint representation.
    pub fn from_persisted(state: PersistedSequenceDb) -> Result<Self> {
        let mut entries = Vec::with_capacity(state.entries.len());
        for entry in state.entries {
            if entry.steps.len() != entry.seed.len()
                || entry.steps.len() != entry.step_produced_types.len()
                || entry.steps.len() != entry.step_consumed_types.len()
            {
                bail!("persisted sequence entry has mismatched dimensions");
            }
            entries.push(SequenceEntry {
                id: entry.id,
                steps: entry.steps,
                seed: entry
                    .seed
                    .into_iter()
                    .map(PersistedSeedInput::into_seed)
                    .collect::<Result<Vec<_>>>()?,
                produced_types: entry.produced_types,
                consumed_types: entry.consumed_types,
                step_produced_types: entry.step_produced_types,
                step_consumed_types: entry.step_consumed_types,
                all_succeeded: entry.all_succeeded,
            });
        }

        let next_id = state.next_id.max(
            entries
                .iter()
                .map(|entry| entry.id)
                .max()
                .unwrap_or(0)
                .saturating_add(1),
        );
        Ok(Self { entries, next_id })
    }

    // -----------------------------------------------------------------------
    // Sequence-level mutation operations
    // -----------------------------------------------------------------------

    /// Propose chains by deleting a single step from coverage-producing sequences.
    ///
    /// For each entry with 3+ steps, tries removing each step except the last
    /// (target). Only returns chains where remaining dependencies are still
    /// satisfied per the DUG.
    fn mutate_step_deletion(
        &self,
        dug: &DefUseGraph,
        max_chain_length: usize,
        max_repetition: usize,
    ) -> Vec<(Chain, Vec<SeedInput>)> {
        let mut results = Vec::new();

        for entry in &self.entries {
            if entry.steps.len() < 3 {
                continue;
            }

            // Try removing each step except the last (target)
            for remove_idx in 0..entry.steps.len() - 1 {
                let mut new_steps = entry.steps.clone();
                new_steps.remove(remove_idx);

                if candidate_chain_is_valid(dug, &new_steps, max_chain_length, max_repetition) {
                    let mut new_seed = entry.seed.clone();
                    new_seed.remove(remove_idx);
                    results.push((Chain { steps: new_steps }, new_seed));
                }
            }
        }

        results
    }

    /// Propose chains by duplicating a step immediately after itself.
    ///
    /// Only produces chains within the `max_chain_length` bound. Tests whether
    /// consumers can handle repeated state transitions (e.g., double-mint).
    fn mutate_step_duplication(
        &self,
        dug: &DefUseGraph,
        max_chain_length: usize,
        max_repetition: usize,
    ) -> Vec<(Chain, Vec<SeedInput>)> {
        let mut results = Vec::new();

        for entry in &self.entries {
            if entry.steps.len() >= max_chain_length {
                continue;
            }

            for dup_idx in 0..entry.steps.len() {
                let mut new_steps = entry.steps.clone();
                new_steps.insert(dup_idx + 1, entry.steps[dup_idx]);

                if candidate_chain_is_valid(dug, &new_steps, max_chain_length, max_repetition) {
                    let mut new_seed = entry.seed.clone();
                    new_seed.insert(dup_idx + 1, entry.seed[dup_idx].clone());
                    results.push((Chain { steps: new_steps }, new_seed));
                }
            }
        }

        results
    }

    /// Propose chains by extracting contiguous sub-sequences of length >= 2.
    ///
    /// Only returns sub-sequences whose dependencies are self-satisfied per the DUG.
    fn mutate_subsequence_extraction(
        &self,
        dug: &DefUseGraph,
        max_chain_length: usize,
        max_repetition: usize,
    ) -> Vec<(Chain, Vec<SeedInput>)> {
        let mut results = Vec::new();

        for entry in &self.entries {
            if entry.steps.len() < 3 {
                continue;
            }

            for start in 0..entry.steps.len() {
                for end in (start + 2)..=entry.steps.len() {
                    if end - start == entry.steps.len() {
                        // Skip the full sequence (it's the original)
                        continue;
                    }

                    let sub_steps: Vec<usize> = entry.steps[start..end].to_vec();
                    if candidate_chain_is_valid(dug, &sub_steps, max_chain_length, max_repetition) {
                        let sub_seed = entry.seed[start..end].to_vec();
                        results.push((Chain { steps: sub_steps }, sub_seed));
                    }
                }
            }
        }

        results
    }

    /// Propose chains by splicing prefix of one entry with suffix of another.
    ///
    /// Uses DUG dependency validation to ensure the combined chain is valid.
    fn mutate_sequence_splicing(
        &self,
        dug: &DefUseGraph,
        max_chain_length: usize,
        max_repetition: usize,
    ) -> Vec<(Chain, Vec<SeedInput>)> {
        let mut results = Vec::new();

        for (i, entry_a) in self.entries.iter().enumerate() {
            for (j, entry_b) in self.entries.iter().enumerate() {
                if i == j {
                    continue;
                }

                for prefix_len in 1..entry_a.steps.len() {
                    for suffix_start in 1..entry_b.steps.len() {
                        let combined_len = prefix_len + (entry_b.steps.len() - suffix_start);
                        if combined_len < 2 || combined_len > max_chain_length {
                            continue;
                        }

                        let mut new_steps = entry_a.steps[..prefix_len].to_vec();
                        new_steps.extend_from_slice(&entry_b.steps[suffix_start..]);

                        if candidate_chain_is_valid(
                            dug,
                            &new_steps,
                            max_chain_length,
                            max_repetition,
                        ) {
                            let mut new_seed = entry_a.seed[..prefix_len].to_vec();
                            new_seed.extend_from_slice(&entry_b.seed[suffix_start..]);
                            results.push((Chain { steps: new_steps }, new_seed));
                        }
                    }
                }
            }
        }

        results
    }

    /// Propose mutated sequences from all mutation strategies.
    ///
    /// Combines step deletion, duplication, subsequence extraction, and splicing.
    /// Deduplicates by step sequence and caps output at `max_mutations`.
    pub fn propose_mutations(
        &self,
        dug: &DefUseGraph,
        max_chain_length: usize,
        max_repetition: usize,
        max_mutations: usize,
    ) -> Vec<(Chain, Vec<SeedInput>)> {
        let mut all_candidates = Vec::new();
        let mut seen_steps: BTreeSet<Vec<usize>> = BTreeSet::new();

        // Collect from all mutation strategies
        let deletions = self.mutate_step_deletion(dug, max_chain_length, max_repetition);
        let duplications = self.mutate_step_duplication(dug, max_chain_length, max_repetition);
        let subsequences =
            self.mutate_subsequence_extraction(dug, max_chain_length, max_repetition);
        let splicings = self.mutate_sequence_splicing(dug, max_chain_length, max_repetition);

        // Interleave strategies for variety (round-robin from each source)
        let sources = vec![deletions, duplications, subsequences, splicings];
        let max_len = sources.iter().map(|s| s.len()).max().unwrap_or(0);

        for round in 0..max_len {
            for source in &sources {
                if round < source.len() {
                    let (ref chain, ref seed) = source[round];
                    if seen_steps.insert(chain.steps.clone()) {
                        all_candidates.push((chain.clone(), seed.clone()));
                        if all_candidates.len() >= max_mutations {
                            return all_candidates;
                        }
                    }
                }
            }
        }

        all_candidates.sort_by(|(a, _), (b, _)| {
            let a_target = a.target();
            let b_target = b.target();
            let a_score = (
                dug.unmet_deps(a_target).len(),
                dug.defs_of(a_target).len(),
                !dug.script_ever_succeeded(a_target),
                a.len(),
            );
            let b_score = (
                dug.unmet_deps(b_target).len(),
                dug.defs_of(b_target).len(),
                !dug.script_ever_succeeded(b_target),
                b.len(),
            );
            b_score.cmp(&a_score)
        });
        all_candidates.truncate(max_mutations);
        all_candidates
    }
}

// ---------------------------------------------------------------------------
// Chain construction
// ---------------------------------------------------------------------------

/// Construct dependency chains from the DUG.
///
/// For each target script that has unmet dependencies, build a chain by backward
/// traversal of the DUG. Targets are prioritized: never-succeeded scripts first,
/// then by number of unmet dependencies (more = more interesting).
pub fn construct_chains(
    dug: &DefUseGraph,
    max_chain_length: usize,
    max_repetition: usize,
    max_chains: usize,
    rng: &mut StdRng,
) -> Vec<Chain> {
    let mut chains = Vec::new();

    // Collect and prioritize targets
    let mut targets: Vec<usize> = (0..dug.num_scripts()).collect();
    targets.sort_by(|a, b| {
        let a_failed = !dug.script_ever_succeeded(*a);
        let b_failed = !dug.script_ever_succeeded(*b);
        // Never-succeeded scripts first, then by number of unmet deps (descending)
        b_failed.cmp(&a_failed).then_with(|| {
            let a_unmet = dug.unmet_deps(*a).len();
            let b_unmet = dug.unmet_deps(*b).len();
            b_unmet.cmp(&a_unmet)
        })
    });

    for &target in &targets {
        if chains.len() >= max_chains {
            break;
        }

        // Only build chains for scripts with unmet dependencies
        let unmet = dug.unmet_deps(target);
        if unmet.is_empty() {
            continue;
        }

        if let Some(chain) = build_one_chain(dug, target, max_chain_length, max_repetition, rng) {
            debug!(
                "chain for target {}: [{}] (length {})",
                target,
                chain
                    .steps
                    .iter()
                    .map(|s| s.to_string())
                    .collect::<Vec<_>>()
                    .join(", "),
                chain.len(),
            );
            chains.push(chain);
        }
    }

    chains
}

/// A chain with concrete seed inputs selected from DUG seed nodes.
#[derive(Debug, Clone)]
pub struct SeedChain {
    pub chain: Chain,
    pub seed_inputs: Vec<SeedInput>,
    pub target_seed_id: u64,
}

fn unresolved_seed_types(dug: &DefUseGraph, seed_node: usize) -> Vec<usize> {
    dug.seed_uses_of(seed_node)
        .iter()
        .filter(|&&type_idx| !dug.type_is_available(&dug.initial_types, type_idx))
        .copied()
        .collect()
}

fn seed_target_priority(dug: &DefUseGraph, seed_node: usize) -> (usize, usize, usize, bool, usize) {
    let unresolved = unresolved_seed_types(dug, seed_node);
    let producible = unresolved
        .iter()
        .filter(|&&type_idx| !dug.approx_seed_producers_of(type_idx).is_empty())
        .count();
    let produced = dug.seed_defs_of(seed_node).len();
    let consumed = dug.seed_uses_of(seed_node).len();
    let statefulness = produced.saturating_mul(2).saturating_sub(consumed);
    (
        producible,
        produced,
        statefulness,
        !dug.seed_node(seed_node).succeeded,
        unresolved.len(),
    )
}

fn ranked_producer_seed_nodes(
    dug: &DefUseGraph,
    current_seed_nodes: &[usize],
    occurrence_count: &BTreeMap<usize, usize>,
    needed_type: usize,
    max_repetition: usize,
) -> Vec<usize> {
    let mut currently_unresolved = BTreeSet::new();
    let mut available = dug.initial_types.clone();
    for &seed_node in current_seed_nodes {
        for &read in dug.seed_uses_of(seed_node) {
            if !dug.type_is_available(&available, read) {
                currently_unresolved.insert(read);
            }
        }
        available.extend(dug.seed_defs_of(seed_node).iter().copied());
    }

    let mut eligible: Vec<usize> = dug
        .approx_seed_producers_of(needed_type)
        .into_iter()
        .filter(|&seed_node_idx| {
            let script = dug.seed_node(seed_node_idx).script_index;
            occurrence_count.get(&script).copied().unwrap_or(0) < max_repetition
        })
        .collect();
    eligible.sort_by(|a, b| {
        let a_defs = dug.seed_defs_of(*a);
        let b_defs = dug.seed_defs_of(*b);
        let a_resolves = dug.resolved_dependency_count(a_defs, &currently_unresolved);
        let b_resolves = dug.resolved_dependency_count(b_defs, &currently_unresolved);
        let a_score = (
            a_resolves,
            a_defs.len(),
            !dug.seed_node(*a).succeeded,
            dug.seed_uses_of(*a).len(),
        );
        let b_score = (
            b_resolves,
            b_defs.len(),
            !dug.seed_node(*b).succeeded,
            dug.seed_uses_of(*b).len(),
        );
        b_score.cmp(&a_score)
    });
    eligible
}

fn collect_target_seed_nodes(
    dug: &DefUseGraph,
    target_scripts: Option<&BTreeSet<usize>>,
    rng: &mut StdRng,
) -> Vec<usize> {
    let mut targets: Vec<usize> = (0..dug.num_seeds())
        .filter(|&seed_node| {
            target_scripts
                .map(|targets| targets.contains(&dug.seed_node(seed_node).script_index))
                .unwrap_or(true)
        })
        .collect();
    targets.shuffle(rng);
    targets.sort_by(|a, b| seed_target_priority(dug, *b).cmp(&seed_target_priority(dug, *a)));
    targets
}

fn construct_seed_chains_from_target_nodes(
    dug: &DefUseGraph,
    target_seed_nodes: Vec<usize>,
    max_chain_length: usize,
    max_repetition: usize,
    max_chains: usize,
    rng: &mut StdRng,
) -> Vec<SeedChain> {
    let mut chains = Vec::new();
    let mut seen_steps = BTreeSet::new();
    for target_seed_node in target_seed_nodes {
        if chains.len() >= max_chains {
            break;
        }
        if let Some(chain) =
            build_one_seed_chain(dug, target_seed_node, max_chain_length, max_repetition, rng)
        {
            if seen_steps.insert(chain.chain.steps.clone()) {
                chains.push(chain);
            }
        }
    }
    chains
}

/// Construct dependency chains by picking arbitrary seed nodes from the DUG.
pub fn construct_seed_chains(
    dug: &DefUseGraph,
    max_chain_length: usize,
    max_repetition: usize,
    max_chains: usize,
    rng: &mut StdRng,
) -> Vec<SeedChain> {
    let targets = collect_target_seed_nodes(dug, None, rng);
    construct_seed_chains_from_target_nodes(
        dug,
        targets,
        max_chain_length,
        max_repetition,
        max_chains,
        rng,
    )
}

/// Construct dependency chains prioritizing seeds from a target set of scripts.
pub fn construct_seed_chains_for_targets(
    dug: &DefUseGraph,
    target_scripts: &BTreeSet<usize>,
    max_chain_length: usize,
    max_repetition: usize,
    max_chains: usize,
    rng: &mut StdRng,
) -> Vec<SeedChain> {
    let targets = collect_target_seed_nodes(dug, Some(target_scripts), rng);
    construct_seed_chains_from_target_nodes(
        dug,
        targets,
        max_chain_length,
        max_repetition,
        max_chains,
        rng,
    )
}

fn build_one_seed_chain(
    dug: &DefUseGraph,
    target_seed_node: usize,
    max_length: usize,
    max_repetition: usize,
    rng: &mut StdRng,
) -> Option<SeedChain> {
    if target_seed_node >= dug.num_seeds() {
        return None;
    }
    let target_seed = dug.seed_node(target_seed_node);

    let mut seed_nodes: Vec<usize> = vec![target_seed_node];
    let mut occurrence_count: BTreeMap<usize, usize> = BTreeMap::new();
    *occurrence_count
        .entry(target_seed.script_index)
        .or_insert(0) += 1;

    while seed_nodes.len() < max_length {
        let Some((insert_pos, needed_type)) = dug.first_unmet_dependency_in_seed_chain(&seed_nodes)
        else {
            break;
        };
        if needed_type == usize::MAX {
            return None;
        }

        let ranked = ranked_producer_seed_nodes(
            dug,
            &seed_nodes,
            &occurrence_count,
            needed_type,
            max_repetition,
        );
        if ranked.is_empty() {
            return None;
        }

        let top_n = ranked.len().min(3);
        let producer_seed_node = ranked[rng.gen_range(0, top_n)];
        let producer_script = dug.seed_node(producer_seed_node).script_index;
        seed_nodes.insert(insert_pos, producer_seed_node);
        *occurrence_count.entry(producer_script).or_insert(0) += 1;
    }

    if seed_nodes.len() <= 1 || !dug.are_seed_dependencies_satisfied(&seed_nodes) {
        return None;
    }

    let steps: Vec<usize> = seed_nodes
        .iter()
        .map(|&seed_node_idx| dug.seed_node(seed_node_idx).script_index)
        .collect();
    let seed_inputs: Vec<SeedInput> = seed_nodes
        .iter()
        .map(|&seed_node_idx| dug.seed_node(seed_node_idx).seed.clone())
        .collect();
    Some(SeedChain {
        chain: Chain { steps },
        seed_inputs,
        target_seed_id: target_seed.id,
    })
}

/// Build one chain ending at `target` by backward greedy traversal of the DUG.
///
/// Algorithm:
/// 1. Start with `chain_reversed = [target]` and `resolved_types = defs_of(target)`
/// 2. Queue all unmet dependencies (types target reads but doesn't write)
/// 3. While queue non-empty and chain length < max_length:
///    - Pop a needed type (random for variety)
///    - Find a producer, preferring scripts that have succeeded (70/30 bias)
///    - Add producer to chain, mark its defs as resolved, enqueue its unmet deps
/// 4. Reverse to get execution order (producers first, target last)
fn build_one_chain(
    dug: &DefUseGraph,
    target: usize,
    max_length: usize,
    max_repetition: usize,
    rng: &mut StdRng,
) -> Option<Chain> {
    let mut steps = vec![target];
    let mut occurrence_count: BTreeMap<usize, usize> = BTreeMap::new();
    *occurrence_count.entry(target).or_insert(0) += 1;

    while steps.len() < max_length {
        let Some((insert_pos, needed_type)) = dug.first_unmet_dependency_in_steps(&steps) else {
            break;
        };
        if needed_type == usize::MAX {
            return None;
        }

        let mut currently_unresolved = BTreeSet::new();
        let mut available = dug.initial_types.clone();
        for &step in &steps {
            for &read in dug.uses_of(step) {
                if !dug.type_is_available(&available, read) {
                    currently_unresolved.insert(read);
                }
            }
            available.extend(dug.defs_of(step).iter().copied());
        }

        let producers = dug.approx_producers_of(needed_type);
        if producers.is_empty() {
            return None;
        }

        let mut eligible: Vec<usize> = producers
            .into_iter()
            .filter(|&p| occurrence_count.get(&p).copied().unwrap_or(0) < max_repetition)
            .collect();
        if eligible.is_empty() {
            return None;
        }

        eligible.sort_by(|a, b| {
            let a_defs = dug.defs_of(*a);
            let b_defs = dug.defs_of(*b);
            let a_score = (
                dug.resolved_dependency_count(a_defs, &currently_unresolved),
                a_defs.len(),
                dug.script_ever_succeeded(*a),
                dug.unmet_deps(*a).len(),
            );
            let b_score = (
                dug.resolved_dependency_count(b_defs, &currently_unresolved),
                b_defs.len(),
                dug.script_ever_succeeded(*b),
                dug.unmet_deps(*b).len(),
            );
            b_score.cmp(&a_score)
        });
        let top_n = eligible.len().min(3);
        let producer = eligible[rng.gen_range(0, top_n)];
        steps.insert(insert_pos, producer);
        *occurrence_count.entry(producer).or_insert(0) += 1;
    }

    if steps.len() <= 1 || !dug.are_dependencies_satisfied(&steps) {
        return None;
    }
    Some(Chain { steps })
}

// ---------------------------------------------------------------------------
// ChainFuzzer: generalized sequence executor for arbitrary-length chains
// ---------------------------------------------------------------------------

/// A chain fuzzer that executes an ordered sequence of scripts (arbitrary length).
///
/// Replaces the pair-only `SequenceFuzzer`. A chain of length 2 is equivalent to
/// the old predecessor→successor pair.
pub struct ChainFuzzer {
    /// The chain this fuzzer executes
    chain: Chain,

    /// Per-step script signatures
    step_sigs: Vec<ScriptSignature>,

    /// Per-step compiled bytecode
    step_codes: Vec<Vec<u8>>,

    /// Per-step mutators (independently seeded)
    mutators: Vec<Mutator>,

    /// Execution state (independent clone per fuzzer)
    executor: TracingExecutor,

    /// Path to the coverage trace file
    trace_path: PathBuf,

    /// Accumulated coverage map
    coverage: ExecCoverageMap,

    /// Corpus: each seed stores per-step concrete invocation inputs.
    seedpool: Vec<ChainSeedRecord>,

    /// Ordered replay transcript for reconstructing executor state on resume.
    replay_log: Vec<Vec<SeedInput>>,

    /// Concrete bootstrap seed that distinguishes this chain instance.
    identity_seed: Vec<SeedInput>,

    // Statistics
    exec_count: u64,
    last_new_coverage_time: Option<Instant>,
    coverage_at_last_report: usize,
}

impl ChainFuzzer {
    /// Create a new chain fuzzer
    pub fn new(
        executor: TracingExecutor,
        seed: u64,
        chain: Chain,
        entrypoints: &[(ScriptSignature, Vec<u8>)],
        type_pool: TypePool,
        trace_path: PathBuf,
        dict_string: Vec<String>,
    ) -> Self {
        let addresses = executor.all_addresses_by_kind();
        let step_sigs: Vec<_> = chain
            .steps
            .iter()
            .map(|&idx| entrypoints[idx].0.clone())
            .collect();
        let step_codes: Vec<_> = chain
            .steps
            .iter()
            .map(|&idx| entrypoints[idx].1.clone())
            .collect();
        let mutators: Vec<_> = chain
            .steps
            .iter()
            .enumerate()
            .map(|(i, _)| {
                Mutator::new(
                    seed.wrapping_add(i as u64),
                    addresses.clone(),
                    type_pool.clone(),
                    dict_string.clone(),
                )
            })
            .collect();

        Self {
            chain,
            step_sigs,
            step_codes,
            mutators,
            executor,
            trace_path,
            coverage: ExecCoverageMap::new(String::new()),
            seedpool: vec![],
            replay_log: vec![],
            identity_seed: vec![],
            exec_count: 0,
            last_new_coverage_time: None,
            coverage_at_last_report: 0,
        }
    }

    /// Get a human-readable description of the chain
    pub fn script_desc(&self) -> String {
        self.step_sigs
            .iter()
            .map(|sig| sig.ident.to_string())
            .collect::<Vec<_>>()
            .join(" -> ")
    }

    /// Get a short description: `mod::fn -> mod::fn -> ...`
    pub fn script_short_desc(&self) -> String {
        self.step_sigs
            .iter()
            .map(|sig| format!("{}::{}", sig.ident.module_name(), sig.ident.function_name()))
            .collect::<Vec<_>>()
            .join(" -> ")
    }

    /// Get the current corpus (seed pool) size
    pub fn corpus_size(&self) -> usize {
        self.seedpool.len()
    }

    fn pick_seed_index(&mut self) -> usize {
        debug_assert!(!self.seedpool.is_empty());
        let total_weight = self
            .seedpool
            .iter()
            .map(|record| u64::from(record.score.max(1)))
            .sum::<u64>();
        let mut ticket = self.mutators[0].rng_mut().gen_range(0, total_weight);
        for (idx, record) in self.seedpool.iter_mut().enumerate() {
            let weight = u64::from(record.score.max(1));
            if ticket < weight {
                record.last_used_at = self.exec_count;
                return idx;
            }
            ticket -= weight;
        }
        let last_idx = self.seedpool.len() - 1;
        self.seedpool[last_idx].last_used_at = self.exec_count;
        last_idx
    }

    fn prune_seedpool(&mut self) {
        while self.seedpool.len() > MAX_CHAIN_CORPUS {
            let remove_idx = self
                .seedpool
                .iter()
                .enumerate()
                .min_by_key(|(_, record)| (record.score, record.last_used_at))
                .map(|(idx, _)| idx)
                .expect("non-empty seedpool");
            self.seedpool.swap_remove(remove_idx);
        }
    }

    /// Get the total number of covered bytecode positions across all modules
    pub fn coverage_count(&self) -> usize {
        count_coverage_entries(&self.coverage)
    }

    /// Globalized coverage keys for cross-fuzzer deduplication.
    pub fn coverage_keys(&self) -> BTreeSet<String> {
        collect_coverage_keys(&self.coverage)
    }

    /// Get the total number of executions
    pub fn exec_count(&self) -> u64 {
        self.exec_count
    }

    /// Get when coverage was last found
    pub fn last_new_coverage_time(&self) -> Option<Instant> {
        self.last_new_coverage_time
    }

    /// Get the coverage delta since last report and reset the snapshot
    pub fn coverage_delta_since_report(&mut self) -> usize {
        coverage_delta(&self.coverage, &mut self.coverage_at_last_report)
    }

    /// Get the chain length
    pub fn chain_len(&self) -> usize {
        self.chain.len()
    }

    pub fn average_seed_score(&self) -> f64 {
        if self.seedpool.is_empty() {
            return 0.0;
        }
        let total: u64 = self
            .seedpool
            .iter()
            .map(|record| u64::from(record.score))
            .sum();
        total as f64 / self.seedpool.len() as f64
    }

    pub fn best_seed_score(&self) -> u32 {
        self.seedpool
            .iter()
            .map(|record| record.score)
            .max()
            .unwrap_or(0)
    }

    /// Get the chain's steps (for deduplication when reconstructing chains)
    pub fn chain_steps(&self) -> &[usize] {
        &self.chain.steps
    }

    pub fn identity_seed(&self) -> &[SeedInput] {
        &self.identity_seed
    }

    pub fn set_identity_seed(&mut self, seed: Vec<SeedInput>) {
        self.identity_seed = seed;
    }

    /// Access the local corpus for persistence.
    pub fn seed_pool_snapshot(&self) -> Vec<Vec<SeedInput>> {
        self.seedpool
            .iter()
            .map(|record| record.input.clone())
            .collect()
    }

    pub fn seed_record_snapshot(&self) -> Result<Vec<PersistedChainSeedRecord>> {
        self.seedpool
            .iter()
            .map(|record| {
                Ok(PersistedChainSeedRecord {
                    input: record
                        .input
                        .iter()
                        .map(PersistedSeedInput::try_from_seed)
                        .collect::<Result<Vec<_>>>()?,
                    score: record.score,
                    last_used_at: record.last_used_at,
                })
            })
            .collect()
    }

    pub fn replay_log_snapshot(&self) -> Result<Vec<Vec<PersistedSeedInput>>> {
        self.replay_log
            .iter()
            .map(|seed| seed.iter().map(PersistedSeedInput::try_from_seed).collect())
            .collect()
    }

    pub fn identity_seed_snapshot(&self) -> Result<Vec<PersistedSeedInput>> {
        self.identity_seed
            .iter()
            .map(PersistedSeedInput::try_from_seed)
            .collect()
    }

    /// Clone the current coverage snapshot for persistence.
    pub fn coverage_snapshot(&self) -> ExecCoverageMap {
        clone_exec_coverage_map(&self.coverage)
    }

    /// Restore a checkpointed corpus and coverage snapshot.
    pub fn restore_checkpoint(&mut self, seeds: Vec<Vec<SeedInput>>, coverage: ExecCoverageMap) {
        for seed in seeds {
            self.remember_seed(seed);
        }
        self.exec_count = self.seedpool.len() as u64;
        self.coverage = coverage;
        self.coverage_at_last_report = count_coverage_entries(&self.coverage);
        if !self.seedpool.is_empty() || self.coverage_at_last_report > 0 {
            self.last_new_coverage_time = Some(Instant::now());
        }
    }

    pub fn restore_checkpoint_records(
        &mut self,
        seeds: Vec<PersistedChainSeedRecord>,
        coverage: ExecCoverageMap,
    ) -> Result<()> {
        self.seedpool.clear();
        for record in seeds {
            let input = self.normalize_seed_len(
                record
                    .input
                    .into_iter()
                    .map(PersistedSeedInput::into_seed)
                    .collect::<Result<Vec<_>>>()?,
            );
            self.seedpool.push(ChainSeedRecord {
                input,
                score: record.score.max(1),
                last_used_at: record.last_used_at,
            });
        }
        self.prune_seedpool();
        self.exec_count = self
            .seedpool
            .iter()
            .map(|record| record.last_used_at)
            .max()
            .unwrap_or(0)
            .saturating_add(1);
        self.coverage = coverage;
        self.coverage_at_last_report = count_coverage_entries(&self.coverage);
        if !self.seedpool.is_empty() || self.coverage_at_last_report > 0 {
            self.last_new_coverage_time = Some(Instant::now());
        }
        Ok(())
    }

    pub fn restore_identity_seed(&mut self, identity_seed: Vec<PersistedSeedInput>) -> Result<()> {
        self.identity_seed = identity_seed
            .into_iter()
            .map(PersistedSeedInput::into_seed)
            .collect::<Result<Vec<_>>>()?;
        Ok(())
    }

    pub fn replay_checkpoint_log(
        &mut self,
        replay_log: Vec<Vec<PersistedSeedInput>>,
    ) -> Result<()> {
        self.replay_log.clear();
        for record in replay_log {
            if record.len() > self.chain.len() {
                bail!("persisted chain replay log entry exceeds chain length");
            }
            let seed = record
                .into_iter()
                .map(PersistedSeedInput::into_seed)
                .collect::<Result<Vec<_>>>()?;
            for (step_idx, input) in seed.iter().enumerate() {
                self.replay_step(step_idx, input)?;
            }
            self.replay_log.push(seed);
        }
        Ok(())
    }

    fn build_step_payload(&self, step_idx: usize, seed_input: &SeedInput) -> TransactionPayload {
        TransactionPayload::Script(Script::new(
            self.step_codes[step_idx].clone(),
            seed_input.ty_args.clone(),
            seed_input
                .args
                .iter()
                .map(|arg| {
                    TransactionArgument::Serialized(
                        MoveValue::simple_serialize(arg).expect("arguments must serialize"),
                    )
                })
                .collect(),
        ))
    }

    fn replay_step(&mut self, step_idx: usize, seed_input: &SeedInput) -> Result<()> {
        let payload = self.build_step_payload(step_idx, seed_input);
        let _ = self
            .executor
            .run_payload_with_sender(seed_input.sender, payload)?;
        Ok(())
    }

    /// Export a persisted chain-fuzzer checkpoint payload.
    pub fn snapshot(&self) -> Result<PersistedChainFuzzer> {
        Ok(PersistedChainFuzzer {
            steps: self.chain.steps.clone(),
            step_identities: vec![],
            identity_seed: self.identity_seed_snapshot()?,
            replay_log: self.replay_log_snapshot()?,
            seedpool: self.seed_record_snapshot()?,
            coverage: PersistedExecCoverageMap::from_exec_coverage_map(&self.coverage_snapshot()),
        })
    }

    /// Export the current shared object-discovery state.
    pub fn object_state_snapshot(&self) -> PersistedObjectState {
        self.mutators
            .first()
            .map(Mutator::snapshot_object_state)
            .unwrap_or_default()
    }

    /// Restore the shared object-discovery state into every step mutator.
    pub fn restore_object_state(&mut self, state: &PersistedObjectState) -> Result<()> {
        for mutator in &mut self.mutators {
            mutator.restore_object_state(state)?;
        }
        Ok(())
    }

    /// Execute one chain iteration.
    ///
    /// Returns `(exec_status, corpus_size, found_new_coverage, resource_writes, profiles, seed_clone)`.
    /// - `exec_status` is the status of the last step that executed (or the first failure)
    /// - Resource writes are accumulated from all successful steps
    /// - `profiles` contains per-step resource profiles for all executed steps
    /// - `seed_clone` is the executed step inputs (prefix if the chain failed early)
    ///
    /// When `seq_db` is provided, there is a chance (SEQ_DB_PROB = 20%) that inputs
    /// for the prefix steps are drawn from a compatible SequenceDb entry instead of
    /// the local seed pool.
    pub fn run_one(
        &mut self,
        seq_db: Option<&SequenceDb>,
        dug: Option<&DefUseGraph>,
    ) -> Result<(
        ExecStatus,
        usize,
        bool,
        Vec<ResourceWrite>,
        Vec<ExecResourceProfile>,
        Vec<SeedInput>,
    )> {
        let num_steps = self.chain.len();

        // Decide seed source: adaptive SequenceDb prefix share, local corpus, or fresh generation.
        let seq_db_prob = if self.seedpool.is_empty() {
            SEQ_DB_PROB_EMPTY_CORPUS
        } else if self
            .last_new_coverage_time
            .is_none_or(|t| t.elapsed().as_secs() >= CORPUS_STALE_SECS)
        {
            SEQ_DB_PROB_STALE
        } else {
            SEQ_DB_PROB_BASE
        };
        let db_prefix_seed: Option<Vec<SeedInput>> = seq_db
            .zip(dug)
            .filter(|(db, dug)| db.concrete_prefix_compatible_count(dug, &self.chain.steps) > 0)
            .filter(|_| self.mutators[0].random_percent() < seq_db_prob)
            .and_then(|(db, dug)| {
                db.pick_concrete_prefix_seed(dug, &self.chain.steps, self.mutators[0].rng_mut())
            });

        // Generate or mutate inputs for ALL steps up front
        let step_inputs: Vec<SeedInput> = (0..num_steps)
            .map(|i| {
                // If we have a SequenceDb prefix seed that covers this step, use it
                // (with mutation applied)
                if let Some(ref prefix) = db_prefix_seed {
                    if i < prefix.len() {
                        let sig = &self.step_sigs[i];
                        let non_signer_params: Vec<_> = sig
                            .parameters
                            .iter()
                            .filter(|ty| !matches!(ty, BasicInput::Signer))
                            .collect();
                        let seed_input = &prefix[i];
                        let seed_ty = &seed_input.ty_args;
                        let seed_args = &seed_input.args;
                        let ty_args = if !sig.generics.is_empty()
                            && self.mutators[i].should_mutate_type_args()
                        {
                            self.mutators[i].mutate_type_args(&sig.generics, seed_ty)
                        } else {
                            seed_ty.clone()
                        };
                        let args = seed_args
                            .iter()
                            .zip(non_signer_params.iter())
                            .map(|(val, ty)| self.mutators[i].mutate_value(ty, val))
                            .collect();
                        let sender = if self.mutators[i].random_percent() < 70 {
                            seed_input.sender
                        } else {
                            self.mutators[i].random_signer()
                        };
                        return SeedInput {
                            sender,
                            ty_args,
                            args,
                        };
                    }
                }

                match self.mutators[i].should_mutate(self.seedpool.len()) {
                    None => {
                        let sig = &self.step_sigs[i];
                        let non_signer_params: Vec<_> = sig
                            .parameters
                            .iter()
                            .filter(|ty| !matches!(ty, BasicInput::Signer))
                            .collect();
                        // Generate fresh inputs
                        let ty_args = self.mutators[i].random_type_args(&sig.generics);
                        let args = non_signer_params
                            .iter()
                            .map(|ty| self.mutators[i].random_value(ty))
                            .collect();
                        SeedInput {
                            sender: self.mutators[i].random_signer(),
                            ty_args,
                            args,
                        }
                    },
                    Some(_) => {
                        let index = self.pick_seed_index();
                        let sig = &self.step_sigs[i];
                        let non_signer_params: Vec<_> = sig
                            .parameters
                            .iter()
                            .filter(|ty| !matches!(ty, BasicInput::Signer))
                            .collect();
                        // Mutate from corpus seed
                        let seed_input = &self.seedpool[index].input[i];
                        let seed_ty = &seed_input.ty_args;
                        let seed_args = &seed_input.args;
                        let ty_args = if !sig.generics.is_empty()
                            && self.mutators[i].should_mutate_type_args()
                        {
                            self.mutators[i].mutate_type_args(&sig.generics, seed_ty)
                        } else {
                            seed_ty.clone()
                        };
                        let args = seed_args
                            .iter()
                            .zip(non_signer_params.iter())
                            .map(|(val, ty)| self.mutators[i].mutate_value(ty, val))
                            .collect();
                        let sender = if self.mutators[i].random_percent() < 70 {
                            seed_input.sender
                        } else {
                            self.mutators[i].random_signer()
                        };
                        SeedInput {
                            sender,
                            ty_args,
                            args,
                        }
                    },
                }
            })
            .collect();

        // Clear trace buffer ONCE at start of entire chain
        clear_tracing_buffer();

        // Execute chain steps sequentially, collecting per-step resource data
        let mut all_writes = vec![];
        let mut last_status = ExecStatus::Success;
        let mut step_raw_profiles: Vec<(usize, Vec<ResourceWrite>, Vec<ResourceRead>, bool)> =
            Vec::with_capacity(num_steps);

        for (step_idx, seed_input) in step_inputs.iter().enumerate() {
            let payload = self.build_step_payload(step_idx, seed_input);

            let (vm_status, txn_status, writes, reads) = self
                .executor
                .run_payload_with_sender_tracking(seed_input.sender, payload)?;

            let step_status: ExecStatus = (vm_status, txn_status).into();
            let succeeded = matches!(step_status, ExecStatus::Success);

            // Collect raw profile data for this step (clone writes before moving)
            step_raw_profiles.push((self.chain.steps[step_idx], writes.clone(), reads, succeeded));

            if succeeded {
                for mutator in self.mutators.iter_mut() {
                    mutator.update_object_dict(&writes);
                }
                all_writes.extend(writes);
            }

            // If a step fails, abort the chain early (predecessor setup failed)
            if !succeeded {
                last_status = step_status;
                break;
            }
        }

        // Flush trace buffer and read coverage from all executed steps.
        flush_tracing_buffer();

        self.exec_count += 1;
        let coverage_map = CoverageMap::from_trace_file(&self.trace_path)?;
        let found_new = self.update_coverage(coverage_map);
        if found_new {
            self.last_new_coverage_time = Some(Instant::now());
        }
        let seed_clone = step_inputs[..step_raw_profiles.len()].to_vec();
        self.replay_log.push(seed_clone.clone());
        let profiles = step_raw_profiles
            .iter()
            .map(|(script_index, writes, reads, succeeded)| {
                ExecResourceProfile::from_execution(*script_index, writes, reads, *succeeded)
            })
            .collect();

        Ok((
            last_status,
            self.seedpool.len(),
            found_new,
            all_writes,
            profiles,
            seed_clone,
        ))
    }

    fn random_seed_for_step(&mut self, step_idx: usize) -> SeedInput {
        let sig = &self.step_sigs[step_idx];
        let ty_args = self.mutators[step_idx].random_type_args(&sig.generics);
        let args: Vec<MoveValue> = sig
            .parameters
            .iter()
            .filter(|ty| !matches!(ty, BasicInput::Signer))
            .map(|ty| self.mutators[step_idx].random_value(ty))
            .collect();
        SeedInput {
            sender: self.mutators[step_idx].random_signer(),
            ty_args,
            args,
        }
    }

    fn normalize_seed_len(&mut self, mut seed: Vec<SeedInput>) -> Vec<SeedInput> {
        let chain_len = self.chain.len();
        while seed.len() < chain_len {
            seed.push(self.random_seed_for_step(seed.len()));
        }
        seed.truncate(chain_len);
        seed
    }

    /// Remember a concrete chain seed in local corpus (deduplicated).
    pub fn remember_seed(&mut self, seed: Vec<SeedInput>) {
        self.remember_seed_with_score(seed, 1);
    }

    pub fn remember_seed_with_score(&mut self, seed: Vec<SeedInput>, score: u32) {
        let normalized = self.normalize_seed_len(seed);
        if let Some(existing) = self
            .seedpool
            .iter_mut()
            .find(|record| record.input == normalized)
        {
            existing.score = existing.score.saturating_add(score.max(1));
            existing.last_used_at = self.exec_count;
        } else {
            self.seedpool.push(ChainSeedRecord {
                input: normalized,
                score: score.max(1),
                last_used_at: self.exec_count,
            });
            self.prune_seedpool();
        }
    }

    /// Absorb shared object discoveries from other fuzzers
    pub fn absorb_shared_object_writes(&mut self, writes: &[ResourceWrite]) {
        for mutator in self.mutators.iter_mut() {
            mutator.update_object_dict(writes);
        }
    }

    /// Import a seed from a parent sequence (for sequence extension).
    ///
    /// The `parent_seed` covers the first `parent_seed.len()` steps of this chain.
    /// Remaining steps get random inputs generated by their respective mutators.
    pub fn import_parent_seed<S: Into<SeedInput>>(&mut self, parent_seed: Vec<S>) {
        let mut parent_seed: Vec<SeedInput> = parent_seed.into_iter().map(Into::into).collect();
        self.identity_seed = parent_seed.clone();
        let chain_len = self.chain.len();
        while parent_seed.len() < chain_len {
            parent_seed.push(self.random_seed_for_step(parent_seed.len()));
        }
        self.remember_seed(parent_seed);
    }

    /// Update coverage map, return true if new coverage is found
    fn update_coverage(&mut self, new_map: CoverageMap) -> bool {
        merge_coverage(&mut self.coverage, new_map)
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::prep::ident::FunctionIdent;
    use move_core_types::{
        identifier::Identifier, language_storage::TypeTag as VmTypeTag, value::MoveValue,
    };
    use rand::SeedableRng;
    use std::path::PathBuf;

    /// Helper: create a ResourceTag from a simple name
    fn make_tag(name: &str) -> ResourceTag {
        ResourceTag {
            account: aptos_types::account_address::AccountAddress::ONE,
            struct_tag: StructTag {
                address: aptos_types::account_address::AccountAddress::ONE,
                module: Identifier::new("test").unwrap(),
                name: Identifier::new(name).unwrap(),
                type_args: vec![],
            },
        }
    }

    /// Helper: create a ResourceTag for a specific storage account
    fn make_tag_at(name: &str, account: AccountAddress) -> ResourceTag {
        ResourceTag {
            account,
            struct_tag: StructTag {
                address: aptos_types::account_address::AccountAddress::ONE,
                module: Identifier::new("test").unwrap(),
                name: Identifier::new(name).unwrap(),
                type_args: vec![],
            },
        }
    }

    fn make_object_group_write(account: AccountAddress) -> ResourceWrite {
        ResourceWrite {
            address: account,
            struct_tag: StructTag {
                address: aptos_types::account_address::AccountAddress::ONE,
                module: Identifier::new("object").unwrap(),
                name: Identifier::new("ObjectGroup").unwrap(),
                type_args: vec![],
            },
            is_resource_group: true,
        }
    }

    /// Helper: create a ScriptProfile
    fn make_profile(
        index: usize,
        reads: Vec<&str>,
        writes: Vec<&str>,
        succeeded: bool,
    ) -> ScriptProfile {
        ScriptProfile {
            script_index: index,
            reads: reads.into_iter().map(make_tag).collect(),
            writes: writes.into_iter().map(make_tag).collect(),
            ever_succeeded: succeeded,
        }
    }

    /// Helper: create a ResourceWrite from a simple name
    fn make_resource_write(name: &str) -> ResourceWrite {
        let tag = make_tag(name);
        ResourceWrite {
            address: tag.account,
            struct_tag: tag.struct_tag,
            is_resource_group: false,
        }
    }

    // -----------------------------------------------------------------------
    // DUG construction tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_dug_from_profiles_basic() {
        // S0: writes {T_A}, reads {}
        // S1: writes {T_B}, reads {T_A}
        // S2: writes {}, reads {T_A, T_B}
        let profiles = vec![
            make_profile(0, vec![], vec!["A"], true),
            make_profile(1, vec!["A"], vec!["B"], true),
            make_profile(2, vec!["A", "B"], vec![], false),
        ];

        let dug = DefUseGraph::from_profiles(&profiles);

        assert_eq!(dug.num_scripts(), 3);
        assert_eq!(dug.num_types(), 2); // A, B

        // S0 defs A
        assert_eq!(dug.defs_of(0).len(), 1);
        // S1 defs B
        assert_eq!(dug.defs_of(1).len(), 1);
        // S2 never succeeded → no defs recorded
        assert_eq!(dug.defs_of(2).len(), 0);

        // S0 uses nothing
        assert!(dug.uses_of(0).is_empty());
        // S1 uses A
        assert_eq!(dug.uses_of(1).len(), 1);
        // S2 uses A, B
        assert_eq!(dug.uses_of(2).len(), 2);

        // Producers of A = {S0}, producers of B = {S1}
        let tag_a = make_tag("A");
        let tag_b = make_tag("B");
        let ti_a = *dug.type_index.get(&tag_a).unwrap();
        let ti_b = *dug.type_index.get(&tag_b).unwrap();
        assert_eq!(dug.producers_of(ti_a), &BTreeSet::from([0]));
        assert_eq!(dug.producers_of(ti_b), &BTreeSet::from([1]));

        // S2 has unmet deps = {A, B}
        assert_eq!(dug.unmet_deps(2).len(), 2);
        // S1 has unmet deps = {A}
        assert_eq!(dug.unmet_deps(1).len(), 1);
        // S0 has no unmet deps
        assert!(dug.unmet_deps(0).is_empty());

        // ever_succeeded
        assert!(dug.script_ever_succeeded(0));
        assert!(dug.script_ever_succeeded(1));
        assert!(!dug.script_ever_succeeded(2));
    }

    #[test]
    fn test_dug_empty() {
        let profiles: Vec<ScriptProfile> = vec![];
        let dug = DefUseGraph::from_profiles(&profiles);
        assert_eq!(dug.num_scripts(), 0);
        assert_eq!(dug.num_types(), 0);
    }

    #[test]
    fn test_dug_no_producers() {
        // S0: reads {T_X}, writes nothing, never succeeded
        let profiles = vec![make_profile(0, vec!["X"], vec![], false)];
        let dug = DefUseGraph::from_profiles(&profiles);

        assert_eq!(dug.num_types(), 1);
        let tag_x = make_tag("X");
        let ti_x = *dug.type_index.get(&tag_x).unwrap();
        assert!(dug.producers_of(ti_x).is_empty());
    }

    #[test]
    fn test_dug_resource_tag_distinguishes_accounts() {
        // Same struct type under different storage accounts should map to distinct nodes.
        let account_1 = AccountAddress::from_hex_literal("0x1").unwrap();
        let account_2 = AccountAddress::from_hex_literal("0x2").unwrap();

        let profiles = vec![
            ScriptProfile {
                script_index: 0,
                reads: BTreeSet::new(),
                writes: BTreeSet::from([make_tag_at("A", account_1)]),
                ever_succeeded: true,
            },
            ScriptProfile {
                script_index: 1,
                reads: BTreeSet::new(),
                writes: BTreeSet::from([make_tag_at("A", account_2)]),
                ever_succeeded: true,
            },
        ];
        let dug = DefUseGraph::from_profiles(&profiles);

        assert_eq!(dug.num_types(), 2);
        let t1 = dug.type_index_of(&make_tag_at("A", account_1)).copied();
        let t2 = dug.type_index_of(&make_tag_at("A", account_2)).copied();
        assert!(t1.is_some());
        assert!(t2.is_some());
        assert_ne!(t1, t2);
    }

    #[test]
    fn test_dug_object_aware_dependency_satisfaction() {
        let account_1 = AccountAddress::from_hex_literal("0x11").unwrap();
        let account_2 = AccountAddress::from_hex_literal("0x12").unwrap();
        let mut dug = DefUseGraph::new(2);
        dug.ingest_initial_writes(&[
            make_object_group_write(account_1),
            make_object_group_write(account_2),
        ]);
        dug.ingest_profile(&ExecResourceProfile {
            script_index: 0,
            reads: BTreeSet::new(),
            writes: BTreeSet::from([make_tag_at("Vault", account_1)]),
            succeeded: true,
        });
        dug.ingest_profile(&ExecResourceProfile {
            script_index: 1,
            reads: BTreeSet::from([make_tag_at("Vault", account_2)]),
            writes: BTreeSet::new(),
            succeeded: false,
        });

        assert!(dug.are_dependencies_satisfied(&[0, 1]));
    }

    #[test]
    fn test_dug_object_aware_seed_dependency_satisfaction() {
        let account_1 = AccountAddress::from_hex_literal("0x21").unwrap();
        let account_2 = AccountAddress::from_hex_literal("0x22").unwrap();
        let mut dug = DefUseGraph::new(2);
        dug.ingest_initial_writes(&[
            make_object_group_write(account_1),
            make_object_group_write(account_2),
        ]);
        let producer = ExecResourceProfile {
            script_index: 0,
            reads: BTreeSet::new(),
            writes: BTreeSet::from([make_tag_at("Position", account_1)]),
            succeeded: true,
        };
        let consumer = ExecResourceProfile {
            script_index: 1,
            reads: BTreeSet::from([make_tag_at("Position", account_2)]),
            writes: BTreeSet::new(),
            succeeded: false,
        };
        let seed = SeedInput::new(AccountAddress::ONE, vec![], vec![]);
        dug.add_seed_observation(&producer, seed.clone());
        dug.add_seed_observation(&consumer, seed);

        assert!(dug.are_seed_dependencies_satisfied(&[0, 1]));
    }

    #[test]
    fn test_sequence_db_object_aware_prefix_matching() {
        let account_1 = AccountAddress::from_hex_literal("0x31").unwrap();
        let account_2 = AccountAddress::from_hex_literal("0x32").unwrap();
        let mut dug = DefUseGraph::new(2);
        dug.ingest_initial_writes(&[
            make_object_group_write(account_1),
            make_object_group_write(account_2),
        ]);
        dug.ingest_profile(&ExecResourceProfile {
            script_index: 1,
            reads: BTreeSet::from([make_tag_at("Vault", account_2)]),
            writes: BTreeSet::new(),
            succeeded: false,
        });

        let mut seq_db = SequenceDb::new();
        let producer_profile = ExecResourceProfile {
            script_index: 0,
            reads: BTreeSet::new(),
            writes: BTreeSet::from([make_tag_at("Vault", account_1)]),
            succeeded: true,
        };
        seq_db.add_entry(
            vec![0],
            vec![SeedInput::new(AccountAddress::ONE, vec![], vec![])],
            &[producer_profile],
        );

        let compatible = seq_db.find_concrete_prefix_seeds(&dug, &[0, 1]);
        assert_eq!(compatible.len(), 1);
        assert_eq!(
            SequenceDb::entry_prefix_relevance(&dug, &[0, 1], compatible[0]),
            1
        );
        assert!(SequenceDb::entry_prefix_is_state_consistent(
            &dug,
            compatible[0]
        ));

        let exact_needs = dug.exact_unmet_dependency_tags(1);
        assert_eq!(
            dug.compatible_resource_overlap(&compatible[0].produced_types, &exact_needs),
            1,
        );
        assert!(!compatible[0]
            .produced_types
            .contains(&make_tag_at("Vault", account_2)));
    }

    #[test]
    fn test_dug_snapshot_roundtrip_preserves_seed_catalog() {
        let mut dug = DefUseGraph::new(2);
        let profile = ExecResourceProfile {
            script_index: 1,
            reads: BTreeSet::from([make_tag("A")]),
            writes: BTreeSet::from([make_tag("B")]),
            succeeded: true,
        };
        let seed = SeedInput::from((vec![VmTypeTag::Bool], vec![MoveValue::U64(7)]));
        let (_, seed_id) = dug.add_seed_observation(&profile, seed.clone());

        let snapshot = dug.snapshot().unwrap();
        let restored = DefUseGraph::from_persisted(snapshot).unwrap();

        assert_eq!(restored.num_scripts(), 2);
        assert_eq!(restored.num_seeds(), 1);
        assert_eq!(restored.seed_node(0).id, seed_id);
        assert_eq!(restored.seed_node(0).seed, seed);
        assert!(restored.script_ever_succeeded(1));
    }

    // -----------------------------------------------------------------------
    // Chain construction tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_chain_linear() {
        // S0: writes {A}, reads {}
        // S1: writes {B}, reads {A}
        // S2: writes {}, reads {B}   (never succeeded)
        let profiles = vec![
            make_profile(0, vec![], vec!["A"], true),
            make_profile(1, vec!["A"], vec!["B"], true),
            make_profile(2, vec!["B"], vec![], false),
        ];
        let dug = DefUseGraph::from_profiles(&profiles);
        let mut rng = StdRng::seed_from_u64(42);

        let chains = construct_chains(&dug, 5, 2, 10, &mut rng);

        // Should produce at least one chain ending at S2
        assert!(!chains.is_empty());
        let chain = &chains[0];
        assert_eq!(chain.target(), 2);
        // Chain must contain S1 (produces B) and may contain S0 (produces A for S1)
        assert!(chain.steps.contains(&1));
        // Target is last
        assert_eq!(*chain.steps.last().unwrap(), 2);
    }

    #[test]
    fn test_chain_diamond() {
        // S0: writes {A}, reads {}
        // S1: writes {B}, reads {}
        // S2: writes {}, reads {A, B}  (never succeeded)
        let profiles = vec![
            make_profile(0, vec![], vec!["A"], true),
            make_profile(1, vec![], vec!["B"], true),
            make_profile(2, vec!["A", "B"], vec![], false),
        ];
        let dug = DefUseGraph::from_profiles(&profiles);
        let mut rng = StdRng::seed_from_u64(42);

        let chains = construct_chains(&dug, 5, 2, 10, &mut rng);

        assert!(!chains.is_empty());
        let chain = &chains[0];
        assert_eq!(chain.target(), 2);
        // Chain must include both S0 and S1
        assert!(chain.steps.contains(&0));
        assert!(chain.steps.contains(&1));
        // Both must come before S2
        let pos_0 = chain.steps.iter().position(|&s| s == 0).unwrap();
        let pos_1 = chain.steps.iter().position(|&s| s == 1).unwrap();
        let pos_2 = chain.steps.iter().position(|&s| s == 2).unwrap();
        assert!(pos_0 < pos_2);
        assert!(pos_1 < pos_2);
    }

    #[test]
    fn test_chain_max_length() {
        // Deep chain: S0->A, S1 reads A writes B, S2 reads B writes C, S3 reads C writes D, S4 reads D
        let profiles = vec![
            make_profile(0, vec![], vec!["A"], true),
            make_profile(1, vec!["A"], vec!["B"], true),
            make_profile(2, vec!["B"], vec!["C"], true),
            make_profile(3, vec!["C"], vec!["D"], true),
            make_profile(4, vec!["D"], vec![], false),
        ];
        let dug = DefUseGraph::from_profiles(&profiles);
        let mut rng = StdRng::seed_from_u64(42);

        // max_chain_length = 3 is insufficient to satisfy S4's transitive dependencies.
        // With strict unresolved-dependency rejection, no chain targeting S4 should be returned.
        let chains = construct_chains(&dug, 3, 2, 10, &mut rng);
        assert!(chains.iter().all(|c| c.target() != 4));
    }

    #[test]
    fn test_sequence_db_snapshot_roundtrip_preserves_entry() {
        let mut db = SequenceDb::new();
        db.add_entry(
            vec![0, 1],
            vec![
                SeedInput::from((vec![], vec![MoveValue::U8(1)])),
                SeedInput::from((vec![], vec![MoveValue::U8(2)])),
            ],
            &[
                ExecResourceProfile {
                    script_index: 0,
                    reads: BTreeSet::new(),
                    writes: BTreeSet::from([make_tag("A")]),
                    succeeded: true,
                },
                ExecResourceProfile {
                    script_index: 1,
                    reads: BTreeSet::from([make_tag("A")]),
                    writes: BTreeSet::from([make_tag("B")]),
                    succeeded: true,
                },
            ],
        );

        let snapshot = db.snapshot().unwrap();
        let restored = SequenceDb::from_persisted(snapshot).unwrap();

        assert_eq!(restored.len(), 1);
        assert_eq!(restored.entries()[0].steps, vec![0, 1]);
        assert_eq!(restored.entries()[0].seed.len(), 2);
        assert!(restored.entries()[0].all_succeeded);
    }

    #[test]
    fn test_seq_db_add_entry_deduplicates_identical_seed_and_steps() {
        let mut db = SequenceDb::new();
        let profiles = vec![make_exec_profile(0, vec![], vec!["A"], true)];
        let seed = vec![(vec![], vec![MoveValue::U64(1)])];

        let first_id = db.add_entry(vec![0], seed.clone(), &profiles);
        let second_id = db.add_entry(vec![0], seed, &profiles);

        assert_eq!(first_id, second_id);
        assert_eq!(db.len(), 1);
    }

    #[test]
    fn test_chain_fuzzer_scoring_updates_existing_and_prunes_lowest_score() {
        let sig = ScriptSignature {
            name: "script_0".to_string(),
            ident: FunctionIdent::from_function_tuple(
                AccountAddress::ONE,
                Identifier::new("m").unwrap(),
                Identifier::new("f").unwrap(),
            ),
            generics: vec![],
            parameters: vec![BasicInput::U64],
        };
        let entrypoints = vec![
            (sig.clone(), vec![]),
            (
                ScriptSignature {
                    name: "script_1".to_string(),
                    ..sig
                },
                vec![],
            ),
        ];
        let mut fuzzer = ChainFuzzer::new(
            TracingExecutor::new(),
            13,
            Chain { steps: vec![0, 1] },
            &entrypoints,
            TypePool::new(),
            PathBuf::from("/tmp/move-fuzz-chain-tests.trace"),
            vec![],
        );

        let low_score_seed = vec![
            SeedInput::from((vec![], vec![MoveValue::U64(0)])),
            SeedInput::from((vec![], vec![MoveValue::U64(1000)])),
        ];
        fuzzer.remember_seed_with_score(low_score_seed.clone(), 1);

        for value in 1..=MAX_CHAIN_CORPUS as u64 {
            fuzzer.remember_seed_with_score(
                vec![
                    SeedInput::from((vec![], vec![MoveValue::U64(value)])),
                    SeedInput::from((vec![], vec![MoveValue::U64(value + 1000)])),
                ],
                10,
            );
        }

        assert_eq!(fuzzer.corpus_size(), MAX_CHAIN_CORPUS);
        assert!(
            !fuzzer
                .seedpool
                .iter()
                .any(|record| record.input == low_score_seed),
            "lowest-score chain seed should be pruned when the corpus exceeds the cap"
        );

        let boosted_seed = vec![
            SeedInput::from((vec![], vec![MoveValue::U64(1)])),
            SeedInput::from((vec![], vec![MoveValue::U64(1001)])),
        ];
        fuzzer.remember_seed_with_score(boosted_seed.clone(), 7);
        let retained = fuzzer
            .seedpool
            .iter()
            .find(|record| record.input == boosted_seed)
            .expect("existing seed should still be present");
        assert_eq!(retained.score, 17);
        assert_eq!(fuzzer.best_seed_score(), 17);
        assert!(fuzzer.average_seed_score() > 0.0);
        assert_eq!(fuzzer.seed_pool_snapshot().len(), fuzzer.corpus_size());
    }

    #[test]
    fn test_chain_self_sufficient() {
        // S0: reads {A}, writes {A}. Without initial state or a predecessor
        // producer, the chain builder should not treat this as self-sufficient.
        let profiles = vec![make_profile(0, vec!["A"], vec!["A"], true)];
        let dug = DefUseGraph::from_profiles(&profiles);
        let mut rng = StdRng::seed_from_u64(42);

        let chains = construct_chains(&dug, 5, 2, 10, &mut rng);

        assert_eq!(dug.unmet_deps(0).len(), 1);
        assert!(chains.is_empty());
    }

    #[test]
    fn test_chain_no_producer() {
        // S0: reads {X}, writes {} (never succeeded, X has no producer)
        let profiles = vec![make_profile(0, vec!["X"], vec![], false)];
        let dug = DefUseGraph::from_profiles(&profiles);
        let mut rng = StdRng::seed_from_u64(42);

        let chains = construct_chains(&dug, 5, 2, 10, &mut rng);

        // S0 has unmet deps but no producer can be found → no chain
        assert!(chains.is_empty());
    }

    #[test]
    fn test_chain_repetition_limit() {
        // S0: reads {A}, writes {A, B}  (succeeded, needs itself for A)
        // S1: reads {B}, writes {}  (never succeeded)
        let profiles = vec![
            make_profile(0, vec!["A"], vec!["A", "B"], true),
            make_profile(1, vec!["B"], vec![], false),
        ];
        let mut dug = DefUseGraph::from_profiles(&profiles);
        dug.ingest_initial_writes(&[make_resource_write("A")]);
        let mut rng = StdRng::seed_from_u64(42);

        let chains = construct_chains(&dug, 5, 1, 10, &mut rng);

        // With A provisioned initially, S0 can run once and produce B for S1.
        assert!(!chains.is_empty());
        let chain = &chains[0];
        assert_eq!(chain.target(), 1);
        assert_eq!(chain.steps.iter().filter(|&&s| s == 0).count(), 1);
    }

    #[test]
    fn test_chain_multiple_producers() {
        // S0 and S1 both produce A; S2 reads A (never succeeded)
        let profiles = vec![
            make_profile(0, vec![], vec!["A"], true),
            make_profile(1, vec![], vec!["A"], true),
            make_profile(2, vec!["A"], vec![], false),
        ];
        let dug = DefUseGraph::from_profiles(&profiles);
        let mut rng = StdRng::seed_from_u64(42);

        let chains = construct_chains(&dug, 5, 2, 10, &mut rng);

        assert!(!chains.is_empty());
        let chain = &chains[0];
        assert_eq!(chain.target(), 2);
        // Chain should have one of S0 or S1 as producer (either is fine)
        assert!(chain.steps.contains(&0) || chain.steps.contains(&1));
        assert_eq!(chain.len(), 2);
    }

    // -----------------------------------------------------------------------
    // DUG mutation tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_dug_add_def_basic() {
        let profiles = vec![
            make_profile(0, vec![], vec!["A"], true),
            make_profile(1, vec!["A"], vec![], false),
        ];
        let mut dug = DefUseGraph::from_profiles(&profiles);
        assert_eq!(dug.num_types(), 1); // only A

        // Add a new def: S1 writes B (new type)
        let tag_b = make_tag("B");
        let changed = dug.add_def(1, &tag_b);
        assert!(changed);
        assert_eq!(dug.num_types(), 2); // A and B
        assert_eq!(dug.defs_of(1).len(), 1);
        let ti_b = *dug.type_index.get(&tag_b).unwrap();
        assert!(dug.producers_of(ti_b).contains(&1));
    }

    #[test]
    fn test_dug_add_def_idempotent() {
        let profiles = vec![make_profile(0, vec![], vec!["A"], true)];
        let mut dug = DefUseGraph::from_profiles(&profiles);
        let marker = dug.modification_marker();

        // Adding the same def again should not change the DUG
        let tag_a = make_tag("A");
        let changed = dug.add_def(0, &tag_a);
        assert!(!changed);
        assert!(!dug.has_changed_since(marker));
    }

    #[test]
    fn test_dug_add_use_basic() {
        let profiles = vec![
            make_profile(0, vec![], vec!["A"], true),
            make_profile(1, vec![], vec![], true),
        ];
        let mut dug = DefUseGraph::from_profiles(&profiles);
        assert!(dug.uses_of(1).is_empty());

        // S1 now reads A
        let tag_a = make_tag("A");
        let changed = dug.add_use(1, &tag_a);
        assert!(changed);
        assert_eq!(dug.uses_of(1).len(), 1);
    }

    #[test]
    fn test_dug_add_use_new_type() {
        let profiles = vec![make_profile(0, vec![], vec![], false)];
        let mut dug = DefUseGraph::from_profiles(&profiles);
        assert_eq!(dug.num_types(), 0);

        // S0 reads X (type X does not exist yet)
        let tag_x = make_tag("X");
        let changed = dug.add_use(0, &tag_x);
        assert!(changed);
        assert_eq!(dug.num_types(), 1);
        assert_eq!(dug.uses_of(0).len(), 1);
    }

    #[test]
    fn test_dug_mark_succeeded() {
        let profiles = vec![make_profile(0, vec!["A"], vec![], false)];
        let mut dug = DefUseGraph::from_profiles(&profiles);
        assert!(!dug.script_ever_succeeded(0));

        let changed = dug.mark_succeeded(0);
        assert!(changed);
        assert!(dug.script_ever_succeeded(0));

        // Idempotent
        let changed2 = dug.mark_succeeded(0);
        assert!(!changed2);
    }

    #[test]
    fn test_dug_initial_state_satisfies_dependencies() {
        let profiles = vec![
            make_profile(0, vec!["A"], vec![], false),
            make_profile(1, vec!["A"], vec!["B"], true),
        ];
        let mut dug = DefUseGraph::from_profiles(&profiles);

        assert_eq!(dug.unmet_deps(0).len(), 1);
        assert!(!dug.are_dependencies_satisfied(&[0]));
        assert!(!dug.exact_unmet_dependency_tags(0).is_empty());

        let changed = dug.ingest_initial_writes(&[make_resource_write("A")]);
        assert!(changed);
        assert!(dug.unmet_deps(0).is_empty());
        assert!(dug.are_dependencies_satisfied(&[0]));
        assert!(dug.exact_unmet_dependency_tags(0).is_empty());
        assert!(dug.initial_resource_tags().contains(&make_tag("A")));
    }

    #[test]
    fn test_dug_modification_tracking() {
        let profiles = vec![
            make_profile(0, vec![], vec!["A"], true),
            make_profile(1, vec!["A"], vec![], false),
        ];
        let mut dug = DefUseGraph::from_profiles(&profiles);
        let m0 = dug.modification_marker();

        // No change yet
        assert!(!dug.has_changed_since(m0));

        // Add a new def
        dug.add_def(1, &make_tag("B"));
        assert!(dug.has_changed_since(m0));
        let m1 = dug.modification_marker();
        assert!(!dug.has_changed_since(m1));
    }

    #[test]
    fn test_dug_ingest_profile() {
        let profiles = vec![
            make_profile(0, vec![], vec!["A"], true),
            make_profile(1, vec![], vec![], false),
        ];
        let mut dug = DefUseGraph::from_profiles(&profiles);
        let m0 = dug.modification_marker();

        // Ingest a profile where S1 reads A and writes B, and succeeded
        let profile = ExecResourceProfile {
            script_index: 1,
            reads: vec![make_tag("A")].into_iter().collect(),
            writes: vec![make_tag("B")].into_iter().collect(),
            succeeded: true,
        };
        dug.ingest_profile(&profile);

        assert!(dug.has_changed_since(m0));
        assert!(dug.script_ever_succeeded(1));
        assert_eq!(dug.uses_of(1).len(), 1); // reads A
        assert_eq!(dug.defs_of(1).len(), 1); // writes B
        assert_eq!(dug.num_types(), 2); // A and B
    }

    #[test]
    fn test_dug_seed_catalog_tracking() {
        let mut dug = DefUseGraph::new(1);
        let marker = dug.seed_modification_marker();
        assert!(!dug.has_seed_catalog_changed_since(marker));

        let profile = ExecResourceProfile {
            script_index: 0,
            reads: BTreeSet::new(),
            writes: BTreeSet::from([make_tag("A")]),
            succeeded: true,
        };
        let seed = SeedInput::new(AccountAddress::ONE, vec![], vec![MoveValue::U64(1)]);
        dug.add_seed_observation(&profile, seed);

        assert!(dug.has_seed_catalog_changed_since(marker));
        let new_marker = dug.seed_modification_marker();
        assert!(!dug.has_seed_catalog_changed_since(new_marker));
    }

    #[test]
    fn test_dug_chain_reconstruction_after_mutation() {
        // Initially: S0 writes A, S1 reads nothing (no chain possible to S1)
        let profiles = vec![
            make_profile(0, vec![], vec!["A"], true),
            make_profile(1, vec![], vec![], false),
        ];
        let mut dug = DefUseGraph::from_profiles(&profiles);
        let mut rng = StdRng::seed_from_u64(42);

        let chains_before = construct_chains(&dug, 5, 2, 10, &mut rng);
        // S1 has no unmet deps, so no chains to it
        assert!(chains_before.iter().all(|c| c.target() != 1));

        // Now S1 reads A — this creates an unmet dependency that S0 can resolve
        dug.add_use(1, &make_tag("A"));

        let mut rng2 = StdRng::seed_from_u64(42);
        let chains_after = construct_chains(&dug, 5, 2, 10, &mut rng2);
        // Now there should be a chain [S0, S1]
        let chain_to_1 = chains_after.iter().find(|c| c.target() == 1);
        assert!(chain_to_1.is_some());
        let chain = chain_to_1.unwrap();
        assert!(chain.steps.contains(&0));
    }

    #[test]
    fn test_construct_seed_chain_uses_seed_nodes_and_inputs() {
        let mut dug = DefUseGraph::new(2);
        let sender_1 = AccountAddress::from_hex_literal("0x1").unwrap();
        let sender_2 = AccountAddress::from_hex_literal("0x2").unwrap();

        let seed0 = SeedInput::new(sender_1, vec![], vec![MoveValue::U64(7)]);
        let p0 = ExecResourceProfile {
            script_index: 0,
            reads: BTreeSet::new(),
            writes: BTreeSet::from([make_tag("A")]),
            succeeded: true,
        };
        let (_, id0) = dug.add_seed_observation(&p0, seed0.clone());

        let seed1 = SeedInput::new(sender_2, vec![], vec![MoveValue::U64(9)]);
        let p1 = ExecResourceProfile {
            script_index: 1,
            reads: BTreeSet::from([make_tag("A")]),
            writes: BTreeSet::new(),
            succeeded: false,
        };
        let (_, id1) = dug.add_seed_observation(&p1, seed1.clone());

        let mut rng = StdRng::seed_from_u64(11);
        let chains = construct_seed_chains(&dug, 5, 2, 10, &mut rng);
        let chain = chains
            .iter()
            .find(|c| c.target_seed_id == id1)
            .expect("expected a seed chain for seed1");

        assert_eq!(id0, 0);
        assert_eq!(id1, 1);
        assert_eq!(chain.chain.steps, vec![0, 1]);
        assert_eq!(chain.seed_inputs, vec![seed0, seed1]);
    }

    #[test]
    fn test_construct_seed_chains_for_targets_only_returns_requested_targets() {
        let profiles = vec![
            make_profile(0, vec![], vec!["A"], true),
            make_profile(1, vec!["A"], vec!["B"], true),
            make_profile(2, vec!["B"], vec![], false),
            make_profile(3, vec![], vec!["X"], true),
            make_profile(4, vec!["X"], vec![], false),
        ];
        let mut dug = DefUseGraph::from_profiles(&profiles);
        for (idx, profile) in [
            make_exec_profile(0, vec![], vec!["A"], true),
            make_exec_profile(1, vec!["A"], vec!["B"], true),
            make_exec_profile(2, vec!["B"], vec![], false),
            make_exec_profile(3, vec![], vec!["X"], true),
            make_exec_profile(4, vec!["X"], vec![], false),
        ]
        .into_iter()
        .enumerate()
        {
            dug.add_seed_observation(
                &profile,
                SeedInput::from((vec![], vec![MoveValue::U64(idx as u64)])),
            );
        }

        let mut rng = StdRng::seed_from_u64(11);
        let chains =
            construct_seed_chains_for_targets(&dug, &BTreeSet::from([2usize]), 5, 2, 8, &mut rng);

        assert!(
            !chains.is_empty(),
            "expected a chain for the requested target"
        );
        assert!(chains.iter().all(|chain| chain.chain.target() == 2));
        assert!(
            chains
                .iter()
                .any(|chain| chain.chain.steps == vec![0, 1, 2]),
            "expected the dependency chain for script 2"
        );
    }

    // -----------------------------------------------------------------------
    // DUG accessor tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_dug_type_index_of() {
        let profiles = vec![
            make_profile(0, vec![], vec!["A"], true),
            make_profile(1, vec!["A"], vec!["B"], true),
        ];
        let dug = DefUseGraph::from_profiles(&profiles);

        // Known tags should return Some
        assert!(dug.type_index_of(&make_tag("A")).is_some());
        assert!(dug.type_index_of(&make_tag("B")).is_some());
        // Indices should be distinct
        assert_ne!(
            dug.type_index_of(&make_tag("A")),
            dug.type_index_of(&make_tag("B"))
        );
        // Unknown tag should return None
        assert!(dug.type_index_of(&make_tag("C")).is_none());
    }

    #[test]
    fn test_dug_consumers_of() {
        // S0 writes A, S1 and S2 read A
        let profiles = vec![
            make_profile(0, vec![], vec!["A"], true),
            make_profile(1, vec!["A"], vec![], false),
            make_profile(2, vec!["A"], vec![], false),
        ];
        let dug = DefUseGraph::from_profiles(&profiles);

        let ti_a = *dug.type_index_of(&make_tag("A")).unwrap();
        let consumers = dug.consumers_of(ti_a);
        assert_eq!(consumers, BTreeSet::from([1, 2]));

        // S0 doesn't read A, so it's not a consumer
        assert!(!consumers.contains(&0));
    }

    // -----------------------------------------------------------------------
    // SequenceDb tests
    // -----------------------------------------------------------------------

    /// Helper: create an ExecResourceProfile for testing
    fn make_exec_profile(
        script_index: usize,
        reads: Vec<&str>,
        writes: Vec<&str>,
        succeeded: bool,
    ) -> ExecResourceProfile {
        ExecResourceProfile {
            script_index,
            reads: reads.into_iter().map(make_tag).collect(),
            writes: writes.into_iter().map(make_tag).collect(),
            succeeded,
        }
    }

    #[test]
    fn test_seq_db_add_and_retrieve() {
        let mut db = SequenceDb::new();
        assert!(db.is_empty());
        assert_eq!(db.len(), 0);

        let profiles = vec![
            make_exec_profile(0, vec![], vec!["A"], true),
            make_exec_profile(1, vec!["A"], vec!["B"], true),
        ];
        let seed = vec![
            (vec![], vec![MoveValue::U64(42)]),
            (vec![], vec![MoveValue::Bool(true)]),
        ];

        let id = db.add_entry(vec![0, 1], seed.clone(), &profiles);
        assert_eq!(id, 0);
        assert_eq!(db.len(), 1);
        assert!(!db.is_empty());

        // Verify the entry
        let entries = db.find_prefix_seeds(&[0, 1]);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].steps, vec![0, 1]);
        assert!(entries[0].all_succeeded);
        assert!(entries[0].produced_types.contains(&make_tag("A")));
        assert!(entries[0].produced_types.contains(&make_tag("B")));
        assert!(entries[0].consumed_types.contains(&make_tag("A")));
    }

    #[test]
    fn test_seq_db_prefix_matching_exact() {
        let mut db = SequenceDb::new();
        let profiles = vec![
            make_exec_profile(0, vec![], vec!["A"], true),
            make_exec_profile(1, vec!["A"], vec!["B"], true),
            make_exec_profile(2, vec!["B"], vec![], true),
        ];
        let seed = vec![
            (vec![], vec![MoveValue::U64(1)]),
            (vec![], vec![MoveValue::U64(2)]),
            (vec![], vec![MoveValue::U64(3)]),
        ];
        db.add_entry(vec![0, 1, 2], seed, &profiles);

        // Exact match
        let matches = db.find_prefix_seeds(&[0, 1, 2]);
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].steps, vec![0, 1, 2]);
    }

    #[test]
    fn test_seq_db_prefix_matching_superchain() {
        let mut db = SequenceDb::new();
        let profiles = vec![
            make_exec_profile(0, vec![], vec!["A"], true),
            make_exec_profile(1, vec!["A"], vec![], true),
        ];
        let seed = vec![
            (vec![], vec![MoveValue::U64(1)]),
            (vec![], vec![MoveValue::U64(2)]),
        ];
        db.add_entry(vec![0, 1], seed, &profiles);

        // Entry [0,1] is a prefix of chain [0,1,2]
        let matches = db.find_prefix_seeds(&[0, 1, 2]);
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].steps, vec![0, 1]);
    }

    #[test]
    fn test_seq_db_prefix_no_match() {
        let mut db = SequenceDb::new();
        let profiles = vec![
            make_exec_profile(0, vec![], vec!["A"], true),
            make_exec_profile(1, vec![], vec![], true),
            make_exec_profile(3, vec![], vec![], true),
        ];
        let seed = vec![
            (vec![], vec![MoveValue::U64(1)]),
            (vec![], vec![MoveValue::U64(2)]),
            (vec![], vec![MoveValue::U64(3)]),
        ];
        db.add_entry(vec![0, 1, 3], seed, &profiles);

        // [0,1,3] is not a prefix of [0,1,2] (step 2 at position 2 differs from step 3)
        let matches = db.find_prefix_seeds(&[0, 1, 2]);
        assert_eq!(matches.len(), 0);
    }

    #[test]
    fn test_seq_db_prefix_longer_entry() {
        let mut db = SequenceDb::new();
        let profiles = vec![
            make_exec_profile(0, vec![], vec!["A"], true),
            make_exec_profile(1, vec![], vec![], true),
            make_exec_profile(2, vec![], vec![], true),
            make_exec_profile(3, vec![], vec![], true),
        ];
        let seed = vec![
            (vec![], vec![MoveValue::U64(1)]),
            (vec![], vec![MoveValue::U64(2)]),
            (vec![], vec![MoveValue::U64(3)]),
            (vec![], vec![MoveValue::U64(4)]),
        ];
        db.add_entry(vec![0, 1, 2, 3], seed, &profiles);

        // Entry [0,1,2,3] is longer than chain [0,1], not a prefix
        let matches = db.find_prefix_seeds(&[0, 1]);
        assert_eq!(matches.len(), 0);
    }

    #[test]
    fn test_seq_db_prefix_seed_ignores_failed_entries() {
        let mut db = SequenceDb::new();
        let chain = vec![0, 1];

        let failed_profiles = vec![
            make_exec_profile(0, vec![], vec!["A"], true),
            make_exec_profile(1, vec!["A"], vec!["B"], false),
        ];
        let failed_seed = vec![
            SeedInput::new(
                AccountAddress::from_hex_literal("0x1").unwrap(),
                vec![],
                vec![MoveValue::U64(10)],
            ),
            SeedInput::new(
                AccountAddress::from_hex_literal("0x1").unwrap(),
                vec![],
                vec![MoveValue::U64(11)],
            ),
        ];
        db.add_entry(chain.clone(), failed_seed, &failed_profiles);

        let success_profiles = vec![
            make_exec_profile(0, vec![], vec!["A"], true),
            make_exec_profile(1, vec!["A"], vec!["B"], true),
        ];
        let success_seed = vec![
            SeedInput::new(
                AccountAddress::from_hex_literal("0x2").unwrap(),
                vec![],
                vec![MoveValue::U64(20)],
            ),
            SeedInput::new(
                AccountAddress::from_hex_literal("0x2").unwrap(),
                vec![],
                vec![MoveValue::U64(21)],
            ),
        ];
        db.add_entry(chain.clone(), success_seed.clone(), &success_profiles);

        assert_eq!(db.prefix_compatible_count(&chain), 1);

        let mut rng = StdRng::seed_from_u64(7);
        let picked = db.pick_prefix_seed(&chain, &mut rng).unwrap();
        assert_eq!(picked, success_seed);
    }

    #[test]
    fn test_seq_db_concrete_prefix_prefers_exact_needed_tag() {
        let profiles = vec![
            make_profile(0, vec![], vec!["A"], true),
            make_profile(1, vec![], vec!["B"], true),
            make_profile(2, vec!["A"], vec![], false),
        ];
        let dug = DefUseGraph::from_profiles(&profiles);

        let mut db = SequenceDb::new();
        let seed_a = vec![SeedInput::new(
            AccountAddress::from_hex_literal("0x1").unwrap(),
            vec![],
            vec![MoveValue::U64(10)],
        )];
        let seed_b = vec![SeedInput::new(
            AccountAddress::from_hex_literal("0x2").unwrap(),
            vec![],
            vec![MoveValue::U64(20)],
        )];
        db.add_entry(vec![0], seed_a.clone(), &[make_exec_profile(
            0,
            vec![],
            vec!["A"],
            true,
        )]);
        db.add_entry(vec![1], seed_b, &[make_exec_profile(
            1,
            vec![],
            vec!["B"],
            true,
        )]);

        let compatible = db.find_concrete_prefix_seeds(&dug, &[0, 2]);
        assert_eq!(compatible.len(), 1);
        assert_eq!(compatible[0].steps, vec![0]);
        assert_eq!(db.concrete_prefix_compatible_count(&dug, &[0, 2]), 1);

        let mut rng = StdRng::seed_from_u64(9);
        let picked = db
            .pick_concrete_prefix_seed(&dug, &[0, 2], &mut rng)
            .unwrap();
        assert_eq!(picked, seed_a);
    }

    #[test]
    fn test_seq_db_concrete_prefix_allows_initial_state_prefix() {
        let mut dug = DefUseGraph::from_profiles(&[
            make_profile(0, vec!["A"], vec!["B"], true),
            make_profile(1, vec!["B"], vec![], false),
        ]);
        dug.ingest_initial_writes(&[make_resource_write("A")]);

        let mut db = SequenceDb::new();
        let seed = vec![SeedInput::new(
            AccountAddress::from_hex_literal("0x1").unwrap(),
            vec![],
            vec![MoveValue::U64(30)],
        )];
        db.add_entry(vec![0], seed.clone(), &[make_exec_profile(
            0,
            vec!["A"],
            vec!["B"],
            true,
        )]);

        let compatible = db.find_concrete_prefix_seeds(&dug, &[0, 1]);
        assert_eq!(compatible.len(), 1);

        let mut rng = StdRng::seed_from_u64(5);
        let picked = db
            .pick_concrete_prefix_seed(&dug, &[0, 1], &mut rng)
            .unwrap();
        assert_eq!(picked, seed);
    }

    #[test]
    fn test_seq_db_propose_extensions() {
        // DUG: S0 writes A, S1 reads A and writes B, S2 reads B
        let profiles = vec![
            make_profile(0, vec![], vec!["A"], true),
            make_profile(1, vec!["A"], vec!["B"], true),
            make_profile(2, vec!["B"], vec![], false),
        ];
        let dug = DefUseGraph::from_profiles(&profiles);

        let mut db = SequenceDb::new();
        // Stored sequence [0, 1] that produced type B
        let exec_profiles = vec![
            make_exec_profile(0, vec![], vec!["A"], true),
            make_exec_profile(1, vec!["A"], vec!["B"], true),
        ];
        let seed = vec![
            (vec![], vec![MoveValue::U64(1)]),
            (vec![], vec![MoveValue::U64(2)]),
        ];
        db.add_entry(vec![0, 1], seed, &exec_profiles);

        // Should propose [0, 1, 2] because S2 reads B which is produced by the sequence.
        // May also propose [0, 1, 1] (recursive: S1 reads B which it produced).
        let extensions = db.propose_extensions(&dug, 5, 2, 10);
        assert!(!extensions.is_empty());
        let has_012 = extensions
            .iter()
            .any(|(chain, seed)| chain.steps == vec![0, 1, 2] && seed.len() == 2);
        assert!(has_012, "expected extension [0, 1, 2]");
    }

    #[test]
    fn test_seq_db_no_extension_for_failing_sequence() {
        let profiles = vec![
            make_profile(0, vec![], vec!["A"], true),
            make_profile(1, vec!["A"], vec!["B"], true),
            make_profile(2, vec!["B"], vec![], false),
        ];
        let dug = DefUseGraph::from_profiles(&profiles);

        let mut db = SequenceDb::new();
        // Entry where one step failed
        let exec_profiles = vec![
            make_exec_profile(0, vec![], vec!["A"], true),
            make_exec_profile(1, vec!["A"], vec!["B"], false), // failed!
        ];
        let seed = vec![
            (vec![], vec![MoveValue::U64(1)]),
            (vec![], vec![MoveValue::U64(2)]),
        ];
        db.add_entry(vec![0, 1], seed, &exec_profiles);

        // Should not propose extensions for failing sequences
        let extensions = db.propose_extensions(&dug, 5, 2, 10);
        assert!(extensions.is_empty());
    }

    #[test]
    fn test_seq_db_extension_dependency_validation() {
        // S0 writes A; S2 reads A and C. Extending [S0] with S2 should be rejected
        // because C is not produced by the sequence.
        let profiles = vec![
            make_profile(0, vec![], vec!["A"], true),
            make_profile(1, vec![], vec!["C"], true),
            make_profile(2, vec!["A", "C"], vec![], false),
        ];
        let dug = DefUseGraph::from_profiles(&profiles);

        let mut db = SequenceDb::new();
        let exec_profiles = vec![make_exec_profile(0, vec![], vec!["A"], true)];
        db.add_entry(
            vec![0],
            vec![(vec![], vec![MoveValue::U64(1)])],
            &exec_profiles,
        );

        let extensions = db.propose_extensions(&dug, 5, 2, 10);
        assert!(!extensions.iter().any(|(c, _)| c.steps == vec![0, 2]));
    }

    #[test]
    fn test_seq_db_no_extension_at_max_length() {
        let profiles = vec![
            make_profile(0, vec![], vec!["A"], true),
            make_profile(1, vec!["A"], vec!["B"], true),
            make_profile(2, vec!["B"], vec![], false),
        ];
        let dug = DefUseGraph::from_profiles(&profiles);

        let mut db = SequenceDb::new();
        let exec_profiles = vec![
            make_exec_profile(0, vec![], vec!["A"], true),
            make_exec_profile(1, vec!["A"], vec!["B"], true),
        ];
        let seed = vec![
            (vec![], vec![MoveValue::U64(1)]),
            (vec![], vec![MoveValue::U64(2)]),
        ];
        db.add_entry(vec![0, 1], seed, &exec_profiles);

        // Max chain length is 2, entry has 2 steps → no extension possible
        let extensions = db.propose_extensions(&dug, 2, 2, 10);
        assert!(extensions.is_empty());
    }

    #[test]
    fn test_seq_db_extension_recursive_sequence() {
        // DUG: S0 writes A, S1 reads A and writes B, S1 also reads B (self-loop)
        // This mirrors the slides: a script that both reads and writes a type
        // can appear multiple times in a chain once the first invocation has
        // its initial B dependency satisfied.
        let profiles = vec![
            make_profile(0, vec![], vec!["A"], true),
            make_profile(1, vec!["A", "B"], vec!["B"], true),
        ];
        let mut dug = DefUseGraph::from_profiles(&profiles);
        dug.ingest_initial_writes(&[make_resource_write("B")]);

        let mut db = SequenceDb::new();
        let exec_profiles = vec![
            make_exec_profile(0, vec![], vec!["A"], true),
            make_exec_profile(1, vec!["A", "B"], vec!["B"], true),
        ];
        let seed = vec![
            (vec![], vec![MoveValue::U64(1)]),
            (vec![], vec![MoveValue::U64(2)]),
        ];
        db.add_entry(vec![0, 1], seed, &exec_profiles);

        // With max_repetition=2, S1 can appear again → [0, 1, 1]
        let extensions = db.propose_extensions(&dug, 5, 2, 10);
        assert!(!extensions.is_empty());
        // Should propose [0, 1, 1] — S1 reads B which the sequence produces
        let has_recursive = extensions
            .iter()
            .any(|(chain, _)| chain.steps == vec![0, 1, 1]);
        assert!(has_recursive, "expected recursive extension [0, 1, 1]");

        // With max_repetition=1, S1 already appears once → no extension
        let extensions = db.propose_extensions(&dug, 5, 1, 10);
        assert!(extensions.is_empty());
    }

    #[test]
    fn test_seq_db_extension_slides_example() {
        // Mirrors the slides example:
        //   S1 reads T1, T3. S2 writes T1. S3 writes T3.
        //   S1 writes T2 (discovered during execution). S4 reads T2.
        //   S4 writes T3 (discovered during execution).
        //
        // Sequence P2 = <S2, S3, S1, S4> produces T3.
        // S1 reads T3, so forward extension yields P3 = <S2, S3, S1, S4, S1>.
        //
        // Using indices: S1=0, S2=1, S3=2, S4=3
        let profiles = vec![
            make_profile(0, vec!["T1", "T3"], vec!["T2"], true), // S1
            make_profile(1, vec![], vec!["T1"], true),           // S2
            make_profile(2, vec![], vec!["T3"], true),           // S3
            make_profile(3, vec!["T2"], vec!["T3"], true),       // S4
        ];
        let dug = DefUseGraph::from_profiles(&profiles);

        let mut db = SequenceDb::new();
        // P2 = <S2, S3, S1, S4> = [1, 2, 0, 3]
        let exec_profiles = vec![
            make_exec_profile(1, vec![], vec!["T1"], true),
            make_exec_profile(2, vec![], vec!["T3"], true),
            make_exec_profile(0, vec!["T1", "T3"], vec!["T2"], true),
            make_exec_profile(3, vec!["T2"], vec!["T3"], true),
        ];
        let seed = vec![
            (vec![], vec![MoveValue::U64(1)]),
            (vec![], vec![MoveValue::U64(2)]),
            (vec![], vec![MoveValue::U64(3)]),
            (vec![], vec![MoveValue::U64(4)]),
        ];
        db.add_entry(vec![1, 2, 0, 3], seed, &exec_profiles);

        // P2 produces T3 (from S4). S1 (index 0) reads T3.
        // With max_repetition=2, S1 can appear again → P3 = [1, 2, 0, 3, 0]
        let extensions = db.propose_extensions(&dug, 6, 2, 20);
        let has_p3 = extensions
            .iter()
            .any(|(chain, _)| chain.steps == vec![1, 2, 0, 3, 0]);
        assert!(has_p3, "expected slides example P3 = [1, 2, 0, 3, 0]");
    }

    // -----------------------------------------------------------------------
    // Tests for are_dependencies_satisfied
    // -----------------------------------------------------------------------

    #[test]
    fn test_dug_dependencies_satisfied_linear() {
        // S0: writes A, S1: reads A writes B, S2: reads B
        let profiles = vec![
            make_profile(0, vec![], vec!["A"], true),
            make_profile(1, vec!["A"], vec!["B"], true),
            make_profile(2, vec!["B"], vec![], false),
        ];
        let dug = DefUseGraph::from_profiles(&profiles);

        // [0, 1, 2] is valid: A from S0, B from S1
        assert!(dug.are_dependencies_satisfied(&[0, 1, 2]));
        // [1, 2] invalid: S1 reads A which is not produced
        assert!(!dug.are_dependencies_satisfied(&[1, 2]));
        // [0, 2] invalid: S2 reads B which is not produced by S0
        assert!(!dug.are_dependencies_satisfied(&[0, 2]));
        // [0, 1] valid
        assert!(dug.are_dependencies_satisfied(&[0, 1]));
        // [0] valid (self-sufficient, no reads)
        assert!(dug.are_dependencies_satisfied(&[0]));
        // empty is valid
        assert!(dug.are_dependencies_satisfied(&[]));
        // out-of-bounds script index
        assert!(!dug.are_dependencies_satisfied(&[99]));
    }

    // -----------------------------------------------------------------------
    // Tests for sequence-level mutation
    // -----------------------------------------------------------------------

    #[test]
    fn test_seq_db_mutate_step_deletion() {
        // S0: writes A, S1: reads A writes B, S2: reads A (not B!)
        let profiles = vec![
            make_profile(0, vec![], vec!["A"], true),
            make_profile(1, vec!["A"], vec!["B"], true),
            make_profile(2, vec!["A"], vec![], false),
        ];
        let dug = DefUseGraph::from_profiles(&profiles);

        let mut db = SequenceDb::new();
        let exec_profiles = vec![
            make_exec_profile(0, vec![], vec!["A"], true),
            make_exec_profile(1, vec!["A"], vec!["B"], true),
            make_exec_profile(2, vec!["A"], vec![], true),
        ];
        let seed = vec![
            (vec![], vec![MoveValue::U64(1)]),
            (vec![], vec![MoveValue::U64(2)]),
            (vec![], vec![MoveValue::U64(3)]),
        ];
        db.add_entry(vec![0, 1, 2], seed, &exec_profiles);

        let deletions = db.mutate_step_deletion(&dug, 5, 2);
        // Removing S1 (index 1) should be valid: [0, 2] works because S2 reads A (from S0)
        assert!(deletions.iter().any(|(c, _)| c.steps == vec![0, 2]));
        // The seed for [0, 2] should have 2 entries
        let del_02 = deletions
            .iter()
            .find(|(c, _)| c.steps == vec![0, 2])
            .unwrap();
        assert_eq!(del_02.1.len(), 2);
        // Removing S0 (index 0) should be invalid: [1, 2] has S1 reading A with no producer
        assert!(!deletions.iter().any(|(c, _)| c.steps == vec![1, 2]));
    }

    #[test]
    fn test_seq_db_mutate_step_duplication() {
        // S0: writes A, S1: reads A
        let profiles = vec![
            make_profile(0, vec![], vec!["A"], true),
            make_profile(1, vec!["A"], vec![], false),
        ];
        let dug = DefUseGraph::from_profiles(&profiles);

        let mut db = SequenceDb::new();
        let exec_profiles = vec![
            make_exec_profile(0, vec![], vec!["A"], true),
            make_exec_profile(1, vec!["A"], vec![], true),
        ];
        let seed = vec![
            (vec![], vec![MoveValue::U64(1)]),
            (vec![], vec![MoveValue::U64(2)]),
        ];
        db.add_entry(vec![0, 1], seed, &exec_profiles);

        let duplications = db.mutate_step_duplication(&dug, 5, 2);
        // Duplicating S0 -> [0, 0, 1] should be valid
        assert!(duplications.iter().any(|(c, _)| c.steps == vec![0, 0, 1]));
        // Seed should have 3 entries
        let dup = duplications
            .iter()
            .find(|(c, _)| c.steps == vec![0, 0, 1])
            .unwrap();
        assert_eq!(dup.1.len(), 3);

        // Duplicating S1 -> [0, 1, 1] should also be valid (A is still available from S0)
        assert!(duplications.iter().any(|(c, _)| c.steps == vec![0, 1, 1]));

        // With max_chain_length = 2, no duplications possible
        let no_dups = db.mutate_step_duplication(&dug, 2, 2);
        assert!(no_dups.is_empty());

        // With max_repetition = 1, duplications would violate the chain repetition limit.
        let no_repeat_dups = db.mutate_step_duplication(&dug, 5, 1);
        assert!(no_repeat_dups.is_empty());
    }

    #[test]
    fn test_seq_db_mutate_subsequence_extraction() {
        // S0: writes A, S1: reads A writes B, S2: reads B writes C, S3: reads C
        let profiles = vec![
            make_profile(0, vec![], vec!["A"], true),
            make_profile(1, vec!["A"], vec!["B"], true),
            make_profile(2, vec!["B"], vec!["C"], true),
            make_profile(3, vec!["C"], vec![], false),
        ];
        let dug = DefUseGraph::from_profiles(&profiles);

        let mut db = SequenceDb::new();
        let exec_profiles = vec![
            make_exec_profile(0, vec![], vec!["A"], true),
            make_exec_profile(1, vec!["A"], vec!["B"], true),
            make_exec_profile(2, vec!["B"], vec!["C"], true),
            make_exec_profile(3, vec!["C"], vec![], true),
        ];
        let seed = vec![
            (vec![], vec![MoveValue::U64(1)]),
            (vec![], vec![MoveValue::U64(2)]),
            (vec![], vec![MoveValue::U64(3)]),
            (vec![], vec![MoveValue::U64(4)]),
        ];
        db.add_entry(vec![0, 1, 2, 3], seed, &exec_profiles);

        let subsequences = db.mutate_subsequence_extraction(&dug, 5, 2);
        // [0, 1] should be valid (S0 writes A, S1 reads A)
        assert!(subsequences.iter().any(|(c, _)| c.steps == vec![0, 1]));
        // [1, 2] should be invalid (S1 reads A, which is not in the subsequence)
        assert!(!subsequences.iter().any(|(c, _)| c.steps == vec![1, 2]));
        // [0, 1, 2] should be valid
        assert!(subsequences.iter().any(|(c, _)| c.steps == vec![0, 1, 2]));
        // [0, 1, 2] seed should have 3 entries
        let sub = subsequences
            .iter()
            .find(|(c, _)| c.steps == vec![0, 1, 2])
            .unwrap();
        assert_eq!(sub.1.len(), 3);
    }

    #[test]
    fn test_seq_db_mutate_sequence_splicing() {
        // S0: writes A, S1: reads A writes B, S2: reads A writes C, S3: reads C
        let profiles = vec![
            make_profile(0, vec![], vec!["A"], true),
            make_profile(1, vec!["A"], vec!["B"], true),
            make_profile(2, vec!["A"], vec!["C"], true),
            make_profile(3, vec!["C"], vec![], false),
        ];
        let dug = DefUseGraph::from_profiles(&profiles);

        let mut db = SequenceDb::new();
        // Entry 1: [0, 1]
        let ep1 = vec![
            make_exec_profile(0, vec![], vec!["A"], true),
            make_exec_profile(1, vec!["A"], vec!["B"], true),
        ];
        let seed1 = vec![
            (vec![], vec![MoveValue::U64(10)]),
            (vec![], vec![MoveValue::U64(20)]),
        ];
        db.add_entry(vec![0, 1], seed1, &ep1);

        // Entry 2: [0, 2, 3]
        let ep2 = vec![
            make_exec_profile(0, vec![], vec!["A"], true),
            make_exec_profile(2, vec!["A"], vec!["C"], true),
            make_exec_profile(3, vec!["C"], vec![], true),
        ];
        let seed2 = vec![
            (vec![], vec![MoveValue::U64(30)]),
            (vec![], vec![MoveValue::U64(40)]),
            (vec![], vec![MoveValue::U64(50)]),
        ];
        db.add_entry(vec![0, 2, 3], seed2, &ep2);

        let splicings = db.mutate_sequence_splicing(&dug, 5, 2);
        // [0, 2, 3] = prefix [0] from entry1 + suffix [2, 3] from entry2
        // S0 writes A, S2 reads A (ok), S2 writes C, S3 reads C (ok). Valid!
        assert!(splicings.iter().any(|(c, _)| c.steps == vec![0, 2, 3]));

        // Verify the seed is correctly spliced: entry1.seed[0] + entry2.seed[1..3]
        let splice = splicings
            .iter()
            .find(|(c, _)| c.steps == vec![0, 2, 3])
            .unwrap();
        assert_eq!(splice.1.len(), 3);
        // First seed element from entry1 (value 10)
        assert_eq!(splice.1[0].args[0], MoveValue::U64(10));
        // Second seed element from entry2 index 1 (value 40)
        assert_eq!(splice.1[1].args[0], MoveValue::U64(40));
    }

    #[test]
    fn test_seq_db_propose_mutations_dedup_and_cap() {
        // S0: writes A, S1: reads A writes B, S2: reads B
        let profiles = vec![
            make_profile(0, vec![], vec!["A"], true),
            make_profile(1, vec!["A"], vec!["B"], true),
            make_profile(2, vec!["B"], vec![], false),
        ];
        let dug = DefUseGraph::from_profiles(&profiles);

        let mut db = SequenceDb::new();
        let exec_profiles = vec![
            make_exec_profile(0, vec![], vec!["A"], true),
            make_exec_profile(1, vec!["A"], vec!["B"], true),
            make_exec_profile(2, vec!["B"], vec![], true),
        ];
        let seed = vec![
            (vec![], vec![MoveValue::U64(1)]),
            (vec![], vec![MoveValue::U64(2)]),
            (vec![], vec![MoveValue::U64(3)]),
        ];
        db.add_entry(vec![0, 1, 2], seed, &exec_profiles);

        // Request at most 3 mutations
        let mutations = db.propose_mutations(&dug, 5, 2, 3);
        assert!(mutations.len() <= 3);

        // All returned chains should have unique step sequences
        let step_sets: BTreeSet<Vec<usize>> =
            mutations.iter().map(|(c, _)| c.steps.clone()).collect();
        assert_eq!(step_sets.len(), mutations.len());
    }

    #[test]
    fn test_seq_db_propose_mutations_empty() {
        let profiles = vec![make_profile(0, vec![], vec!["A"], true)];
        let dug = DefUseGraph::from_profiles(&profiles);
        let db = SequenceDb::new();

        let mutations = db.propose_mutations(&dug, 5, 2, 10);
        assert!(mutations.is_empty());
    }

    #[test]
    fn test_seq_db_propose_mutations_respects_repetition_limit() {
        let profiles = vec![
            make_profile(0, vec![], vec!["A"], true),
            make_profile(1, vec!["A"], vec![], true),
        ];
        let dug = DefUseGraph::from_profiles(&profiles);

        let mut db = SequenceDb::new();
        let exec_profiles = vec![
            make_exec_profile(0, vec![], vec!["A"], true),
            make_exec_profile(1, vec!["A"], vec![], true),
        ];
        let seed = vec![
            (vec![], vec![MoveValue::U64(1)]),
            (vec![], vec![MoveValue::U64(2)]),
        ];
        db.add_entry(vec![0, 1], seed, &exec_profiles);

        let mutations = db.propose_mutations(&dug, 5, 1, 10);
        assert!(mutations.is_empty());
    }

    #[test]
    fn test_seq_db_propose_mutations_respects_chain_length_limit() {
        let profiles = vec![
            make_profile(0, vec![], vec!["A"], true),
            make_profile(1, vec!["A"], vec!["B"], true),
            make_profile(2, vec!["B"], vec![], true),
            make_profile(3, vec!["A"], vec!["C"], true),
            make_profile(4, vec!["C"], vec![], true),
        ];
        let dug = DefUseGraph::from_profiles(&profiles);

        let mut db = SequenceDb::new();
        let exec_profiles = vec![
            make_exec_profile(0, vec![], vec!["A"], true),
            make_exec_profile(1, vec!["A"], vec!["B"], true),
            make_exec_profile(2, vec!["B"], vec![], true),
            make_exec_profile(3, vec!["A"], vec!["C"], true),
            make_exec_profile(4, vec!["C"], vec![], true),
        ];
        let seed = vec![
            (vec![], vec![MoveValue::U64(1)]),
            (vec![], vec![MoveValue::U64(2)]),
            (vec![], vec![MoveValue::U64(3)]),
            (vec![], vec![MoveValue::U64(4)]),
            (vec![], vec![MoveValue::U64(5)]),
        ];
        db.add_entry(vec![0, 1, 2, 3, 4], seed, &exec_profiles);

        let mutations = db.propose_mutations(&dug, 3, 2, 20);
        assert!(mutations.iter().all(|(chain, _)| chain.steps.len() <= 3));
    }
}
