// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{frame::Frame, frame_type_cache::FrameTypeCache, LoadedFunction};
use fxhash::FxBuildHasher;
use hashbrown::HashMap;
use move_binary_format::{
    errors::{PartialVMError, PartialVMResult},
    file_format::Bytecode,
    safe_assert, safe_unwrap, safe_unwrap_err,
};
use move_core_types::{
    function::ClosureMask,
    vm_status::{sub_status::unknown_invariant_violation::EREFERENCE_SAFETY_FAILURE, StatusCode},
};
use move_vm_types::loaded_data::runtime_types::Type;
use std::{collections::BTreeSet, slice};

type UnorderedMap<K, V> = HashMap<K, V, FxBuildHasher>;

macro_rules! ref_check_failure {
    ($msg:ident) => {
        Err(
            PartialVMError::new(StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR)
                .with_message($msg)
                .with_sub_status(EREFERENCE_SAFETY_FAILURE),
        )
    };
}

pub(crate) trait RuntimeRefCheck {
    fn pre_execution_transition(
        frame: &Frame,
        instruction: &Bytecode,
        ref_state: &mut RefCheckState,
    ) -> PartialVMResult<()>;

    fn post_execution_transition(
        frame: &Frame,
        instruction: &Bytecode,
        ref_state: &mut RefCheckState,
        ty_cache: &mut FrameTypeCache,
    ) -> PartialVMResult<()>;

    fn core_call_transition(
        num_params: usize,
        num_locals: usize,
        mask: ClosureMask,
        ref_state: &mut RefCheckState,
    ) -> PartialVMResult<()>;

    fn init_entry(function: &LoadedFunction, ref_state: &mut RefCheckState) -> PartialVMResult<()>;
}

pub(crate) struct NoRuntimeRefCheck;
pub(crate) struct FullRuntimeRefCheck;

impl RuntimeRefCheck for NoRuntimeRefCheck {
    fn pre_execution_transition(
        _frame: &Frame,
        _instruction: &Bytecode,
        _ref_state: &mut RefCheckState,
    ) -> PartialVMResult<()> {
        Ok(())
    }

    fn post_execution_transition(
        _frame: &Frame,
        _instruction: &Bytecode,
        _ref_state: &mut RefCheckState,
        _ty_cache: &mut FrameTypeCache,
    ) -> PartialVMResult<()> {
        Ok(())
    }

    fn core_call_transition(
        _num_params: usize,
        _num_locals: usize,
        _mask: ClosureMask,
        _ref_state: &mut RefCheckState,
    ) -> PartialVMResult<()> {
        Ok(())
    }

    fn init_entry(
        _function: &LoadedFunction,
        _ref_state: &mut RefCheckState,
    ) -> PartialVMResult<()> {
        Ok(())
    }
}

impl RuntimeRefCheck for FullRuntimeRefCheck {
    fn pre_execution_transition(
        frame: &Frame,
        instruction: &Bytecode,
        ref_state: &mut RefCheckState,
    ) -> PartialVMResult<()> {
        use Bytecode::*;
        match instruction {
            Call(_) | CallGeneric(_) | Branch(_) => {
                // `Call` and `CallGeneric` are handled by calling `core_call_transition` elsewhere
            },
            BrFalse(_) | BrTrue(_) | CallClosure(_) | Abort => {
                // remove the top value from the shadow stack
                let _ = ref_state.pop_from_shadow_stack()?;
            },
            Ret => {
                ref_state.return_(frame.function.return_tys().len())?;
            },
            StLoc(_)
            | Pop
            | LdU8(_)
            | LdU16(_)
            | LdU32(_)
            | LdU64(_)
            | LdU128(_)
            | LdU256(_)
            | LdTrue
            | LdFalse
            | LdConst(_)
            | CopyLoc(_)
            | MoveLoc(_)
            | MutBorrowLoc(_)
            | ImmBorrowLoc(_)
            | ImmBorrowField(_)
            | MutBorrowField(_)
            | ImmBorrowFieldGeneric(_)
            | MutBorrowFieldGeneric(_)
            | PackClosure(..)
            | PackClosureGeneric(..)
            | Pack(_)
            | PackGeneric(_)
            | Unpack(_)
            | UnpackGeneric(_)
            | ReadRef
            | WriteRef
            | CastU8
            | CastU16
            | CastU32
            | CastU64
            | CastU128
            | CastU256
            | Add
            | Sub
            | Mul
            | Mod
            | Div
            | BitOr
            | BitAnd
            | Xor
            | Or
            | And
            | Shl
            | Shr
            | Lt
            | Le
            | Gt
            | Ge
            | Eq
            | Neq
            | MutBorrowGlobal(_)
            | ImmBorrowGlobal(_)
            | MutBorrowGlobalGeneric(_)
            | ImmBorrowGlobalGeneric(_)
            | Exists(_)
            | ExistsGeneric(_)
            | MoveTo(_)
            | MoveToGeneric(_)
            | MoveFrom(_)
            | MoveFromGeneric(_)
            | FreezeRef
            | Nop
            | Not
            | VecPack(_, _)
            | VecLen(_)
            | VecImmBorrow(_)
            | VecMutBorrow(_)
            | VecPushBack(_)
            | VecPopBack(_)
            | VecUnpack(_, _)
            | VecSwap(_)
            | PackVariant(_)
            | PackVariantGeneric(_)
            | UnpackVariant(_)
            | UnpackVariantGeneric(_)
            | TestVariant(_)
            | TestVariantGeneric(_)
            | MutBorrowVariantField(_)
            | MutBorrowVariantFieldGeneric(_)
            | ImmBorrowVariantField(_)
            | ImmBorrowVariantFieldGeneric(_) => {
                // handled in `post_execution_transition`
            },
        };
        Ok(())
    }

