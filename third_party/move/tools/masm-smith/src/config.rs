// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Generator settings: size limits, feature switches, and one `valid_*` flag per verifier rule.
//!
//! The `valid_*` flags default to `true`, which yields tests that pass the verifier. Setting one
//! to `false` lets the generator break that rule on purpose. Probabilities are bytes read as
//! `value / 256` unless a field says otherwise.

use serde::{Deserialize, Serialize};

/// Which verifier dimension a generated violation targets.
#[derive(Copy, Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
pub enum DimensionFlag {
    TypeSafety,
    LocalsSafety,
    ReferenceSafety,
    Abilities,
    ControlFlow,
    StackBalance,
    Acquires,
    Bounds,
    Duplication,
    SignatureV2,
    InstantiationLoops,
    Constants,
    StructDefs,
    Friends,
    Limits,
    StructApi,
    InstructionConsistency,
}

impl DimensionFlag {
    pub const ALL: &'static [DimensionFlag] = &[
        DimensionFlag::TypeSafety,
        DimensionFlag::LocalsSafety,
        DimensionFlag::ReferenceSafety,
        DimensionFlag::Abilities,
        DimensionFlag::ControlFlow,
        DimensionFlag::StackBalance,
        DimensionFlag::Acquires,
        DimensionFlag::Bounds,
        DimensionFlag::Duplication,
        DimensionFlag::SignatureV2,
        DimensionFlag::InstantiationLoops,
        DimensionFlag::Constants,
        DimensionFlag::StructDefs,
        DimensionFlag::Friends,
        DimensionFlag::Limits,
        DimensionFlag::StructApi,
        DimensionFlag::InstructionConsistency,
    ];

    pub fn as_kebab(self) -> &'static str {
        match self {
            DimensionFlag::TypeSafety => "type-safety",
            DimensionFlag::LocalsSafety => "locals-safety",
            DimensionFlag::ReferenceSafety => "reference-safety",
            DimensionFlag::Abilities => "abilities",
            DimensionFlag::ControlFlow => "control-flow",
            DimensionFlag::StackBalance => "stack-balance",
            DimensionFlag::Acquires => "acquires",
            DimensionFlag::Bounds => "bounds",
            DimensionFlag::Duplication => "duplication",
            DimensionFlag::SignatureV2 => "signature-v2",
            DimensionFlag::InstantiationLoops => "instantiation-loops",
            DimensionFlag::Constants => "constants",
            DimensionFlag::StructDefs => "struct-defs",
            DimensionFlag::Friends => "friends",
            DimensionFlag::Limits => "limits",
            DimensionFlag::StructApi => "struct-api",
            DimensionFlag::InstructionConsistency => "instruction-consistency",
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    // ===== Per-verifier-dimension validity flags. Default: true =====
    pub valid_type_safety: bool,
    pub valid_locals_safety: bool,
    pub valid_reference_safety: bool,
    pub valid_abilities: bool,
    pub valid_control_flow: bool,
    pub valid_stack_balance: bool,
    pub valid_acquires: bool,
    pub valid_bounds: bool,
    pub valid_duplication: bool,
    pub valid_signature_v2: bool,
    pub valid_instantiation_loops: bool,
    pub valid_constants: bool,
    pub valid_struct_defs: bool,
    pub valid_friends: bool,
    pub valid_limits: bool,
    pub valid_struct_api: bool,
    pub valid_instruction_consistency: bool,

    // ===== Size / shape knobs =====
    pub module_count_max: u8,
    pub structs_per_module_max: u8,
    pub variants_per_enum_max: u8,
    pub fields_per_struct_max: u8,
    pub functions_per_module_max: u8,
    pub type_params_max: u8,
    pub max_type_depth: u8,
    pub locals_per_fun_max: u8,
    pub instrs_per_fun_max: u16,
    pub blocks_per_fun_max: u8,
    pub steps_max: u8,
    /// 0..=255 — among run steps, share that are RunScript (the rest are RunCall).
    pub run_script_vs_call_prob: u8,
    /// 0..=255 — per-decision probability of injecting a violation when its flag is `false`.
    pub violation_injection_prob: u8,

    // ===== Feature toggles =====
    pub enable_variants: bool,
    pub enable_closures: bool,
    pub enable_globals: bool,
    pub enable_references: bool,
    pub enable_generics: bool,
    pub enable_vectors: bool,
    pub enable_friends: bool,
    /// Enable bounded-counter loops with reducible back-edges. When `valid_control_flow`
    /// is also `false`, the generator may occasionally emit an irreducible back-edge
    /// instead, which the verifier's control-flow pass rejects.
    pub enable_loops: bool,
    /// Allows leading `&signer` parameters and resource stores via `move_to`.
    pub enable_signers: bool,
    /// Emit struct API wrapper functions (`pack$S`, `borrow$S$0`, ...) for local structs.
    pub enable_struct_api: bool,
    /// Calls stdlib functions backed by natives, including BCS, hashing, strings, and vectors.
    /// Harnesses must seed the assembler with [`crate::stdlib_modules`].
    pub enable_natives: bool,
    /// Emit `eq` and `neq` on operands whose type mentions a function type, and `bcs` and `cmp`
    /// calls whose type argument does. A type parameter instantiated with a function type is
    /// not detected, so generic code may still compare or serialize function values.
    pub enable_function_value_ops: bool,
    /// Call `type_name::get`. Only applies with [`Config::enable_natives`].
    pub enable_type_name_get: bool,
    /// 0..=255 — per-instruction probability of trying the global storage operations before
    /// the uniform pick, so programs store and access resources often enough for later
    /// transactions to observe them.
    pub global_ops_bias: u8,
    /// 0..=255 — probability that a function's final block ends in `abort` rather than `ret`.
    pub final_abort_prob: u8,
    /// 0..=255 — probability that an integer literal is a boundary value (zero, one, minimum,
    /// maximum, maximum minus one).
    pub boundary_literal_prob: u8,
    /// 0..=255 — probability that a non-boundary integer literal is uniformly random rather
    /// than small.
    pub random_literal_prob: u8,
    /// 0..=255 — per call site, probability of also offering earlier sibling functions and the
    /// caller itself as call targets, allowing recursion. 255 always offers them.
    pub recursion_prob: u8,
    /// 0..=255 — probability that a conditional block terminator branches on `exists` of a
    /// resource, so the fall-through block can read it without aborting.
    pub guarded_read_prob: u8,
    /// 0..=255 — probability that an acquiring global read (`borrow_global`, `move_from`) may
    /// target a resource not known to exist. 255 always allows it.
    pub unguarded_read_prob: u8,
    /// 0..=255 — probability that a global operation on a generic resource uses its canonical
    /// instantiation (every type argument `u64`) rather than random type arguments. Stores
    /// and reads only meet when they name the same instantiation.
    pub canonical_resource_prob: u8,
    /// Maximum calls and packed closures per generated callee within one function; 0 is
    /// unlimited. Limits repeated calls that multiply along call chains.
    pub max_call_sites_per_callee: u8,
    /// 0..=255 — probability that a function with a `u64` parameter recurses on it as a
    /// bounded depth: it returns at zero and otherwise calls itself with the depth minus one.
    pub bounded_recursion_prob: u8,
    /// 0..=255 — per-instruction probability of storing a vector, struct or enum in a fresh
    /// local and working on it in place through references.
    pub in_place_prob: u8,
    /// 0..=255 — per-field probability that a struct field holds a vector, or a struct or enum
    /// declared earlier in the module, rather than a primitive. Such fields can be copied,
    /// dropped and stored, so they leave the struct's abilities unchanged.
    pub rich_field_prob: u8,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            valid_type_safety: true,
            valid_locals_safety: true,
            valid_reference_safety: true,
            valid_abilities: true,
            valid_control_flow: true,
            valid_stack_balance: true,
            valid_acquires: true,
            valid_bounds: true,
            valid_duplication: true,
            valid_signature_v2: true,
            valid_instantiation_loops: true,
            valid_constants: true,
            valid_struct_defs: true,
            valid_friends: true,
            valid_limits: true,
            valid_struct_api: true,
            valid_instruction_consistency: true,

            module_count_max: 2,
            structs_per_module_max: 4,
            variants_per_enum_max: 3,
            fields_per_struct_max: 4,
            functions_per_module_max: 6,
            type_params_max: 2,
            max_type_depth: 4,
            locals_per_fun_max: 8,
            instrs_per_fun_max: 64,
            blocks_per_fun_max: 8,
            steps_max: 8,
            run_script_vs_call_prob: 128,
            violation_injection_prob: 32,

            enable_variants: true,
            enable_closures: true,
            enable_globals: true,
            enable_references: true,
            enable_generics: true,
            enable_vectors: true,
            enable_friends: true,
            enable_loops: true,
            enable_signers: true,
            enable_struct_api: true,
            enable_natives: true,
            enable_function_value_ops: true,
            enable_type_name_get: true,
            global_ops_bias: 0,
            final_abort_prob: 128,
            boundary_literal_prob: 0,
            random_literal_prob: 255,
            recursion_prob: 255,
            guarded_read_prob: 0,
            unguarded_read_prob: 255,
            canonical_resource_prob: 0,
            max_call_sites_per_callee: 0,
            bounded_recursion_prob: 0,
            in_place_prob: 0,
            rich_field_prob: 0,
        }
    }
}

