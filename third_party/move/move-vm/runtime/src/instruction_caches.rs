// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! Instruction caches keep a record of cached data for each bytecode instruction and are indexed
//! by the program counter. Instruction caches are pre-populated at load-time, when loading code.
//! At runtime, instruction caches are copied into interpreter's context to be able to cache more
//! data, e.g., quickening function calls by caching loaded functions, or caching instantiated
//! types.

use crate::{frame_type_cache::FrameTypeCache, LoadedFunction};
use move_binary_format::{
    errors::{PartialVMError, PartialVMResult},
    file_format::Bytecode,
};
use move_core_types::{gas_algebra::NumTypeNodes, vm_status::StatusCode};
use move_vm_types::loaded_data::runtime_types::Type;
use std::{cell::RefCell, rc::Rc};

// SAFETY:
//
// We only ever share immutable caches across threads, which are guaranteed to contain NO `Call`
// variants (those use `Rc`/`RefCell` and are strictly thread-local). The only public way to obtain
// this cache is to clone it out.
unsafe impl Send for PerInstructionCache {}
unsafe impl Sync for PerInstructionCache {}

#[derive(Clone)]
pub(crate) enum PerInstructionCache {
    /// Default empty variant when nothing is cached.
    Nothing,
    Pack(u16),
    PackGeneric(u16),
    TypeInfo(Type, NumTypeNodes),
    /// Cached function call. This item can only be added at runtime to thread-local copy of the
    /// cache.
    Call(Rc<LoadedFunction>, Rc<RefCell<FrameTypeCache>>),
}

impl Default for PerInstructionCache {
    fn default() -> Self {
        Self::Nothing
    }
}

/// Used for instruction quickening: each instruction maps to cached information, if needed.
#[derive(Clone)]
pub(crate) struct InstructionCache {
    cache: Vec<PerInstructionCache>,
}

impl InstructionCache {
    /// Returns an empty instruction cache of the specified size.
    pub(crate) fn new_empty(code_size: usize) -> Self {
        Self {
            cache: vec![PerInstructionCache::Nothing; code_size],
        }
    }