    fn post_execution_transition(
        frame: &Frame,
        instruction: &Bytecode,
        ref_state: &mut RefCheckState,
        ty_cache: &mut FrameTypeCache,
    ) -> PartialVMResult<()> {
        use Bytecode::*;
        match instruction {
            Pop => {
                // remove the top value from the shadow stack
                // if the removed value is a reference, purge it from the ref-to-apt bi-directional mapping
                let top = ref_state.pop_from_shadow_stack()?;
                if let Value::Ref(ref_id) = top {
                    ref_state.purge_reference(ref_id)?;
                }
            },
            Ret => {
                // not reachable here

                // if any of the returned values is a reference, it must be derived from a special APT root
                // node representing the value behind a reference parameter
                // otherwise, it is an invariant violation
                // they also must not be poisoned

                // for each returned value that is:
                // - a mutable reference: get the corresponding APT node and lock it and its descendants
                //   with exclusive lock
                // - an immutable reference: get the corresponding APT node and lock it and its descendants
                //   with shared lock
                // if when obtaining exclusive or shared lock, the node is already locked with exclusive lock,
                // then it is an invariant violation

                // the access path of any returned reference must be used to transform the reference
                // w.r.t the caller's APT

                // clean up the frame-specific data structures
            },
            BrTrue(_) | BrFalse(_) | Branch(_) => {
                // not reachable here
            },
            CastU8 | CastU16 | CastU32 | CastU64 | CastU128 | CastU256 | Not | Nop | Exists(_)
            | ExistsGeneric(_) => {
                // no-op
            },
            LdU8(_) | LdU16(_) | LdU32(_) | LdU64(_) | LdU128(_) | LdU256(_) | LdConst(_)
            | LdTrue | LdFalse => {
                // push a non-reference value onto the shadow stack
                ref_state.push_non_refs_to_shadow_stack(1);
            },
            CopyLoc(index) => {
                // if the local value in the shadow list at the `index` is non-ref:
                //   push it onto the shadow stack
                //   poison all mutable references to the value subtree in the APT
                //   (value subtree would be rooted at the local's value, so no need to go to ancestors)
                // else (it is a reference):
                //   generate a new reference to the value behind the APT node
                //   add it to the ref-to-apt bi-directional mapping
                //   push the new reference onto the shadow stack
                //   should this be considered an access of the reference value (??)
                //   thus, access checks (based on whether the reference is poisoned or not) should be performed
                //   if it is not considered an access, then the poisoning should be copied over to the new reference
                ref_state.copy_loc(*index)?;
            },
            MoveLoc(index) => {
                // push the local value in the shadow list at `index` onto the shadow stack

                // if the value is a non-ref:
                //   get the APT node corresponding to the local value at `index`
                //   poison all refs to the subtree rooted at this node

                // replace the local value in the shadow list with non-ref value (or None?)

                // this should not be considered an access of the reference value?
                // because it is okay to move a poisoned reference to the stack and then pop it
                ref_state.move_loc(*index)?;
            },
            StLoc(index) => {
                // if the value stored at `index` is not a reference, then we need to:
                //   get the APT node corresponding to the local value at `index`
                //   poison all refs to the subtree rooted at this node
                //   this is because the previous value at `index` is being overwritten
                //   and references to the previous value should not be used anymore
                // if the value stored at `index` is a reference, then we need to:
                //   purge the ref-to-apt bi-directional mapping for the old reference

                // pop the top value from the shadow stack
                // store it in the shadow list at the `index`

                // this should not be considered an access of the reference value?
                // it is okay to overwrite a poisoned reference in the shadow list
                ref_state.st_loc(*index)?;
            },
            Call(_) => {
                // this check should be done in `make_call_frame` as this is not reachable

                // for each arg value that is:
                // - a mutable reference: get the corresponding APT node and lock it and its descendants
                //   with exclusive lock, perform a destructive update on the node
                // - an immutable reference: get the corresponding APT node and lock it and its descendants
                //   with shared lock
                // if when obtaining exclusive or shared lock, the node is already locked with exclusive lock,
                // then it is an invariant violation
                // check if any of the reference arguments are poisoned (invariant violation) or not

                // pop the call's args from the shadow stack

                // setup the ref check state for the new frame:
                // - create a new shadow list of locals with ref/non-ref values
                // - for each arg value that is a reference:
                //   - create a new root of APT corresponding to the value behind the reference
                //   - add it to the ref-to-apt bi-directional mapping
                //   - place it in the appropriate slot in the shadow list of locals
                // - initialize the rest of the shadow list of locals with non-ref values
            },
            CallGeneric(_) => {
                // same as `Call`
            },
            Pack(index) => {
                let num_fields = frame.field_count(*index).into();
                // pop `num_fields` values from the shadow stack (these should all be non-ref values)
                // push a new non-ref value onto the shadow stack
                ref_state.pop_many_from_shadow_stack(num_fields)?;
                ref_state.push_non_refs_to_shadow_stack(1);
            },
            PackGeneric(index) => {
                let num_fields = frame.field_instantiation_count(*index).into();
                // pop `num_fields` values from the shadow stack (these should all be non-ref values)
                // push a new non-ref value onto the shadow stack
                ref_state.pop_many_from_shadow_stack(num_fields)?;
                ref_state.push_non_refs_to_shadow_stack(1);
            },
            PackVariant(index) => {
                let struct_variant_info = frame.get_struct_variant_at(*index);
                let num_fields = struct_variant_info.field_count.into();
                // pop `num_fields` values from the shadow stack (these should all be non-ref values)
                // push a new non-ref value onto the shadow stack
                ref_state.pop_many_from_shadow_stack(num_fields)?;
                ref_state.push_non_refs_to_shadow_stack(1);
            },
            PackVariantGeneric(index) => {
                let struct_variant_info = frame.get_struct_variant_instantiation_at(*index);
                let num_fields = struct_variant_info.field_count.into();
                // pop `num_fields` values from the shadow stack (these should all be non-ref values)
                // push a new non-ref value onto the shadow stack
                ref_state.pop_many_from_shadow_stack(num_fields)?;
                ref_state.push_non_refs_to_shadow_stack(1);
            },
            Unpack(index) => {
                // you can only unpack a value that is pushed onto the stack
                // we need to maintain the invariant that a value on a stack has no pending references

                // pop the top value from the shadow stack
                ref_state.pop_from_shadow_stack()?;
                let num_fields = frame.field_count(*index).into();
                // push `num_fields` non-ref values onto the shadow stack
                ref_state.push_non_refs_to_shadow_stack(num_fields);
            },
            UnpackGeneric(index) => {
                // pop the top value from the shadow stack
                ref_state.pop_from_shadow_stack()?;
                let num_fields = frame.field_instantiation_count(*index).into();
                // push `num_fields` non-ref values onto the shadow stack
                ref_state.push_non_refs_to_shadow_stack(num_fields);
            },
            UnpackVariant(index) => {
                // pop the top value from the shadow stack
                ref_state.pop_from_shadow_stack()?;
                let struct_variant_info = frame.get_struct_variant_at(*index);
                let num_fields = struct_variant_info.field_count.into();
                // push `num_fields` non-ref values onto the shadow stack
                ref_state.push_non_refs_to_shadow_stack(num_fields);
            },
            UnpackVariantGeneric(index) => {
                // pop the top value from the shadow stack
                ref_state.pop_from_shadow_stack()?;
                let struct_variant_info = frame.get_struct_variant_instantiation_at(*index);
                let num_fields = struct_variant_info.field_count.into();
                // push `num_fields` non-ref values onto the shadow stack
                ref_state.push_non_refs_to_shadow_stack(num_fields);
            },
            TestVariant(_) => {
                // pop the top value from the shadow stack, which should be a reference
                // this should be considered an access of the reference value: so we should
                // check if the reference is poisoned (invariant violation) or not
                // purge it from the ref-to-apt bi-directional mapping

                // push a non-ref value onto the shadow stack
                ref_state.pop_ref_push_non_ref()?;
            },
            TestVariantGeneric(_) => {
                // pop the top value from the shadow stack, which should be a reference
                // this should be considered an access of the reference value: so we should
                // check if the reference is poisoned (invariant violation) or not
                // purge it from the ref-to-apt bi-directional mapping

                // push a non-ref value onto the shadow stack
                ref_state.pop_ref_push_non_ref()?;
            },
            ReadRef => {
                // pop the top value from the shadow stack, which should be a reference
                // this should be considered an access of the reference value: so we should
                // check if the reference is poisoned (invariant violation) or not
                // purge it from the ref-to-apt bi-directional mapping

                // push a non-ref value onto the shadow stack
                ref_state.pop_ref_push_non_ref()?;
            },
            WriteRef => {
                // pop the top 2 values from the shadow stack

                // the topmost value should be a mutable reference
                // this is considered as an access: check if the reference is poisoned (invariant violation) or not

                // perform a "destructive update":
                // get the APT node corresponding to the reference
                // poison all immutable references to the subtree rooted at this node
                // poison all the immutable references to the node's ancestors
                // poison all the mutable references to the strict descendants of the node
                // TODO: do we also need to do this on:
                // - function calls?
                // - vector operations (like pop back)?

                // purge it from the ref-to-apt bi-directional mapping
                ref_state.write_ref()?;
            },
            FreezeRef => {
                // pop the top value from the shadow stack, which should be a mutable reference
                // this should be considered an access of the reference value: so we should
                // check if the reference is poisoned (invariant violation) or not

                // poison all mutable references to ancestors, self, and descendants of the
                // corresponding APT node

                // purge it from the ref-to-apt bi-directional mapping

                // push a newly created immutable reference onto the shadow stack
                // update the ref-to-apt bi-directional mapping with the new immutable reference

                ref_state.freeze_ref()?;
            },
            MutBorrowLoc(index) => {
                // ensure an APT root node exists corresponding to the local value at `index`
                // create a new mutable reference to that APT root node
                // add it to the ref-to-apt bi-directional mapping

                // push the new mutable reference onto the shadow stack

                // creation of a reference to a local should not create invariant violations
                ref_state.borrow_loc(*index, true)?;
            },
            ImmBorrowLoc(index) => {
                // same as `MutBorrowLoc`, but for an immutable reference
                ref_state.borrow_loc(*index, false)?;
            },
            MutBorrowField(index) => {
                // pop the top value from the shadow stack, which should be a mutable reference
                // this should be considered an access of the reference value: so we should
                // check if the reference is poisoned (invariant violation) or not

                // this reference should be purged from the ref-to-apt bi-directional mapping

                // create the edge label for the child node
                let label = frame.field_offset(*index);
                // get the APT node corresponding to the reference popped from the shadow stack
                // get/create its child node with the label
                // create a new mutable reference to the child node
                // add it to the ref-to-apt bi-directional mapping

                // push the new mutable reference onto the shadow stack
                ref_state.borrow_child_with_label(label, true)?;
            },
            MutBorrowVariantField(index) => {
                // pop the top value from the shadow stack, which should be a mutable reference
                // this should be considered an access of the reference value: so we should
                // check if the reference is poisoned (invariant violation) or not

                // create the edge label for the child node
                let field_info = frame.variant_field_info_at(*index);
                let label = field_info.offset;
                // get the APT node corresponding to the reference popped from the shadow stack
                // get/create its child node with the label
                // create a new mutable reference to the child node
                // add it to the ref-to-apt bi-directional mapping

                // push the new mutable reference onto the shadow stack
                ref_state.borrow_child_with_label(label, true)?;
            },
            MutBorrowFieldGeneric(index) => {
                // pop the top value from the shadow stack, which should be a mutable reference
                // this should be considered an access of the reference value: so we should
                // check if the reference is poisoned (invariant violation) or not

                // this reference should be purged from the ref-to-apt bi-directional mapping

                // create the edge label for the child node
                let label = frame.field_instantiation_offset(*index);
                // get the APT node corresponding to the reference popped from the shadow stack
                // get/create its child node with the label
                // create a new mutable reference to the child node
                // add it to the ref-to-apt bi-directional mapping

                // push the new mutable reference onto the shadow stack
                ref_state.borrow_child_with_label(label, true)?;
            },
            MutBorrowVariantFieldGeneric(index) => {
                // pop the top value from the shadow stack, which should be a mutable reference
                // this should be considered an access of the reference value: so we should
                // check if the reference is poisoned (invariant violation) or not

                // this reference should be purged from the ref-to-apt bi-directional mapping

                // create the edge label for the child node
                let field_info = frame.variant_field_instantiation_info_at(*index);
                let label = field_info.offset;
                // get the APT node corresponding to the reference popped from the shadow stack
                // get/create its child node with the label
                // create a new mutable reference to the child node
                // add it to the ref-to-apt bi-directional mapping

                // push the new mutable reference onto the shadow stack
                ref_state.borrow_child_with_label(label, true)?;
            },
            ImmBorrowField(index) => {
                // same as `MutBorrowField`, but for an immutable reference
                let label = frame.field_offset(*index);
                ref_state.borrow_child_with_label(label, false)?;
            },
            ImmBorrowVariantField(index) => {
                // same as `MutBorrowVariantField`, but for an immutable reference
                let field_info = frame.variant_field_info_at(*index);
                let label = field_info.offset;
                ref_state.borrow_child_with_label(label, false)?;
            },
            ImmBorrowFieldGeneric(index) => {
                // same as `MutBorrowFieldGeneric`, but for an immutable reference
                let label = frame.field_instantiation_offset(*index);
                ref_state.borrow_child_with_label(label, false)?;
            },
            ImmBorrowVariantFieldGeneric(index) => {
                // same as `MutBorrowVariantFieldGeneric`, but for an immutable reference
                let field_info = frame.variant_field_instantiation_info_at(*index);
                let label = field_info.offset;
                ref_state.borrow_child_with_label(label, false)?;
            },
            MutBorrowGlobal(index) => {
                // pop the top value from the shadow stack, which should be a non-ref value

                let struct_ty = frame.get_struct_ty(*index);
                // create an APT root corresponding to the global type
                // if one already exists, then check the ref-to-apt bi-directional mapping
                // to see if it has any references, then it is an invariant violation

                // create a new mutable reference to the APT root node
                // add it to the ref-to-apt bi-directional mapping

                // push the new mutable reference onto the shadow stack
                ref_state.borrow_global(struct_ty, true)?;
            },
            MutBorrowGlobalGeneric(index) => {
                // pop the top value from the shadow stack, which should be a non-ref value

                let struct_ty = ty_cache.get_struct_type(*index, frame)?.0;
                // create an APT root corresponding to the global type
                // if one already exists, then check the ref-to-apt bi-directional mapping
                // to see if it has any references, then it is an invariant violation

                // create a new mutable reference to the APT root node
                // add it to the ref-to-apt bi-directional mapping

                // push the new mutable reference onto the shadow stack
                ref_state.borrow_global(struct_ty.clone(), true)?;
            },
            ImmBorrowGlobal(index) => {
                // pop the top value from the shadow stack, which should be a non-ref value

                let struct_ty = frame.get_struct_ty(*index);
                // create an APT root corresponding to the global type
                // if one already exists, then check the ref-to-apt bi-directional mapping
                // to see if it has any mutable references, then it is an invariant violation

                // create a new immutable reference to the APT root node
                // add it to the ref-to-apt bi-directional mapping

                // push the new immutable reference onto the shadow stack
                ref_state.borrow_global(struct_ty, false)?;
            },
            ImmBorrowGlobalGeneric(index) => {
                // pop the top value from the shadow stack, which should be a non-ref value

                let struct_ty = ty_cache.get_struct_type(*index, frame)?.0;
                // create an APT root corresponding to the global type
                // if one already exists, then check the ref-to-apt bi-directional mapping
                // to see if it has any mutable references, then it is an invariant violation

                // create a new immutable reference to the APT root node
                // add it to the ref-to-apt bi-directional mapping

                // push the new immutable reference onto the shadow stack
                ref_state.borrow_global(struct_ty.clone(), false)?;
            },
            Add | Sub | Mul | Mod | Div | BitOr | BitAnd | Xor | Or | And | Lt | Gt | Le | Ge
            | Shl | Shr => {
                // pop the top value on the shadow stack (which should be a non-ref value)
                let _ = ref_state.pop_from_shadow_stack()?;
            },
            Eq | Neq => {
                // pop the top 2 values from the shadow stack (which can be ref or non-ref values)
                // push a non-ref value onto the shadow stack
                ref_state.pop_many_from_shadow_stack(2)?;
                ref_state.push_non_refs_to_shadow_stack(1);
            },
            Abort => {
                // we should not be able to reach here
                // make it an invariant violation
                let msg = "Abort should not be reachable".to_string();
                return ref_check_failure!(msg);
            },
            MoveFrom(index) => {
                // pop the top value from the shadow stack, which should be a non-ref value
                let struct_ty = frame.get_struct_ty(*index);
                // poison all the references pertaining to subtree rooted global value

                // push a new non-ref value onto the shadow stack
                ref_state.move_from(struct_ty)?;
            },
            MoveFromGeneric(index) => {
                let struct_ty = ty_cache.get_struct_type(*index, frame)?.0;
                // same as `MoveFrom`, but for a generic type
                ref_state.move_from(struct_ty.clone())?;
            },
            MoveTo(_) => {
                // pop the top value from the shadow stack, which should be a non-ref value
                // pop the top value from the shadow stack, which should be an immutable reference

                // there is no overwriting of the global value (move_to only succeeds if the global
                // value is not already set), so we don't need to poison its references

                // the reference popped should be purged from the ref-to-apt bi-directional mapping
                // we should also check if the reference is poisoned (invariant violation) or not
                ref_state.move_to()?;
            },
            MoveToGeneric(_) => {
                // same as `MoveTo`, but for a generic type
                ref_state.move_to()?;
            },
            VecPack(_, n) => {
                // pop `n` values from the shadow stack (these should all be non-ref values)
                // push a new non-ref value onto the shadow stack
                ref_state.pop_many_from_shadow_stack(safe_unwrap_err!((*n).try_into()))?;
                ref_state.push_non_refs_to_shadow_stack(1);
            },
            VecLen(_) => {
                // pop the top value from the shadow stack, which should be a ref
                // purge it from the ref-to-apt bi-directional mapping
                // this should be considered an access of the reference value: so we should
                // check if the reference is poisoned (invariant violation) or not

                // push a non-ref value onto the shadow stack
                ref_state.vec_len()?;
            },
            VecImmBorrow(_) => {
                // pop the top value from the shadow stack, which should be a non-ref value
                // pop the next value from the shadow stack, which should be a ref
                // this should be considered an access of the reference value: so we should
                // check if the reference is poisoned (invariant violation) or not

                // this reference should be purged from the ref-to-apt bi-directional mapping

                // create the edge label for the child node
                // we ignore the actual offset of the vector element, and instead always use `0`
                // as an abstraction
                // this should not cause conflict with struct field labels due to type checking

                // get the APT node corresponding to the reference popped from the shadow stack
                // get/create its child node with the label
                // create a new immutable reference to the child node
                // add it to the ref-to-apt bi-directional mapping

                // push the new immutable reference onto the shadow stack
                ref_state.vec_borrow(false)?;
            },
            VecMutBorrow(_) => {
                // same as `VecImmBorrow`, but for a mutable reference
                ref_state.vec_borrow(true)?;
            },
            VecPushBack(_) => {
                // pop the top value from the shadow stack, which should be a non-ref value
                // pop the next value from the shadow stack, which should be a mutable ref
                // this should be considered an access of the reference value: so we should
                // check if the reference is poisoned (invariant violation) or not

                // this reference should be purged from the ref-to-apt bi-directional mapping

                // we don't need to do a destructive update here, because
                // none of the existing references to the vector elements are affected?
                ref_state.vec_push_back()?;
            },
            VecPopBack(_) => {
                // pop the top value from the shadow stack, which should be a mutable reference
                // this should be considered an access of the reference value: so we should
                // check if the reference is poisoned (invariant violation) or not

                // this reference should be purged from the ref-to-apt bi-directional mapping

                // we should consider this a destructive update on the mutable reference
                // (thus, doing the same operations as in `WriteRef`)

                // push a non-ref value onto the shadow stack
                ref_state.vec_pop_back()?;
            },
            VecUnpack(_, n) => {
                // pop the top value from the shadow stack, which should be a non-ref value
                // push `n` non-ref values onto the shadow stack
                let _ = ref_state.pop_from_shadow_stack()?;
                ref_state.push_non_refs_to_shadow_stack(safe_unwrap_err!((*n).try_into()));
            },
            VecSwap(_) => {
                // pop the top 2 values from the shadow stack, which should be non-ref values
                // pop the next value from the shadow stack, which should be a mutable reference
                // this should be considered an access of the reference value: so we should
                // check if the reference is poisoned (invariant violation) or not

                // this reference should be purged from the ref-to-apt bi-directional mapping

                // swapping the elements is the same as moving the values around
                // so we need to perform a destructive update on the mutable reference
                ref_state.vec_swap()?;
            },
            PackClosure(_, mask) => {
                let captured = mask.captured_count();
                // pop `captured` values from the shadow stack (these should all be non-ref values)
                // push a new non-ref value onto the shadow stack
                // note: we are not checking that values captured are non-ref values, as we expect
                // the runtime type checks to catch validate this
                ref_state.pop_many_from_shadow_stack(captured.into())?;
                ref_state.push_non_refs_to_shadow_stack(1);
            },
            PackClosureGeneric(_, mask) => {
                let captured = mask.captured_count();
                // pop `captured` values from the shadow stack (these should all be non-ref values)
                // push a new non-ref value onto the shadow stack
                // note: we are not checking that values captured are non-ref values, as we expect
                // the runtime type checks to catch validate this
                ref_state.pop_many_from_shadow_stack(captured.into())?;
                ref_state.push_non_refs_to_shadow_stack(1);
            },
            CallClosure(_) => {
                // similar to `Call`
            },
        };
        Ok(())
    }

