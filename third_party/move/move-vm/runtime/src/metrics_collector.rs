// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! Metrics collection for MoveVM runtime analysis.
//!
//! This module provides infrastructure for collecting runtime metrics during
//! MoveVM execution, including instruction frequency and stats about
//! operations on locals, references, resources, and structs.
//!
//! Enable via environment variable: `MOVE_VM_METRICS=/path/to/output.json`

use indexmap::IndexMap;
use move_vm_types::{
    instr::Instruction,
    loaded_data::runtime_types::Type,
    values::{Value, ValueSize},
};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashSet,
    env,
    fs::File,
    io::{BufWriter, Write},
    sync::{
        atomic::{AtomicBool, Ordering},
        Mutex, OnceLock,
    },
};

// ============================================================================
// Configuration
// ============================================================================

const MOVE_VM_METRICS_ENV_VAR: &str = "MOVE_VM_METRICS";

static METRICS_ENABLED: OnceLock<AtomicBool> = OnceLock::new();
static METRICS_OUTPUT_PATH: OnceLock<String> = OnceLock::new();
static METRICS_COLLECTOR: OnceLock<Mutex<MetricsCollector>> = OnceLock::new();

/// Check if metrics collection is enabled via environment variable.
#[inline(always)]
pub fn are_metrics_enabled() -> bool {
    METRICS_ENABLED
        .get_or_init(|| AtomicBool::new(env::var(MOVE_VM_METRICS_ENV_VAR).is_ok()))
        .load(Ordering::Relaxed)
}

/// Get the output path for metrics JSON file.
fn metrics_output_path() -> &'static str {
    METRICS_OUTPUT_PATH.get_or_init(|| {
        env::var(MOVE_VM_METRICS_ENV_VAR).unwrap_or_else(|_| "vm_metrics.json".to_string())
    })
}

/// Get the global metrics collector instance.
fn metrics_collector() -> &'static Mutex<MetricsCollector> {
    METRICS_COLLECTOR.get_or_init(|| Mutex::new(MetricsCollector::default()))
}

// ============================================================================
// Data Structures
// ============================================================================

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ValueKind {
    Primitive,
    Reference,
    Other,
}

impl From<&Value> for ValueKind {
    fn from(value: &Value) -> Self {
        match value {
            Value::U8(_)
            | Value::U16(_)
            | Value::U32(_)
            | Value::U64(_)
            | Value::U128(_)
            | Value::U256(_)
            | Value::I8(_)
            | Value::I16(_)
            | Value::I32(_)
            | Value::I64(_)
            | Value::I128(_)
            | Value::I256(_)
            | Value::Bool(_)
            | Value::Address(_) => ValueKind::Primitive,
            Value::ContainerRef(..) | Value::IndexedRef(..) => ValueKind::Reference,
            _ => ValueKind::Other,
        }
    }
}

/// Aggregate size estimation for multiple values (used for pack/unpack operations).
/// Combines all values into a single ValueSize representing the total.
fn estimate_value_size_aggregate<'a>(values: impl IntoIterator<Item = &'a Value>) -> ValueSize {
    let mut total_inline = 0;
    let mut all_heap_allocations = Vec::new();

    for value in values {
        let size = value.estimate_size();
        total_inline += size.inline_bytes;
        all_heap_allocations.extend(size.heap_allocations);
    }

    ValueSize {
        inline_bytes: total_inline,
        heap_allocations: all_heap_allocations,
    }
}

// ============================================================================
// Metrics collector
// ============================================================================

/// Metric for a local
#[derive(Clone, Debug, Serialize, Deserialize)]
struct LocalMetric {
    value_kind: ValueKind,
    value_size: ValueSize,
}

/// Metric for a reference
#[derive(Clone, Debug, Serialize, Deserialize)]
struct RefMetric {
    value_kind: ValueKind,
    value_size: ValueSize,
}

/// Metric for a resource
#[derive(Clone, Debug, Serialize, Deserialize)]
struct ResourceMetric {
    is_generic: bool,
    struct_type: String,
    size: ValueSize,
}

/// Metric for a struct/enum
#[derive(Clone, Debug, Serialize, Deserialize)]
struct StructMetric {
    is_generic: bool,
    struct_type: String,
    field_kinds: Vec<ValueKind>,
    size: ValueSize,
}

/// Metrics collector that tracks all execution metrics
#[derive(Default)]
pub struct MetricsCollector {
    instruction_counts: IndexMap<String, u64>,

