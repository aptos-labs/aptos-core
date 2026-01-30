// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! Data types for Move query tool
//!
//! This module defines all data structures used by the query tool, following
//! the Move model hierarchy: Package → Module → {Function, Struct, Constant}

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

// =============================================================================
// Level 0: Package
// =============================================================================

/// Package-level information (root of hierarchy)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageInfo {
    /// Package name from Move.toml
    pub name: String,
    /// Package directory path
    pub path: String,
    /// Move.toml location
    pub manifest_path: String,
    /// Named addresses defined in the package
    pub named_addresses: BTreeMap<String, String>,
    /// Package dependencies
    pub dependencies: Vec<String>,
    /// Module summaries (not full details)
    pub modules: Vec<ModuleSummary>,
}

/// Summary of a module (used in package listing)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleSummary {
    /// Full module identifier (e.g., "0xCAFE::liquidity_pool")
    pub id: String,
    /// Simple module name (e.g., "liquidity_pool")
    pub name: String,
    /// Whether this is a compilation target (vs dependency)
    pub is_target: bool,
    /// Source location of module declaration
    pub location: Location,
    /// Number of functions in the module
    pub function_count: usize,
    /// Number of structs in the module
    pub struct_count: usize,
    /// Number of constants in the module
    pub constant_count: usize,
}

// =============================================================================
// Level 1: Module
// =============================================================================

/// Full module information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleInfo {
    /// Full module identifier (e.g., "0xCAFE::liquidity_pool")
    pub id: String,
    /// Module address
    pub address: String,
    /// Simple module name
    pub name: String,
    /// Documentation comment
    pub doc: String,
    /// Source location of module declaration
    pub location: Location,
    /// Whether this is a compilation target (vs dependency)
    pub is_target: bool,

    // Relationships
    /// Friend modules
    pub friends: Vec<String>,
    /// Modules this module uses (imports)
    pub uses: Vec<String>,
    /// Modules that use this module
    pub used_by: Vec<String>,

    // Children (summaries, not full details)
    /// Function summaries
    pub functions: Vec<FunctionSummary>,
    /// Struct summaries
    pub structs: Vec<StructSummary>,
    /// Constant summaries
    pub constants: Vec<ConstantSummary>,
}

/// Summary of a function (used in module listing)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionSummary {
    /// Function name
    pub name: String,
    /// Module containing this function
    pub module: String,
    /// Function visibility
    pub visibility: Visibility,
    /// Whether this is an entry function
    pub is_entry: bool,
    /// Whether this is a view function
    pub is_view: bool,
    /// Source location of function
    pub location: Location,
    /// Number of type parameters
    pub type_parameter_count: usize,
    /// Number of parameters
    pub parameter_count: usize,
    /// Human-readable signature
    pub signature: String,
}

/// Summary of a struct (used in module listing)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StructSummary {
    /// Struct name
    pub name: String,
    /// Module containing this struct
    pub module: String,
    /// Struct abilities
    pub abilities: Vec<Ability>,
    /// Whether this is a resource (has key ability)
    pub is_resource: bool,
    /// Source location of struct
    pub location: Location,
    /// Number of type parameters
    pub type_parameter_count: usize,
    /// Number of fields
    pub field_count: usize,
}

/// Summary of a constant (used in module listing)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConstantSummary {
    /// Constant name
    pub name: String,
    /// Module containing this constant
    pub module: String,
    /// Source location of constant
    pub location: Location,
    /// Simplified type string
    #[serde(rename = "type")]
    pub type_: String,
    /// Constant value (if available)
    pub value: Option<String>,
}

// =============================================================================
// Level 2: Full Details
// =============================================================================

/// Full function information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionInfo {
    // Identity
    /// Module containing this function
    pub module: String,
    /// Function name
    pub name: String,
    /// Documentation comment
    pub doc: String,
    /// Source location
    pub location: Location,

    // Properties
    /// Function visibility
    pub visibility: Visibility,
    /// Whether this is an entry function
    pub is_entry: bool,
    /// Whether this is a view function
    pub is_view: bool,
    /// Whether this is an inline function
    pub is_inline: bool,
    /// Whether this is a native function
    pub is_native: bool,
    /// Attributes on the function
    pub attributes: Vec<String>,

    // Signature (children)
    /// Type parameters (generics)
    pub type_parameters: Vec<TypeParameter>,
    /// Function parameters
    pub parameters: Vec<Parameter>,
    /// Return type
    pub return_type: Type,
    /// Resources acquired by this function
    pub acquires: Vec<String>,

    // Specification (optional)
    /// Function specification
    pub spec: Option<Specification>,
}