    fn core_call_transition(
        num_params: usize,
        num_locals: usize,
        mask: ClosureMask,
        ref_state: &mut RefCheckState,
    ) -> PartialVMResult<()> {
        ref_state.core_call(num_params, num_locals, mask)
    }

    fn init_entry(function: &LoadedFunction, ref_state: &mut RefCheckState) -> PartialVMResult<()> {
        let num_locals = function.local_tys().len();
        let mut mut_ref_indexes = vec![];
        let mut immut_ref_indexes = vec![];
        for (i, ty) in function.param_tys().iter().enumerate() {
            match ty {
                Type::Reference(_) => immut_ref_indexes.push(i),
                Type::MutableReference(_) => mut_ref_indexes.push(i),
                _ => continue,
            }
        }
        let caller_ref_param_map = UnorderedMap::with_hasher(FxBuildHasher::default());
        ref_state.push_new_frame(
            num_locals,
            mut_ref_indexes,
            immut_ref_indexes,
            caller_ref_param_map,
        )?;

        Ok(())
    }
}

/// Represents a value in the shadow stack or shadow locals list.
#[derive(Clone, Copy)]
enum Value {
    /// A non-reference value
    NonRef,
    /// A reference value
    Ref(RefID),
}

/// Unique (within a frame) identifier for a reference.
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
struct RefID(usize);

