// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! This module implements the runtime reference checks for Move bytecode.
//!
//! Move bytecode has a bytecode verifier pass for enforcing reference safety rules:
//! the runtime checks implemented here are the relaxed dynamic semantics of that pass.
//! If the bytecode verifier pass succeeds, then the runtime checks should also succeed
//! for any execution path.
//! However, there may be Move bytecode that the bytecode verifier pass rejects, but
//! the runtime checks may still succeed, as long as reference-safety rules are not
//! violated (i.e., relaxed semantics).
//!
//! This checker maintains shadow state as the execution proceeds: the shadow state
//! contains information needed about the references in order to check for reference safety.
//! Note that simpler techniques such as reference counting are insufficient to
//! implement the dynamic relaxed semantics of the bytecode verifier pass.
//!
//! The shadow state contains:
//! - A shadow stack of values. Values which can either be non-references (we don't
//!   keep track of their type or actual value) or references (represented by a
//!   unique-per-caller-frame identifier). The shadow stack is shared across all
//!   active frames in the call stack.
//! - A shadow frame stack of per-function data structures (described below).
//!   The shadow frame stack grows and shrinks with the call stack.
//!
//! An access path tree, which is built out lazily as needed (for performance
//! reasons), represents a non-reference value. A reference always points to a node
//! in some access path tree.
//!
//! Consider, for example, a value of type `A`:
//! ```move
//! struct A {
//!     x: B
//! }
//!
//! struct B {
//!    y: vector<u64>
//!    z: u64
//! }
//! ```
//! An access path tree for a value of type `A` would look like this:
//! ```tree
//! root
//!   └──0── .x
//!           ├──0── .y
//!           │       └──0── all elements of the vector
//!           └──1── .z
//! ```
//! The edges are ordered by labels, given by field offsets for structs and variants.
//! For vectors, we use `0` to abstract all elements, instead of tracking
//! each element of the vector separately. This is done for performance reasons.
//!
//! The per-function frame data structure contains:
//! - A shadow list of values for all locals.
//! - An access path tree for:
//!   - each local that is not a reference
//!   - each resource type globally borrowed by the function
//!   - each value behind reference parameters passed to the function
//! - A map from reference identifiers to the following info:
//!   - whether the reference is mutable or immutable
//!   - whether the reference is poisoned or not
//!   - the access path tree node corresponding to the reference
//! - A map from each reference parameter index to the corresponding access path tree
//!   node in the caller's frame (if it exists)
//!
//! The informal idea is that we allow borrowing of references (mutable or immutable),
//! but poison references when the underlying value is moved, or when a destructive
//! update is performed via a mutable reference. Later, any use of a poisoned reference
//! will result in an invariant violation error.
//!
//! When a call is made with reference parameters, the corresponding access path tree
//! node subtree is locked (with exclusive lock for mutable references, and shared
//! lock for immutable references). This is to make sure that values behind mutable
//! references are unique at function call boundaries.
//!
//! When we return references on the shadow stack, we ensure that they are derived from
//! one of the reference parameters. They are also transformed to point to the
//! corresponding access path tree node in the caller's frame (if it exists).

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

/// A deterministic hash map (used in the Rust compiler), expected to perform well.
/// Not resistant to hash collision attacks, nor is it cryptographically secure.
/// Should not be used for iterating over keys without sorting first.
type UnorderedMap<K, V> = HashMap<K, V, FxBuildHasher>;