/// Full struct information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StructInfo {
    // Identity
    /// Module containing this struct
    pub module: String,
    /// Struct name
    pub name: String,
    /// Documentation comment
    pub doc: String,
    /// Source location
    pub location: Location,

    // Properties
    /// Struct abilities
    pub abilities: Vec<Ability>,
    /// Whether this is a resource (has key ability)
    pub is_resource: bool,
    /// Whether this is a native struct
    pub is_native: bool,
    /// Attributes on the struct
    pub attributes: Vec<String>,

    // Structure (children)
    /// Type parameters (generics)
    pub type_parameters: Vec<TypeParameter>,
    /// Struct fields (None for native structs)
    pub fields: Option<Vec<Field>>,
    /// Enum variants (for enums only)
    pub variants: Option<Vec<Variant>>,

    // Specification
    /// Struct invariants
    pub invariants: Vec<String>,
}

/// Full constant information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConstantInfo {
    /// Module containing this constant
    pub module: String,
    /// Constant name
    pub name: String,
    /// Documentation comment
    pub doc: String,
    /// Source location
    pub location: Location,
    /// Constant type
    #[serde(rename = "type")]
    pub type_: Type,
    /// Constant value
    pub value: Option<String>,
}

// =============================================================================
// Source Code
// =============================================================================

/// Source code information for an item
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceInfo {
    /// Full path (e.g., "0xCAFE::module::function")
    pub path: String,
    /// Type of item
    pub item_type: ItemType,
    /// Source location
    pub location: Location,
    /// The actual source code
    pub source: String,
}

/// Type of code item
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ItemType {
    Module,
    Function,
    Struct,
    Constant,
}

impl std::fmt::Display for ItemType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ItemType::Module => write!(f, "module"),
            ItemType::Function => write!(f, "function"),
            ItemType::Struct => write!(f, "struct"),
            ItemType::Constant => write!(f, "constant"),
        }
    }
}

impl std::str::FromStr for ItemType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "module" => Ok(ItemType::Module),
            "function" | "fun" => Ok(ItemType::Function),
            "struct" => Ok(ItemType::Struct),
            "constant" | "const" => Ok(ItemType::Constant),
            _ => Err(format!("Unknown item type: {}", s)),
        }
    }
}

// =============================================================================
// Nested Types
// =============================================================================

/// Type parameter (generic)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypeParameter {
    /// Parameter name (e.g., "T")
    pub name: String,
    /// Whether this is a phantom parameter
    pub is_phantom: bool,
    /// Ability constraints
    pub constraints: Vec<Ability>,
}

/// Function parameter
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Parameter {
    /// Parameter name
    pub name: String,
    /// Parameter type
    #[serde(rename = "type")]
    pub type_: Type,
}

/// Struct field
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Field {
    /// Field name
    pub name: String,
    /// Field type
    #[serde(rename = "type")]
    pub type_: Type,
    /// Documentation comment
    pub doc: String,
}

/// Enum variant
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Variant {
    /// Variant name
    pub name: String,
    /// Variant fields
    pub fields: Vec<Field>,
    /// Documentation comment
    pub doc: String,
}

/// Function specification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Specification {
    /// Preconditions (requires clauses)
    pub requires: Vec<Condition>,
    /// Postconditions (ensures clauses)
    pub ensures: Vec<Condition>,
    /// Abort conditions
    pub aborts_if: Vec<Condition>,
    /// Abort codes
    pub aborts_with: Vec<String>,
    /// Modified resources
    pub modifies: Vec<String>,
    /// Emitted events
    pub emits: Vec<String>,
}

/// A condition in a specification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Condition {
    /// The condition expression
    pub expression: String,
    /// Optional error message
    pub message: Option<String>,
}

// =============================================================================
// Common Types
// =============================================================================

/// Type representation (recursive)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Type {
    /// Primitive type (u8, u64, bool, address, signer, etc.)
    Primitive { name: String },
    /// Vector type
    Vector { element: Box<Type> },
    /// Struct type
    Struct {
        module: String,
        name: String,
        type_args: Vec<Type>,
    },
    /// Reference type
    Reference { is_mutable: bool, referred: Box<Type> },
    /// Type parameter reference
    TypeParameter { name: String },
    /// Tuple type
    Tuple { elements: Vec<Type> },
    /// Unit type (empty tuple)
    Unit,
}