impl Config {
    /// Configuration favoring successful execution and persistent resource writes in VM fuzzing.
    pub fn vm_fuzzing() -> Self {
        Self {
            // The runner publishes one module at a time and the VM requires declared friends to be
            // published first, so a module calling a friend function can never be published here.
            // Friend linking is covered by `verify_prod`.
            enable_friends: false,
            // Resources stored by one transaction are what later transactions and upgrades observe,
            // and an abort discards them, so aborting function exits are rare.
            global_ops_bias: 24,
            final_abort_prob: 8,
            // Mostly small literals so arithmetic mostly succeeds; boundary values still drive
            // overflow and division by zero.
            boundary_literal_prob: 24,
            random_literal_prob: 24,
            // Arbitrary call cycles have no generated base case. Disable them to avoid spending
            // runs on stack exhaustion; bounded self-recursion is enabled separately below.
            recursion_prob: 0,
            // Reads mostly target resources known to exist, so they do not abort the transaction
            // before its stores run.
            guarded_read_prob: 200,
            unguarded_read_prob: 4,
            canonical_resource_prob: 224,
            // Repeated calls multiply along call chains; two sites per callee keeps runs fast.
            max_call_sites_per_callee: 2,
            // Recursion with a base case, often through a closure so the module is re-entered and
            // its resources are locked. Nested recursive calls get shallow depths.
            bounded_recursion_prob: 96,
            // Vectors, structs and enums held in locals and changed through references.
            in_place_prob: 16,
            // Struct fields holding vectors and nested structs and enums, so stored resources do too.
            rich_field_prob: 48,
            ..Self::default()
        }
    }

