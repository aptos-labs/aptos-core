// Copyright (c) The Diem Core Contributors
// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

//! # Number Operation Tracking for Bitvector Analysis
//!
//! This module tracks whether numeric values in Move code should be represented as:
//! - Mathematical integers (`int`) - for arithmetic operations
//! - Fixed-width bitvectors (`bv8`, `bv16`, etc.) - for bitwise operations
//!
//! ## Purpose
//! The Move Prover translates Move code to Boogie for verification. For integer types,
//! we need to decide whether to use Boogie's mathematical integers or bitvector types:
//! - Bitvectors model exact wrapping arithmetic and bitwise operations
//! - Mathematical integers are simpler for SMT solvers when exact bit-level semantics aren't needed
//!
//! ## Workflow
//! 1. Parse `pragma bv` annotations to seed initial classifications
//! 2. Run dataflow analysis (in number_operation_analysis.rs) to propagate classifications
//! 3. Use the final state during Boogie code generation to emit correct types
//!
//! ## Example
//! ```move
//! // Mark parameter 0 as bitwise
//! fun bitwise_op(x: u8, y: u8): u8 {
//!     x & y  // Bitwise AND requires bitvector representation
//! }
//! spec bitwise_op {
//!     pragma bv = b"0";  // x should use bitvector type
//! }
//!
//! // Mark return value 0 as bitwise
//! fun returns_bitwise(x: u8, y: u8): u8 {
//!     x | y
//! }
//! spec returns_bitwise {
//!     pragma bv_ret = b"0";  // Return value uses bitvector type
//! }
//!
//! // Arithmetic operations can use mathematical integers (no pragma needed)
//! fun arithmetic_op(x: u8, y: u8): u8 {
//!     x + y  // Uses int type in Boogie
//! }
//!
//! // Mark struct field 0 as bitwise
//! struct Flags has store {
//!     bits: u8,  // Field 0
//!     count: u64 // Field 1
//! }
//! spec Flags {
//!     pragma bv = b"0";  // bits should use bitvector type
//! }
//! ```

use itertools::Itertools;
use move_model::{
    ast::{PropertyValue, TempIndex, Value},
    model::{FieldId, FunId, FunctionEnv, ModuleId, NodeId, SpecFunId, StructEnv, StructId},
    pragmas::{BV_PARAM_PROP, BV_RET_PROP},
};
use std::{collections::BTreeMap, ops::Deref, str};

static PARSING_ERROR: &str = "error happened when parsing the bv pragma";

/// Represents the type of numeric operations a value participates in.
/// This forms a lattice with Bottom < Arithmetic and Bottom < Bitwise.
/// Arithmetic and Bitwise are incompatible (conflict with each other).
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Default, Hash)]
pub enum NumOperation {
    /// Default value: not yet involved in any operations, or can be either arithmetic or bitwise.
    /// This is the bottom element in the lattice.
    #[default]
    Bottom,
    /// Involved in arithmetic operations (+, -, *, /, %).
    /// Should be represented as mathematical integers in Boogie.
    Arithmetic,
    /// Involved in bitwise operations (&, |, ^).
    /// Must be represented as bitvectors (bv8, bv16, etc.) in Boogie to model wrapping semantics.
    /// Note: Shift operations (<<, >>) are not classified as Bitwise; they propagate the existing type.
    Bitwise,
}

impl NumOperation {
    /// Check whether two operations are conflicting.
    /// Returns true if one is Arithmetic and the other is Bitwise.
    /// This indicates an error condition: a value cannot be used in both contexts.
    pub fn conflict(&self, other: &NumOperation) -> bool {
        use NumOperation::*;
        (*self == Arithmetic && *other == Bitwise) || (*self == Bitwise && *other == Arithmetic)
    }

    /// Merge two operations according to the partial order in NumOperation.
    /// Returns the greater element in the lattice:
    /// - Bottom merges with anything to produce that thing
    /// - Arithmetic and Bitwise don't merge (they conflict)
    /// - Identical values merge to themselves
    pub fn merge(&self, other: &NumOperation) -> NumOperation {
        if self.ge(other) {
            *self
        } else {
            *other
        }
    }
}

// Type aliases for various operation mappings

/// Maps temporary variable indices to their NumOperation within a function.
pub type OperationMap = BTreeMap<usize, NumOperation>;