    copy_loc_ops: Vec<LocalMetric>,
    move_loc_ops: Vec<LocalMetric>,
    st_loc_ops: Vec<LocalMetric>,

    read_ref_ops: Vec<RefMetric>,
    write_ref_ops: Vec<RefMetric>,

    move_to_ops: Vec<ResourceMetric>,
    move_from_ops: Vec<ResourceMetric>,
    borrow_global_ops: Vec<ResourceMetric>,

    pack_ops: Vec<StructMetric>,
    unpack_ops: Vec<StructMetric>,

    pack_variant_ops: Vec<StructMetric>,
    unpack_variant_ops: Vec<StructMetric>,
}

impl MetricsCollector {
    pub fn record_successful_instruction(&mut self, instr: &Instruction) {
        let name = instr.name().to_string();
        *self.instruction_counts.entry(name).or_default() += 1;
    }

    pub fn record_copy_loc(&mut self, value: &Value) {
        let metric = LocalMetric {
            value_size: value.estimate_size(),
            value_kind: ValueKind::from(value),
        };
        self.copy_loc_ops.push(metric);
    }

    pub fn record_move_loc(&mut self, value: &Value) {
        let metric = LocalMetric {
            value_size: value.estimate_size(),
            value_kind: ValueKind::from(value),
        };
        self.move_loc_ops.push(metric);
    }

    pub fn record_st_loc(&mut self, value: &Value) {
        let metric = LocalMetric {
            value_size: value.estimate_size(),
            value_kind: ValueKind::from(value),
        };
        self.st_loc_ops.push(metric);
    }

    pub fn record_read_ref(&mut self, value: &Value) {
        let metric = RefMetric {
            value_size: value.estimate_size(),
            value_kind: ValueKind::from(value),
        };
        self.read_ref_ops.push(metric);
    }

    pub fn record_write_ref(&mut self, value: &Value) {
        let metric = RefMetric {
            value_size: value.estimate_size(),
            value_kind: ValueKind::from(value),
        };
        self.write_ref_ops.push(metric);
    }

    pub fn record_move_to(&mut self, is_generic: bool, struct_type: &Type, value: &Value) {
        let metric = ResourceMetric {
            is_generic,
            struct_type: struct_type.to_string(),
            size: value.estimate_size(),
        };
        self.move_to_ops.push(metric);
    }

    pub fn record_move_from(&mut self, is_generic: bool, struct_type: &Type, value: &Value) {
        let metric = ResourceMetric {
            is_generic,
            struct_type: struct_type.to_string(),
            size: value.estimate_size(),
        };
        self.move_from_ops.push(metric);
    }

    pub fn record_borrow_global(
        &mut self,
        _is_mut: bool,
        is_generic: bool,
        struct_type: &Type,
        value: &Value,
    ) {
        let metric = ResourceMetric {
            is_generic,
            struct_type: struct_type.to_string(),
            size: value.estimate_size(),
        };
        self.borrow_global_ops.push(metric);
    }

    pub fn record_pack(&mut self, is_generic: bool, struct_type: &Type, values: &[Value]) {
        let struct_type = struct_type.to_string();
        let field_kinds = values.iter().map(ValueKind::from).collect();
        let size = estimate_value_size_aggregate(values);

        let metric = StructMetric {
            is_generic,
            struct_type,
            field_kinds,
            size,
        };
        self.pack_ops.push(metric);
    }

    pub fn record_pack_variant(&mut self, is_generic: bool, struct_type: &Type, values: &[Value]) {
        let struct_type = struct_type.to_string();
        let field_kinds = values.iter().map(ValueKind::from).collect();
        let size = estimate_value_size_aggregate(values);

        let metric = StructMetric {
            is_generic,
            struct_type,
            field_kinds,
            size,
        };
        self.pack_variant_ops.push(metric);
    }

    pub fn record_unpack(&mut self, is_generic: bool, struct_type: &Type, values: Vec<&Value>) {
        let struct_type = struct_type.to_string();
        let field_kinds = values.iter().copied().map(ValueKind::from).collect();
        let size = estimate_value_size_aggregate(values.iter().copied());

        let metric = StructMetric {
            is_generic,
            struct_type,
            field_kinds,
            size,
        };
        self.unpack_ops.push(metric);
    }