    pub(crate) fn warmup(
        &mut self,
        code: &[Bytecode],
        signature_table: &[Vec<Type>],
        is_fully_instantiated_signature: &[bool],
    ) -> PartialVMResult<()> {
        for (pc, instr) in code.iter().enumerate() {
            match instr {
                Bytecode::VecPack(idx, _)
                | Bytecode::VecLen(idx)
                | Bytecode::VecImmBorrow(idx)
                | Bytecode::VecMutBorrow(idx)
                | Bytecode::VecPushBack(idx)
                | Bytecode::VecPopBack(idx)
                | Bytecode::VecUnpack(idx, _)
                | Bytecode::VecSwap(idx) => {
                    let idx = idx.0 as usize;
                    if !is_fully_instantiated_signature[idx] {
                        continue;
                    }

                    let tys = &signature_table[idx];
                    if tys.len() != 1 {
                        return Err(
                            PartialVMError::new(StatusCode::VERIFIER_INVARIANT_VIOLATION)
                                .with_message(format!(
                                    "A single token signature is expected for {:?}",
                                    instr
                                )),
                        );
                    }
                    let ty = tys[0].clone();
                    let count = NumTypeNodes::new(ty.num_nodes() as u64);
                    self.cache[pc] = PerInstructionCache::TypeInfo(ty, count);
                },
                Bytecode::LdConst(_) => {
                    // TODO
                },
                Bytecode::Pack(_) => {
                    // TODO
                },
                Bytecode::PackGeneric(_) => {
                    // TODO
                },
                Bytecode::ImmBorrowField(_) | Bytecode::MutBorrowField(_) => {
                    // TODO
                },
                Bytecode::ImmBorrowFieldGeneric(_) | Bytecode::MutBorrowFieldGeneric(_) => {
                    // TODO
                },
                Bytecode::Unpack(_) => {}, // TODO
                Bytecode::UnpackGeneric(_) => {
                    // TODO
                },
                Bytecode::PackVariant(_) => {
                    // TODO
                },
                Bytecode::PackVariantGeneric(_) => {
                    // TODO
                },
                Bytecode::TestVariant(_) => {
                    // TODO
                },
                Bytecode::TestVariantGeneric(_) => {
                    // TODO
                },
                Bytecode::ImmBorrowVariantField(_) | Bytecode::MutBorrowVariantField(_) => {
                    // TODO
                },
                Bytecode::ImmBorrowVariantFieldGeneric(_)
                | Bytecode::MutBorrowVariantFieldGeneric(_) => {
                    // TODO
                },
                Bytecode::UnpackVariant(_) => {
                    // TODO
                },
                Bytecode::UnpackVariantGeneric(_) => {
                    // TODO
                },
                Bytecode::MoveTo(_) => {
                    // TODO
                },
                Bytecode::MoveToGeneric(_) => {
                    // TODO
                },
                Bytecode::Exists(_) => {
                    // TODO
                },
                Bytecode::ExistsGeneric(_) => {
                    // TODO
                },
                Bytecode::ImmBorrowGlobal(_) | Bytecode::MutBorrowGlobal(_) => {
                    // TODO
                },
                Bytecode::ImmBorrowGlobalGeneric(_) | Bytecode::MutBorrowGlobalGeneric(_) => {
                    // TODO
                },
                Bytecode::MoveFrom(_) => {
                    // TODO
                },
                Bytecode::MoveFromGeneric(_) => {
                    // TODO
                },

                Bytecode::ReadRef
                | Bytecode::WriteRef
                | Bytecode::FreezeRef
                | Bytecode::Call(_)
                | Bytecode::CallGeneric(_)
                | Bytecode::CallClosure(_)
                | Bytecode::PackClosure(_, _)
                | Bytecode::PackClosureGeneric(_, _)
                | Bytecode::CopyLoc(_)
                | Bytecode::MutBorrowLoc(_)
                | Bytecode::ImmBorrowLoc(_)
                | Bytecode::MoveLoc(_)
                | Bytecode::StLoc(_)
                | Bytecode::BrTrue(_)
                | Bytecode::BrFalse(_)
                | Bytecode::Branch(_)
                | Bytecode::Pop
                | Bytecode::Ret
                | Bytecode::Abort
                | Bytecode::Nop
                | Bytecode::Add
                | Bytecode::Sub
                | Bytecode::Mul
                | Bytecode::Mod
                | Bytecode::Div
                | Bytecode::Negate
                | Bytecode::BitOr
                | Bytecode::BitAnd
                | Bytecode::Xor
                | Bytecode::Shl
                | Bytecode::Shr
                | Bytecode::Or
                | Bytecode::And
                | Bytecode::Not
                | Bytecode::Lt
                | Bytecode::Gt
                | Bytecode::Le
                | Bytecode::Ge
                | Bytecode::Eq
                | Bytecode::Neq
                | Bytecode::LdTrue
                | Bytecode::LdFalse
                | Bytecode::LdU8(_)
                | Bytecode::LdU16(_)
                | Bytecode::LdU32(_)
                | Bytecode::LdU64(_)
                | Bytecode::LdU128(_)
                | Bytecode::LdI8(_)
                | Bytecode::LdI16(_)
                | Bytecode::LdI32(_)
                | Bytecode::LdI64(_)
                | Bytecode::LdI128(_)
                | Bytecode::LdI256(_)
                | Bytecode::LdU256(_)
                | Bytecode::CastU8
                | Bytecode::CastU16
                | Bytecode::CastU32
                | Bytecode::CastU64
                | Bytecode::CastU128
                | Bytecode::CastU256
                | Bytecode::CastI8
                | Bytecode::CastI16
                | Bytecode::CastI32
                | Bytecode::CastI64
                | Bytecode::CastI128
                | Bytecode::CastI256 => (),
            }
        }
        Ok(())
    }

    /// Returns an error if the cache contains "unsafe" variants which should only be thread-local.
    pub(crate) fn validate_cache_safety(&self) -> PartialVMResult<()> {
        if self.cache.iter().any(|i| match i {
            // These variants are allowed.
            PerInstructionCache::Nothing
            | PerInstructionCache::Pack(_)
            | PerInstructionCache::PackGeneric(_)
            | PerInstructionCache::TypeInfo(_, _) => false,
            // Variants below are not allowed.
            PerInstructionCache::Call(_, _) => true,
        }) {
            return Err(PartialVMError::new_invariant_violation(
                "Instruction cache contains unsafe variants!",
            ));
        }
        Ok(())
    }

    /// Extras pre-populated cache from the loaded function.
    pub(crate) fn new_for_execution(function: &LoadedFunction) -> Self {
        debug_assert!(function.function.instruction_cache.cache.len() == function.code_size());
        debug_assert!(function
            .function
            .instruction_cache
            .validate_cache_safety()
            .is_ok());

        function.function.instruction_cache.clone()
    }

    /// Returns the cached item at the current program counter.
    #[inline(always)]
    pub(crate) fn get(&mut self, pc: u16) -> &PerInstructionCache {
        &self.cache[pc as usize]
    }

    /// Sets the cached item at the current program counter.
    #[inline(always)]
    pub(crate) fn set(&mut self, pc: u16, item: PerInstructionCache) {
        self.cache[pc as usize] = item;
    }
}