type NodeID = usize;

/// Access Path Tree (APT), representing the access paths corresponding
/// to a value (local, global, or value behind a reference parameter) in a frame.
struct AccessPathTree {
    nodes: Vec<AccessPathTreeNode>,
}

impl AccessPathTree {
    /// Create a new Access Path Tree (APT) with a fresh root node.
    fn new() -> Self {
        Self {
            nodes: vec![AccessPathTreeNode::fresh_root()],
        }
    }

    fn make_new_node(&mut self, parent_id: NodeID, label: usize) -> NodeID {
        let new_node = AccessPathTreeNode::fresh_node(parent_id, label);
        self.nodes.push(new_node);
        self.nodes.len() - 1
    }

    fn get_or_create_child_node(
        &mut self,
        parent_id: NodeID,
        label: usize,
    ) -> PartialVMResult<NodeID> {
        let parent_node = safe_unwrap!(self.nodes.get_mut(parent_id));
        let child_id = parent_node.children.get(label);
        let resize: bool;
        if let Some(child_id) = child_id {
            // child slot exists
            if let Some(child_id) = child_id {
                // child slot is occupied, return its ID
                return Ok(*child_id);
            } else {
                // child slot is empty, but no need to resize
                resize = false;
            }
        } else {
            // child slot does not exist, we need to resize
            resize = true;
        }
        if resize {
            parent_node
                .children
                .resize(safe_unwrap!(label.checked_add(1)), None);
        }

        // Create a new child node, and update the parent's children slot.
        let new_child_id = self.make_new_node(parent_id, label);
        // Re-borrow to satisfy Rust's borrow checker.
        let parent_node = safe_unwrap!(self.nodes.get_mut(parent_id));
        *safe_unwrap!(parent_node.children.get_mut(label)) = Some(new_child_id);
        Ok(new_child_id)
    }

    fn visit_strict_descendants<F>(&mut self, node_id: NodeID, mut f: F) -> PartialVMResult<()>
    where
        F: FnMut(&mut AccessPathTreeNode) -> PartialVMResult<()>,
    {
        // Visit all descendants of the node, excluding the node itself.
        // We need to collect the descendants first, because we are mutating the tree while visiting.
        for descendant in self
            .get_descendants_iter(node_id)
            .skip(1)
            .collect::<Vec<_>>()
        {
            let node = safe_unwrap!(self.nodes.get_mut(descendant));
            f(node)?;
        }
        Ok(())
    }

    fn visit_self<F>(&mut self, node_id: NodeID, mut f: F) -> PartialVMResult<()>
    where
        F: FnMut(&mut AccessPathTreeNode) -> PartialVMResult<()>,
    {
        let node = safe_unwrap!(self.nodes.get_mut(node_id));
        f(node)?;
        Ok(())
    }

    fn visit_strict_ancestors<F>(&mut self, node_id: NodeID, mut f: F) -> PartialVMResult<()>
    where
        F: FnMut(&mut AccessPathTreeNode) -> PartialVMResult<()>,
    {
        // Visit all ancestors of the node, excluding the node itself.
        let mut current_node_id = node_id;
        while let Some((parent_id, _label)) = safe_unwrap!(self.nodes.get(current_node_id)).parent {
            let parent_node = safe_unwrap!(self.nodes.get_mut(parent_id));
            f(parent_node)?;
            current_node_id = parent_id;
        }
        Ok(())
    }

    fn get_access_path_from_root(&self, node_id: NodeID) -> PartialVMResult<Vec<usize>> {
        let mut current_node_id = node_id;
        let mut path = Vec::new();
        while let Some((parent_id, label)) = safe_unwrap!(self.nodes.get(current_node_id)).parent {
            current_node_id = parent_id;
            path.push(label);
        }
        path.reverse();
        Ok(path)
    }

    fn get_descendants_iter(&self, node_id: NodeID) -> DescendantsTraversalIter {
        DescendantsTraversalIter {
            stack: vec![node_id],
            access_path_tree: self,
        }
    }
}

struct DescendantsTraversalIter<'a> {
    stack: Vec<NodeID>,
    access_path_tree: &'a AccessPathTree,
}

impl<'a> Iterator for DescendantsTraversalIter<'a> {
    type Item = NodeID;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(node_id) = self.stack.pop() {
            // When processing a node, its children are added to the stack in reverse order.
            if let Some(node) = self.access_path_tree.nodes.get(node_id) {
                self.stack.extend(node.children.iter().rev().flatten());
            } // else should be unreachable, as we should not have invalid node IDs
            Some(node_id)
        } else {
            None
        }
    }
}

/// A node in the Access Path Tree (APT).
struct AccessPathTreeNode {
    /// Parent node and edge label (`None` for root nodes)
    parent: Option<(NodeID, usize)>,
    /// Child nodes, edge label is the index in this vector
    children: Vec<Option<NodeID>>,
    /// References to this node
    refs: BTreeSet<RefID>,
    /// Current lock on this node
    lock: Option<Lock>,
}

impl AccessPathTreeNode {
    fn fresh_root() -> Self {
        Self {
            parent: None,
            children: Vec::new(),
            refs: BTreeSet::new(),
            lock: None,
        }
    }

    fn fresh_node(parent_id: NodeID, label: usize) -> Self {
        Self {
            parent: Some((parent_id, label)),
            children: Vec::new(),
            refs: BTreeSet::new(),
            lock: None,
        }
    }
}

/// Represents the type of lock on an Access Path Tree (APT) node.
#[derive(Copy, Clone, PartialEq, Eq)]
enum Lock {
    /// Shared lock - multiple shared locks on the same node are allowed
    Shared,
    /// Exclusive lock - conflicts with any other lock
    Exclusive,
}

/// Different kinds of root nodes in the Access Path Tree (APT).
#[derive(Clone)]
enum APTRoot {
    /// Root representing a local (non-ref) value
    Local { index: usize },
    /// Root representing a global type
    Global { type_: Type },
    /// Special node representing the value behind a reference parameter
    ReferenceParameter { param_index: usize },
}

/// Collection of APT roots information for a frame.
struct APTRootsInfo {
    /// Mapping from local index to the corresponding APT
    locals: UnorderedMap<usize, AccessPathTree>,
    /// Mapping from global type to the corresponding APT
    globals: UnorderedMap<Type, AccessPathTree>,
    /// Mapping from reference parameter index to the corresponding APT
    reference_params: UnorderedMap<usize, AccessPathTree>,
}

/// The root of the Access Path Tree (APT) and the node ID within that tree.
#[derive(Clone)]
struct QualifiedNodeID {
    root: APTRoot,
    node_id: NodeID,
}

impl QualifiedNodeID {
    fn local_root(index: usize) -> Self {
        Self {
            root: APTRoot::Local { index },
            node_id: 0, // root is always at 0
        }
    }

    fn global_root(type_: Type) -> Self {
        Self {
            root: APTRoot::Global { type_ },
            node_id: 0, // root is always at 0
        }
    }

