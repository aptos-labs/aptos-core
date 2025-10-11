//! Assignment Tracker
//!
//! This module provides value equivalence analysis for Move bytecode temporary variables.
//! Its only purpose is to determine when two temporary variables hold the same logical value
//! by tracking assignment chains, field borrows, and constant propagation.
//!
//! The `AssignmentTracker` processes bytecode instructions sequentially to build a provenance
//! chain for each temporary, then uses this information to answer equivalence queries via
//! the `are_equivalent()` method.
//!
//! **Limitations:**
//! - Single-pass analysis without control flow sensitivity
//! - Designed for straight-line code analysis within basic blocks

use std::collections::{HashMap, HashSet};

use move_model::{
    ast::TempIndex,
    model::{ModuleId, StructId},
    symbol::Symbol,
};
use move_stackless_bytecode::stackless_bytecode::{Bytecode, Constant, Operation};

/// Represents information about a field access
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct FieldInfo {
    module_id: ModuleId,
    struct_id: StructId,
    variant_path: Option<Vec<Symbol>>,
    field_offset: usize,
}

/// Represents a constant value that can be compared across temps
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum ConstantValue {
    U8(u8),
    U16(u16),
    U32(u32),
    U64(u64),
    U128(u128),
    Bool(bool),
    Vector(Vec<u8>),
}

impl ConstantValue {
    /// Try to extract a constant value from a Move Constant
    fn from_move_constant(constant: &Constant) -> Option<Self> {
        match constant {
            Constant::Bool(b) => Some(ConstantValue::Bool(*b)),
            Constant::U8(n) => Some(ConstantValue::U8(*n)),
            Constant::U16(n) => Some(ConstantValue::U16(*n)),
            Constant::U32(n) => Some(ConstantValue::U32(*n)),
            Constant::U64(n) => Some(ConstantValue::U64(*n)),
            Constant::U128(n) => Some(ConstantValue::U128(*n)),
            Constant::ByteArray(bytes) => Some(ConstantValue::Vector(bytes.clone())),
            _ => None,
        }
    }
}

/// Represents the source of a temporary variable
#[derive(Debug, Clone, PartialEq)]
enum TempSource {
    /// Direct assignment from another temp
    Assignment(TempIndex),
    /// Field borrow: base_temp.field
    FieldBorrow(TempIndex, FieldInfo),
    /// Local borrow: &local_temp
    LocalBorrow(TempIndex),
    /// Constant value loaded into temp
    Constant(ConstantValue),
    /// Unknown or initial source
    Unknown,
}

/// Tracks assignments and references across control flow
#[derive(Debug, Clone)]
pub struct AssignmentTracker {
    temp_sources: HashMap<TempIndex, TempSource>,
}

impl AssignmentTracker {
    pub fn new() -> Self {
        Self {
            temp_sources: HashMap::new(),
        }
    }