    pub fn record_unpack_variant(
        &mut self,
        is_generic: bool,
        struct_type: &Type,
        values: Vec<&Value>,
    ) {
        let struct_type = struct_type.to_string();
        let field_kinds = values.iter().copied().map(ValueKind::from).collect();
        let size = estimate_value_size_aggregate(values.iter().copied());

        let metric = StructMetric {
            is_generic,
            struct_type,
            field_kinds,
            size,
        };
        self.unpack_variant_ops.push(metric);
    }

    pub fn build_report(mut self) -> MetricsReport {
        self.instruction_counts
            .sort_by(|_k1, v1, _k2, v2| v2.cmp(v1));

        let local_stats = LocalStats::build(self.copy_loc_ops, self.move_loc_ops, self.st_loc_ops);
        let ref_stats = RefStats::build(self.read_ref_ops, self.write_ref_ops);
        let resource_stats =
            ResourceStats::build(self.move_to_ops, self.move_from_ops, self.borrow_global_ops);
        let struct_stats = StructStats::build(self.pack_ops, self.unpack_ops);
        let enum_stats = StructStats::build(self.pack_variant_ops, self.unpack_variant_ops);

        MetricsReport {
            instruction_counts: self.instruction_counts,
            local_stats,
            ref_stats,
            resource_stats,
            struct_stats,
            enum_stats,
        }
    }
}