    fn reference_param_root(param_index: usize) -> Self {
        Self {
            root: APTRoot::ReferenceParameter { param_index },
            node_id: 0, // root is always at 0
        }
    }
}

impl APTRootsInfo {
    fn get_access_path_tree(&self, root: &APTRoot) -> PartialVMResult<&AccessPathTree> {
        match root {
            APTRoot::Local { index } => Ok(safe_unwrap!(self.locals.get(index))),
            APTRoot::Global { type_ } => Ok(safe_unwrap!(self.globals.get(type_))),
            APTRoot::ReferenceParameter { param_index } => {
                Ok(safe_unwrap!(self.reference_params.get(param_index)))
            },
        }
    }

    fn get_mut_access_path_tree(&mut self, root: &APTRoot) -> PartialVMResult<&mut AccessPathTree> {
        Ok(safe_unwrap!(self.maybe_get_mut_access_path_tree(root)))
    }

    fn maybe_get_mut_access_path_tree(&mut self, root: &APTRoot) -> Option<&mut AccessPathTree> {
        match root {
            APTRoot::Local { index } => self.locals.get_mut(index),
            APTRoot::Global { type_ } => self.globals.get_mut(type_),
            APTRoot::ReferenceParameter { param_index } => {
                self.reference_params.get_mut(param_index)
            },
        }
    }

    fn get_mut_access_path_tree_node(
        &mut self,
        node: &QualifiedNodeID,
    ) -> PartialVMResult<&mut AccessPathTreeNode> {
        let apt = self.get_mut_access_path_tree(&node.root)?;
        Ok(safe_unwrap!(apt.nodes.get_mut(node.node_id)))
    }
}

/// Per frame reference checking state.
struct FrameRefState {
    // - a shadow list of locals with ref/non-ref values
    // - access path tree (APT)
    //   - root of an APT can be one of:
    //     - local (non-ref) value
    //     - global (type) (we abstract over all addresses and only use types)
    //     - special node representing the value behind a reference parameter
    //   - nodes of an APT can be:
    //     - marked as locked (shared or exclusive)
    //     - exclusive lock conflicts with any other lock on the same node
    //     - multiple shared locks on the same node are allowed
    // - mapping from references to a node in the APT (and back), called ref-to-apt bi-directional mapping
    //   - per frame basis
    //   - references can be marked as poisoned
    //   - a poisoned reference should not be used (if used, it should lead to an invariant violation)
    // - each ref can be a mutable or immutable reference
    /// Shadow list of local values.
    shadow_locals: Vec<Value>,
    /// Roots of the Access Path Tree (APT) for this frame.
    apt_roots: APTRootsInfo,
    /// Mapping from references to their information.
    ref_table: UnorderedMap<RefID, ReferenceInfo>,
    /// Next available reference ID.
    next_ref_id: usize,
    /// Map the reference parameter's index to the APT node (in the caller's `FrameRefState`)
    /// corresponding to the reference parameter.
    caller_ref_param_map: UnorderedMap<usize, QualifiedNodeID>,
}

/// Filter for references when applying actions such as poisoning.
enum ReferenceFilter {
    /// Apply action to mutable references only
    MutOnly,
    /// Apply action to immutable references only
    ImmutOnly,
    /// Apply action to all references
    All,
}

enum VisitKind {
    /// Visit the node itself
    SelfOnly,
    /// Visit strict descendants of the node
    StrictDescendants,
    /// Visit strict ancestors of the node
    StrictAncestors,
}

impl FrameRefState {
    fn new(
        num_locals: usize,
        mut_ref_indexes: Vec<usize>,
        immut_ref_indexes: Vec<usize>,
        caller_ref_param_map: UnorderedMap<usize, QualifiedNodeID>,
    ) -> PartialVMResult<Self> {
        debug_assert!(
            num_locals >= mut_ref_indexes.len() + immut_ref_indexes.len(),
            "locals should be enough for all reference parameters"
        );
        debug_assert!(
            caller_ref_param_map
                .keys()
                .all(|index| mut_ref_indexes.contains(index) || immut_ref_indexes.contains(index)),
            "consistency check for caller_ref_param_map"
        );
        let mut this = Self {
            shadow_locals: vec![Value::NonRef; num_locals],
            apt_roots: APTRootsInfo {
                locals: UnorderedMap::with_hasher(FxBuildHasher::default()),
                globals: UnorderedMap::with_hasher(FxBuildHasher::default()),
                reference_params: UnorderedMap::with_hasher(FxBuildHasher::default()),
            },
            ref_table: UnorderedMap::with_hasher(FxBuildHasher::default()),
            next_ref_id: 0,
            caller_ref_param_map,
        };
        for index in mut_ref_indexes {
            let node_id = QualifiedNodeID::reference_param_root(index);
            this.ensure_reference_param_root_exists(index);
            let new_ref_id = this.make_new_ref_to_existing_node(node_id, true)?;
            *safe_unwrap!(this.shadow_locals.get_mut(index)) = Value::Ref(new_ref_id);
        }
        for index in immut_ref_indexes {
            let node_id = QualifiedNodeID::reference_param_root(index);
            this.ensure_reference_param_root_exists(index);
            let new_ref_id = this.make_new_ref_to_existing_node(node_id, false)?;
            *safe_unwrap!(this.shadow_locals.get_mut(index)) = Value::Ref(new_ref_id);
        }
        Ok(this)
    }

    /// Check if the reference has been poisoned.
    fn poison_check(&self, ref_: RefID) -> PartialVMResult<()> {
        let poisoned = safe_unwrap!(self.ref_table.get(&ref_)).poisoned;
        if poisoned {
            let msg = "Poisoned reference accessed".to_string();
            return ref_check_failure!(msg);
        }
        Ok(())
    }

    fn get_ref_info(&self, ref_: &RefID) -> PartialVMResult<&ReferenceInfo> {
        Ok(safe_unwrap!(self.ref_table.get(ref_)))
    }

    /// Poison the references related to the given `node`.
    fn poison_refs_of_node(
        &mut self,
        node: &QualifiedNodeID,
        visit_kind: VisitKind,
        filter: ReferenceFilter,
    ) -> PartialVMResult<()> {
        let Some(tree) = self.apt_roots.maybe_get_mut_access_path_tree(&node.root) else {
            // If the tree is not present, there are no references to poison.
            return Ok(());
        };
        let action = |node: &mut AccessPathTreeNode| {
            for ref_ in node.refs.iter() {
                let info = safe_unwrap!(self.ref_table.get_mut(ref_));
                match filter {
                    ReferenceFilter::All => {
                        info.poisoned = true;
                    },
                    ReferenceFilter::MutOnly if info.is_mutable => {
                        info.poisoned = true;
                    },
                    ReferenceFilter::ImmutOnly if !info.is_mutable => {
                        info.poisoned = true;
                    },
                    _ => {},
                }
            }
            Ok(())
        };
        match visit_kind {
            VisitKind::SelfOnly => tree.visit_self(node.node_id, action)?,
            VisitKind::StrictDescendants => tree.visit_strict_descendants(node.node_id, action)?,
            VisitKind::StrictAncestors => tree.visit_strict_ancestors(node.node_id, action)?,
        }
        Ok(())
    }

    fn destructive_write_via_mut_ref(&mut self, node: &QualifiedNodeID) -> PartialVMResult<()> {
        self.poison_refs_of_node(node, VisitKind::SelfOnly, ReferenceFilter::ImmutOnly)?;
        self.poison_refs_of_node(
            node,
            VisitKind::StrictDescendants,
            ReferenceFilter::ImmutOnly,
        )?;
        self.poison_refs_of_node(node, VisitKind::StrictAncestors, ReferenceFilter::ImmutOnly)?;

        self.poison_refs_of_node(node, VisitKind::StrictDescendants, ReferenceFilter::MutOnly)?;

        Ok(())
    }

    /// Lock the entire subtree rooted at the given `node` with the specified `lock`.
    /// If any node in the subtree is already exclusively locked, it returns an invariant error.
    fn lock_node_subtree(&mut self, node: &QualifiedNodeID, lock: Lock) -> PartialVMResult<()> {
        let tree = self.apt_roots.get_mut_access_path_tree(&node.root)?;
        let action = |node: &mut AccessPathTreeNode| {
            if let Some(node_lock) = node.lock {
                if lock == Lock::Exclusive || node_lock == Lock::Exclusive {
                    let msg = "Exclusive lock conflict".to_string();
                    return ref_check_failure!(msg);
                }
            }
            node.lock = Some(lock);
            Ok(())
        };
        tree.visit_self(node.node_id, action)?;
        tree.visit_strict_descendants(node.node_id, action)?;
        Ok(())
    }

    fn release_lock_node_subtree(&mut self, node: &QualifiedNodeID) -> PartialVMResult<()> {
        let tree = self.apt_roots.get_mut_access_path_tree(&node.root)?;
        let action = |node: &mut AccessPathTreeNode| {
            node.lock = None;
            Ok(())
        };
        tree.visit_self(node.node_id, action)?;
        tree.visit_strict_descendants(node.node_id, action)?;
        Ok(())
    }