/// Maps AST node IDs to their NumOperation (used for expressions in specs).
pub type ExpMap = BTreeMap<NodeId, NumOperation>;

/// A vector of NumOperations (used for function parameters/returns).
pub type OperationVec = Vec<NumOperation>;

/// Maps struct field IDs to their NumOperation.
pub type StructFieldOperationMap = BTreeMap<FieldId, NumOperation>;

/// Maps (ModuleId, FunId) to the operation map for that function's variables.
pub type FuncOperationMap = BTreeMap<(ModuleId, FunId), OperationMap>;

/// Maps (ModuleId, SpecFunId) to (parameter operations, return operations).
pub type SpecFuncOperationMap = BTreeMap<(ModuleId, SpecFunId), (OperationVec, OperationVec)>;

/// Maps (ModuleId, StructId) to the field operation map for that struct.
pub type StructOperationMap = BTreeMap<(ModuleId, StructId), StructFieldOperationMap>;

/// Global state tracking NumOperation for all program elements.
/// This is stored as an extension in GlobalEnv and is populated by the analysis phase,
/// then queried during Boogie code generation.
///
/// The state distinguishes between:
/// - Function parameters (seeded from `pragma bv`)
/// - Local variables (inferred from usage)
/// - Return values (seeded from `pragma bv_ret` or inferred)
/// - Struct fields (seeded from `pragma bv` on struct specs)
/// - Expression nodes (computed during spec translation)
#[derive(Default, Debug, Clone, Eq, PartialEq, PartialOrd)]
pub struct GlobalNumberOperationState {
    /// Maps (ModuleId, FunId) to operations for function parameters.
    /// Each parameter's TempIndex is mapped to its NumOperation.
    /// Seeded from `pragma bv` annotations, then refined by dataflow analysis.
    temp_index_operation_map: FuncOperationMap,

    /// Maps (ModuleId, FunId) to operations for return values.
    /// Each return position is mapped to its NumOperation.
    /// Seeded from `pragma bv_ret` annotations, then refined by dataflow analysis.
    ret_operation_map: FuncOperationMap,

    /// Maps (ModuleId, FunId) to operations for local variables (non-parameters).
    /// Used for verification variant processing.
    local_oper: FuncOperationMap,

    /// Same as local_oper, but for baseline variant of functions.
    /// Baseline and verification variants may have different inferred types.
    local_oper_baseline: FuncOperationMap,

    /// Maps AST node IDs to NumOperation for spec expressions.
    /// Public because it's accessed during spec translation.
    pub exp_operation_map: ExpMap,

    /// Maps (ModuleId, SpecFunId) to (parameter operations, return operations).
    /// Tracks operations for specification functions.
    pub spec_fun_operation_map: SpecFuncOperationMap,

    /// Maps (ModuleId, StructId) to field operations for that struct.
    /// Seeded from `pragma bv` on struct specs, then refined by pack/unpack analysis.
    pub struct_operation_map: StructOperationMap,
}

impl GlobalNumberOperationState {
    /// Parse pragma bv=b"..." or pragma bv_ret=b"..." from function or struct specs.
    /// Returns a list of positions (0-indexed) that should use bitvector representation.
    ///
    /// # Examples
    /// - `pragma bv = b"0,2"` → returns [0, 2] (parameters at positions 0 and 2 use bv types)
    /// - `pragma bv_ret = b"1"` → returns [1] (return value at position 1 uses bv type)
    /// - No pragma → returns [] (all positions use default int types)
    fn extract_bv_vars(bv_temp_opt: Option<&PropertyValue>) -> Vec<usize> {
        let mut bv_temp_vec = vec![];
        if let Some(PropertyValue::Value(Value::ByteArray(arr))) = bv_temp_opt {
            let param_str = str::from_utf8(arr).expect(PARSING_ERROR);
            let idx_vec = param_str
                .split(',')
                .map(|s| s.trim().parse::<usize>().expect(PARSING_ERROR))
                .collect_vec();
            bv_temp_vec = idx_vec;
        }
        bv_temp_vec
    }

    /// Helper to populate an operation map with Bitwise for specified indices, Bottom for others.
    fn populate_operation_map(count: usize, bv_indices: &[usize]) -> OperationMap {
        use NumOperation::*;
        (0..count)
            .map(|i| {
                let oper = if bv_indices.contains(&i) {
                    Bitwise
                } else {
                    Bottom
                };
                (i, oper)
            })
            .collect()
    }

