// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! Flow-sensitive temp equivalence analyzer for stackless bytecode temporaries.

use std::collections::{BTreeMap, BTreeSet};

use move_binary_format::file_format::CodeOffset;
use move_model::{
    ast::{Address, TempIndex},
    symbol::Symbol,
};
use move_stackless_bytecode::{
    dataflow_analysis::{DataflowAnalysis, StateMap, TransferFunctions},
    dataflow_domains::{AbstractDomain, JoinResult},
    stackless_bytecode::{Bytecode, Constant, Operation},
    stackless_control_flow_graph::StacklessControlFlowGraph,
};

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct FieldInfo {
    pub module_id: move_model::model::ModuleId,
    pub struct_id: move_model::model::StructId,
    pub variant_path: Option<Vec<Symbol>>,
    pub field_offset: usize,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum ConstantValue {
    U8(u8),
    U16(u16),
    U32(u32),
    U64(u64),
    U128(u128),
    Bool(bool),
    Vector(Vec<u8>),
    Address(Address),
}

impl ConstantValue {
    fn from_constant(constant: &Constant) -> Option<Self> {
        match constant {
            Constant::Bool(b) => Some(ConstantValue::Bool(*b)),
            Constant::U8(n) => Some(ConstantValue::U8(*n)),
            Constant::U16(n) => Some(ConstantValue::U16(*n)),
            Constant::U32(n) => Some(ConstantValue::U32(*n)),
            Constant::U64(n) => Some(ConstantValue::U64(*n)),
            Constant::U128(n) => Some(ConstantValue::U128(*n)),
            Constant::ByteArray(bytes) => Some(ConstantValue::Vector(bytes.clone())),
            Constant::Address(addr) => Some(ConstantValue::Address(addr.clone())),
            _ => None,
        }
    }

    pub fn is_zero_address(&self) -> bool {
        match self {
            ConstantValue::Address(Address::Numerical(account)) => {
                account.as_ref().iter().all(|byte| *byte == 0)
            },
            ConstantValue::Address(Address::Symbolic(_)) => false,
            _ => false,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct FieldReference {
    base_class: BTreeSet<TempIndex>,
    info: FieldInfo,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct TempEquivalenceState {
    // Each temp maps to its equivalence class
    classes: BTreeMap<TempIndex, BTreeSet<TempIndex>>,
    // Constant values known for temps
    constants: BTreeMap<TempIndex, ConstantValue>,
    // Field reference information
    field_refs: BTreeMap<TempIndex, FieldReference>,
}

impl TempEquivalenceState {
    pub fn equivalence_class(&self, temp: TempIndex) -> BTreeSet<TempIndex> {
        self.class_of(temp)
    }

    pub fn constant_for(&self, temp: TempIndex) -> Option<&ConstantValue> {
        self.constants.get(&temp)
    }

    fn class_of(&self, temp: TempIndex) -> BTreeSet<TempIndex> {
        self.classes.get(&temp).cloned().unwrap_or_else(|| {
            let mut set = BTreeSet::new();
            set.insert(temp);
            set
        })
    }

    fn set_class(&mut self, members: &BTreeSet<TempIndex>) {
        for &member in members {
            self.classes.insert(member, members.clone());
        }
    }

    fn isolate(&mut self, temp: TempIndex) {
        let current_class = self.class_of(temp);

        // Remove temp from its current class
        if current_class.len() > 1 {
            let remaining: BTreeSet<_> = current_class
                .iter()
                .filter(|&&t| t != temp)
                .copied()
                .collect();
            self.set_class(&remaining);
        }

        // Make temp standalone
        let mut standalone = BTreeSet::new();
        standalone.insert(temp);
        self.classes.insert(temp, standalone);

        self.constants.remove(&temp);
        self.field_refs.remove(&temp);
    }

    fn merge(&mut self, dest: TempIndex, src: TempIndex) {
        let dest_class = self.class_of(dest);
        let src_class = self.class_of(src);

        let combined: BTreeSet<_> = dest_class.union(&src_class).copied().collect();
        self.set_class(&combined);

        // Propagate properties to entire combined class
        if let Some(const_val) = self.constants.get(&src).cloned() {
            for &member in &combined {
                self.constants.insert(member, const_val.clone());
            }
        }

        if let Some(field_ref) = self.field_refs.get(&src).cloned() {
            for &member in &combined {
                self.field_refs.insert(member, field_ref.clone());
            }
        }
    }

    fn set_constant(&mut self, temp: TempIndex, value: ConstantValue) {
        let class = self.class_of(temp);
        for &member in &class {
            self.constants.insert(member, value.clone());
        }
    }

    fn set_field_ref(&mut self, temp: TempIndex, base: TempIndex, info: FieldInfo) {
        let temp_class = self.class_of(temp);
        let base_class = self.class_of(base);
        let field_ref = FieldReference { base_class, info };

        // Update entire equivalence class
        for &member in &temp_class {
            self.field_refs.insert(member, field_ref.clone());
        }
    }

    pub fn are_equivalent(&self, left: TempIndex, right: TempIndex) -> bool {
        let mut visited = BTreeSet::new();
        self.are_equivalent_internal(left, right, &mut visited)
    }

    fn are_equivalent_internal(
        &self,
        left: TempIndex,
        right: TempIndex,
        visited: &mut BTreeSet<(TempIndex, TempIndex)>,
    ) -> bool {
        if left == right {
            return true;
        }

        // Guard against cycles by tracking visited pairs
        let pair = if left <= right {
            (left, right)
        } else {
            (right, left)
        };
        if !visited.insert(pair) {
            return false;
        }

        // Check if in same equivalence class
        if self.class_of(left).contains(&right) {
            return true;
        }

        // Check if both have same constant value
        if let (Some(a), Some(b)) = (self.constants.get(&left), self.constants.get(&right)) {
            if a == b {
                return true;
            }
        }

        // Check if both reference the same field of equivalent bases
        if let (Some(l_field), Some(r_field)) =
            (self.field_refs.get(&left), self.field_refs.get(&right))
        {
            if l_field.info == r_field.info {
                // Check if any base in left's class is equivalent to any base in right's class
                for &base_left in &l_field.base_class {
                    for &base_right in &r_field.base_class {
                        if self.are_equivalent_internal(base_left, base_right, visited) {
                            return true;
                        }
                    }
                }
            }
        }

        false
    }

    fn all_temps(&self) -> BTreeSet<TempIndex> {
        let mut temps = BTreeSet::new();
        for (&temp, class) in &self.classes {
            temps.insert(temp);
            temps.extend(class);
        }
        temps.extend(self.constants.keys());
        for (&temp, field_ref) in &self.field_refs {
            temps.insert(temp);
            temps.extend(&field_ref.base_class);
        }
        temps
    }

    // Normalize base class to canonical representative
    fn normalize_base_class(&self, base_class: &BTreeSet<TempIndex>) -> BTreeSet<TempIndex> {
        if base_class.is_empty() {
            return BTreeSet::new();
        }
        // Use the class of any member as the canonical representative
        self.class_of(*base_class.iter().next().unwrap())
    }
}

impl AbstractDomain for TempEquivalenceState {
    fn join(&mut self, other: &Self) -> JoinResult {
        let old_state = self.clone();

        // Compute new equivalence classes by intersecting
        let all_temps = self
            .all_temps()
            .union(&other.all_temps())
            .copied()
            .collect::<BTreeSet<_>>();
        let mut new_classes: BTreeMap<TempIndex, BTreeSet<TempIndex>> = BTreeMap::new();

        for &temp in &all_temps {
            let self_class = self.class_of(temp);
            let other_class = other.class_of(temp);
            let intersection: BTreeSet<_> =
                self_class.intersection(&other_class).copied().collect();

            let new_class = if intersection.is_empty() {
                let mut singleton = BTreeSet::new();
                singleton.insert(temp);
                singleton
            } else {
                intersection
            };

            new_classes.insert(temp, new_class);
        }

        // Group temps by their equivalence classes
        let mut unique_classes: Vec<BTreeSet<TempIndex>> = Vec::new();
        for class in new_classes.values() {
            if !unique_classes.contains(class) {
                unique_classes.push(class.clone());
            }
        }

        // Rebuild class map
        self.classes.clear();
        for class in &unique_classes {
            self.set_class(class);
        }

        // Join constants: keep only if consistent across all members of a class
        let mut new_constants = BTreeMap::new();
        for class in &unique_classes {
            if let Some(value) =
                Self::join_constants_for_class(class, &old_state.constants, &other.constants)
            {
                for &member in class {
                    new_constants.insert(member, value.clone());
                }
            }
        }
        self.constants = new_constants;

        // Join field refs: keep only if consistent across all members
        let mut new_field_refs = BTreeMap::new();
        for class in &unique_classes {
            if let Some(field_ref) =
                self.join_field_refs_for_class(class, &old_state.field_refs, &other.field_refs)
            {
                for &member in class {
                    new_field_refs.insert(member, field_ref.clone());
                }
            }
        }
        self.field_refs = new_field_refs;

        if *self == old_state {
            JoinResult::Unchanged
        } else {
            JoinResult::Changed
        }
    }
}

impl TempEquivalenceState {
    fn join_constants_for_class(
        class: &BTreeSet<TempIndex>,
        self_constants: &BTreeMap<TempIndex, ConstantValue>,
        other_constants: &BTreeMap<TempIndex, ConstantValue>,
    ) -> Option<ConstantValue> {
        let mut result: Option<ConstantValue> = None;

        for &member in class {
            match (self_constants.get(&member), other_constants.get(&member)) {
                (Some(a), Some(b)) if a == b => match &result {
                    Some(existing) if existing != a => return None,
                    None => result = Some(a.clone()),
                    _ => {},
                },
                (None, None) => {},
                _ => return None,
            }
        }

        result
    }

    // Properly normalize and intersect base classes
    fn join_field_refs_for_class(
        &self,
        class: &BTreeSet<TempIndex>,
        self_field_refs: &BTreeMap<TempIndex, FieldReference>,
        other_field_refs: &BTreeMap<TempIndex, FieldReference>,
    ) -> Option<FieldReference> {
        let mut result: Option<FieldReference> = None;

        for &member in class {
            match (self_field_refs.get(&member), other_field_refs.get(&member)) {
                (Some(self_ref), Some(other_ref)) if self_ref.info == other_ref.info => {
                    // Normalize both base classes before comparing
                    let self_base_norm = self.normalize_base_class(&self_ref.base_class);
                    let other_base_norm = self.normalize_base_class(&other_ref.base_class);

                    // Intersect normalized base classes
                    let intersected_bases: BTreeSet<_> = self_base_norm
                        .intersection(&other_base_norm)
                        .copied()
                        .collect();

                    if intersected_bases.is_empty() {
                        return None;
                    }

                    let candidate = FieldReference {
                        base_class: intersected_bases,
                        info: self_ref.info.clone(),
                    };

                    match &result {
                        // Compare after normalizing
                        Some(existing) => {
                            let existing_norm = FieldReference {
                                base_class: self.normalize_base_class(&existing.base_class),
                                info: existing.info.clone(),
                            };
                            let candidate_norm = FieldReference {
                                base_class: self.normalize_base_class(&candidate.base_class),
                                info: candidate.info.clone(),
                            };
                            if existing_norm != candidate_norm {
                                return None;
                            }
                        },
                        None => result = Some(candidate),
                    }
                },
                (None, None) => {},
                _ => return None,
            }
        }

        result
    }
}

pub struct TempEquivalenceAnalyzer;

impl TempEquivalenceAnalyzer {
    pub fn analyze_function(
        &self,
        code: &[Bytecode],
        cfg: &StacklessControlFlowGraph,
    ) -> StateMap<TempEquivalenceState> {
        DataflowAnalysis::analyze_function(self, TempEquivalenceState::default(), code, cfg)
    }

    pub fn state_at_each_instruction(
        &self,
        code: &[Bytecode],
        cfg: &StacklessControlFlowGraph,
    ) -> BTreeMap<CodeOffset, TempEquivalenceState> {
        let state_map = self.analyze_function(code, cfg);
        self.state_per_instruction(state_map, code, cfg, |pre, _post| pre.clone())
    }
}

impl TransferFunctions for TempEquivalenceAnalyzer {
    type State = TempEquivalenceState;
    const BACKWARD: bool = false;

    fn execute(&self, state: &mut Self::State, instr: &Bytecode, _offset: CodeOffset) {
        match instr {
            Bytecode::Assign(_, dest, src, _) => {
                state.isolate(*dest);
                state.merge(*dest, *src);
            },

            Bytecode::Load(_, dest, constant) => {
                state.isolate(*dest);
                if let Some(value) = ConstantValue::from_constant(constant) {
                    state.set_constant(*dest, value);
                }
            },

            Bytecode::Call(_, dests, operation, srcs, _) => {
                match operation {
                    Operation::Unpack(mid, sid, _) => {
                        if let Some(&base) = srcs.first() {
                            for (field_offset, &dest) in dests.iter().enumerate() {
                                state.isolate(dest);
                                state.set_field_ref(
                                    dest,
                                    base,
                                    FieldInfo {
                                        module_id: *mid,
                                        struct_id: *sid,
                                        variant_path: None,
                                        field_offset,
                                    },
                                );
                            }
                            return;
                        }
                    },

                    Operation::BorrowField(mid, sid, _, field_offset) => {
                        if let (Some(&dest), Some(&base)) = (dests.first(), srcs.first()) {
                            state.isolate(dest);
                            state.set_field_ref(
                                dest,
                                base,
                                FieldInfo {
                                    module_id: *mid,
                                    struct_id: *sid,
                                    variant_path: None,
                                    field_offset: *field_offset,
                                },
                            );
                            return;
                        }
                    },

                    Operation::BorrowVariantField(mid, sid, variants, _, field_offset) => {
                        if let (Some(&dest), Some(&base)) = (dests.first(), srcs.first()) {
                            state.isolate(dest);
                            state.set_field_ref(
                                dest,
                                base,
                                FieldInfo {
                                    module_id: *mid,
                                    struct_id: *sid,
                                    variant_path: Some(variants.clone()),
                                    field_offset: *field_offset,
                                },
                            );
                            return;
                        }
                    },

                    Operation::BorrowLoc | Operation::ReadRef | Operation::FreezeRef(_) => {
                        if let (Some(&dest), Some(&src)) = (dests.first(), srcs.first()) {
                            state.isolate(dest);
                            state.merge(dest, src);
                            return;
                        }
                    },

                    _ => {},
                }

                // Default: kill all destinations
                for &dest in dests {
                    state.isolate(dest);
                }
            },
            _ => {},
        }
    }
}

impl DataflowAnalysis for TempEquivalenceAnalyzer {}
