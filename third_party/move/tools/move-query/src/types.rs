// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! Serializable types for Move model queries.
//!
//! ```text
//! Package ─┬─ Dependency* ── module names
//!          └─ module names
//!
//! Module ──┬─ Function*
//!          ├─ Struct* ── Field* | Variant*
//!          └─ Constant*
//! ```

use serde::{Deserialize, Serialize};

// ========================================
// Package Level
// ========================================

/// Move package with dependencies and target modules.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Package {
    pub name: String,
    pub path: String,
    pub manifest_path: String,
    pub dependencies: Vec<Dependency>,
    /// Module full names (e.g., "0x1::coin").
    pub modules: Vec<String>,
}

/// Dependency package.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Dependency {
    pub name: String,
    pub path: String,
    /// Module full names (e.g., "0x1::coin").
    pub modules: Vec<String>,
}

// ========================================
// Module Level
// ========================================

/// Move module with relationships and item names.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Module {
    pub address: String,
    pub name: String,
    pub doc: String,
    pub location: Location,
    pub is_target: bool,
    pub friends: Vec<String>,
    pub uses: Vec<String>,
    pub used_by: Vec<String>,
    pub functions: Vec<String>,
    pub structs: Vec<String>,
    pub constants: Vec<String>,
}

// ========================================
// Item Level
// ========================================

/// Move function.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Function {
    /// Module full name (e.g., "0x1::coin").
    pub module: String,
    pub name: String,
    pub signature: String,
    pub doc: String,
    pub location: Location,
    pub visibility: String,
    pub is_entry: bool,
    pub is_view: bool,
    pub is_inline: bool,
    pub is_native: bool,
    pub attributes: Vec<String>,
    pub acquires: Vec<String>,
    /// Functions that this function calls (full names, e.g., "0x1::coin::transfer").
    pub callees: Vec<String>,
    /// Functions that call this function (full names, e.g., "0x1::coin::transfer").
    pub callers: Vec<String>,
}

/// Move struct or enum.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Struct {
    /// Module full name (e.g., "0x1::coin").
    pub module: String,
    pub name: String,
    pub doc: String,
    pub location: Location,
    pub abilities: Vec<String>,
    pub is_resource: bool,
    pub is_native: bool,
    pub is_enum: bool,
    pub attributes: Vec<String>,
    pub type_parameters: Vec<String>,
    /// None for enums.
    pub fields: Option<Vec<Field>>,
    /// None for structs.
    pub variants: Option<Vec<Variant>>,
}

/// Enum variant.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Variant {
    pub name: String,
    pub fields: Vec<Field>,
}

/// Move constant.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Constant {
    /// Module full name (e.g., "0x1::coin").
    pub module: String,
    pub name: String,
    pub doc: String,
    pub location: Location,
    #[serde(rename = "type")]
    pub type_: String,
    pub value: Option<String>,
}

// ========================================
// Nested Types
// ========================================

/// Struct or variant field.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Field {
    pub name: String,
    #[serde(rename = "type")]
    pub type_: String,
}

/// Source location.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Location {
    pub file: String,
    pub start_line: u32,
    pub start_column: u32,
    pub end_line: u32,
    pub end_column: u32,
}
