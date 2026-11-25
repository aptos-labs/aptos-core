// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! The data model of a generated test: an optional `//# init` directive, then steps that publish
//! modules and run functions or scripts. It follows `.masm` syntax closely (opcodes are
//! `move-asm`'s names), so rendering is a direct walk.

use crate::config::DimensionFlag;
use move_binary_format::file_format::{FunctionAttribute, Visibility};
use move_core_types::{
    ability::AbilitySet,
    account_address::AccountAddress,
    identifier::Identifier,
    int256::{I256, U256},
    language_storage::ModuleId,
};
use serde::{Deserialize, Serialize};

// ============================================================================
// Top-level test
// ============================================================================

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct MasmTest {
    pub init: Option<InitDirective>,
    pub steps: Vec<Step>,
    /// Violations present in the rendered text. Binary-level violations are not listed here;
    /// [`MasmTest::post_process_module`] reports the ones it applies.
    pub injected_violations: Vec<DimensionFlag>,
    pub post_process: Option<PostProcessRecipe>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct InitDirective {
    /// Named address bindings: `name=0x...`.
    pub addresses: Vec<(Identifier, AccountAddress)>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Step {
    Publish(Module),
    RunScript(Script),
    RunCall(RunCall),
}

impl Step {
    /// The functions this step defines: a module's functions or a script's entry.
    pub fn functions(&self) -> &[Function] {
        match self {
            Step::Publish(module) => &module.functions,
            Step::RunScript(script) => std::slice::from_ref(&script.entry),
            Step::RunCall(_) => &[],
        }
    }

    pub fn functions_mut(&mut self) -> &mut [Function] {
        match self {
            Step::Publish(module) => &mut module.functions,
            Step::RunScript(script) => std::slice::from_mut(&mut script.entry),
            Step::RunCall(_) => &mut [],
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RunCall {
    pub target: QualifiedFunction,
    pub type_args: Vec<Type>,
    pub args: Vec<RuntimeArg>,
    pub signers: Vec<AccountAddress>,
}

/// Runtime arguments accepted by the transactional-test runner's `--args` flag. The text
/// representation depends on the `--syntax` mode; we use the default (Move literal) syntax.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum RuntimeArg {
    U8(u8),
    U16(u16),
    U32(u32),
    U64(u64),
    U128(u128),
    U256(U256),
    Bool(bool),
    Address(AccountAddress),
    Vector(Vec<RuntimeArg>),
}

// ============================================================================
// PostProcessRecipe — binary-level mutations applied after assembly
// ============================================================================

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct PostProcessRecipe {
    /// Mutations paired with the publish-step indices of the modules they target.
    pub intents: Vec<(usize, PostProcessIntent)>,
}

/// A mutation applied after assembly. Generators record it in
/// [`crate::state::FunctionState::post_process_intents`]; the orchestrator adds the publish-step
/// index when constructing [`PostProcessRecipe::intents`].
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum PostProcessIntent {
    /// Replace the constant at the given module-wide `constant_pool` index with bytes
    /// guaranteed not to BCS-decode to the declared type. Drives the `valid_constants`
    /// violation knob. The generator records the index of the `ld_const` it just emitted;
    /// when the assembler dedupes constants and `index` ends up out of bounds, the
    /// mutation targets the last constant instead. An empty pool is left unchanged.
    CorruptConstant { index: u16 },
    /// Rewrite the first function definition's `FunctionHandleIndex` to past the end of
    /// the function-handle table. Drives the `valid_bounds` violation knob — the
    /// verifier's `check_bounds` pass detects out-of-range indices.
    CorruptBounds,
    /// Rewrite the first `VecPack` instruction's `num` operand past `u16::MAX`, which the
    /// `instruction_consistency` verifier rejects with `CONSTRAINT_NOT_SATISFIED`. The
    /// mutation no-ops when the body contains no `VecPack`.
    CorruptInstructionConsistency,
}

impl PostProcessIntent {
    /// The verifier dimension this mutation violates.
    pub fn dimension(&self) -> DimensionFlag {
        match self {
            PostProcessIntent::CorruptConstant { .. } => DimensionFlag::Constants,
            PostProcessIntent::CorruptBounds => DimensionFlag::Bounds,
            PostProcessIntent::CorruptInstructionConsistency => {
                DimensionFlag::InstructionConsistency
            },
        }
    }
}

impl PostProcessRecipe {
    pub fn is_empty(&self) -> bool {
        self.intents.is_empty()
    }

    /// Apply every mutation targeting `step_idx` to `module`, returning the dimensions of
    /// the mutations that changed it. Shared between [`MasmTest::post_process_module`] and
    /// the CLI's `check --recipe` path so both honor every [`PostProcessIntent`] variant.
    pub fn apply_to_module(
        &self,
        step_idx: usize,
        module: &mut move_binary_format::file_format::CompiledModule,
    ) -> Vec<DimensionFlag> {
        let mut applied = Vec::new();
        for (idx, intent) in &self.intents {
            if *idx != step_idx {
                continue;
            }
            let mut mutated = false;
            match intent {
                PostProcessIntent::CorruptConstant { index } => {
                    // The payload exceeds every primitive's encoded width. Its leading
                    // 0xFF bytes also form an invalid ULEB128 vector length.
                    let target =
                        (*index as usize).min(module.constant_pool.len().saturating_sub(1));
                    // The trailing index keeps two corrupted constants distinct, so the
                    // duplication check does not reject the module first.
                    let mut payload = vec![0xFF; 33];
                    payload.extend_from_slice(&(target as u32).to_le_bytes());
                    if let Some(c) = module.constant_pool.get_mut(target) {
                        c.data = payload;
                        mutated = true;
                    }
                },
                PostProcessIntent::CorruptBounds => {
                    // Bump the first function definition's `FunctionHandleIndex` to one
                    // past the end of the function-handle table. The verifier's
                    // `check_bounds` pass walks every table reference and rejects
                    // out-of-range indices.
                    let handles_len = module.function_handles.len() as u16;
                    if let Some(first_def) = module.function_defs.get_mut(0) {
                        first_def.function =
                            move_binary_format::file_format::FunctionHandleIndex(handles_len);
                        mutated = true;
                    }
                },
                PostProcessIntent::CorruptInstructionConsistency => {
                    // Rewrite the first `VecPack` across all function bodies. Counts above
                    // `u16::MAX` violate instruction consistency; no `VecPack` means no mutation.
                    'outer: for def in module.function_defs.iter_mut() {
                        if let Some(code) = def.code.as_mut() {
                            for instr in code.code.iter_mut() {
                                if let move_binary_format::file_format::Bytecode::VecPack(_, num) =
                                    instr
                                {
                                    *num = u32::MAX as u64;
                                    mutated = true;
                                    break 'outer;
                                }
                            }
                        }
                    }
                },
            }
            if mutated {
                applied.push(intent.dimension());
            }
        }
        applied
    }
}

impl MasmTest {
    /// Apply the [`PostProcessRecipe`] (if any) to a freshly-assembled compiled module that
    /// corresponds to step `step_idx`, returning the dimensions of the mutations that changed
    /// it.
    ///
    /// The pipeline expects callers to walk the assembled artifacts in step order and call
    /// this once per `Publish` step, before invoking the bytecode verifier.
    pub fn post_process_module(
        &self,
        step_idx: usize,
        module: &mut move_binary_format::file_format::CompiledModule,
    ) -> Vec<DimensionFlag> {
        match &self.post_process {
            Some(recipe) => recipe.apply_to_module(step_idx, module),
            None => Vec::new(),
        }
    }
}

// ============================================================================
// Module + Script
// ============================================================================

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Module {
    pub id: ModuleId,
    pub address_aliases: Vec<(Identifier, AccountAddress)>,
    pub module_aliases: Vec<(Identifier, ModuleId)>,
    pub friends: Vec<ModuleId>,
    pub structs: Vec<StructDecl>,
    pub functions: Vec<Function>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Script {
    pub module_aliases: Vec<(Identifier, ModuleId)>,
    pub entry: Function,
    /// Accounts passed to the entry function's leading `&signer` parameters.
    pub signers: Vec<AccountAddress>,
}

// ============================================================================
// Struct / enum declarations
// ============================================================================

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StructDecl {
    pub name: Identifier,
    pub type_params: Vec<TypeParam>,
    pub abilities: AbilitySet,
    pub layout: StructLayout,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum StructLayout {
    Singleton(Vec<Field>),
    Variants(Vec<Variant>),
}

impl StructLayout {
    /// Every field, of every variant for an enum.
    pub fn all_fields(&self) -> Box<dyn Iterator<Item = &Field> + '_> {
        match self {
            StructLayout::Singleton(fields) => Box::new(fields.iter()),
            StructLayout::Variants(variants) => {
                Box::new(variants.iter().flat_map(|variant| &variant.fields))
            },
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Variant {
    pub name: Identifier,
    pub fields: Vec<Field>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Field {
    pub name: Identifier,
    pub ty: Type,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TypeParam {
    pub name: Identifier,
    pub constraints: AbilitySet,
    pub is_phantom: bool,
}

// ============================================================================
// Function
// ============================================================================

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Function {
    pub name: Identifier,
    pub visibility: Visibility,
    pub is_entry: bool,
    pub is_native: bool,
    #[serde(with = "function_attributes")]
    pub attributes: Vec<FunctionAttribute>,
    pub type_params: Vec<TypeParam>,
    pub params: Vec<Local>,
    pub locals: Vec<Local>,
    pub return_types: Vec<Type>,
    /// Declared `acquires` set: unqualified struct names (resolved in the current module).
    /// The bytecode-level acquires list is by struct-def index only, so any generic
    /// instantiation of a generic key resource is covered by listing the name alone.
    pub acquires: Vec<Identifier>,
    pub body: Vec<Block>,
}

impl Function {
    /// Whether this is a generated struct API wrapper, whose name contains `$`.
    pub fn is_struct_api_wrapper(&self) -> bool {
        self.name.as_str().contains('$')
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Local {
    pub name: Identifier,
    pub ty: Type,
}

// ============================================================================
// Blocks & instructions
// ============================================================================

#[derive(
    Copy, Clone, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize,
)]
pub struct BlockId(pub u32);

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Block {
    pub id: BlockId,
    pub instrs: Vec<Instr>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Instr {
    /// Canonical opcode mnemonic (lowercase, as accepted by `move-asm`).
    pub opcode: String,
    pub operands: Vec<Operand>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Operand {
    Imm(Value),
    Local(Identifier),
    Label(BlockId),
    Function(QualifiedFunction, Vec<Type>),
    Struct(QualifiedStruct, Vec<Type>),
    Field {
        owner: QualifiedStruct,
        type_args: Vec<Type>,
        field: Identifier,
    },
    /// `(variant, field)` pairs naming the same field offset across variants.
    VariantFields {
        owner: QualifiedStruct,
        type_args: Vec<Type>,
        fields: Vec<(Identifier, Identifier)>,
    },
    Type(Type),
    /// A free-form bit mask used by closure operations.
    ClosureMask(u64),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Value {
    U8(u8),
    U16(u16),
    U32(u32),
    U64(u64),
    U128(u128),
    U256(U256),
    I8(i8),
    I16(i16),
    I32(i32),
    I64(i64),
    I128(i128),
    I256(I256),
    Bool(bool),
    Address(AccountAddress),
    /// Generic byte payload (e.g., for `ld_const` of vector<u8>).
    Bytes(Vec<u8>),
    /// Structured vector literal — used for `ld_const` of `vector<T>` when `T` is not
    /// `u8`. Each element is itself a [`Value`]; the renderer emits a bracketed
    /// numeric-literal list (the assembler resolves each element against the declared
    /// element type via [`AsmValue::to_move_value`]).
    Vector(Vec<Value>),
}

// ============================================================================
// Types
// ============================================================================

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum Type {
    Bool,
    U8,
    U16,
    U32,
    U64,
    U128,
    U256,
    I8,
    I16,
    I32,
    I64,
    I128,
    I256,
    Address,
    Signer,
    Vector(Box<Type>),
    Struct(QualifiedStruct, Vec<Type>),
    Reference(bool /* is_mut */, Box<Type>),
    TypeParam(u16),
    /// A function type for closures: `|args| -> results` with declared abilities.
    Function {
        args: Vec<Type>,
        results: Vec<Type>,
        abilities: AbilitySet,
    },
}

impl Type {
    /// Whether `pred` holds for this type or any type nested in it.
    pub fn any(&self, pred: &impl Fn(&Type) -> bool) -> bool {
        pred(self)
            || match self {
                Type::Vector(inner) | Type::Reference(_, inner) => inner.any(pred),
                Type::Struct(_, args) => args.iter().any(|arg| arg.any(pred)),
                Type::Function { args, results, .. } => {
                    args.iter().chain(results).any(|ty| ty.any(pred))
                },
                Type::Bool
                | Type::U8
                | Type::U16
                | Type::U32
                | Type::U64
                | Type::U128
                | Type::U256
                | Type::I8
                | Type::I16
                | Type::I32
                | Type::I64
                | Type::I128
                | Type::I256
                | Type::Address
                | Type::Signer
                | Type::TypeParam(_) => false,
            }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct QualifiedFunction {
    pub module: ModuleId,
    pub name: Identifier,
}

#[derive(Clone, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct QualifiedStruct {
    pub module: ModuleId,
    pub name: Identifier,
}

/// Serde for `FunctionAttribute`, which `move-binary-format` does not derive.
mod function_attributes {
    use move_binary_format::file_format::{FunctionAttribute, MemberCount, VariantIndex};
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    #[derive(Serialize, Deserialize)]
    enum Repr {
        Persistent,
        ModuleLock,
        Pack,
        PackVariant(VariantIndex),
        Unpack,
        UnpackVariant(VariantIndex),
        TestVariant(VariantIndex),
        BorrowFieldImmutable(MemberCount),
        BorrowFieldMutable(MemberCount),
    }

    fn to_repr(attr: &FunctionAttribute) -> Repr {
        match attr {
            FunctionAttribute::Persistent => Repr::Persistent,
            FunctionAttribute::ModuleLock => Repr::ModuleLock,
            FunctionAttribute::Pack => Repr::Pack,
            FunctionAttribute::PackVariant(idx) => Repr::PackVariant(*idx),
            FunctionAttribute::Unpack => Repr::Unpack,
            FunctionAttribute::UnpackVariant(idx) => Repr::UnpackVariant(*idx),
            FunctionAttribute::TestVariant(idx) => Repr::TestVariant(*idx),
            FunctionAttribute::BorrowFieldImmutable(offset) => Repr::BorrowFieldImmutable(*offset),
            FunctionAttribute::BorrowFieldMutable(offset) => Repr::BorrowFieldMutable(*offset),
        }
    }

    fn from_repr(repr: Repr) -> FunctionAttribute {
        match repr {
            Repr::Persistent => FunctionAttribute::Persistent,
            Repr::ModuleLock => FunctionAttribute::ModuleLock,
            Repr::Pack => FunctionAttribute::Pack,
            Repr::PackVariant(idx) => FunctionAttribute::PackVariant(idx),
            Repr::Unpack => FunctionAttribute::Unpack,
            Repr::UnpackVariant(idx) => FunctionAttribute::UnpackVariant(idx),
            Repr::TestVariant(idx) => FunctionAttribute::TestVariant(idx),
            Repr::BorrowFieldImmutable(offset) => FunctionAttribute::BorrowFieldImmutable(offset),
            Repr::BorrowFieldMutable(offset) => FunctionAttribute::BorrowFieldMutable(offset),
        }
    }

    pub fn serialize<S: Serializer>(
        attrs: &[FunctionAttribute],
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        attrs
            .iter()
            .map(to_repr)
            .collect::<Vec<_>>()
            .serialize(serializer)
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<Vec<FunctionAttribute>, D::Error> {
        Ok(Vec::<Repr>::deserialize(deserializer)?
            .into_iter()
            .map(from_repr)
            .collect())
    }
}