    pub fn get_ret_map(&self) -> &FuncOperationMap {
        &self.ret_operation_map
    }

    pub fn get_mut_ret_map(&mut self) -> &mut FuncOperationMap {
        &mut self.ret_operation_map
    }

    /// Helper to select the correct local operation map based on baseline flag.
    fn select_local_map(&self, baseline_flag: bool) -> &FuncOperationMap {
        if baseline_flag {
            &self.local_oper_baseline
        } else {
            &self.local_oper
        }
    }

    /// Helper to select the correct mutable local operation map based on baseline flag.
    fn select_local_map_mut(&mut self, baseline_flag: bool) -> &mut FuncOperationMap {
        if baseline_flag {
            &mut self.local_oper_baseline
        } else {
            &mut self.local_oper
        }
    }

    pub fn get_non_param_local_map(
        &self,
        mid: ModuleId,
        fid: FunId,
        baseline_flag: bool,
    ) -> &OperationMap {
        self.select_local_map(baseline_flag)
            .get(&(mid, fid))
            .unwrap()
    }

    pub fn get_mut_non_param_local_map(
        &mut self,
        mid: ModuleId,
        fid: FunId,
        baseline_flag: bool,
    ) -> &mut OperationMap {
        self.select_local_map_mut(baseline_flag)
            .get_mut(&(mid, fid))
            .unwrap()
    }

    pub fn get_temp_index_oper(
        &self,
        mid: ModuleId,
        fid: FunId,
        idx: TempIndex,
        baseline_flag: bool,
    ) -> Option<&NumOperation> {
        let func_key = (mid, fid);
        let local_map = self.select_local_map(baseline_flag).get(&func_key).unwrap();

        // Check locals first, then fall back to parameters
        if local_map.contains_key(&idx) {
            local_map.get(&idx)
        } else {
            self.temp_index_operation_map
                .get(&func_key)
                .unwrap()
                .get(&idx)
        }
    }

    pub fn get_mut_temp_index_oper(
        &mut self,
        mid: ModuleId,
        fid: FunId,
        idx: TempIndex,
        baseline_flag: bool,
    ) -> Option<&mut NumOperation> {
        let func_key = (mid, fid);

        // Check locals first, then fall back to parameters
        // Need to check containment first to avoid multiple mutable borrows
        let is_in_locals = self
            .select_local_map(baseline_flag)
            .get(&func_key)
            .unwrap()
            .contains_key(&idx);

        if is_in_locals {
            self.select_local_map_mut(baseline_flag)
                .get_mut(&func_key)
                .unwrap()
                .get_mut(&idx)
        } else {
            self.temp_index_operation_map
                .get_mut(&func_key)
                .unwrap()
                .get_mut(&idx)
        }
    }

    /// Create the initial NumOperation state for a function.
    /// This seeds the analysis with explicit `pragma bv` and `pragma bv_ret` annotations.
    /// - Parameters marked in pragma bv are set to Bitwise
    /// - Return values marked in pragma bv_ret are set to Bitwise
    /// - All other parameters/returns are set to Bottom (to be inferred)
    pub fn create_initial_func_oper_state(&mut self, func_env: &FunctionEnv) {
        let spec = func_env.get_spec();
        let spec = spec.deref();
        let symbol_pool = func_env.module_env.env.symbol_pool();

        // Extract pragma annotations
        let para_idx_vec =
            Self::extract_bv_vars(spec.properties.get(&symbol_pool.make(BV_PARAM_PROP)));
        let ret_idx_vec =
            Self::extract_bv_vars(spec.properties.get(&symbol_pool.make(BV_RET_PROP)));

        // Populate operation maps using the helper
        let param_map = Self::populate_operation_map(func_env.get_parameter_count(), &para_idx_vec);
        let ret_map = Self::populate_operation_map(func_env.get_return_count(), &ret_idx_vec);

        let func_key = (func_env.module_env.get_id(), func_env.get_id());
        self.temp_index_operation_map.insert(func_key, param_map);
        self.ret_operation_map.insert(func_key, ret_map);
        self.local_oper_baseline.insert(func_key, BTreeMap::new());
        self.local_oper.insert(func_key, BTreeMap::new());
    }