    fn purge_reference(&mut self, ref_id: RefID) -> PartialVMResult<()> {
        let info = safe_unwrap!(self.ref_table.remove(&ref_id));
        let node = self.get_mut_access_path_tree_node(&info.apt_node)?;
        node.refs.remove(&ref_id);
        Ok(())
    }

    fn make_new_ref_to_existing_node(
        &mut self,
        qualified_node_id: QualifiedNodeID,
        is_mutable: bool,
    ) -> PartialVMResult<RefID> {
        let new_ref_id = RefID(self.next_ref_id);
        self.next_ref_id = safe_unwrap!(self.next_ref_id.checked_add(1));

        let apt_node = self.get_mut_access_path_tree_node(&qualified_node_id)?;
        // Connect the `apt_node` to the new reference.
        // We just made this `new_ref_id`, so it must not already exist in the `apt_node`'s refs.
        safe_assert!(apt_node.refs.insert(new_ref_id));

        // Connect the new reference to the `apt_node`.
        self.ref_table.insert(new_ref_id, ReferenceInfo {
            is_mutable,
            poisoned: false,
            apt_node: qualified_node_id,
        });

        Ok(new_ref_id)
    }

    /// Ensure that the local root exists for the given index.
    fn ensure_local_root_exists(&mut self, index: usize) {
        self.apt_roots
            .locals
            .entry(index)
            .or_insert_with(AccessPathTree::new);
    }

    /// Ensure that the global root exists for the given type.
    fn ensure_global_root_exists(&mut self, type_: Type) {
        self.apt_roots
            .globals
            .entry(type_)
            .or_insert_with(AccessPathTree::new);
    }

    /// Ensure that the reference parameter root exists for the given parameter index.
    fn ensure_reference_param_root_exists(&mut self, param_index: usize) {
        self.apt_roots
            .reference_params
            .entry(param_index)
            .or_insert_with(AccessPathTree::new);
    }

    fn get_or_create_descendant_node(
        &mut self,
        parent_id: &QualifiedNodeID,
        access_path: &[usize],
    ) -> PartialVMResult<QualifiedNodeID> {
        let apt = self.apt_roots.get_mut_access_path_tree(&parent_id.root)?;
        let mut node_id = parent_id.node_id;
        for label in access_path {
            node_id = apt.get_or_create_child_node(node_id, *label)?;
        }
        Ok(QualifiedNodeID {
            root: parent_id.root.clone(),
            node_id,
        })
    }

    fn subtree_has_references(
        &self,
        node: &QualifiedNodeID,
        filter: ReferenceFilter,
    ) -> PartialVMResult<bool> {
        let apt = self.apt_roots.get_access_path_tree(&node.root)?;
        // Note that the node itself is included in the descendants.
        for descendant in apt.get_descendants_iter(node.node_id) {
            let apt_node = safe_unwrap!(apt.nodes.get(descendant));
            for ref_ in apt_node.refs.iter() {
                match filter {
                    ReferenceFilter::All => return Ok(true),
                    ReferenceFilter::MutOnly
                        if safe_unwrap!(self.ref_table.get(ref_)).is_mutable =>
                    {
                        return Ok(true)
                    },
                    ReferenceFilter::ImmutOnly
                        if !safe_unwrap!(self.ref_table.get(ref_)).is_mutable =>
                    {
                        return Ok(true)
                    },
                    _ => {},
                };
            }
        }
        Ok(false)
    }

    fn get_mut_access_path_tree_node(
        &mut self,
        node: &QualifiedNodeID,
    ) -> PartialVMResult<&mut AccessPathTreeNode> {
        self.apt_roots.get_mut_access_path_tree_node(node)
    }

    fn get_access_path_from_ref_param(
        &self,
        qualified_node_id: &QualifiedNodeID,
    ) -> PartialVMResult<Option<(usize, Vec<usize>)>> {
        let APTRoot::ReferenceParameter { param_index } = qualified_node_id.root else {
            return Ok(None);
        };
        let apt = self
            .apt_roots
            .get_access_path_tree(&qualified_node_id.root)?;
        let path = apt.get_access_path_from_root(qualified_node_id.node_id)?;
        Ok(Some((param_index, path)))
    }
}

/// Various information about a reference.
struct ReferenceInfo {
    /// Whether this reference is mutable
    is_mutable: bool,
    /// Whether this reference is poisoned
    poisoned: bool,
    /// The APT node this reference points to
    apt_node: QualifiedNodeID,
}

impl ReferenceInfo {
    fn poison_check(&self) -> PartialVMResult<()> {
        if self.poisoned {
            let msg = "Poisoned reference accessed".to_string();
            return ref_check_failure!(msg);
        }
        Ok(())
    }
}

/// State associated with the reference checker.
pub(crate) struct RefCheckState {
    /// Shadow stack of ref/non-ref values.
    /// This is shared between all the frames in the call stack.
    shadow_stack: Vec<Value>,

    /// Stack of per-frame reference states.
    frame_stack: Vec<FrameRefState>,
}

impl RefCheckState {
    pub fn new() -> Self {
        Self {
            shadow_stack: Vec::new(),
            frame_stack: Vec::new(),
        }
    }

    /// Push `num` non-reference values onto the shadow stack.
    fn push_non_refs_to_shadow_stack(&mut self, num: usize) {
        self.shadow_stack
            .extend(std::iter::repeat(Value::NonRef).take(num));
    }

    /// Push the given `ref_id` onto the shadow stack as a reference value.
    fn push_ref_to_shadow_stack(&mut self, ref_id: RefID) {
        self.shadow_stack.push(Value::Ref(ref_id));
    }

    /// Push the given `value` onto the shadow stack.
    fn push_to_shadow_stack(&mut self, value: Value) {
        self.shadow_stack.push(value);
    }

    /// Pop and get the value on top of the shadow stack.
    fn pop_from_shadow_stack(&mut self) -> PartialVMResult<Value> {
        Ok(safe_unwrap!(self.shadow_stack.pop()))
    }

    /// Pop `num` values from the shadow stack.
    fn pop_many_from_shadow_stack(&mut self, num: usize) -> PartialVMResult<()> {
        self.shadow_stack
            .truncate(safe_unwrap!(self.shadow_stack.len().checked_sub(num)));
        Ok(())
    }

    fn get_latest_frame_state(&self) -> PartialVMResult<&FrameRefState> {
        Ok(safe_unwrap!(self.frame_stack.last()))
    }

    fn get_mut_latest_frame_state(&mut self) -> PartialVMResult<&mut FrameRefState> {
        Ok(safe_unwrap!(self.frame_stack.last_mut()))
    }

    /// Is there a function that called the current function?
    fn has_caller(&self) -> bool {
        self.frame_stack.len() >= 2
    }

    /// Assumes that there is a caller frame, invariant violation if not.
    fn get_mut_callers_frame_state(&mut self) -> PartialVMResult<&mut FrameRefState> {
        let caller_index = safe_unwrap!(self.frame_stack.len().checked_sub(2));
        Ok(safe_unwrap!(self.frame_stack.get_mut(caller_index)))
    }

    fn poison_check(&self, ref_id: RefID) -> PartialVMResult<()> {
        self.get_latest_frame_state()?.poison_check(ref_id)
    }

    fn purge_reference(&mut self, ref_id: RefID) -> PartialVMResult<()> {
        self.get_mut_latest_frame_state()?.purge_reference(ref_id)
    }

    fn copy_loc(&mut self, index: u8) -> PartialVMResult<()> {
        let index = index.into();
        let frame_state_immut = self.get_latest_frame_state()?;
        let value = safe_unwrap!(frame_state_immut.shadow_locals.get(index));
        match value {
            Value::NonRef => {
                self.push_non_refs_to_shadow_stack(1);
                let node = QualifiedNodeID::local_root(index);
                let frame_state_mut = self.get_mut_latest_frame_state()?;
                frame_state_mut.poison_refs_of_node(
                    &node,
                    VisitKind::SelfOnly,
                    ReferenceFilter::MutOnly,
                )?;
                frame_state_mut.poison_refs_of_node(
                    &node,
                    VisitKind::StrictDescendants,
                    ReferenceFilter::MutOnly,
                )?;
            },
            Value::Ref(ref_id) => {
                self.poison_check(*ref_id)?;
                let ref_info = frame_state_immut.get_ref_info(ref_id)?;
                let apt_node = ref_info.apt_node.clone();
                let is_mutable = ref_info.is_mutable;
                let frame_state_mut = self.get_mut_latest_frame_state()?;
                let new_ref_id =
                    frame_state_mut.make_new_ref_to_existing_node(apt_node, is_mutable)?;
                self.push_ref_to_shadow_stack(new_ref_id);
            },
        }
        Ok(())
    }