// ============================================================================
// Metrics report
// ============================================================================

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MetricsReport {
    instruction_counts: IndexMap<String, u64>,
    local_stats: LocalStats,
    ref_stats: RefStats,
    resource_stats: ResourceStats,
    struct_stats: StructStats,
    enum_stats: StructStats,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct LocalStats {
    count: u64,
    copy_loc_count: u64,
    move_loc_count: u64,
    st_loc_count: u64,

    // Breakdown by value
    by_kind: IndexMap<ValueKind, u64>,
    copy_loc_by_kind: IndexMap<ValueKind, u64>,
    move_loc_by_kind: IndexMap<ValueKind, u64>,
    st_loc_by_kind: IndexMap<ValueKind, u64>,

    // Average size statistics
    avg_size: f64,
    avg_copy_loc_size: f64,
    avg_move_loc_size: f64,
    avg_st_loc_size: f64,
}

impl LocalStats {
    fn build(
        copy_loc_ops: Vec<LocalMetric>,
        move_loc_ops: Vec<LocalMetric>,
        st_loc_ops: Vec<LocalMetric>,
    ) -> Self {
        let mut stats = Self::default();

        stats.copy_loc_count = copy_loc_ops.len() as u64;
        stats.move_loc_count = move_loc_ops.len() as u64;
        stats.st_loc_count = st_loc_ops.len() as u64;
        stats.count = stats.copy_loc_count + stats.move_loc_count + stats.st_loc_count;

        let mut copy_loc_size = 0;
        let mut move_loc_size = 0;
        let mut st_loc_size = 0;

        for metric in copy_loc_ops {
            copy_loc_size += metric.value_size.total_bytes();
            *stats.by_kind.entry(metric.value_kind).or_default() += 1;
            *stats.copy_loc_by_kind.entry(metric.value_kind).or_default() += 1;
        }

        for metric in move_loc_ops {
            move_loc_size += metric.value_size.total_bytes();
            *stats.by_kind.entry(metric.value_kind).or_default() += 1;
            *stats.move_loc_by_kind.entry(metric.value_kind).or_default() += 1;
        }

        for metric in st_loc_ops {
            st_loc_size += metric.value_size.total_bytes();
            *stats.by_kind.entry(metric.value_kind).or_default() += 1;
            *stats.st_loc_by_kind.entry(metric.value_kind).or_default() += 1;
        }

        stats.by_kind.sort_by(|_k1, v1, _k2, v2| v2.cmp(v1));
        stats
            .copy_loc_by_kind
            .sort_by(|_k1, v1, _k2, v2| v2.cmp(v1));
        stats
            .move_loc_by_kind
            .sort_by(|_k1, v1, _k2, v2| v2.cmp(v1));
        stats.st_loc_by_kind.sort_by(|_k1, v1, _k2, v2| v2.cmp(v1));

        if stats.count > 0 {
            let total_size = copy_loc_size + move_loc_size + st_loc_size;
            stats.avg_size = total_size as f64 / stats.count as f64;
        }
        if stats.copy_loc_count > 0 {
            stats.avg_copy_loc_size = copy_loc_size as f64 / stats.copy_loc_count as f64;
        }
        if stats.move_loc_count > 0 {
            stats.avg_move_loc_size = move_loc_size as f64 / stats.move_loc_count as f64;
        }
        if stats.st_loc_count > 0 {
            stats.avg_st_loc_size = st_loc_size as f64 / stats.st_loc_count as f64;
        }

        stats
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct RefStats {
    count: u64,
    read_ref_count: u64,
    write_ref_count: u64,

    // Breakdown by value
    by_kind: IndexMap<ValueKind, u64>,
    read_ref_by_kind: IndexMap<ValueKind, u64>,
    write_ref_by_kind: IndexMap<ValueKind, u64>,

    // Average size statistics
    avg_size: f64,
    avg_read_ref_size: f64,
    avg_write_ref_size: f64,
}

impl RefStats {
    fn build(read_ref_ops: Vec<RefMetric>, write_ref_ops: Vec<RefMetric>) -> Self {
        let mut stats = Self::default();

        stats.read_ref_count = read_ref_ops.len() as u64;
        stats.write_ref_count = write_ref_ops.len() as u64;
        stats.count = stats.read_ref_count + stats.write_ref_count;

        let mut read_ref_size = 0;
        let mut write_ref_size = 0;

        for metric in read_ref_ops {
            read_ref_size += metric.value_size.total_bytes();
            *stats.by_kind.entry(metric.value_kind).or_default() += 1;
            *stats.read_ref_by_kind.entry(metric.value_kind).or_default() += 1;
        }

        for metric in write_ref_ops {
            write_ref_size += metric.value_size.total_bytes();
            *stats.by_kind.entry(metric.value_kind).or_default() += 1;
            *stats
                .write_ref_by_kind
                .entry(metric.value_kind)
                .or_default() += 1;
        }

        stats.by_kind.sort_by(|_k1, v1, _k2, v2| v2.cmp(v1));
        stats
            .read_ref_by_kind
            .sort_by(|_k1, v1, _k2, v2| v2.cmp(v1));
        stats
            .write_ref_by_kind
            .sort_by(|_k1, v1, _k2, v2| v2.cmp(v1));

        if stats.count > 0 {
            let total_size = read_ref_size + write_ref_size;
            stats.avg_size = total_size as f64 / stats.count as f64;
        }
        if stats.read_ref_count > 0 {
            stats.avg_read_ref_size = read_ref_size as f64 / stats.read_ref_count as f64;
        }
        if stats.write_ref_count > 0 {
            stats.avg_write_ref_size = write_ref_size as f64 / stats.write_ref_count as f64;
        }

        stats
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ResourceStats {
    count: u64,
    move_to_count: u64,
    move_from_count: u64,
    borrow_global_count: u64,
    unique_count: u64,

    // Generic breakdown
    move_to_generic_count: u64,
    move_from_generic_count: u64,
    borrow_global_generic_count: u64,

    // Average size statistics
    avg_size: f64,
    avg_move_to_size: f64,
    avg_move_from_size: f64,
    avg_borrow_global_size: f64,
}

impl ResourceStats {
    fn build(
        move_to_ops: Vec<ResourceMetric>,
        move_from_ops: Vec<ResourceMetric>,
        borrow_global_ops: Vec<ResourceMetric>,
    ) -> Self {
        let mut stats = Self::default();

        stats.move_to_count = move_to_ops.len() as u64;
        stats.move_from_count = move_from_ops.len() as u64;
        stats.borrow_global_count = borrow_global_ops.len() as u64;
        stats.count = stats.move_to_count + stats.move_from_count + stats.borrow_global_count;

        let mut unique_types = HashSet::new();
        let mut move_to_size = 0;
        let mut move_from_size = 0;
        let mut borrow_global_size = 0;

        for metric in move_to_ops {
            unique_types.insert(metric.struct_type);
            move_to_size += metric.size.total_bytes();
            if metric.is_generic {
                stats.move_to_generic_count += 1;
            }
        }

        for metric in move_from_ops {
            unique_types.insert(metric.struct_type);
            move_from_size += metric.size.total_bytes();
            if metric.is_generic {
                stats.move_from_generic_count += 1;
            }
        }

        for metric in borrow_global_ops {
            unique_types.insert(metric.struct_type);
            borrow_global_size += metric.size.total_bytes();
            if metric.is_generic {
                stats.borrow_global_generic_count += 1;
            }
        }

        stats.unique_count = unique_types.len() as u64;

        if stats.count > 0 {
            let total_size = move_to_size + move_from_size + borrow_global_size;
            stats.avg_size = total_size as f64 / stats.count as f64;
        }
        if stats.move_to_count > 0 {
            stats.avg_move_to_size = move_to_size as f64 / stats.move_to_count as f64;
        }
        if stats.move_from_count > 0 {
            stats.avg_move_from_size = move_from_size as f64 / stats.move_from_count as f64;
        }
        if stats.borrow_global_count > 0 {
            stats.avg_borrow_global_size =
                borrow_global_size as f64 / stats.borrow_global_count as f64;
        }

        stats
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct StructStats {
    count: u64,
    pack_count: u64,
    unpack_count: u64,
    unique_count: u64,

    // Breakdown by value
    fields_by_kind: IndexMap<ValueKind, u64>,
    pack_fields_by_kind: IndexMap<ValueKind, u64>,
    unpack_fields_by_kind: IndexMap<ValueKind, u64>,

    // Average field counts
    avg_fields: f64,
    avg_fields_per_pack: f64,
    avg_fields_per_unpack: f64,

    // Average size statistics
    avg_size: f64,
    avg_size_per_pack: f64,
    avg_size_per_unpack: f64,
}

impl StructStats {
    fn build(pack_ops: Vec<StructMetric>, unpack_ops: Vec<StructMetric>) -> Self {
        let mut stats = Self::default();

        stats.pack_count = pack_ops.len() as u64;
        stats.unpack_count = unpack_ops.len() as u64;
        stats.count = stats.pack_count + stats.unpack_count;

        let mut unique_types = HashSet::new();
        let mut pack_fields = 0;
        let mut unpack_fields = 0;
        let mut pack_size = 0;
        let mut unpack_size = 0;

        for metric in pack_ops {
            unique_types.insert(metric.struct_type);
            pack_fields += metric.field_kinds.len();
            pack_size += metric.size.total_bytes();
            for kind in metric.field_kinds {
                *stats.fields_by_kind.entry(kind).or_default() += 1;
                *stats.pack_fields_by_kind.entry(kind).or_default() += 1;
            }
        }

        for metric in unpack_ops {
            unique_types.insert(metric.struct_type);
            unpack_fields += metric.field_kinds.len();
            unpack_size += metric.size.total_bytes();
            for kind in metric.field_kinds {
                *stats.fields_by_kind.entry(kind).or_default() += 1;
                *stats.unpack_fields_by_kind.entry(kind).or_default() += 1;
            }
        }

        stats.unique_count = unique_types.len() as u64;

        stats.fields_by_kind.sort_by(|_k1, v1, _k2, v2| v2.cmp(v1));
        stats
            .pack_fields_by_kind
            .sort_by(|_k1, v1, _k2, v2| v2.cmp(v1));
        stats
            .unpack_fields_by_kind
            .sort_by(|_k1, v1, _k2, v2| v2.cmp(v1));

        if stats.count > 0 {
            let total_fields = pack_fields + unpack_fields;
            let total_size = pack_size + unpack_size;
            stats.avg_fields = total_fields as f64 / stats.count as f64;
            stats.avg_size = total_size as f64 / stats.count as f64;
        }
        if stats.pack_count > 0 {
            stats.avg_fields_per_pack = pack_fields as f64 / stats.pack_count as f64;
            stats.avg_size_per_pack = pack_size as f64 / stats.pack_count as f64;
        }
        if stats.unpack_count > 0 {
            stats.avg_fields_per_unpack = unpack_fields as f64 / stats.unpack_count as f64;
            stats.avg_size_per_unpack = unpack_size as f64 / stats.unpack_count as f64;
        }

        stats
    }
}

// ============================================================================
// Global recording functions
// ============================================================================

/// Record a successfully executed instruction.
#[inline(always)]
pub(crate) fn record_successful_instruction(instr: &Instruction) {
    if are_metrics_enabled() {
        metrics_collector()
            .lock()
            .unwrap()
            .record_successful_instruction(instr);
    }
}

/// Record a `copy_loc` instruction.
#[inline(always)]
pub(crate) fn record_copy_loc(value: &Value) {
    if are_metrics_enabled() {
        metrics_collector().lock().unwrap().record_copy_loc(value);
    }
}

/// Record a `move_loc` instruction.
#[inline(always)]
pub(crate) fn record_move_loc(value: &Value) {
    if are_metrics_enabled() {
        metrics_collector().lock().unwrap().record_move_loc(value);
    }
}

/// Record a `st_loc` instruction.
#[inline(always)]
pub(crate) fn record_st_loc(value: &Value) {
    if are_metrics_enabled() {
        metrics_collector().lock().unwrap().record_st_loc(value);
    }
}

/// Record a `read_ref` instruction.
#[inline(always)]
pub(crate) fn record_read_ref(value: &Value) {
    if are_metrics_enabled() {
        metrics_collector().lock().unwrap().record_read_ref(value);
    }
}

/// Record a `write_ref` instruction.
#[inline(always)]
pub(crate) fn record_write_ref(value: &Value) {
    if are_metrics_enabled() {
        metrics_collector().lock().unwrap().record_write_ref(value);
    }
}

/// Record a `pack` or `pack_generic` instruction.
#[inline(always)]
pub(crate) fn record_pack(is_generic: bool, struct_type: &Type, values: &[Value]) {
    if are_metrics_enabled() {
        metrics_collector()
            .lock()
            .unwrap()
            .record_pack(is_generic, struct_type, values);
    }
}

/// Record an `unpack` or `unpack_generic` instruction.
#[inline(always)]
pub(crate) fn record_unpack(is_generic: bool, struct_type: &Type, values: Vec<&Value>) {
    if are_metrics_enabled() {
        metrics_collector()
            .lock()
            .unwrap()
            .record_unpack(is_generic, struct_type, values);
    }
}

/// Record a `pack_variant` or `pack_variant_generic` instruction.
#[inline(always)]
pub(crate) fn record_pack_variant(is_generic: bool, struct_type: &Type, values: &[Value]) {
    if are_metrics_enabled() {
        metrics_collector()
            .lock()
            .unwrap()
            .record_pack_variant(is_generic, struct_type, values);
    }
}

/// Record an `unpack_variant` or `unpack_variant_generic` instruction.
#[inline(always)]
pub(crate) fn record_unpack_variant(is_generic: bool, struct_type: &Type, values: Vec<&Value>) {
    if are_metrics_enabled() {
        metrics_collector()
            .lock()
            .unwrap()
            .record_unpack_variant(is_generic, struct_type, values);
    }
}

/// Record a `move_to` or `move_to_generic` instruction.
#[inline(always)]
pub(crate) fn record_move_to(is_generic: bool, struct_type: &Type, value: &Value) {
    if are_metrics_enabled() {
        metrics_collector()
            .lock()
            .unwrap()
            .record_move_to(is_generic, struct_type, value);
    }
}

/// Record a `move_from` or `move_from_generic` instruction.
#[inline(always)]
pub(crate) fn record_move_from(is_generic: bool, struct_type: &Type, value: &Value) {
    if are_metrics_enabled() {
        metrics_collector()
            .lock()
            .unwrap()
            .record_move_from(is_generic, struct_type, value);
    }
}

/// Record a `mut_borrow_global`, `imm_borrow_global`, `mut_borrow_global_generic`, or `imm_borrow_global_generic` instruction.
#[inline(always)]
pub(crate) fn record_borrow_global(
    is_mut: bool,
    is_generic: bool,
    struct_type: &Type,
    value: &Value,
) {
    if are_metrics_enabled() {
        metrics_collector().lock().unwrap().record_borrow_global(
            is_mut,
            is_generic,
            struct_type,
            value,
        );
    }
}

/// Write accumulated metrics to file and clear for next transaction.
/// This should be called once at the end of transaction execution.
pub fn write_and_clear_metrics() {
    if !are_metrics_enabled() {
        return;
    }

    if let Some(global) = METRICS_COLLECTOR.get() {
        let mut collector = global.lock().unwrap();

        // Clear for next transaction
        let collector = std::mem::take(&mut *collector);

        // Build report from current metrics
        let report = collector.build_report();

        // Write to file
        if let Err(e) = write_report_to_file(&report) {
            eprintln!("Warning: Failed to write metrics report: {}", e);
        }
    }
}

fn write_report_to_file(report: &MetricsReport) -> std::io::Result<()> {
    let json = serde_json::to_string_pretty(&report).map_err(std::io::Error::other)?;

    let file = File::create(metrics_output_path())?;
    let mut writer = BufWriter::new(file);
    writer.write_all(json.as_bytes())?;
    writer.flush()?;

    Ok(())
}