    /// Process a bytecode instruction and update tracking state
    pub fn process_bytecode(&mut self, bytecode: &Bytecode) {
        match bytecode {
            // Track direct assignments
            Bytecode::Assign(_, dest, src, _) => {
                self.set_temp_source(*dest, TempSource::Assignment(*src))
            },
            // Constants loaded into temps
            Bytecode::Load(_, dest, constant) => {
                let source = if let Some(const_value) = ConstantValue::from_move_constant(constant)
                {
                    TempSource::Constant(const_value)
                } else {
                    TempSource::Unknown
                };
                self.set_temp_source(*dest, source);
            },
            // Track field borrows
            Bytecode::Call(_, dests, operation, srcs, _) => {
                match operation {
                    Operation::Unpack(mid, sid, _) => {
                        if let Some(&src) = srcs.first() {
                            for (i, &dest) in dests.iter().enumerate() {
                                let field_info = FieldInfo {
                                    module_id: *mid,
                                    struct_id: *sid,
                                    variant_path: None,
                                    field_offset: i,
                                };
                                self.set_temp_source(
                                    dest,
                                    TempSource::FieldBorrow(src, field_info),
                                );
                            }
                        }
                    },
                    Operation::ReadRef => {
                        if let (Some(&dest), Some(&src)) = (dests.first(), srcs.first()) {
                            self.set_temp_source(dest, TempSource::Assignment(src));
                        }
                    },
                    Operation::BorrowField(mid, sid, _, field_offset) => {
                        if let (Some(&dest), Some(&src)) = (dests.first(), srcs.first()) {
                            let field_info = FieldInfo {
                                module_id: *mid,
                                struct_id: *sid,
                                variant_path: None,
                                field_offset: *field_offset,
                            };
                            self.set_temp_source(dest, TempSource::FieldBorrow(src, field_info));
                        }
                    },
                    Operation::BorrowVariantField(mid, sid, variant_path, _, field_offset) => {
                        if let (Some(&dest), Some(&src)) = (dests.first(), srcs.first()) {
                            let field_info = FieldInfo {
                                module_id: *mid,
                                struct_id: *sid,
                                variant_path: Some(variant_path.clone()),
                                field_offset: *field_offset,
                            };
                            self.set_temp_source(dest, TempSource::FieldBorrow(src, field_info));
                        }
                    },
                    Operation::BorrowLoc => {
                        if let (Some(&dest), Some(&src)) = (dests.first(), srcs.first()) {
                            self.set_temp_source(dest, TempSource::LocalBorrow(src));
                        }
                    },
                    _ => {
                        // Function calls create new values in destination temps
                        for &dest in dests {
                            if !self.temp_sources.contains_key(&dest) {
                                self.set_temp_source(dest, TempSource::Unknown);
                            }
                        }
                    },
                }
            },
            _ => {},
        }
    }

    fn set_temp_source(&mut self, temp: TempIndex, source: TempSource) {
        self.temp_sources.insert(temp, source);
    }

    fn find_root_source(&self, temp: TempIndex) -> (TempIndex, Vec<TempSource>) {
        let mut current = temp;
        let mut path = Vec::new();
        let mut visited = HashSet::new();

        while !visited.contains(&current) {
            visited.insert(current);

            if let Some(source) = self.temp_sources.get(&current) {
                path.push(source.clone());
                current = match source {
                    TempSource::Assignment(src)
                    | TempSource::LocalBorrow(src)
                    | TempSource::FieldBorrow(src, _) => *src,
                    TempSource::Constant(_) | TempSource::Unknown => break,
                };
            } else {
                break;
            }
        }

        (current, path)
    }

    fn get_constant_value(&self, temp: TempIndex) -> Option<ConstantValue> {
        let (_, path) = self.find_root_source(temp);
        path.iter().find_map(|source| {
            if let TempSource::Constant(const_val) = source {
                Some(const_val.clone())
            } else {
                None
            }
        })
    }

    /// Check if two temps refer to the same logical value
    pub fn are_equivalent(&self, temp1: TempIndex, temp2: TempIndex) -> bool {
        if temp1 == temp2 {
            return true;
        }

        // Check if both temps ultimately hold the same constant value
        if let (Some(const1), Some(const2)) = (
            self.get_constant_value(temp1),
            self.get_constant_value(temp2),
        ) {
            return const1 == const2;
        }

        // Check if they have the same root and field access
        let (root1, path1) = self.find_root_source(temp1);
        let (root2, path2) = self.find_root_source(temp2);

        root1 == root2 && self.extract_field_info(&path1) == self.extract_field_info(&path2)
    }

    /// Extract field information from the last field access in a path
    fn extract_field_info(&self, path: &[TempSource]) -> Option<FieldInfo> {
        for source in path.iter().rev() {
            if let TempSource::FieldBorrow(_, field_info) = source {
                return Some(field_info.clone());
            }
        }
        None
    }
}