impl Type {
    /// Create a simple string representation of the type
    pub fn to_simple_string(&self) -> String {
        match self {
            Type::Primitive { name } => name.clone(),
            Type::Vector { element } => format!("vector<{}>", element.to_simple_string()),
            Type::Struct {
                module,
                name,
                type_args,
            } => {
                if type_args.is_empty() {
                    format!("{}::{}", module, name)
                } else {
                    let args: Vec<String> =
                        type_args.iter().map(|t| t.to_simple_string()).collect();
                    format!("{}::{}<{}>", module, name, args.join(", "))
                }
            }
            Type::Reference { is_mutable, referred } => {
                if *is_mutable {
                    format!("&mut {}", referred.to_simple_string())
                } else {
                    format!("&{}", referred.to_simple_string())
                }
            }
            Type::TypeParameter { name } => name.clone(),
            Type::Tuple { elements } => {
                let elems: Vec<String> =
                    elements.iter().map(|t| t.to_simple_string()).collect();
                format!("({})", elems.join(", "))
            }
            Type::Unit => "()".to_string(),
        }
    }
}

/// Source location with full span information
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Location {
    /// Source file path
    pub file: String,
    /// Starting line (1-indexed)
    pub start_line: u32,
    /// Starting column (1-indexed)
    pub start_column: u32,
    /// Ending line (1-indexed)
    pub end_line: u32,
    /// Ending column (1-indexed)
    pub end_column: u32,
}

impl Location {
    /// Create a location with only line information (columns default to 1)
    pub fn line_only(file: String, start_line: u32, end_line: u32) -> Self {
        Self {
            file,
            start_line,
            start_column: 1,
            end_line,
            end_column: 1,
        }
    }
}

/// Function/struct visibility
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Visibility {
    Private,
    Public,
    PublicFriend,
    PublicPackage,
}

impl std::fmt::Display for Visibility {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Visibility::Private => write!(f, "private"),
            Visibility::Public => write!(f, "public"),
            Visibility::PublicFriend => write!(f, "public(friend)"),
            Visibility::PublicPackage => write!(f, "public(package)"),
        }
    }
}

/// Move ability
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Ability {
    Copy,
    Drop,
    Store,
    Key,
}

impl std::fmt::Display for Ability {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Ability::Copy => write!(f, "copy"),
            Ability::Drop => write!(f, "drop"),
            Ability::Store => write!(f, "store"),
            Ability::Key => write!(f, "key"),
        }
    }
}

// =============================================================================
// Cross-Cutting Query Types
// =============================================================================

/// Search result (spans all levels)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    /// Level of the item
    pub level: SearchLevel,
    /// Full path (e.g., "0xCAFE::module::function")
    pub path: String,
    /// Source location
    pub location: Location,
}

/// Level for search results
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SearchLevel {
    Module,
    Function,
    Struct,
    Constant,
}

impl std::fmt::Display for SearchLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SearchLevel::Module => write!(f, "module"),
            SearchLevel::Function => write!(f, "function"),
            SearchLevel::Struct => write!(f, "struct"),
            SearchLevel::Constant => write!(f, "constant"),
        }
    }
}

impl std::str::FromStr for SearchLevel {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "module" => Ok(SearchLevel::Module),
            "function" | "fun" => Ok(SearchLevel::Function),
            "struct" => Ok(SearchLevel::Struct),
            "constant" | "const" => Ok(SearchLevel::Constant),
            _ => Err(format!("Unknown search level: {}", s)),
        }
    }
}

/// Call graph for a function
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CallGraph {
    /// The function being analyzed
    pub function: String,
    /// Functions called by this function
    pub calls: Vec<String>,
    /// Functions that call this function
    pub called_by: Vec<String>,
}

/// Module dependencies
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleDeps {
    /// The module being analyzed
    pub module: String,
    /// Modules this module uses (imports)
    pub uses: Vec<String>,
    /// Modules that use this module
    pub used_by: Vec<String>,
    /// Friend modules
    pub friends: Vec<String>,
}

// =============================================================================
// Error Types
// =============================================================================

/// Query error response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryErrorResponse {
    /// Error message
    pub error: String,
    /// Error code
    pub error_code: QueryErrorCode,
    /// Additional details
    pub details: Option<serde_json::Value>,
}

/// Error codes for query operations
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum QueryErrorCode {
    BuildFailed,
    ModuleNotFound,
    FunctionNotFound,
    StructNotFound,
    ConstantNotFound,
    InvalidPattern,
    InvalidArgument,
    InternalError,
}