    /// Returns the validity flag for the given dimension.
    pub fn flag(&self, dim: DimensionFlag) -> bool {
        match dim {
            DimensionFlag::TypeSafety => self.valid_type_safety,
            DimensionFlag::LocalsSafety => self.valid_locals_safety,
            DimensionFlag::ReferenceSafety => self.valid_reference_safety,
            DimensionFlag::Abilities => self.valid_abilities,
            DimensionFlag::ControlFlow => self.valid_control_flow,
            DimensionFlag::StackBalance => self.valid_stack_balance,
            DimensionFlag::Acquires => self.valid_acquires,
            DimensionFlag::Bounds => self.valid_bounds,
            DimensionFlag::Duplication => self.valid_duplication,
            DimensionFlag::SignatureV2 => self.valid_signature_v2,
            DimensionFlag::InstantiationLoops => self.valid_instantiation_loops,
            DimensionFlag::Constants => self.valid_constants,
            DimensionFlag::StructDefs => self.valid_struct_defs,
            DimensionFlag::Friends => self.valid_friends,
            DimensionFlag::Limits => self.valid_limits,
            DimensionFlag::StructApi => self.valid_struct_api,
            DimensionFlag::InstructionConsistency => self.valid_instruction_consistency,
        }
    }

    /// Sets the validity flag for the given dimension.
    pub fn set_flag(&mut self, dim: DimensionFlag, valid: bool) {
        let flag = match dim {
            DimensionFlag::TypeSafety => &mut self.valid_type_safety,
            DimensionFlag::LocalsSafety => &mut self.valid_locals_safety,
            DimensionFlag::ReferenceSafety => &mut self.valid_reference_safety,
            DimensionFlag::Abilities => &mut self.valid_abilities,
            DimensionFlag::ControlFlow => &mut self.valid_control_flow,
            DimensionFlag::StackBalance => &mut self.valid_stack_balance,
            DimensionFlag::Acquires => &mut self.valid_acquires,
            DimensionFlag::Bounds => &mut self.valid_bounds,
            DimensionFlag::Duplication => &mut self.valid_duplication,
            DimensionFlag::SignatureV2 => &mut self.valid_signature_v2,
            DimensionFlag::InstantiationLoops => &mut self.valid_instantiation_loops,
            DimensionFlag::Constants => &mut self.valid_constants,
            DimensionFlag::StructDefs => &mut self.valid_struct_defs,
            DimensionFlag::Friends => &mut self.valid_friends,
            DimensionFlag::Limits => &mut self.valid_limits,
            DimensionFlag::StructApi => &mut self.valid_struct_api,
            DimensionFlag::InstructionConsistency => &mut self.valid_instruction_consistency,
        };
        *flag = valid;
    }
}