    fn move_loc(&mut self, index: u8) -> PartialVMResult<()> {
        let index = index.into();
        let frame_state = self.get_mut_latest_frame_state()?;
        let mut value = Value::NonRef;
        // Replace the shadow local at `index` with a non-ref value.
        std::mem::swap(
            safe_unwrap!(frame_state.shadow_locals.get_mut(index)),
            &mut value,
        );

        match value {
            Value::NonRef => {
                let node = QualifiedNodeID::local_root(index);
                let frame_state_mut = self.get_mut_latest_frame_state()?;
                frame_state_mut.poison_refs_of_node(
                    &node,
                    VisitKind::SelfOnly,
                    ReferenceFilter::All,
                )?;
                frame_state_mut.poison_refs_of_node(
                    &node,
                    VisitKind::StrictDescendants,
                    ReferenceFilter::All,
                )?;
            },
            Value::Ref(_) => {
                // Reference is being moved from a local to the stack
            },
        }

        self.push_to_shadow_stack(value);
        Ok(())
    }

    fn st_loc(&mut self, index: u8) -> PartialVMResult<()> {
        let index = index.into();
        let mut value_1 = self.pop_from_shadow_stack()?;
        let frame_state = self.get_mut_latest_frame_state()?;
        let value_2 = safe_unwrap!(frame_state.shadow_locals.get_mut(index));

        // Store the value from the shadow stack into the local at `index`.
        // `value_1` will then have the value that was previously in the local.
        std::mem::swap(value_2, &mut value_1);

        match value_1 {
            Value::NonRef => {
                let node = QualifiedNodeID::local_root(index);
                let frame_state_mut = self.get_mut_latest_frame_state()?;
                frame_state_mut.poison_refs_of_node(
                    &node,
                    VisitKind::SelfOnly,
                    ReferenceFilter::All,
                )?;
                frame_state_mut.poison_refs_of_node(
                    &node,
                    VisitKind::StrictDescendants,
                    ReferenceFilter::All,
                )?;
            },
            Value::Ref(ref_id) => {
                let frame_state_mut = self.get_mut_latest_frame_state()?;
                frame_state_mut.purge_reference(ref_id)?;
            },
        }

        Ok(())
    }

    /// Pop a reference from the shadow stack, check if it is poisoned, purge it,
    /// and push a non-reference value onto the shadow stack.
    fn pop_ref_push_non_ref(&mut self) -> PartialVMResult<()> {
        let top = self.pop_from_shadow_stack()?;
        let Value::Ref(ref_id) = top else {
            let msg = "Expected a reference on the stack".to_string();
            return ref_check_failure!(msg);
        };
        self.poison_check(ref_id)?;
        self.purge_reference(ref_id)?;

        self.push_non_refs_to_shadow_stack(1);

        Ok(())
    }

    fn write_ref(&mut self) -> PartialVMResult<()> {
        let ref_to_write = self.pop_from_shadow_stack()?;
        let _ = self.pop_from_shadow_stack()?;

        let Value::Ref(ref_id) = ref_to_write else {
            let msg = "WriteRef expected a reference on the stack".to_string();
            return ref_check_failure!(msg);
        };
        self.poison_check(ref_id)?;

        let frame_state = self.get_mut_latest_frame_state()?;
        let ref_info = frame_state.get_ref_info(&ref_id)?;
        safe_assert!(ref_info.is_mutable);
        let node = ref_info.apt_node.clone();
        frame_state.destructive_write_via_mut_ref(&node)?;

        frame_state.purge_reference(ref_id)?;

        Ok(())
    }

    fn freeze_ref(&mut self) -> PartialVMResult<()> {
        let ref_to_freeze = self.pop_from_shadow_stack()?;
        let Value::Ref(ref_id) = ref_to_freeze else {
            let msg = "FreezeRef expected a reference on the stack".to_string();
            return ref_check_failure!(msg);
        };
        self.poison_check(ref_id)?;

        let frame_state = self.get_mut_latest_frame_state()?;
        let ref_info = frame_state.get_ref_info(&ref_id)?;
        safe_assert!(ref_info.is_mutable);
        let node = ref_info.apt_node.clone();
        // Note: freeze_ref does not poison any references, as it is the same as purging the mut-ref
        // and creating a new immutable ref to the same node.
        frame_state.purge_reference(ref_id)?;
        let new_ref_id = frame_state.make_new_ref_to_existing_node(node, false)?;
        self.push_ref_to_shadow_stack(new_ref_id);

        Ok(())
    }

    /// Borrow a local value at the given `index`.
    /// The mutability of the reference given by `is_mutable`.
    fn borrow_loc(&mut self, index: u8, is_mutable: bool) -> PartialVMResult<()> {
        let index = index.into();
        let frame_state = self.get_mut_latest_frame_state()?;
        frame_state.ensure_local_root_exists(index);
        let node_id = QualifiedNodeID::local_root(index);
        let new_ref_id = frame_state.make_new_ref_to_existing_node(node_id, is_mutable)?;
        self.push_ref_to_shadow_stack(new_ref_id);

        Ok(())
    }

    fn borrow_child_with_label(&mut self, label: usize, is_mutable: bool) -> PartialVMResult<()> {
        let ref_to_borrow_from = self.pop_from_shadow_stack()?;
        let Value::Ref(parent_ref_id) = ref_to_borrow_from else {
            let msg = "Expected a reference on the stack".to_string();
            return ref_check_failure!(msg);
        };
        self.poison_check(parent_ref_id)?;

        let frame_state = self.get_mut_latest_frame_state()?;
        let ref_info = frame_state.get_ref_info(&parent_ref_id)?;
        // If we are borrowing a mutable reference, the parent reference must also be mutable.
        safe_assert!(!is_mutable || ref_info.is_mutable);

        let parent_node_id = ref_info.apt_node.clone();
        let child_node_id =
            frame_state.get_or_create_descendant_node(&parent_node_id, slice::from_ref(&label))?;

        frame_state.purge_reference(parent_ref_id)?;

        let new_ref_id = frame_state.make_new_ref_to_existing_node(child_node_id, is_mutable)?;
        self.push_ref_to_shadow_stack(new_ref_id);

        Ok(())
    }

    fn borrow_global(&mut self, type_: Type, is_mutable: bool) -> PartialVMResult<()> {
        let _ = self.pop_from_shadow_stack()?;

        let frame_state = self.get_mut_latest_frame_state()?;
        frame_state.ensure_global_root_exists(type_.clone());

        let node_id = QualifiedNodeID::global_root(type_);
        if is_mutable && frame_state.subtree_has_references(&node_id, ReferenceFilter::All)? {
            let msg = "Cannot borrow_global_mut while there are existing references".to_string();
            return ref_check_failure!(msg);
        } else if !is_mutable
            && frame_state.subtree_has_references(&node_id, ReferenceFilter::MutOnly)?
        {
            let msg = "Cannot borrow_global while there are mutable references".to_string();
            return ref_check_failure!(msg);
        }

        let new_ref_id = frame_state.make_new_ref_to_existing_node(node_id, is_mutable)?;
        self.push_ref_to_shadow_stack(new_ref_id);

        Ok(())
    }

    fn move_from(&mut self, type_: Type) -> PartialVMResult<()> {
        let _ = self.pop_from_shadow_stack()?;

        let frame_state = self.get_mut_latest_frame_state()?;
        frame_state.ensure_global_root_exists(type_.clone());

        let node_id = QualifiedNodeID::global_root(type_);
        frame_state.poison_refs_of_node(&node_id, VisitKind::SelfOnly, ReferenceFilter::All)?;
        frame_state.poison_refs_of_node(
            &node_id,
            VisitKind::StrictDescendants,
            ReferenceFilter::All,
        )?;

        self.push_non_refs_to_shadow_stack(1);

        Ok(())
    }

    fn move_to(&mut self) -> PartialVMResult<()> {
        let _ = self.pop_from_shadow_stack()?;
        let signer_ref = self.pop_from_shadow_stack()?;

        let Value::Ref(signer_ref_id) = signer_ref else {
            let msg = "Expected a reference to a signer on the stack".to_string();
            return ref_check_failure!(msg);
        };
        self.poison_check(signer_ref_id)?;

        let frame_state = self.get_mut_latest_frame_state()?;
        frame_state.purge_reference(signer_ref_id)?;

        Ok(())
    }

    fn vec_len(&mut self) -> PartialVMResult<()> {
        let vec_ref = self.pop_from_shadow_stack()?;
        let Value::Ref(vec_ref_id) = vec_ref else {
            let msg = "vec_len expected a reference on the stack".to_string();
            return ref_check_failure!(msg);
        };
        self.poison_check(vec_ref_id)?;

        let frame_state = self.get_mut_latest_frame_state()?;
        frame_state.purge_reference(vec_ref_id)?;

        self.push_non_refs_to_shadow_stack(1);

        Ok(())
    }