/// `ref_check_failure!(msg)` will return a `PartialVMError` with the given message
/// and a sub-status code indicating a reference safety failure.
macro_rules! ref_check_failure {
    ($msg:ident) => {
        Err(
            PartialVMError::new(StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR)
                .with_message($msg)
                .with_sub_status(EREFERENCE_SAFETY_FAILURE),
        )
    };
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

/// Access Path Tree, representing the access paths corresponding
/// to a value (local, global, or value behind a reference parameter) in a frame.
/// It is built up lazily as needed.
struct AccessPathTree {
    nodes: Vec<AccessPathTreeNode>,
}

/// Node ID in a given access path tree, acts an index into the access path tree's
/// node list.
type NodeID = usize;
/// Edge label for an edge between two nodes in a given access path tree.
type EdgeLabel = usize;

/// A node in the access path tree.
struct AccessPathTreeNode {
    /// Parent node id and edge label (`None` for root nodes)
    parent: Option<(NodeID, EdgeLabel)>,
    /// Child nodes, edge label is the index in this vector
    children: Vec<Option<NodeID>>,
    /// References to this node
    refs: BTreeSet<RefID>,
    /// Current lock on this node
    lock: Option<Lock>,
}

/// Represents the type of lock on an access path tree node.
#[derive(Copy, Clone, PartialEq, Eq)]
enum Lock {
    /// Shared lock - multiple shared locks on the same node are allowed
    Shared,
    /// Exclusive lock - conflicts with any other lock
    Exclusive,
}

/// Different kinds of root nodes in a frame.
#[derive(Clone)]
enum AccessPathTreeRoot {
    /// Root representing a local (non-ref) value
    Local { index: usize },
    /// Root representing a global type
    Global { type_: Type },
    /// Special node representing the value behind a reference parameter
    ReferenceParameter { param_index: usize },
}

/// Collection of access path tree roots information for a frame.
struct AccessPathTreeRootsInfo {
    /// Mapping from local index to the corresponding access path tree
    locals: UnorderedMap<usize, AccessPathTree>,
    /// Mapping from global type to the corresponding access path tree
    globals: UnorderedMap<Type, AccessPathTree>,
    /// Mapping from reference parameter index to the corresponding access path tree
    reference_params: UnorderedMap<usize, AccessPathTree>,
}

/// The root of the access path tree and the node ID within that tree.
#[derive(Clone)]
struct QualifiedNodeID {
    root: AccessPathTreeRoot,
    node_id: NodeID,
}

/// Per frame reference checking state.
struct FrameRefState {
    /// Shadow list of local values.
    locals: Vec<Value>,
    /// Roots of the Access Path Tree for this frame.
    access_path_tree_roots: AccessPathTreeRootsInfo,
    /// Mapping from references to their information.
    /// Reference ID is unique within the frame.
    ref_table: UnorderedMap<RefID, ReferenceInfo>,
    /// Next available reference ID.
    next_ref_id: usize,
    /// Map the reference parameter's index to the access path tree node
    /// (in the caller's `FrameRefState`) corresponding to the reference parameter.
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

/// Various information about a reference.
struct ReferenceInfo {
    /// Whether this reference is mutable
    is_mutable: bool,
    /// Whether this reference is poisoned
    poisoned: bool,
    /// The access path tree node this reference points to
    access_path_tree_node: QualifiedNodeID,
}

/// State associated with the reference checker.
/// This state is transitioned and checked as the bytecode is executed.
pub(crate) struct RefCheckState {
    /// Shadow stack of ref/non-ref values.
    /// This is shared between all the frames in the call stack.
    shadow_stack: Vec<Value>,

    /// Stack of per-frame reference states.
    frame_stack: Vec<FrameRefState>,
}

/// A trait for determining the behavior of the runtime reference checks.
pub(crate) trait RuntimeRefCheck {
    /// Transitions the reference check state before executing a bytecode instruction.
    fn pre_execution_transition(
        frame: &Frame,
        instruction: &Bytecode,
        ref_state: &mut RefCheckState,
    ) -> PartialVMResult<()>;

    /// Transitions the reference check state after executing a bytecode instruction.
    fn post_execution_transition(
        frame: &Frame,
        instruction: &Bytecode,
        ref_state: &mut RefCheckState,
        ty_cache: &mut FrameTypeCache,
    ) -> PartialVMResult<()>;

    /// Transitions the reference check state during various forms of function calls.
    fn core_call_transition(
        num_params: usize,
        num_locals: usize,
        mask: ClosureMask,
        ref_state: &mut RefCheckState,
    ) -> PartialVMResult<()>;

    /// Initializes the reference check state on the entrypoint function.
    fn init_entry(function: &LoadedFunction, ref_state: &mut RefCheckState) -> PartialVMResult<()>;
}

/// A no-op implementation of the `RuntimeRefCheck` trait, which does not perform
/// any runtime reference checks.
pub(crate) struct NoRuntimeRefCheck;

/// An implementation of the `RuntimeRefCheck` trait that performs the reference checks
/// as described in the module documentation.
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
    /// It may be preferred to have as many transitions in the `post_execution_transition`, because
    /// gas is charged during execution, but we may want to validate this preference.
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
            ReadRef => {
                ref_state.pop_ref_push_non_ref()?;
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
                let top = ref_state.pop_from_shadow_stack()?;
                if let Value::Ref(ref_id) = top {
                    ref_state.purge_reference(ref_id)?;
                }
            },
            Ret | BrTrue(_) | BrFalse(_) | Branch(_) => {
                // not reachable here, transition is handled in `pre_execution_transition`
            },
            CastU8 | CastU16 | CastU32 | CastU64 | CastU128 | CastU256 | Not | Nop | Exists(_)
            | ExistsGeneric(_) => {
                // no-op
            },
            LdU8(_) | LdU16(_) | LdU32(_) | LdU64(_) | LdU128(_) | LdU256(_) | LdConst(_)
            | LdTrue | LdFalse => {
                ref_state.push_non_refs_to_shadow_stack(1);
            },
            CopyLoc(index) => {
                ref_state.copy_loc(*index)?;
            },
            MoveLoc(index) => {
                ref_state.move_loc(*index)?;
            },
            StLoc(index) => {
                ref_state.st_loc(*index)?;
            },
            Call(_) | CallGeneric(_) | CallClosure(_) => {
                // not reachable here, transition handled in `core_call_transition`
            },
            Pack(index) => {
                let num_fields = frame.field_count(*index).into();
                ref_state.pop_many_from_shadow_stack(num_fields)?;
                ref_state.push_non_refs_to_shadow_stack(1);
            },
            PackGeneric(index) => {
                let num_fields = frame.field_instantiation_count(*index).into();
                ref_state.pop_many_from_shadow_stack(num_fields)?;
                ref_state.push_non_refs_to_shadow_stack(1);
            },
            PackVariant(index) => {
                let struct_variant_info = frame.get_struct_variant_at(*index);
                let num_fields = struct_variant_info.field_count.into();
                ref_state.pop_many_from_shadow_stack(num_fields)?;
                ref_state.push_non_refs_to_shadow_stack(1);
            },
            PackVariantGeneric(index) => {
                let struct_variant_info = frame.get_struct_variant_instantiation_at(*index);
                let num_fields = struct_variant_info.field_count.into();
                ref_state.pop_many_from_shadow_stack(num_fields)?;
                ref_state.push_non_refs_to_shadow_stack(1);
            },
            Unpack(index) => {
                ref_state.pop_from_shadow_stack()?;
                let num_fields = frame.field_count(*index).into();
                ref_state.push_non_refs_to_shadow_stack(num_fields);
            },
            UnpackGeneric(index) => {
                ref_state.pop_from_shadow_stack()?;
                let num_fields = frame.field_instantiation_count(*index).into();
                ref_state.push_non_refs_to_shadow_stack(num_fields);
            },
            UnpackVariant(index) => {
                ref_state.pop_from_shadow_stack()?;
                let struct_variant_info = frame.get_struct_variant_at(*index);
                let num_fields = struct_variant_info.field_count.into();
                ref_state.push_non_refs_to_shadow_stack(num_fields);
            },
            UnpackVariantGeneric(index) => {
                ref_state.pop_from_shadow_stack()?;
                let struct_variant_info = frame.get_struct_variant_instantiation_at(*index);
                let num_fields = struct_variant_info.field_count.into();
                ref_state.push_non_refs_to_shadow_stack(num_fields);
            },
            TestVariant(_) => {
                ref_state.pop_ref_push_non_ref()?;
            },
            TestVariantGeneric(_) => {
                ref_state.pop_ref_push_non_ref()?;
            },
            ReadRef => {
                // Transition handled in `pre_execution_transition`
            },
            WriteRef => {
                ref_state.write_ref()?;
            },
            FreezeRef => {
                ref_state.freeze_ref()?;
            },
            MutBorrowLoc(index) => {
                ref_state.borrow_loc(*index, true)?;
            },
            ImmBorrowLoc(index) => {
                ref_state.borrow_loc(*index, false)?;
            },
            MutBorrowField(index) => {
                let label = frame.field_offset(*index);
                ref_state.borrow_child_with_label::<true>(label)?;
            },
            MutBorrowVariantField(index) => {
                let field_info = frame.variant_field_info_at(*index);
                let label = field_info.offset;
                ref_state.borrow_child_with_label::<true>(label)?;
            },
            MutBorrowFieldGeneric(index) => {
                let label = frame.field_instantiation_offset(*index);
                ref_state.borrow_child_with_label::<true>(label)?;
            },
            MutBorrowVariantFieldGeneric(index) => {
                let field_info = frame.variant_field_instantiation_info_at(*index);
                let label = field_info.offset;
                ref_state.borrow_child_with_label::<true>(label)?;
            },
            ImmBorrowField(index) => {
                let label = frame.field_offset(*index);
                ref_state.borrow_child_with_label::<false>(label)?;
            },
            ImmBorrowVariantField(index) => {
                let field_info = frame.variant_field_info_at(*index);
                let label = field_info.offset;
                ref_state.borrow_child_with_label::<false>(label)?;
            },
            ImmBorrowFieldGeneric(index) => {
                let label = frame.field_instantiation_offset(*index);
                ref_state.borrow_child_with_label::<false>(label)?;
            },
            ImmBorrowVariantFieldGeneric(index) => {
                let field_info = frame.variant_field_instantiation_info_at(*index);
                let label = field_info.offset;
                ref_state.borrow_child_with_label::<false>(label)?;
            },
            MutBorrowGlobal(index) => {
                let struct_ty = frame.get_struct_ty(*index);
                ref_state.borrow_global::<true>(struct_ty)?;
            },
            MutBorrowGlobalGeneric(index) => {
                let struct_ty = ty_cache.get_struct_type(*index, frame)?.0;
                ref_state.borrow_global::<true>(struct_ty.clone())?;
            },
            ImmBorrowGlobal(index) => {
                let struct_ty = frame.get_struct_ty(*index);
                ref_state.borrow_global::<false>(struct_ty)?;
            },
            ImmBorrowGlobalGeneric(index) => {
                let struct_ty = ty_cache.get_struct_type(*index, frame)?.0;
                ref_state.borrow_global::<false>(struct_ty.clone())?;
            },
            Add | Sub | Mul | Mod | Div | BitOr | BitAnd | Xor | Or | And | Lt | Gt | Le | Ge
            | Shl | Shr => {
                // pop two non-ref values from the shadow stack, push a new non-ref value
                let _ = ref_state.pop_from_shadow_stack()?;
            },
            Eq | Neq => {
                // pop two values from the shadow stack (which can be ref or non-ref values)
                let top_1 = ref_state.pop_from_shadow_stack()?;
                let top_2 = ref_state.pop_from_shadow_stack()?;
                if let (Value::Ref(ref_1), Value::Ref(ref_2)) = (top_1, top_2) {
                    ref_state.purge_reference(ref_1)?;
                    ref_state.purge_reference(ref_2)?;
                }
                // push a non-ref value onto the shadow stack
                ref_state.push_non_refs_to_shadow_stack(1);
            },
            Abort => {
                // not reachable here, transition handled in `pre_execution_transition`
            },
            MoveFrom(index) => {
                let struct_ty = frame.get_struct_ty(*index);
                ref_state.move_from(struct_ty)?;
            },
            MoveFromGeneric(index) => {
                let struct_ty = ty_cache.get_struct_type(*index, frame)?.0;
                ref_state.move_from(struct_ty.clone())?;
            },
            MoveTo(_) => {
                ref_state.move_to()?;
            },
            MoveToGeneric(_) => {
                ref_state.move_to()?;
            },
            VecPack(_, n) => {
                ref_state.pop_many_from_shadow_stack(safe_unwrap_err!((*n).try_into()))?;
                ref_state.push_non_refs_to_shadow_stack(1);
            },
            VecLen(_) => {
                ref_state.vec_len()?;
            },
            VecImmBorrow(_) => {
                ref_state.vec_borrow::<false>()?;
            },
            VecMutBorrow(_) => {
                ref_state.vec_borrow::<true>()?;
            },
            VecPushBack(_) => {
                ref_state.vec_push_back()?;
            },
            VecPopBack(_) => {
                ref_state.vec_pop_back()?;
            },
            VecUnpack(_, n) => {
                let _ = ref_state.pop_from_shadow_stack()?;
                ref_state.push_non_refs_to_shadow_stack(safe_unwrap_err!((*n).try_into()));
            },
            VecSwap(_) => {
                ref_state.vec_swap()?;
            },
            PackClosure(_, mask) => {
                let captured = mask.captured_count();
                // note: we are not checking that values captured are non-ref values, as this belongs
                // to type checks.
                ref_state.pop_many_from_shadow_stack(captured.into())?;
                ref_state.push_non_refs_to_shadow_stack(1);
            },
            PackClosureGeneric(_, mask) => {
                let captured = mask.captured_count();
                ref_state.pop_many_from_shadow_stack(captured.into())?;
                ref_state.push_non_refs_to_shadow_stack(1);
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
        // Empty map, references are not transformed when returning from the entrypoint function.
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

impl AccessPathTree {
    /// Create a new Access Path Tree with a fresh root node.
    fn new() -> Self {
        Self {
            nodes: vec![AccessPathTreeNode::fresh_root()],
        }
    }

    /// Make a new child node in the access path tree, given the parent node ID
    /// and the label for the edge.
    fn make_new_node(&mut self, parent_id: NodeID, label: EdgeLabel) -> NodeID {
        let new_node = AccessPathTreeNode::fresh_node(parent_id, label);
        self.nodes.push(new_node);
        self.nodes.len() - 1
    }

    /// Get a reference to the node at `node_id`.
    fn get_node(&self, node_id: NodeID) -> PartialVMResult<&AccessPathTreeNode> {
        Ok(safe_unwrap!(self.nodes.get(node_id)))
    }

    /// Get a mutable reference to the node at `node_id`.
    fn get_node_mut(&mut self, node_id: NodeID) -> PartialVMResult<&mut AccessPathTreeNode> {
        Ok(safe_unwrap!(self.nodes.get_mut(node_id)))
    }

    /// Given the parent node ID and the label, get the existing child node or create a new one.
    fn get_or_create_child_node(
        &mut self,
        parent_id: NodeID,
        label: EdgeLabel,
    ) -> PartialVMResult<NodeID> {
        let parent_node = self.get_node_mut(parent_id)?;
        let child_id = parent_node.children.get(label);
        // Should we resize the children vector?
        let resize = match child_id {
            // child slot exists and is occupied, return its ID
            Some(Some(child_id)) => return Ok(*child_id),
            // child slot exists but is unoccupied, no need to resize, just occupy it
            Some(None) => false,
            // child slot does not exist, we need to resize and then occupy it
            None => true,
        };

        if resize {
            parent_node
                .children
                .resize(safe_unwrap!(label.checked_add(1)), None);
        }

        // Create a new child node, and update the parent's children slot.
        let new_child_id = self.make_new_node(parent_id, label);
        // Re-borrow to satisfy Rust's borrow checker.
        let parent_node = self.get_node_mut(parent_id)?;
        *safe_unwrap!(parent_node.children.get_mut(label)) = Some(new_child_id);
        Ok(new_child_id)
    }

    /// Visit the strict descendants (i.e., exclude self) of the node and apply `f` to each.
    fn visit_strict_descendants<F>(&mut self, node_id: NodeID, mut f: F) -> PartialVMResult<()>
    where
        F: FnMut(&mut AccessPathTreeNode) -> PartialVMResult<()>,
    {
        // We need to collect the descendants first, because we are mutating nodes while visiting.
        for descendant in self
            .get_descendants_iter(node_id)
            .skip(1)
            .collect::<Vec<_>>()
        {
            let node = self.get_node_mut(descendant)?;
            f(node)?;
        }
        Ok(())
    }

    /// Visit the node itself and apply `f` to it.
    fn visit_self<F>(&mut self, node_id: NodeID, mut f: F) -> PartialVMResult<()>
    where
        F: FnMut(&mut AccessPathTreeNode) -> PartialVMResult<()>,
    {
        let node = self.get_node_mut(node_id)?;
        f(node)?;
        Ok(())
    }

    /// Visit the strict ancestors of the node (i.e., excluding self) and apply `f` to each.
    fn visit_strict_ancestors<F>(&mut self, node_id: NodeID, mut f: F) -> PartialVMResult<()>
    where
        F: FnMut(&mut AccessPathTreeNode) -> PartialVMResult<()>,
    {
        let mut current_node_id = node_id;
        while let Some((parent_id, _label)) = self.get_node(current_node_id)?.parent {
            let parent_node = self.get_node_mut(parent_id)?;
            f(parent_node)?;
            current_node_id = parent_id;
        }
        Ok(())
    }

    /// Get the list of edge labels that can be used to get from the root node to the given node.
    fn get_access_path_from_root(&self, node_id: NodeID) -> PartialVMResult<Vec<EdgeLabel>> {
        let mut current_node_id = node_id;
        let mut path = Vec::new();
        while let Some((parent_id, label)) = self.get_node(current_node_id)?.parent {
            current_node_id = parent_id;
            path.push(label);
        }
        path.reverse();
        Ok(path)
    }

    /// Get an iterator over the descendants (including self) of the given node.
    fn get_descendants_iter<'a>(&'a self, node_id: NodeID) -> DescendantsTraversalIter<'a> {
        DescendantsTraversalIter {
            stack: vec![node_id],
            access_path_tree: self,
        }
    }
}

/// An iterator for traversing the descendants of an access path tree node.
struct DescendantsTraversalIter<'a> {
    stack: Vec<NodeID>,
    access_path_tree: &'a AccessPathTree,
}