    /// Create the initial NumOperation state for a struct.
    /// This seeds the analysis with explicit `pragma bv` annotations on struct specs.
    /// - Fields marked in pragma bv are set to Bitwise
    /// - All other fields are set to Bottom (to be inferred)
    /// - For enum types, pragma bv is currently not supported and will be ignored with a warning
    pub fn create_initial_struct_oper_state(&mut self, struct_env: &StructEnv) {
        use NumOperation::*;

        // Obtain positions that are marked as Bitwise by analyzing the pragma
        let para_sym = &struct_env.module_env.env.symbol_pool().make(BV_PARAM_PROP);
        let struct_spec = struct_env.get_spec();
        let bv_struct_opt = struct_spec.properties.get(para_sym);
        let field_idx_vec = Self::extract_bv_vars(bv_struct_opt);

        let mid = struct_env.module_env.get_id();
        let sid = struct_env.get_id();
        let struct_env = struct_env.module_env.env.get_module(mid).into_struct(sid);
        let mut field_oper_map = BTreeMap::new();

        let update_field_map =
            |field_id: FieldId, field_oper_map: &mut BTreeMap<FieldId, NumOperation>| {
                field_oper_map.insert(field_id, Bottom);
            };

        if !struct_env.has_variants() {
            for (i, field) in struct_env.get_fields().enumerate() {
                if field_idx_vec.contains(&i) {
                    field_oper_map.insert(field.get_id(), Bitwise);
                } else {
                    update_field_map(field.get_id(), &mut field_oper_map);
                }
            }
            self.struct_operation_map.insert((mid, sid), field_oper_map);
        } else {
            if !field_idx_vec.is_empty() {
                let loc = if let Some(loc) = &struct_env.get_spec().loc {
                    loc.clone()
                } else {
                    struct_env.get_loc()
                };
                // enum does support "pragma bv"
                struct_env.module_env.env.warning(
                    &loc,
                    "pragma bv is currently not support in enum types and will be ignored",
                );
            }
            for variant in struct_env.get_variants() {
                for field in struct_env.get_fields_of_variant(variant) {
                    let pool = struct_env.symbol_pool();
                    let new_field_id =
                        FieldId::new(pool.make(&FieldId::make_variant_field_id_str(
                            pool.string(variant).as_str(),
                            pool.string(field.get_name()).as_str(),
                        )));
                    update_field_map(new_field_id, &mut field_oper_map);
                }
            }
            self.struct_operation_map.insert((mid, sid), field_oper_map);
        }
    }

    /// Updates the NumOperation for the given AST node ID.
    ///
    /// # Parameters
    /// - `node_id`: The AST node to update
    /// - `num_oper`: The new NumOperation to assign
    /// - `allow`: If true, always update even if it conflicts; if false, return false on conflict
    ///
    /// # Returns
    /// - `true` if the update succeeded
    /// - `false` if there was a conflict and `allow` was false
    pub fn update_node_oper(
        &mut self,
        node_id: NodeId,
        num_oper: NumOperation,
        allow: bool,
    ) -> bool {
        let mods = &mut self.exp_operation_map;
        let oper = mods.get_mut(&node_id).expect("node exist");
        if !allow && oper.conflict(&num_oper) {
            false
        } else {
            *oper = num_oper;
            true
        }
    }

    pub fn get_num_operation_field(
        &self,
        mid: &ModuleId,
        sid: &StructId,
        field_id: &FieldId,
    ) -> &NumOperation {
        self.struct_operation_map
            .get(&(*mid, *sid))
            .expect("struct must have a struct operation state")
            .get(field_id)
            .expect("expect to get the state")
    }

    /// Gets the number operation of the given node.
    pub fn get_node_num_oper(&self, node_id: NodeId) -> NumOperation {
        self.get_node_num_oper_opt(node_id)
            .expect("node number oper defined")
    }

    /// Gets the number operation of the given node, if available.
    pub fn get_node_num_oper_opt(&self, node_id: NodeId) -> Option<NumOperation> {
        self.exp_operation_map.get(&node_id).copied()
    }

    pub fn update_spec_ret(&mut self, mid: &ModuleId, fid: &SpecFunId, oper: NumOperation) {
        let ret_num_oper_vec = &mut self
            .spec_fun_operation_map
            .get_mut(&(*mid, *fid))
            .unwrap()
            .1;
        ret_num_oper_vec[0] = oper;
    }
}