    fn vec_borrow(&mut self, is_mutable: bool) -> PartialVMResult<()> {
        let _ = self.pop_from_shadow_stack()?;
        let vec_ref = self.pop_from_shadow_stack()?;
        let Value::Ref(parent_ref_id) = vec_ref else {
            let msg = "vec_borrow expected a reference on the stack".to_string();
            return ref_check_failure!(msg);
        };
        self.poison_check(parent_ref_id)?;

        let frame_state = self.get_mut_latest_frame_state()?;
        let ref_info = frame_state.get_ref_info(&parent_ref_id)?;
        // If we are borrowing a mutable reference, the parent reference must also be mutable.
        safe_assert!(!is_mutable || ref_info.is_mutable);

        let parent_node_id = ref_info.apt_node.clone();
        // Note that we abstract over all indices and use `0` to represent the label.
        // This is stricter than necessary, but it is cheaper than maintaining a per-index APT node.
        let abstracted_label = 0;
        let child_node_id = frame_state
            .get_or_create_descendant_node(&parent_node_id, slice::from_ref(&abstracted_label))?;

        frame_state.purge_reference(parent_ref_id)?;

        let new_ref_id = frame_state.make_new_ref_to_existing_node(child_node_id, is_mutable)?;
        self.push_ref_to_shadow_stack(new_ref_id);

        Ok(())
    }

    fn vec_push_back(&mut self) -> PartialVMResult<()> {
        let _ = self.pop_from_shadow_stack()?;
        let vec_ref = self.pop_from_shadow_stack()?;
        let Value::Ref(vec_ref_id) = vec_ref else {
            let msg = "vec_push_back expected a reference on the stack".to_string();
            return ref_check_failure!(msg);
        };
        self.poison_check(vec_ref_id)?;

        // Note: we are not checking if the reference is mutable here.
        let frame_state = self.get_mut_latest_frame_state()?;
        frame_state.purge_reference(vec_ref_id)?;

        // Note: we do not consider this to be a destructive update to the vector,
        // and references to other elements in the vector would still be un-poisoned.
        Ok(())
    }

    fn vec_pop_back(&mut self) -> PartialVMResult<()> {
        let vec_ref = self.pop_from_shadow_stack()?;
        let Value::Ref(vec_ref_id) = vec_ref else {
            let msg = "vec_pop_back expected a reference on the stack".to_string();
            return ref_check_failure!(msg);
        };
        self.poison_check(vec_ref_id)?;

        let frame_state = self.get_mut_latest_frame_state()?;
        let ref_info = frame_state.get_ref_info(&vec_ref_id)?;
        safe_assert!(ref_info.is_mutable);

        let node = ref_info.apt_node.clone();
        frame_state.destructive_write_via_mut_ref(&node)?;

        frame_state.purge_reference(vec_ref_id)?;

        self.push_non_refs_to_shadow_stack(1);

        Ok(())
    }

    fn vec_swap(&mut self) -> PartialVMResult<()> {
        self.pop_many_from_shadow_stack(2)?;
        let vec_ref = self.pop_from_shadow_stack()?;
        let Value::Ref(vec_ref_id) = vec_ref else {
            let msg = "vec_swap expected a reference on the stack".to_string();
            return ref_check_failure!(msg);
        };
        self.poison_check(vec_ref_id)?;

        let frame_state = self.get_mut_latest_frame_state()?;
        let ref_info = frame_state.get_ref_info(&vec_ref_id)?;
        safe_assert!(ref_info.is_mutable);

        let node = ref_info.apt_node.clone();
        frame_state.destructive_write_via_mut_ref(&node)?;

        frame_state.purge_reference(vec_ref_id)?;

        Ok(())
    }

    fn core_call(
        &mut self,
        num_params: usize,
        num_locals: usize,
        mask: ClosureMask,
    ) -> PartialVMResult<()> {
        // for each arg value that is:
        // - a mutable reference: get the corresponding APT node and lock it and its descendants
        //   with exclusive lock, perform a destructive update on the node
        // - an immutable reference: get the corresponding APT node and lock it and its descendants
        //   with shared lock
        // if when obtaining exclusive or shared lock, the node is already locked with exclusive lock,
        // then it is an invariant violation
        // check if any of the reference arguments are poisoned (invariant violation) or not

        // pop the call's args from the shadow stack and purge any references

        // setup the ref check state for the new frame:
        // - create a new shadow list of locals with ref/non-ref values
        // - for each arg value that is a reference:
        //   - create a new root of APT corresponding to the value behind the reference
        //   - add it to the ref-to-apt bi-directional mapping
        //   - place it in the appropriate slot in the shadow list of locals
        // - initialize the rest of the shadow list of locals with non-ref values
        let mut ref_arg_ids = Vec::new();
        let mut mut_ref_indexes = Vec::new();
        let mut immut_ref_indexes = Vec::new();
        let mut ref_param_map = UnorderedMap::with_hasher(FxBuildHasher::default());
        for i in (0..num_params).rev() {
            let is_captured = mask.is_captured(i);
            if !is_captured {
                let Value::Ref(ref_id) = self.pop_from_shadow_stack()? else {
                    continue;
                };

                // We have a reference argument to deal with.
                let frame_state = self.get_mut_latest_frame_state()?;
                let ref_info = frame_state.get_ref_info(&ref_id)?;
                ref_info.poison_check()?;
                let apt_node = ref_info.apt_node.clone();
                if ref_info.is_mutable {
                    frame_state.lock_node_subtree(&apt_node, Lock::Exclusive)?;
                    frame_state.destructive_write_via_mut_ref(&apt_node)?;
                    mut_ref_indexes.push(i);
                } else {
                    frame_state.lock_node_subtree(&apt_node, Lock::Shared)?;
                    immut_ref_indexes.push(i);
                }
                ref_arg_ids.push(ref_id);
                ref_param_map.insert(i, apt_node);
            }
        }
        for ref_id in ref_arg_ids {
            let frame_state = self.get_mut_latest_frame_state()?;
            let ref_info = frame_state.get_ref_info(&ref_id)?;
            let apt_node = ref_info.apt_node.clone();
            frame_state.release_lock_node_subtree(&apt_node)?;
            frame_state.purge_reference(ref_id)?;
        }

        self.push_new_frame(
            num_locals,
            mut_ref_indexes,
            immut_ref_indexes,
            ref_param_map,
        )?;

        Ok(())
    }

    fn push_new_frame(
        &mut self,
        num_locals: usize,
        mut_ref_indexes: Vec<usize>,
        immut_ref_indexes: Vec<usize>,
        ref_param_map: UnorderedMap<usize, QualifiedNodeID>,
    ) -> PartialVMResult<()> {
        let frame = FrameRefState::new(
            num_locals,
            mut_ref_indexes,
            immut_ref_indexes,
            ref_param_map,
        )?;
        self.frame_stack.push(frame);
        Ok(())
    }

    fn return_(&mut self, num_return_values: usize) -> PartialVMResult<()> {
        if !self
            .shadow_stack
            .iter()
            .rev()
            .take(num_return_values)
            .all(|v| matches!(v, Value::NonRef))
        {
            // There is at least one reference value being returned.
            // We perform some checks and transform the references to the caller's frame.
            let has_caller = self.has_caller();
            let mut transformed_values = Vec::new();
            let stack_values = self
                .shadow_stack
                .iter()
                .rev()
                .take(num_return_values)
                .cloned()
                .collect::<Vec<_>>();
            for value in stack_values {
                if let Value::Ref(ref_id) = value {
                    let frame_state = self.get_mut_latest_frame_state()?;
                    let ref_info = frame_state.get_ref_info(&ref_id)?;
                    let is_mutable = ref_info.is_mutable;
                    ref_info.poison_check()?;

                    let apt_node = ref_info.apt_node.clone();
                    // Check if each reference being returned is derived from a reference parameter.
                    let Some((param_index, access_path)) =
                        frame_state.get_access_path_from_ref_param(&apt_node)?
                    else {
                        let msg =
                            "Returning a reference that is not derived from a reference parameter"
                                .to_string();
                        return ref_check_failure!(msg);
                    };
                    // Check that mutable references are returned "exclusively".
                    if is_mutable {
                        frame_state.lock_node_subtree(&apt_node, Lock::Exclusive)?;
                    } else {
                        frame_state.lock_node_subtree(&apt_node, Lock::Shared)?;
                    }
                    // This frame will be thrown away, so no need to unlock the nodes.
                    // Transform the reference to one on the caller's frame, if there is a caller.
                    if !has_caller {
                        continue;
                    }
                    let caller_apt_node =
                        safe_unwrap!(frame_state.caller_ref_param_map.get(&param_index)).clone();
                    let callers_frame = self.get_mut_callers_frame_state()?;
                    let transformed_node = callers_frame
                        .get_or_create_descendant_node(&caller_apt_node, &access_path)?;
                    let transformed_ref_id = callers_frame
                        .make_new_ref_to_existing_node(transformed_node, is_mutable)?;
                    transformed_values.push(Some(transformed_ref_id));
                } else if has_caller {
                    // The returned value is not a reference and there is a caller.
                    transformed_values.push(None);
                }
            }
            if has_caller {
                for (value, transformed_value) in self
                    .shadow_stack
                    .iter_mut()
                    .rev()
                    .take(num_return_values)
                    .zip(transformed_values.iter())
                {
                    if let Some(transformed_ref) = transformed_value {
                        debug_assert!(matches!(value, Value::Ref(_)));
                        *value = Value::Ref(*transformed_ref);
                    }
                }
            }
        }
        self.frame_stack.pop();
        Ok(())
    }
}