impl<'a> Iterator for DescendantsTraversalIter<'a> {
    type Item = NodeID;

    fn next(&mut self) -> Option<Self::Item> {
        let node_id = self.stack.pop()?;
        // When processing a node, its children are added to the stack in reverse order.
        if let Some(node) = self.access_path_tree.nodes.get(node_id) {
            self.stack.extend(node.children.iter().rev().flatten());
        } // else should be unreachable, as we should not have invalid node IDs
        Some(node_id)
    }
}

impl AccessPathTreeNode {
    /// Create a fresh root node for the access path tree.
    fn fresh_root() -> Self {
        Self {
            parent: None,
            children: Vec::new(),
            refs: BTreeSet::new(),
            lock: None,
        }
    }

    /// Create a fresh child node with the given parent and edge label.
    fn fresh_node(parent_id: NodeID, label: EdgeLabel) -> Self {
        Self {
            parent: Some((parent_id, label)),
            children: Vec::new(),
            refs: BTreeSet::new(),
            lock: None,
        }
    }
}

impl AccessPathTreeRootsInfo {
    /// Get a reference to the access path tree for the given root.
    /// Should be called when the root is guaranteed to exist.
    fn get_access_path_tree(&self, root: &AccessPathTreeRoot) -> PartialVMResult<&AccessPathTree> {
        match root {
            AccessPathTreeRoot::Local { index } => Ok(safe_unwrap!(self.locals.get(index))),
            AccessPathTreeRoot::Global { type_ } => Ok(safe_unwrap!(self.globals.get(type_))),
            AccessPathTreeRoot::ReferenceParameter { param_index } => {
                Ok(safe_unwrap!(self.reference_params.get(param_index)))
            },
        }
    }

