// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::loader::{
    FieldHandle, FieldInstantiation, StructDef, StructInstantiation, StructVariantInfo,
    VariantFieldInfo,
};
use move_binary_format::{
    errors::{PartialVMError, PartialVMResult},
    file_format::{
        Bytecode, CodeOffset, ConstantPoolIndex, FieldHandleIndex, FieldInstantiationIndex,
        FunctionHandleIndex, FunctionInstantiationIndex, LocalIndex, SignatureIndex,
        StructDefInstantiationIndex, StructDefinitionIndex, StructVariantHandleIndex,
        StructVariantInstantiationIndex, VariantFieldHandleIndex, VariantFieldInstantiationIndex,
        VariantIndex,
    },
};
use move_core_types::{
    function::ClosureMask,
    int256::{I256, U256},
    vm_status::StatusCode,
};
use move_vm_types::loaded_data::{
    runtime_types::{AbilityInfo, StructType, Type, TypeBuilder},
    struct_name_indexing::StructNameIndex,
};
use std::{
    cell::RefCell,
    collections::{hash_map::Entry, HashMap},
};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TestVariantV2 {
    pub variant_idx: VariantIndex,
    pub struct_name_idx: StructNameIndex,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct BorrowFieldV2 {
    pub is_mut: bool,
    pub field_offset: usize,
    pub struct_name_idx: StructNameIndex,
    pub field_ty: Type,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PackV2 {
    pub is_generic: bool,
    pub field_count: u16,
    pub struct_ty: Type,
    pub field_tys: Vec<Type>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct BorrowVariantFieldV2 {
    pub is_mut: bool,
    pub def_struct_ty: std::sync::Arc<StructType>,
    pub variants: Vec<u16>,
    pub field_offset: usize,
    pub field_ty: Type,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PackVariantV2 {
    pub is_generic: bool,
    pub variant_idx: VariantIndex,
    pub field_count: u16,
    pub struct_ty: Type,
    pub field_tys: Vec<Type>,
}

/// The VM's internal representation of instructions.
///
/// Currently, it is an exact mirror of the Move bytecode, but can be extended with more
/// instructions in the future.
///
/// This provides path for incremental performance optimizations, while making it less painful to
/// maintain backward compatibility.
///
/// Note: large variants are boxed to keep the size of [`Instruction`] small (16 bytes).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Instruction {
    Pop,
    Ret,
    BrTrue(CodeOffset),
    BrFalse(CodeOffset),
    Branch(CodeOffset),
    LdU8(u8),
    LdU64(u64),
    LdU128(Box<u128>),
    CastU8,
    CastU64,
    CastU128,
    LdConst(ConstantPoolIndex),
    LdTrue,
    LdFalse,
    CopyLoc(LocalIndex),
    MoveLoc(LocalIndex),
    StLoc(LocalIndex),
    Call(FunctionHandleIndex),
    CallGeneric(FunctionInstantiationIndex),
    Pack(StructDefinitionIndex),
    PackGeneric(StructDefInstantiationIndex),
    PackVariant(StructVariantHandleIndex),
    PackVariantGeneric(StructVariantInstantiationIndex),
    Unpack(StructDefinitionIndex),
    UnpackGeneric(StructDefInstantiationIndex),
    UnpackVariant(StructVariantHandleIndex),
    UnpackVariantGeneric(StructVariantInstantiationIndex),
    TestVariant(StructVariantHandleIndex),
    TestVariantGeneric(StructVariantInstantiationIndex),
    ReadRef,
    WriteRef,
    FreezeRef,
    MutBorrowLoc(LocalIndex),
    ImmBorrowLoc(LocalIndex),
    MutBorrowField(FieldHandleIndex),
    MutBorrowVariantField(VariantFieldHandleIndex),
    MutBorrowFieldGeneric(FieldInstantiationIndex),
    MutBorrowVariantFieldGeneric(VariantFieldInstantiationIndex),
    ImmBorrowField(FieldHandleIndex),
    ImmBorrowVariantField(VariantFieldHandleIndex),
    ImmBorrowFieldGeneric(FieldInstantiationIndex),
    ImmBorrowVariantFieldGeneric(VariantFieldInstantiationIndex),
    MutBorrowGlobal(StructDefinitionIndex),
    MutBorrowGlobalGeneric(StructDefInstantiationIndex),
    ImmBorrowGlobal(StructDefinitionIndex),
    ImmBorrowGlobalGeneric(StructDefInstantiationIndex),
    Add,
    Sub,
    Mul,
    Mod,
    Div,
    BitOr,
    BitAnd,
    Xor,
    Or,
    And,
    Not,
    Eq,
    Neq,
    Lt,
    Gt,
    Le,
    Ge,
    Abort,
    Nop,
    Exists(StructDefinitionIndex),
    ExistsGeneric(StructDefInstantiationIndex),
    MoveFrom(StructDefinitionIndex),
    MoveFromGeneric(StructDefInstantiationIndex),
    MoveTo(StructDefinitionIndex),
    MoveToGeneric(StructDefInstantiationIndex),
    Shl,
    Shr,
    VecPack(SignatureIndex, u64),
    VecLen(SignatureIndex),
    VecImmBorrow(SignatureIndex),
    VecMutBorrow(SignatureIndex),
    VecPushBack(SignatureIndex),
    VecPopBack(SignatureIndex),
    VecUnpack(SignatureIndex, u64),
    VecSwap(SignatureIndex),
    PackClosure(FunctionHandleIndex, ClosureMask),
    PackClosureGeneric(FunctionInstantiationIndex, ClosureMask),
    CallClosure(SignatureIndex),
    LdU16(u16),
    LdU32(u32),
    LdU256(Box<U256>),
    CastU16,
    CastU32,
    CastU256,
    LdI8(i8),
    LdI16(i16),
    LdI32(i32),
    LdI64(i64),
    LdI128(Box<i128>),
    LdI256(Box<I256>),
    CastI8,
    CastI16,
    CastI32,
    CastI64,
    CastI128,
    CastI256,
    Negate,

    VecLenV2,
    VecSwapV2,
    TestVariantV2(TestVariantV2),
    BorrowFieldV2(Box<BorrowFieldV2>),
    PackV2(Box<PackV2>),
    BorrowVariantFieldV2(Box<BorrowVariantFieldV2>),
    PackVariantV2(Box<PackVariantV2>),
}

/// Factory type that handles the conversion from Move bytecode (as defined in the binary format)
/// to the internal VM instruction representation.
///
/// It has the ability to apply optimizations such as:
/// - **V2 Instructions**: variants of existing instructions that provide direct access
///   to certain runtime info required for execution and can be executed indepdent of the
///   frame context.
/// - **Inline Function Detection**: determines whether a function can be trivially inlined,
///   in order to speed up execution.
pub(crate) struct BytecodeTransformer<'a> {
    pub(crate) use_v2_instructions: bool,

    pub(crate) ty_builder: TypeBuilder,

    pub(crate) structs: &'a [StructDef],
    pub(crate) struct_instantiations: &'a [StructInstantiation],

    pub(crate) struct_variant_infos: &'a [StructVariantInfo],
    pub(crate) struct_variant_instantiation_infos: &'a [StructVariantInfo],

    pub(crate) field_handles: &'a [FieldHandle],
    pub(crate) field_instantiations: &'a [FieldInstantiation],

    pub(crate) variant_field_infos: &'a [VariantFieldInfo],
    pub(crate) variant_field_instantiation_infos: &'a [VariantFieldInfo],

    // Caches for expensive type instantiation operations
    field_ty_cache: RefCell<HashMap<FieldInstantiationIndex, (Type, bool)>>,
    variant_field_ty_cache: RefCell<HashMap<VariantFieldInstantiationIndex, (Type, bool)>>,

    pack_field_tys_cache: RefCell<HashMap<StructDefInstantiationIndex, Vec<Type>>>,
    pack_variant_field_tys_cache: RefCell<HashMap<StructVariantInstantiationIndex, Vec<Type>>>,
}

impl<'a> BytecodeTransformer<'a> {
    /// Creates a new `BytecodeTransformer` instance.
    pub fn new(
        structs: &'a [StructDef],
        struct_instantiations: &'a [StructInstantiation],
        struct_variant_infos: &'a [StructVariantInfo],
        struct_variant_instantiation_infos: &'a [StructVariantInfo],
        field_handles: &'a [FieldHandle],
        field_instantiations: &'a [FieldInstantiation],
        variant_field_infos: &'a [VariantFieldInfo],
        variant_field_instantiation_infos: &'a [VariantFieldInfo],
    ) -> Self {
        Self {
            use_v2_instructions: true, // TODO: get this from config/feature flag
            ty_builder: TypeBuilder::with_limits(128, 20), // TODO: get this from config
            structs,
            struct_instantiations,
            struct_variant_infos,
            struct_variant_instantiation_infos,
            field_handles,
            field_instantiations,
            variant_field_infos,
            variant_field_instantiation_infos,
            field_ty_cache: RefCell::new(HashMap::new()),
            variant_field_ty_cache: RefCell::new(HashMap::new()),
            pack_field_tys_cache: RefCell::new(HashMap::new()),
            pack_variant_field_tys_cache: RefCell::new(HashMap::new()),
        }
    }

    /// Retrieves the type of a field instantiation and whether it is fully instantiated.
    ///
    /// The result is cached internally to avoid repeated expensive computations.
    fn get_field_instantiation_ty(
        &self,
        idx: FieldInstantiationIndex,
    ) -> PartialVMResult<(Type, bool)> {
        let mut cache = self.field_ty_cache.borrow_mut();
        match cache.entry(idx) {
            Entry::Occupied(entry) => Ok(entry.get().clone()),
            Entry::Vacant(entry) => {
                let field_inst = &self.field_instantiations[idx.0 as usize];
                // TODO: check if error code is correct
                let ty = self
                    .ty_builder
                    .create_ty_with_subst_allow_ty_params(
                        &field_inst.uninstantiated_field_ty,
                        &field_inst.instantiation,
                    )
                    .map_err(|e| {
                        PartialVMError::new(StatusCode::TYPE_RESOLUTION_FAILURE)
                            .with_message(format!("Failed to create field type: {}", e))
                    })?;
                let is_fully_instantiated = ty.is_concrete();
                Ok(entry.insert((ty.clone(), is_fully_instantiated)).clone())
            },
        }
    }

    /// Retrieves the type of a variant field instantiation and whether it is fully instantiated.
    ///
    /// The result is cached internally to avoid repeated expensive computations.
    fn get_variant_field_instantiation_ty(
        &self,
        idx: VariantFieldInstantiationIndex,
    ) -> PartialVMResult<(Type, bool)> {
        let mut cache = self.variant_field_ty_cache.borrow_mut();
        match cache.entry(idx) {
            Entry::Occupied(entry) => Ok(entry.get().clone()),
            Entry::Vacant(entry) => {
                let variant_field_inst = &self.variant_field_instantiation_infos[idx.0 as usize];
                // TODO: check if error code is correct
                let ty = self
                    .ty_builder
                    .create_ty_with_subst_allow_ty_params(
                        &variant_field_inst.uninstantiated_field_ty,
                        &variant_field_inst.instantiation,
                    )
                    .map_err(|e| {
                        PartialVMError::new(StatusCode::TYPE_RESOLUTION_FAILURE)
                            .with_message(format!("Failed to create variant field type: {}", e))
                    })?;
                let is_fully_instantiated = ty.is_concrete();
                Ok(entry.insert((ty.clone(), is_fully_instantiated)).clone())
            },
        }
    }

    /// Determines whether a function can be trivially inlined during runtime.
    ///
    /// As of now, a function is considered inlineable if it is a "stack-only-no-branch" function,
    /// i.e. it only manipulates stack values without branches, access to locals, calls, or
    /// other complex operations.
    ///
    /// Optionally, it is allowed to have a series of `move_loc` instructions at the beginning, if
    /// their sole effect is to get the args back on stack, in the original order.
    ///
    /// For generic instructions to be inlineable, their output types must be fully instantiated --
    /// not depedent on type parameters from the current function context. Here is an example:
    /// ```plaintext
    /// struct Foo<T> {
    ///     x: T,
    ///     y: bool,
    /// }
    ///
    /// // Not inlineable because we don't know that `T` is statically.
    /// fun borrow_x<T>(foo: &Foo<T>): &T {
    ///     &foo.x
    /// }
    ///
    /// // Inlineable because the output type is always `bool`.
    /// fun get_y<T>(foo: &Foo<T>): bool {
    ///     foo.y
    /// }
    /// ```
    pub fn is_function_inlineable(
        &self,
        num_params: usize,
        code: &[Bytecode],
    ) -> PartialVMResult<bool> {
        use Bytecode::*;

        // Function must have at least `num_params + 1` instructions.
        if code.len() < num_params + 1 {
            return Ok(false);
        }

        // Last instruction must be `ret`.
        if code.last().expect("last is always present") != &Bytecode::Ret {
            return Ok(false);
        }

        // At the beginning, there must be a series of `move_loc` instructions that
        // get the args back on stack.
        #[allow(clippy::needless_range_loop)]
        for i in 0..num_params {
            if code[i] != MoveLoc(i as u8) {
                return Ok(false);
            }
        }

        #[allow(clippy::needless_range_loop)]
        for i in num_params..code.len() - 1 {
            match &code[i] {
                // Disallow local operations (after the initial move_loc sequence)
                CopyLoc(_) | MoveLoc(_) | StLoc(_) | MutBorrowLoc(_) | ImmBorrowLoc(_) => {
                    return Ok(false);
                },
                // Disallow global operations
                MutBorrowGlobal(_)
                | MutBorrowGlobalGeneric(_)
                | ImmBorrowGlobal(_)
                | ImmBorrowGlobalGeneric(_)
                | Exists(_)
                | ExistsGeneric(_)
                | MoveFrom(_)
                | MoveFromGeneric(_)
                | MoveTo(_)
                | MoveToGeneric(_) => {
                    return Ok(false);
                },
                // Disallow branches (requires PC + rewriting offsets)
                BrTrue(_) | BrFalse(_) | Branch(_) => {
                    return Ok(false);
                },
                // Disallow regular calls (requires recursive loading/reasoning)
                Call(_) | CallGeneric(_) | CallClosure(_) => {
                    return Ok(false);
                },
                // Disallow closures (for now, for simplicity)
                PackClosure(_, _) | PackClosureGeneric(_, _) => {
                    return Ok(false);
                },
                // Disallow abort (complicates error reporting)
                Abort => {
                    return Ok(false);
                },
                // Disallow ret (can only appear once at the end)
                Ret => {
                    return Ok(false);
                },
                // Disallow LdConst (not supported yet -- needs new instruction)
                LdConst(_) => {
                    return Ok(false);
                },
                // Disallow unpack (not supported yet -- needs new instruction)
                Unpack(_) | UnpackGeneric(_) | UnpackVariant(_) | UnpackVariantGeneric(_) => {
                    return Ok(false);
                },
                // Disallow vector operations other than VecLen (not supported yet -- needs new instruction)
                VecPack(_, _)
                | VecImmBorrow(_)
                | VecMutBorrow(_)
                | VecPushBack(_)
                | VecPopBack(_)
                | VecUnpack(_, _) => {
                    return Ok(false);
                },
                // Allow all stack-only operations:
                // - Stack manipulation
                Pop | Nop => {},
                // - Constants
                LdU8(_) | LdU16(_) | LdU32(_) | LdU64(_) | LdU128(_) | LdU256(_) | LdI8(_)
                | LdI16(_) | LdI32(_) | LdI64(_) | LdI128(_) | LdI256(_) | LdTrue | LdFalse => {},
                // - Casting
                CastU8 | CastU16 | CastU32 | CastU64 | CastU128 | CastU256 | CastI8 | CastI16
                | CastI32 | CastI64 | CastI128 | CastI256 => {},
                // - Arithmetic
                Add | Sub | Mul | Div | Mod => {},
                // - Bitwise
                BitOr | BitAnd | Xor | Shl | Shr => {},
                // - Boolean
                Or | And | Not => {},
                // - Comparison
                Eq | Neq | Lt | Gt | Le | Ge => {},
                // - Negate
                Negate => {},
                // - References (stack-only operations)
                ReadRef | WriteRef | FreezeRef => {},
                // - Struct operations (stack-only)
                Pack(_) | PackVariant(_) | TestVariant(_) | TestVariantGeneric(_) => {},
                // - PackGeneric: requires concrete instantiation for V2
                PackGeneric(idx) => {
                    let struct_inst = &self.struct_instantiations[idx.0 as usize];
                    if !struct_inst.is_fully_instantiated {
                        return Ok(false);
                    }
                    // TODO: check depth?
                },
                // - PackVariantGeneric: requires concrete instantiation for V2
                PackVariantGeneric(idx) => {
                    let struct_variant_inst =
                        &self.struct_variant_instantiation_infos[idx.0 as usize];
                    if !struct_variant_inst.is_fully_instantiated {
                        return Ok(false);
                    }
                    // TODO: check depth?
                },
                // - Field borrowing (stack-only, operates on references)
                MutBorrowField(_) | ImmBorrowField(_) => {},
                // - Generic field borrowing: requires concrete instantiation for V2
                MutBorrowFieldGeneric(idx) | ImmBorrowFieldGeneric(idx) => {
                    let (_, is_fully_instantiated) = self.get_field_instantiation_ty(*idx)?;

                    if !is_fully_instantiated {
                        return Ok(false);
                    }
                },
                // - Variant field borrowing (stack-only, operates on references)
                MutBorrowVariantField(_) | ImmBorrowVariantField(_) => {},
                // - Generic variant field borrowing: requires concrete instantiation for V2
                MutBorrowVariantFieldGeneric(idx) | ImmBorrowVariantFieldGeneric(idx) => {
                    let (_, is_fully_instantiated) =
                        self.get_variant_field_instantiation_ty(*idx)?;

                    if !is_fully_instantiated {
                        return Ok(false);
                    }
                },
                // - Vector operations (only VecLen is supported, for now)
                VecLen(_) | VecSwap(_) => {},
            }
        }

        Ok(true)
    }

    fn transform_vec_len(&self, idx: SignatureIndex, inline: bool) -> PartialVMResult<Instruction> {
        Ok(if self.use_v2_instructions || inline {
            Instruction::VecLenV2
        } else {
            Instruction::VecLen(idx)
        })
    }

    fn transform_vec_swap(
        &self,
        idx: SignatureIndex,
        inline: bool,
    ) -> PartialVMResult<Instruction> {
        Ok(if self.use_v2_instructions || inline {
            Instruction::VecSwapV2
        } else {
            Instruction::VecSwap(idx)
        })
    }

    fn transform_test_variant(
        &self,
        idx: StructVariantHandleIndex,
        inline: bool,
    ) -> PartialVMResult<Instruction> {
        Ok(if self.use_v2_instructions || inline {
            let info = &self.struct_variant_infos[idx.0 as usize];
            Instruction::TestVariantV2(TestVariantV2 {
                variant_idx: info.variant,
                struct_name_idx: info.definition_struct_type.idx,
            })
        } else {
            Instruction::TestVariant(idx)
        })
    }

    fn transform_test_variant_generic(
        &self,
        idx: StructVariantInstantiationIndex,
        inline: bool,
    ) -> PartialVMResult<Instruction> {
        Ok(if self.use_v2_instructions || inline {
            let info = &self.struct_variant_instantiation_infos[idx.0 as usize];
            Instruction::TestVariantV2(TestVariantV2 {
                variant_idx: info.variant,
                struct_name_idx: info.definition_struct_type.idx,
            })
        } else {
            Instruction::TestVariantGeneric(idx)
        })
    }

    fn transform_borrow_field(
        &self,
        is_mut: bool,
        idx: FieldHandleIndex,
        inline: bool,
    ) -> PartialVMResult<Instruction> {
        Ok(if self.use_v2_instructions || inline {
            let handle = &self.field_handles[idx.0 as usize];
            Instruction::BorrowFieldV2(Box::new(BorrowFieldV2 {
                is_mut,
                field_offset: handle.offset,
                struct_name_idx: handle.definition_struct_type.idx,
                field_ty: handle.field_ty.clone(),
            }))
        } else if is_mut {
            Instruction::MutBorrowField(idx)
        } else {
            Instruction::ImmBorrowField(idx)
        })
    }

    fn transform_borrow_field_generic(
        &self,
        is_mut: bool,
        idx: FieldInstantiationIndex,
        inline: bool,
    ) -> PartialVMResult<Instruction> {
        // Fallback to original bytecode instruction
        let fallback = || {
            Ok(if is_mut {
                Instruction::MutBorrowFieldGeneric(idx)
            } else {
                Instruction::ImmBorrowFieldGeneric(idx)
            })
        };

        if !(self.use_v2_instructions || inline) {
            return fallback();
        }

        let field_inst = &self.field_instantiations[idx.0 as usize];
        let (field_ty, is_fully_instantiated) = self.get_field_instantiation_ty(idx)?;

        if !is_fully_instantiated {
            // TODO: invariant violation if inline is true
            return fallback();
        }

        Ok(Instruction::BorrowFieldV2(Box::new(BorrowFieldV2 {
            is_mut,
            field_offset: field_inst.offset,
            struct_name_idx: field_inst.definition_struct_type.idx,
            field_ty,
        })))
    }

    fn transform_pack(
        &self,
        idx: StructDefinitionIndex,
        inline: bool,
    ) -> PartialVMResult<Instruction> {
        Ok(if self.use_v2_instructions || inline {
            let struct_def = &self.structs[idx.0 as usize];

            let field_tys = struct_def
                .definition_struct_type
                .fields(None)?
                .iter()
                .map(|(_, ty)| ty.clone())
                .collect();

            let struct_ty = self.ty_builder.create_struct_ty(
                struct_def.definition_struct_type.idx,
                AbilityInfo::struct_(struct_def.definition_struct_type.abilities),
            );
            // TODO: check depth

            Instruction::PackV2(Box::new(PackV2 {
                is_generic: false,
                field_count: struct_def.field_count,
                struct_ty,
                field_tys,
            }))
        } else {
            Instruction::Pack(idx)
        })
    }

    fn transform_pack_generic(
        &self,
        idx: StructDefInstantiationIndex,
        inline: bool,
    ) -> PartialVMResult<Instruction> {
        // Fallback to original bytecode instruction
        let fallback = || Ok(Instruction::PackGeneric(idx));

        if !(self.use_v2_instructions || inline) {
            return fallback();
        }

        let struct_inst = &self.struct_instantiations[idx.0 as usize];

        if !struct_inst.is_fully_instantiated {
            // TODO: invariant violation if inline is true
            return fallback();
        }

        // Check cache first
        let field_tys = {
            let mut cache = self.pack_field_tys_cache.borrow_mut();
            match cache.entry(idx) {
                Entry::Occupied(entry) => entry.get().clone(),
                Entry::Vacant(entry) => {
                    let mut tys = vec![];
                    for (_, ty) in struct_inst.definition_struct_type.fields(None)? {
                        tys.push(
                            self.ty_builder
                                .create_ty_with_subst(ty, &struct_inst.instantiation)
                                .map_err(|e| {
                                    PartialVMError::new(StatusCode::TYPE_RESOLUTION_FAILURE)
                                        .with_message(format!("Failed to create field type: {}", e))
                                })?,
                        );
                    }
                    entry.insert(tys.clone()).clone()
                },
            }
        };

        // TODO: check depth?
        let struct_ty = Type::StructInstantiation {
            idx: struct_inst.definition_struct_type.idx,
            ty_args: triomphe::Arc::new(struct_inst.instantiation.clone()),
            ability: AbilityInfo::generic_struct(
                struct_inst.definition_struct_type.abilities,
                struct_inst
                    .definition_struct_type
                    .phantom_ty_params_mask
                    .clone(),
            ),
        };

        Ok(Instruction::PackV2(Box::new(PackV2 {
            is_generic: true,
            field_count: struct_inst.field_count,
            struct_ty,
            field_tys,
        })))
    }

    fn transform_borrow_variant_field(
        &self,
        is_mut: bool,
        idx: VariantFieldHandleIndex,
        inline: bool,
    ) -> PartialVMResult<Instruction> {
        Ok(if self.use_v2_instructions || inline {
            let variant_field_info = &self.variant_field_infos[idx.0 as usize];
            Instruction::BorrowVariantFieldV2(Box::new(BorrowVariantFieldV2 {
                is_mut,
                def_struct_ty: variant_field_info.definition_struct_type.clone(),
                variants: variant_field_info.variants.clone(),
                field_offset: variant_field_info.offset,
                field_ty: variant_field_info.uninstantiated_field_ty.clone(),
            }))
        } else if is_mut {
            Instruction::MutBorrowVariantField(idx)
        } else {
            Instruction::ImmBorrowVariantField(idx)
        })
    }

    fn transform_borrow_variant_field_generic(
        &self,
        is_mut: bool,
        idx: VariantFieldInstantiationIndex,
        inline: bool,
    ) -> PartialVMResult<Instruction> {
        let fallback = || {
            Ok(if is_mut {
                Instruction::MutBorrowVariantFieldGeneric(idx)
            } else {
                Instruction::ImmBorrowVariantFieldGeneric(idx)
            })
        };

        if !(self.use_v2_instructions || inline) {
            return fallback();
        }

        let info = &self.variant_field_instantiation_infos[idx.0 as usize];
        let (field_ty, is_fully_instantiated) = self.get_variant_field_instantiation_ty(idx)?;

        if !is_fully_instantiated {
            // TODO: invariant violation if inline is true
            return fallback();
        }

        Ok(Instruction::BorrowVariantFieldV2(Box::new(
            BorrowVariantFieldV2 {
                is_mut,
                def_struct_ty: info.definition_struct_type.clone(),
                variants: info.variants.clone(),
                field_offset: info.offset,
                field_ty,
            },
        )))
    }

    fn transform_pack_variant(
        &self,
        idx: StructVariantHandleIndex,
        inline: bool,
    ) -> PartialVMResult<Instruction> {
        Ok(if self.use_v2_instructions || inline {
            let info = &self.struct_variant_infos[idx.0 as usize];

            let field_tys = info
                .definition_struct_type
                .fields(Some(info.variant))?
                .iter()
                .map(|(_, ty)| ty.clone())
                .collect();

            let struct_ty = self.ty_builder.create_struct_ty(
                info.definition_struct_type.idx,
                AbilityInfo::struct_(info.definition_struct_type.abilities),
            );

            Instruction::PackVariantV2(Box::new(PackVariantV2 {
                is_generic: false,
                variant_idx: info.variant,
                field_count: info.field_count,
                struct_ty,
                field_tys,
            }))
        } else {
            Instruction::PackVariant(idx)
        })
    }

    fn transform_pack_variant_generic(
        &self,
        idx: StructVariantInstantiationIndex,
        inline: bool,
    ) -> PartialVMResult<Instruction> {
        // TODO: double check correctness -- seems like right now there isn't a workflow that triggers this.

        let fallback = || Ok(Instruction::PackVariantGeneric(idx));

        if !(self.use_v2_instructions || inline) {
            return fallback();
        }

        let info = &self.struct_variant_instantiation_infos[idx.0 as usize];

        if !info.is_fully_instantiated {
            // TODO: invariant violation if inline is true
            return fallback();
        }

        let field_tys = {
            let mut cache = self.pack_variant_field_tys_cache.borrow_mut();
            match cache.entry(idx) {
                Entry::Occupied(entry) => entry.get().clone(),
                Entry::Vacant(entry) => {
                    let mut tys = vec![];
                    for (_, ty) in info.definition_struct_type.fields(Some(info.variant))? {
                        tys.push(
                            self.ty_builder
                                .create_ty_with_subst(ty, &info.instantiation)
                                .map_err(|e| {
                                    PartialVMError::new(StatusCode::TYPE_RESOLUTION_FAILURE)
                                        .with_message(format!("Failed to create field type: {}", e))
                                })?,
                        );
                    }
                    entry.insert(tys.clone()).clone()
                },
            }
        };

        // TODO: check depth
        let struct_ty = Type::StructInstantiation {
            idx: info.definition_struct_type.idx,
            ty_args: triomphe::Arc::new(info.instantiation.clone()),
            ability: AbilityInfo::generic_struct(
                info.definition_struct_type.abilities,
                info.definition_struct_type.phantom_ty_params_mask.clone(),
            ),
        };

        Ok(Instruction::PackVariantV2(Box::new(PackVariantV2 {
            is_generic: true,
            variant_idx: info.variant,
            field_count: info.field_count,
            struct_ty,
            field_tys,
        })))
    }

    /// Transforms a Move bytecode instruction into a VM `Instruction` type.
    ///
    /// If `self.use_v2_instructions` is set, the transformer will use the V2 instructions
    /// if possible.
    ///
    /// If `inline` is set, the use of v2 instructions is forced, as as we rely on their
    /// ability to be executed indepdent of the frame context.
    /// It is expected that the caller has already checked that the function is inlineable via
    /// [`Self::is_function_inlineable`].
    pub fn transform(&self, bytecode: Bytecode, inline: bool) -> PartialVMResult<Instruction> {
        use Bytecode as B;
        use Instruction as I;

        Ok(match bytecode {
            B::Pop => I::Pop,
            B::Ret => I::Ret,
            B::BrTrue(offset) => I::BrTrue(offset),
            B::BrFalse(offset) => I::BrFalse(offset),
            B::Branch(offset) => I::Branch(offset),
            B::LdU8(val) => I::LdU8(val),
            B::LdU64(val) => I::LdU64(val),
            B::LdU128(val) => I::LdU128(Box::new(val)),
            B::CastU8 => I::CastU8,
            B::CastU64 => I::CastU64,
            B::CastU128 => I::CastU128,
            B::LdConst(idx) => I::LdConst(idx),
            B::LdTrue => I::LdTrue,
            B::LdFalse => I::LdFalse,
            B::CopyLoc(idx) => I::CopyLoc(idx),
            B::MoveLoc(idx) => I::MoveLoc(idx),
            B::StLoc(idx) => I::StLoc(idx),
            B::Call(idx) => I::Call(idx),
            B::CallGeneric(idx) => I::CallGeneric(idx),
            B::Pack(idx) => self.transform_pack(idx, inline)?,
            B::PackGeneric(idx) => self.transform_pack_generic(idx, inline)?,
            B::PackVariant(idx) => self.transform_pack_variant(idx, inline)?,
            B::PackVariantGeneric(idx) => self.transform_pack_variant_generic(idx, inline)?,
            B::Unpack(idx) => I::Unpack(idx),
            B::UnpackGeneric(idx) => I::UnpackGeneric(idx),
            B::UnpackVariant(idx) => I::UnpackVariant(idx),
            B::UnpackVariantGeneric(idx) => I::UnpackVariantGeneric(idx),
            B::TestVariant(idx) => self.transform_test_variant(idx, inline)?,
            B::TestVariantGeneric(idx) => self.transform_test_variant_generic(idx, inline)?,
            B::ReadRef => I::ReadRef,
            B::WriteRef => I::WriteRef,
            B::FreezeRef => I::FreezeRef,
            B::MutBorrowLoc(idx) => I::MutBorrowLoc(idx),
            B::ImmBorrowLoc(idx) => I::ImmBorrowLoc(idx),
            B::MutBorrowField(idx) => self.transform_borrow_field(true, idx, inline)?,
            B::MutBorrowVariantField(idx) => {
                self.transform_borrow_variant_field(true, idx, inline)?
            },
            B::MutBorrowFieldGeneric(idx) => {
                self.transform_borrow_field_generic(true, idx, inline)?
            },
            B::MutBorrowVariantFieldGeneric(idx) => {
                self.transform_borrow_variant_field_generic(true, idx, inline)?
            },
            B::ImmBorrowField(idx) => self.transform_borrow_field(false, idx, inline)?,
            B::ImmBorrowVariantField(idx) => {
                self.transform_borrow_variant_field(false, idx, inline)?
            },
            B::ImmBorrowFieldGeneric(idx) => {
                self.transform_borrow_field_generic(false, idx, inline)?
            },
            B::ImmBorrowVariantFieldGeneric(idx) => {
                self.transform_borrow_variant_field_generic(false, idx, inline)?
            },
            B::MutBorrowGlobal(idx) => I::MutBorrowGlobal(idx),
            B::MutBorrowGlobalGeneric(idx) => I::MutBorrowGlobalGeneric(idx),
            B::ImmBorrowGlobal(idx) => I::ImmBorrowGlobal(idx),
            B::ImmBorrowGlobalGeneric(idx) => I::ImmBorrowGlobalGeneric(idx),
            B::Add => I::Add,
            B::Sub => I::Sub,
            B::Mul => I::Mul,
            B::Mod => I::Mod,
            B::Div => I::Div,
            B::BitOr => I::BitOr,
            B::BitAnd => I::BitAnd,
            B::Xor => I::Xor,
            B::Or => I::Or,
            B::And => I::And,
            B::Not => I::Not,
            B::Eq => I::Eq,
            B::Neq => I::Neq,
            B::Lt => I::Lt,
            B::Gt => I::Gt,
            B::Le => I::Le,
            B::Ge => I::Ge,
            B::Abort => I::Abort,
            B::Nop => I::Nop,
            B::Exists(idx) => I::Exists(idx),
            B::ExistsGeneric(idx) => I::ExistsGeneric(idx),
            B::MoveFrom(idx) => I::MoveFrom(idx),
            B::MoveFromGeneric(idx) => I::MoveFromGeneric(idx),
            B::MoveTo(idx) => I::MoveTo(idx),
            B::MoveToGeneric(idx) => I::MoveToGeneric(idx),
            B::Shl => I::Shl,
            B::Shr => I::Shr,
            B::VecPack(idx, n) => I::VecPack(idx, n),
            B::VecLen(idx) => self.transform_vec_len(idx, inline)?,
            B::VecImmBorrow(idx) => I::VecImmBorrow(idx),
            B::VecMutBorrow(idx) => I::VecMutBorrow(idx),
            B::VecPushBack(idx) => I::VecPushBack(idx),
            B::VecPopBack(idx) => I::VecPopBack(idx),
            B::VecUnpack(idx, n) => I::VecUnpack(idx, n),
            B::VecSwap(idx) => self.transform_vec_swap(idx, inline)?,
            B::PackClosure(idx, mask) => I::PackClosure(idx, mask),
            B::PackClosureGeneric(idx, mask) => I::PackClosureGeneric(idx, mask),
            B::CallClosure(idx) => I::CallClosure(idx),
            B::LdU16(val) => I::LdU16(val),
            B::LdU32(val) => I::LdU32(val),
            B::LdU256(val) => I::LdU256(Box::new(val)),
            B::CastU16 => I::CastU16,
            B::CastU32 => I::CastU32,
            B::CastU256 => I::CastU256,
            B::LdI8(val) => I::LdI8(val),
            B::LdI16(val) => I::LdI16(val),
            B::LdI32(val) => I::LdI32(val),
            B::LdI64(val) => I::LdI64(val),
            B::LdI128(val) => I::LdI128(Box::new(val)),
            B::LdI256(val) => I::LdI256(Box::new(val)),
            B::CastI8 => I::CastI8,
            B::CastI16 => I::CastI16,
            B::CastI32 => I::CastI32,
            B::CastI64 => I::CastI64,
            B::CastI128 => I::CastI128,
            B::CastI256 => I::CastI256,
            B::Negate => I::Negate,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::mem::size_of;

    #[test]
    fn test_vm_operation_size() {
        let size = size_of::<Instruction>();

        assert_eq!(
            size, 16,
            "VMOperation size should be exactly 16 bytes, but got {} bytes",
            size
        );
    }
}