    /// Get a mutable reference to the access path tree for the given root.
    /// Should be called when the root is guaranteed to exist.
    fn get_mut_access_path_tree(
        &mut self,
        root: &AccessPathTreeRoot,
    ) -> PartialVMResult<&mut AccessPathTree> {
        Ok(safe_unwrap!(self.maybe_get_mut_access_path_tree(root)))
    }

    /// Get a mutable reference to the access path tree for the given root, if it exists.
    fn maybe_get_mut_access_path_tree(
        &mut self,
        root: &AccessPathTreeRoot,
    ) -> Option<&mut AccessPathTree> {
        match root {
            AccessPathTreeRoot::Local { index } => self.locals.get_mut(index),
            AccessPathTreeRoot::Global { type_ } => self.globals.get_mut(type_),
            AccessPathTreeRoot::ReferenceParameter { param_index } => {
                self.reference_params.get_mut(param_index)
            },
        }
    }

    /// Get a mutable reference to the given node in the access path tree.
    fn get_mut_access_path_tree_node(
        &mut self,
        node: &QualifiedNodeID,
    ) -> PartialVMResult<&mut AccessPathTreeNode> {
        let access_path_tree = self.get_mut_access_path_tree(&node.root)?;
        Ok(safe_unwrap!(access_path_tree.nodes.get_mut(node.node_id)))
    }
}

impl QualifiedNodeID {
    /// A root node corresponding to a local with the given index.
    fn local_root(index: usize) -> Self {
        Self {
            root: AccessPathTreeRoot::Local { index },
            node_id: 0, // root is always at 0
        }
    }

    /// A root node corresponding to a global type.
    fn global_root(type_: Type) -> Self {
        Self {
            root: AccessPathTreeRoot::Global { type_ },
            node_id: 0, // root is always at 0
        }
    }

    /// A root node corresponding to a reference parameter with the given index.
    fn reference_param_root(param_index: usize) -> Self {
        Self {
            root: AccessPathTreeRoot::ReferenceParameter { param_index },
            node_id: 0, // root is always at 0
        }
    }
}

impl FrameRefState {
    /// Create a new `FrameRefState`.
    /// - `num_locals` is the number of locals in the frame.
    /// - `mut_ref_indexes` are the indexes of mutable reference parameters.
    /// - `immut_ref_indexes` are the indexes of immutable reference parameters.
    /// - `caller_ref_param_map` maps the reference parameter's index to the access path tree
    ///    node corresponding to the reference parameter in the caller's `FrameRefState`.
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
            // initially, all locals are non-ref values
            locals: vec![Value::NonRef; num_locals],
            access_path_tree_roots: AccessPathTreeRootsInfo {
                locals: UnorderedMap::with_hasher(FxBuildHasher::default()),
                globals: UnorderedMap::with_hasher(FxBuildHasher::default()),
                reference_params: UnorderedMap::with_hasher(FxBuildHasher::default()),
            },
            ref_table: UnorderedMap::with_hasher(FxBuildHasher::default()),
            next_ref_id: 0,
            caller_ref_param_map,
        };
        // Locals corresponding to reference parameters are handled below.
        for index in mut_ref_indexes {
            let node_id = QualifiedNodeID::reference_param_root(index);
            this.ensure_reference_param_root_exists(index);
            let new_ref_id = this.make_new_ref_to_existing_node(node_id, true)?;
            *safe_unwrap!(this.locals.get_mut(index)) = Value::Ref(new_ref_id);
        }
        for index in immut_ref_indexes {
            let node_id = QualifiedNodeID::reference_param_root(index);
            this.ensure_reference_param_root_exists(index);
            let new_ref_id = this.make_new_ref_to_existing_node(node_id, false)?;
            *safe_unwrap!(this.locals.get_mut(index)) = Value::Ref(new_ref_id);
        }
        Ok(this)
    }

    /// Check if the reference has been poisoned.
    fn poison_check(&self, ref_id: RefID) -> PartialVMResult<()> {
        let poisoned = safe_unwrap!(self.ref_table.get(&ref_id)).poisoned;
        if poisoned {
            let msg = "Poisoned reference accessed".to_string();
            return ref_check_failure!(msg);
        }
        Ok(())
    }

    /// Get the reference information for the given `ref_id`.
    fn get_ref_info(&self, ref_id: &RefID) -> PartialVMResult<&ReferenceInfo> {
        Ok(safe_unwrap!(self.ref_table.get(ref_id)))
    }

    /// Poison the references related to the given `node`.
    /// Specify which references to poison using `visit_kind` and `filter`.
    fn poison_refs_of_node(
        &mut self,
        node: &QualifiedNodeID,
        visit_kind: VisitKind,
        filter: ReferenceFilter,
    ) -> PartialVMResult<()> {
        let Some(tree) = self
            .access_path_tree_roots
            .maybe_get_mut_access_path_tree(&node.root)
        else {
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

    /// Perform a destructive write via a mutable reference to the given `node`.
    fn destructive_write_via_mut_ref(&mut self, node: &QualifiedNodeID) -> PartialVMResult<()> {
        // Poison all immutable references of the node, its descendants, and ancestors.
        self.poison_refs_of_node(node, VisitKind::SelfOnly, ReferenceFilter::ImmutOnly)?;
        self.poison_refs_of_node(
            node,
            VisitKind::StrictDescendants,
            ReferenceFilter::ImmutOnly,
        )?;
        self.poison_refs_of_node(node, VisitKind::StrictAncestors, ReferenceFilter::ImmutOnly)?;

        // Poison all mutable references of the node's strict descendants.
        // Note that mutable references of the node itself are not poisoned, which is needed
        // to keep consistent with the static bytecode verifier reference rules.
        self.poison_refs_of_node(node, VisitKind::StrictDescendants, ReferenceFilter::MutOnly)?;

        Ok(())
    }

    /// Lock the entire subtree rooted at the given `node` with the specified `lock`.
    /// If any node in the subtree is already exclusively locked, it returns an invariant error.
    fn lock_node_subtree(&mut self, node: &QualifiedNodeID, lock: Lock) -> PartialVMResult<()> {
        let tree = self
            .access_path_tree_roots
            .get_mut_access_path_tree(&node.root)?;
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

    /// Release all locks on the entire subtree rooted at the given `node`.
    fn release_lock_node_subtree(&mut self, node: &QualifiedNodeID) -> PartialVMResult<()> {
        let tree = self
            .access_path_tree_roots
            .get_mut_access_path_tree(&node.root)?;
        let action = |node: &mut AccessPathTreeNode| {
            node.lock = None;
            Ok(())
        };
        tree.visit_self(node.node_id, action)?;
        tree.visit_strict_descendants(node.node_id, action)?;
        Ok(())
    }

    /// Consume the reference with the given `ref_id`.
    /// This will get rid of `ref_id` from various data structures.
    fn purge_reference(&mut self, ref_id: RefID) -> PartialVMResult<()> {
        let info = safe_unwrap!(self.ref_table.remove(&ref_id));
        let node = self.get_mut_access_path_tree_node(&info.access_path_tree_node)?;
        node.refs.remove(&ref_id);
        Ok(())
    }

    /// Make a new reference to an existing node in the access path tree.
    fn make_new_ref_to_existing_node(
        &mut self,
        qualified_node_id: QualifiedNodeID,
        is_mutable: bool,
    ) -> PartialVMResult<RefID> {
        let new_ref_id = RefID(self.next_ref_id);
        self.next_ref_id = safe_unwrap!(self.next_ref_id.checked_add(1));

        let access_path_tree_node = self.get_mut_access_path_tree_node(&qualified_node_id)?;
        // Connect the `access_path_tree_node` to the new reference.
        // We just made this `new_ref_id`, so it must not already exist in the `access_path_tree_node`'s refs.
        safe_assert!(access_path_tree_node.refs.insert(new_ref_id));

        // Connect the new reference to the `access_path_tree_node`.
        self.ref_table.insert(new_ref_id, ReferenceInfo {
            is_mutable,
            poisoned: false,
            access_path_tree_node: qualified_node_id,
        });

        Ok(new_ref_id)
    }

    /// Ensure that the local root exists for the given index.
    fn ensure_local_root_exists(&mut self, index: usize) {
        self.access_path_tree_roots
            .locals
            .entry(index)
            .or_insert_with(AccessPathTree::new);
    }

    /// Ensure that the global root exists for the given type.
    fn ensure_global_root_exists(&mut self, type_: Type) {
        self.access_path_tree_roots
            .globals
            .entry(type_)
            .or_insert_with(AccessPathTree::new);
    }

    /// Ensure that the reference parameter root exists for the given parameter index.
    fn ensure_reference_param_root_exists(&mut self, param_index: usize) {
        self.access_path_tree_roots
            .reference_params
            .entry(param_index)
            .or_insert_with(AccessPathTree::new);
    }

    /// Get or create a new descendant node in the access_path_tree_node, given the parent node ID and the access path.
    /// Will also create any intermediate nodes if needed.
    fn get_or_create_descendant_node(
        &mut self,
        parent_id: &QualifiedNodeID,
        access_path: &[EdgeLabel],
    ) -> PartialVMResult<QualifiedNodeID> {
        let access_path_tree = self
            .access_path_tree_roots
            .get_mut_access_path_tree(&parent_id.root)?;
        let mut node_id = parent_id.node_id;
        for label in access_path {
            node_id = access_path_tree.get_or_create_child_node(node_id, *label)?;
        }
        Ok(QualifiedNodeID {
            root: parent_id.root.clone(),
            node_id,
        })
    }

    /// Does the subtree rooted at `node` have any references that match the given `filter`?
    fn subtree_has_references(
        &self,
        node: &QualifiedNodeID,
        filter: ReferenceFilter,
    ) -> PartialVMResult<bool> {
        let access_path_tree = self
            .access_path_tree_roots
            .get_access_path_tree(&node.root)?;
        // Note that the node itself is included in the descendants.
        for descendant in access_path_tree.get_descendants_iter(node.node_id) {
            let access_path_tree_node = safe_unwrap!(access_path_tree.nodes.get(descendant));
            for ref_ in access_path_tree_node.refs.iter() {
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

    /// Get a mutable reference to the access path tree node.
    fn get_mut_access_path_tree_node(
        &mut self,
        node: &QualifiedNodeID,
    ) -> PartialVMResult<&mut AccessPathTreeNode> {
        self.access_path_tree_roots
            .get_mut_access_path_tree_node(node)
    }

    /// Get the reference param index and the access path (list of edge labels) from it to the
    /// access path tree node. `None` if node is derived from a reference parameter.
    fn get_access_path_from_ref_param(
        &self,
        qualified_node_id: &QualifiedNodeID,
    ) -> PartialVMResult<Option<(usize, Vec<usize>)>> {
        let AccessPathTreeRoot::ReferenceParameter { param_index } = qualified_node_id.root else {
            return Ok(None);
        };
        let access_path_tree = self
            .access_path_tree_roots
            .get_access_path_tree(&qualified_node_id.root)?;
        let path = access_path_tree.get_access_path_from_root(qualified_node_id.node_id)?;
        Ok(Some((param_index, path)))
    }
}

impl ReferenceInfo {
    /// Check if this reference is poisoned.
    /// Invariant violation if it is.
    fn poison_check(&self) -> PartialVMResult<()> {
        if self.poisoned {
            let msg = "Poisoned reference accessed".to_string();
            return ref_check_failure!(msg);
        }
        Ok(())
    }
}

impl RefCheckState {
    /// Create a new `RefCheckState` with empty stacks.
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

    /// Get a reference to the latest frame state.
    fn get_latest_frame_state(&self) -> PartialVMResult<&FrameRefState> {
        Ok(safe_unwrap!(self.frame_stack.last()))
    }

    /// Get a mutable reference to the latest frame state.
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

    /// Check if `ref_id` is poisoned in the latest frame state.
    fn poison_check(&self, ref_id: RefID) -> PartialVMResult<()> {
        self.get_latest_frame_state()?.poison_check(ref_id)
    }

    /// Remove tracking of `ref_id` from the latest frame state.
    fn purge_reference(&mut self, ref_id: RefID) -> PartialVMResult<()> {
        self.get_mut_latest_frame_state()?.purge_reference(ref_id)
    }

    /// Transition for `CopyLoc` instruction.
    fn copy_loc(&mut self, index: u8) -> PartialVMResult<()> {
        let index = index.into();
        let frame_state_immut = self.get_latest_frame_state()?;
        let value = safe_unwrap!(frame_state_immut.locals.get(index));
        match value {
            Value::NonRef => {
                self.push_non_refs_to_shadow_stack(1);
                let node = QualifiedNodeID::local_root(index);
                let frame_state_mut = self.get_mut_latest_frame_state()?;
                // Poison all mutable references to the location rooted at `index`.
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
                let access_path_tree_node = ref_info.access_path_tree_node.clone();
                let is_mutable = ref_info.is_mutable;
                let frame_state_mut = self.get_mut_latest_frame_state()?;
                // Create a new reference to the existing referenced node.
                let new_ref_id = frame_state_mut
                    .make_new_ref_to_existing_node(access_path_tree_node, is_mutable)?;
                self.push_ref_to_shadow_stack(new_ref_id);
            },
        }
        Ok(())
    }

    /// Transition for `MoveLoc` instruction.
    fn move_loc(&mut self, index: u8) -> PartialVMResult<()> {
        let index = index.into();
        let frame_state = self.get_mut_latest_frame_state()?;
        let mut value = Value::NonRef;
        // Replace the shadow local at `index` with a non-ref value.
        std::mem::swap(safe_unwrap!(frame_state.locals.get_mut(index)), &mut value);

        match value {
            Value::NonRef => {
                let node = QualifiedNodeID::local_root(index);
                let frame_state_mut = self.get_mut_latest_frame_state()?;
                // Poison all references to the location rooted at `index`.
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
                // No poison checks here, because we do not consider this a reference access.
                // This will allow a poisoned reference to be moved to the stack and popped.
            },
        }

        self.push_to_shadow_stack(value);
        Ok(())
    }

    /// Transition for `StLoc` instruction.
    fn st_loc(&mut self, index: u8) -> PartialVMResult<()> {
        let index = index.into();
        let mut value_1 = self.pop_from_shadow_stack()?;
        let frame_state = self.get_mut_latest_frame_state()?;
        let value_2 = safe_unwrap!(frame_state.locals.get_mut(index));

        // Store the value from the shadow stack into the local at `index`.
        // `value_1` will then have the value that was previously in the local.
        std::mem::swap(value_2, &mut value_1);

        match value_1 {
            Value::NonRef => {
                let node = QualifiedNodeID::local_root(index);
                let frame_state_mut = self.get_mut_latest_frame_state()?;
                // The value stored at `index` is being overwritten.
                // Poison all references to the location rooted at `index`.
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
                // Note: we do not check if the reference overwritten was poisoned.
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

    /// Transition for `WriteRef` instruction.
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
        let node = ref_info.access_path_tree_node.clone();
        frame_state.destructive_write_via_mut_ref(&node)?;

        frame_state.purge_reference(ref_id)?;

        Ok(())
    }

    /// Transition for `FreezeRef` instruction.
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
        let node = ref_info.access_path_tree_node.clone();
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

    /// Derive a child reference with `label` from the parent reference on the top of the shadow stack.
    /// The mutability of the reference is given by `MUTABLE`.
    /// Transition for family of instructions borrowing fields of structs and variants.
    fn borrow_child_with_label<const MUTABLE: bool>(
        &mut self,
        label: EdgeLabel,
    ) -> PartialVMResult<()> {
        let ref_to_borrow_from = self.pop_from_shadow_stack()?;
        let Value::Ref(parent_ref_id) = ref_to_borrow_from else {
            let msg = "Expected a reference on the stack".to_string();
            return ref_check_failure!(msg);
        };
        // We perform poison check right away, although it could be delayed until reference is used.
        // If we delay, we would need to ensure poisoning is transferred to children.
        self.poison_check(parent_ref_id)?;

        let frame_state = self.get_mut_latest_frame_state()?;
        let ref_info = frame_state.get_ref_info(&parent_ref_id)?;
        // If we are borrowing a mutable reference, the parent reference must also be mutable.
        safe_assert!(!MUTABLE || ref_info.is_mutable);

        let parent_node_id = ref_info.access_path_tree_node.clone();
        let child_node_id =
            frame_state.get_or_create_descendant_node(&parent_node_id, slice::from_ref(&label))?;

        frame_state.purge_reference(parent_ref_id)?;

        let new_ref_id = frame_state.make_new_ref_to_existing_node(child_node_id, MUTABLE)?;
        self.push_ref_to_shadow_stack(new_ref_id);

        Ok(())
    }

    /// Transition for borrow global family of instructions.
    /// We currently abstract over all addresses and only use types.
    fn borrow_global<const MUTABLE: bool>(&mut self, type_: Type) -> PartialVMResult<()> {
        let _ = self.pop_from_shadow_stack()?;

        let frame_state = self.get_mut_latest_frame_state()?;
        frame_state.ensure_global_root_exists(type_.clone());

        let node_id = QualifiedNodeID::global_root(type_);
        // Unlike references to locals (where borrowing itself does not lead to violations, but use of
        // poisoned refs does), we perform a stricter check here (similar to bytecode verifier).
        if MUTABLE && frame_state.subtree_has_references(&node_id, ReferenceFilter::All)? {
            let msg = "Cannot borrow_global_mut while there are existing references".to_string();
            return ref_check_failure!(msg);
        } else if !MUTABLE
            && frame_state.subtree_has_references(&node_id, ReferenceFilter::MutOnly)?
        {
            let msg = "Cannot borrow_global while there are mutable references".to_string();
            return ref_check_failure!(msg);
        }

        let new_ref_id = frame_state.make_new_ref_to_existing_node(node_id, MUTABLE)?;
        self.push_ref_to_shadow_stack(new_ref_id);

        Ok(())
    }

    /// Transition for `MoveFrom` and `MoveFromGeneric` instruction.
    fn move_from(&mut self, type_: Type) -> PartialVMResult<()> {
        let _ = self.pop_from_shadow_stack()?;

        let frame_state = self.get_mut_latest_frame_state()?;
        frame_state.ensure_global_root_exists(type_.clone());

        let node_id = QualifiedNodeID::global_root(type_);
        // Poison all references to the global type's subtree.
        frame_state.poison_refs_of_node(&node_id, VisitKind::SelfOnly, ReferenceFilter::All)?;
        frame_state.poison_refs_of_node(
            &node_id,
            VisitKind::StrictDescendants,
            ReferenceFilter::All,
        )?;

        self.push_non_refs_to_shadow_stack(1);

        Ok(())
    }

    /// Transition for `MoveTo` and `MoveToGeneric` instructions.
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
        // Note: `MoveTo` only succeeds if global value is not already present, so we do not
        // need to poison references to the global type's subtree.

        Ok(())
    }

    /// Transition for `VecLen` instruction.
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

    /// Transition for vector borrow family of instructions.
    fn vec_borrow<const MUTABLE: bool>(&mut self) -> PartialVMResult<()> {
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
        safe_assert!(!MUTABLE || ref_info.is_mutable);

        let parent_node_id = ref_info.access_path_tree_node.clone();
        // Note that we abstract over all indices and use `0` to represent the label.
        // This is stricter than necessary, but it is cheaper than maintaining a per-index access path tree node.
        let abstracted_label = 0;
        let child_node_id = frame_state
            .get_or_create_descendant_node(&parent_node_id, slice::from_ref(&abstracted_label))?;

        frame_state.purge_reference(parent_ref_id)?;

        let new_ref_id = frame_state.make_new_ref_to_existing_node(child_node_id, MUTABLE)?;
        self.push_ref_to_shadow_stack(new_ref_id);

        Ok(())
    }

    /// Transition for `VecPushBack` instruction.
    fn vec_push_back(&mut self) -> PartialVMResult<()> {
        let _ = self.pop_from_shadow_stack()?;
        let vec_ref = self.pop_from_shadow_stack()?;
        let Value::Ref(vec_ref_id) = vec_ref else {
            let msg = "vec_push_back expected a reference on the stack".to_string();
            return ref_check_failure!(msg);
        };
        self.poison_check(vec_ref_id)?;

        // Note: we are not checking if the reference is mutable here, as such type checks
        // are not part of reference checking.
        let frame_state = self.get_mut_latest_frame_state()?;
        frame_state.purge_reference(vec_ref_id)?;

        // Note: we do not consider this to be a destructive update to the vector,
        // and references to other elements in the vector would still be un-poisoned.
        Ok(())
    }

    /// Transition for `VecPopBack` instruction.
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

        let node = ref_info.access_path_tree_node.clone();
        frame_state.destructive_write_via_mut_ref(&node)?;

        frame_state.purge_reference(vec_ref_id)?;

        self.push_non_refs_to_shadow_stack(1);

        Ok(())
    }

    /// Transition for `VecSwap` instruction.
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

        let node = ref_info.access_path_tree_node.clone();
        frame_state.destructive_write_via_mut_ref(&node)?;

        frame_state.purge_reference(vec_ref_id)?;

        Ok(())
    }

    /// Transition for a function call (`Call`, `CallGeneric`, `CallClosure`).
    fn core_call(
        &mut self,
        num_params: usize,
        num_locals: usize,
        mask: ClosureMask,
    ) -> PartialVMResult<()> {
        // Keep track of all reference argument's IDs.
        let mut ref_arg_ids = Vec::new();
        // Keep track of mutable reference param indexes.
        let mut mut_ref_indexes = Vec::new();
        // Keep track of immutable reference param indexes.
        let mut immut_ref_indexes = Vec::new();
        // Map from parameter index to the access path tree node of a reference parameter.
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
                let access_path_tree_node = ref_info.access_path_tree_node.clone();
                // Make sure that there are no overlaps with a mutable reference.
                if ref_info.is_mutable {
                    frame_state.lock_node_subtree(&access_path_tree_node, Lock::Exclusive)?;
                    // Having a mutable reference argument is the same as performing a destructive write.
                    frame_state.destructive_write_via_mut_ref(&access_path_tree_node)?;
                    mut_ref_indexes.push(i);
                } else {
                    frame_state.lock_node_subtree(&access_path_tree_node, Lock::Shared)?;
                    immut_ref_indexes.push(i);
                }
                ref_arg_ids.push(ref_id);
                ref_param_map.insert(i, access_path_tree_node);
            }
        }
        for ref_id in ref_arg_ids {
            let frame_state = self.get_mut_latest_frame_state()?;
            let ref_info = frame_state.get_ref_info(&ref_id)?;
            let access_path_tree_node = ref_info.access_path_tree_node.clone();
            // Release locks so that they don't interfere with the next call.
            frame_state.release_lock_node_subtree(&access_path_tree_node)?;
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

    /// Push a new frame onto the frame stack.
    /// `mut_ref_indexes` and `immut_ref_indexes` are the indexes of mutable and immutable
    /// reference parameters, respectively.
    /// `ref_param_map` maps parameter indexes to the access path tree nodes of reference parameters.
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

    /// Check that any returned references must be derived from reference parameters.
    /// Returned references must also be exclusive (no overlap with any mut refs).
    /// Use the access path of returned references to transform them to the caller's frame.
    /// Current frame will be popped from the stack.
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

                    let access_path_tree_node = ref_info.access_path_tree_node.clone();
                    // Check if each reference being returned is derived from a reference parameter.
                    let Some((param_index, access_path)) =
                        frame_state.get_access_path_from_ref_param(&access_path_tree_node)?
                    else {
                        let msg =
                            "Returning a reference that is not derived from a reference parameter"
                                .to_string();
                        return ref_check_failure!(msg);
                    };
                    // Check that mutable references are returned "exclusively".
                    if is_mutable {
                        frame_state.lock_node_subtree(&access_path_tree_node, Lock::Exclusive)?;
                    } else {
                        frame_state.lock_node_subtree(&access_path_tree_node, Lock::Shared)?;
                    }
                    // This frame will be thrown away, so no need to unlock the nodes.
                    // Compute the transformation of the reference to one on the caller's frame,
                    // if there is a caller.
                    if !has_caller {
                        continue;
                    }
                    let caller_access_path_tree_node =
                        safe_unwrap!(frame_state.caller_ref_param_map.get(&param_index)).clone();
                    let callers_frame = self.get_mut_callers_frame_state()?;
                    let transformed_node = callers_frame.get_or_create_descendant_node(
                        &caller_access_path_tree_node,
                        &access_path,
                    )?;
                    let transformed_ref_id = callers_frame
                        .make_new_ref_to_existing_node(transformed_node, is_mutable)?;
                    transformed_values.push(Some(transformed_ref_id));
                } else if has_caller {
                    // The returned value is not a reference and there is a caller.
                    transformed_values.push(None);
                }
            }
            if has_caller {
                // Transform the shadow stack reference values to the caller's frame.
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
