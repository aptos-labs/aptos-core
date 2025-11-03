// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! Implements memory safety analysis. This is the current default implementation.
//!
//! This implementation replicates the existing bytecode verifier and compiler v1
//! implementation, using the same shared 'borrow graph' logic. For documentation
//! of the borrow graph, see `move_borrow_graph::graph::BorrowGraph`. For
//! comparison with the bytecode verifier, see
//! `move-bytecode-verifier/src/reference_safety/abstract_state.rs`.
//!
//! A main difference between the bytecode verifier and this implementation
//! is that we are dealing with a register machine whereas the bytecode
//! verifier with a stack machine. Therefore, we need to move or copy
//! parameters to instructions explicitly out of their registers, whereas
//! in the stack machine they are put on the stack in independent instructions.
//! Another difference is the need for generating good error messages which
//! the bytecode verifier has not.
//!
//! Prerequisites: there are no uninitialized locals.

use crate::pipeline::{
    livevar_analysis_processor::{LiveVarAnnotation, LiveVarInfoAtCodeOffset},
    reference_safety::{LifetimeAnnotation, LifetimeInfo, LifetimeInfoAtCodeOffset},
};
use codespan_reporting::diagnostic::Severity;
use itertools::Itertools;
use move_binary_format::file_format::CodeOffset;
use move_borrow_graph::{graph::BorrowGraph, references::RefID};
use move_model::{
    ast::{AccessSpecifierKind, ResourceSpecifier, TempIndex},
    model::{FunId, FunctionEnv, GlobalEnv, Loc, QualifiedId, QualifiedInstId, StructId},
    ty::{ReferenceKind, Type},
};
use move_stackless_bytecode::{
    dataflow_analysis::{DataflowAnalysis, TransferFunctions},
    dataflow_domains::{AbstractDomain, JoinResult},
    function_target::{FunctionData, FunctionTarget},
    function_target_pipeline::{FunctionTargetProcessor, FunctionTargetsHolder},
    stackless_bytecode::{AssignKind, AttrId, Bytecode, Operation},
    stackless_control_flow_graph::StacklessControlFlowGraph,
};
use std::{
    collections::{BTreeMap, BTreeSet},
    fmt::{Display, Formatter},
    ops::Range,
    rc::Rc,
};
// =================================================================================================
// Lifetime Analysis Domain

/// The domain with which the analysis framework will be instantiated.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LifetimeState {
    /// An instantiation of the borrow graph shared with the runtime. We use
    /// `AttrId` for locations and the `Label` type defined below. Effectively,
    /// each edge in the graph has associated `AttrId` for identifying instruction
    /// (and location) which created it, and `Vec<Label>` for describing the operations
    /// via which the edge was established.
    borrow_graph: BorrowGraph<AttrId, Label>,
    /// Locals and their associated abstract values, to be indexed by TempIndex.
    locals: Vec<AbstractValue>,
    /// Next available free id for fresh reference ids in the borrow graph.
    next_ref_id: usize,
}

impl Default for LifetimeState {
    fn default() -> Self {
        Self {
            borrow_graph: BorrowGraph::new(),
            locals: vec![],
            next_ref_id: 0,
        }
    }
}

/// A label of an edge in the borrow graph.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
enum Label {
    Local(TempIndex),
    Global(QualifiedId<StructId>),
    /// Field selection at offset.
    Field(QualifiedId<StructId>, usize),
}

/// An abstract value associated with a temporary.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum AbstractValue {
    NonReference,
    Reference(RefID),
}

impl AbstractValue {
    fn ref_id(&self) -> Option<RefID> {
        match self {
            AbstractValue::Reference(id) => Some(*id),
            AbstractValue::NonReference => None,
        }
    }
}

impl LifetimeState {
    fn new(target: &FunctionTarget) -> LifetimeState {
        let num_locals = target.get_local_count();
        let next_id = num_locals + 1;
        let mut state = Self {
            locals: vec![AbstractValue::NonReference; num_locals],
            borrow_graph: BorrowGraph::new(),
            next_ref_id: next_id,
        };
        for param in target.get_parameters() {
            let param_ty = target.get_local_type(param);
            if param_ty.is_reference() {
                let id = RefID::new(param);
                state
                    .borrow_graph
                    .new_ref(id, param_ty.is_mutable_reference());
                state.locals[param] = AbstractValue::Reference(id)
            }
        }
        // Locals are treated like fields of the 'frame' of a function invocation,
        // so create a virtual (mutable) reference which is called the 'frame root'.
        // The frame root is also used to represent borrows on globals.
        state.borrow_graph.new_ref(state.frame_root(), true);
        state
    }

    fn locals_range(&self) -> Range<TempIndex> {
        0..self.locals.len()
    }

    fn frame_root(&self) -> RefID {
        // next_id starts at len() + 1, so we can represent the frame root at len()
        RefID::new(self.locals.len())
    }

    fn is_canonical(&self) -> bool {
        // A state is assumed to be canonical for joining. The canonical form guarantees
        // that a local has assigned the same RefID in each state.
        // TODO: this is a rather inefficient way to do joining but for now we replicate
        //   here what the bytecode verifier also does.
        self.locals.len() + 1 == self.next_ref_id
            && self.locals.iter().enumerate().all(|(local, value)| {
                value
                    .ref_id()
                    .map(|id| RefID::new(local) == id)
                    .unwrap_or(true)
            })
    }

    fn canonicalize(&mut self) {
        let mut id_map = BTreeMap::new();
        id_map.insert(self.frame_root(), self.frame_root());
        let locals = self
            .locals
            .iter()
            .enumerate()
            .map(|(local, value)| match value {
                AbstractValue::Reference(old_id) => {
                    let new_id = RefID::new(local);
                    id_map.insert(*old_id, new_id);
                    AbstractValue::Reference(new_id)
                },
                AbstractValue::NonReference => AbstractValue::NonReference,
            })
            .collect::<Vec<_>>();
        assert!(self.locals.len() == locals.len());
        let mut borrow_graph = self.borrow_graph.clone();
        borrow_graph.remap_refs(&id_map);
        let canonical_state = LifetimeState {
            locals,
            borrow_graph,
            next_ref_id: self.locals.len() + 1,
        };
        *self = canonical_state
    }

    fn release(&mut self, id: RefID) {
        self.borrow_graph.release(id);
    }

    fn release_local(&mut self, local: TempIndex) {
        if let Some(id) = self.locals[local].ref_id() {
            self.release(id);
            self.locals[local] = AbstractValue::NonReference
        }
    }

    fn new_ref(&mut self, is_mut: bool) -> RefID {
        let id = RefID::new(self.next_ref_id);
        self.borrow_graph.new_ref(id, is_mut);
        self.next_ref_id += 1;
        id
    }
}

impl AbstractDomain for LifetimeState {
    /// Implements the join operator for the data flow analysis framework.
    // TODO: this has been replicated from what the bytecode verifier does,
    //   it can be done likely more efficiently.
    fn join(&mut self, other: &Self) -> JoinResult {
        debug_assert_eq!(self.locals.len(), other.locals.len());

        // Ensure both states are canonical
        if !self.is_canonical() {
            self.canonicalize()
        }
        let mut other = other.clone();
        if !other.is_canonical() {
            other.canonicalize()
        }
        debug_assert_eq!(self.next_ref_id, other.next_ref_id);

        // Compute the locals which are references in both graphs.
        let mut self_graph = self.borrow_graph.clone();
        let locals = self
            .locals
            .iter()
            .zip(&other.locals)
            .map(|(self_value, other_value)| {
                match (self_value, other_value) {
                    (AbstractValue::Reference(id), AbstractValue::NonReference) => {
                        self_graph.release(*id);
                        AbstractValue::NonReference
                    },
                    (AbstractValue::NonReference, AbstractValue::Reference(id)) => {
                        other.borrow_graph.release(*id);
                        AbstractValue::NonReference
                    },
                    (v1, v2) => {
                        // The local has a value on each side, add it to the state
                        assert!(v1 == v2);
                        *v1
                    },
                }
            })
            .collect();
        // Join the underlying graph.
        let borrow_graph = self_graph.join(&other.borrow_graph);
        let next_id = self.next_ref_id;
        let joined = Self {
            locals,
            borrow_graph,
            next_ref_id: next_id,
        };
        // Check for diff
        let locals_unchanged = self
            .locals
            .iter()
            .zip(&joined.locals)
            .all(|(self_value, joined_value)| self_value == joined_value);
        if locals_unchanged && self.borrow_graph.leq(&joined.borrow_graph) {
            JoinResult::Unchanged
        } else {
            *self = joined;
            JoinResult::Changed
        }
    }
}

// -------------------------------------------------------------------------------------------------
// Core Operations on Borrow Analysis State

impl LifetimeState {
    fn add_copy(&mut self, code_id: AttrId, parent: RefID, child: RefID) {
        self.borrow_graph.add_strong_borrow(code_id, parent, child)
    }

    fn add_borrow(&mut self, code_id: AttrId, parent: RefID, child: RefID) {
        self.borrow_graph.add_weak_borrow(code_id, parent, child)
    }

    fn add_field_borrow(
        &mut self,
        code_id: AttrId,
        parent: RefID,
        struct_id: QualifiedId<StructId>,
        offset: usize,
        child: RefID,
    ) {
        self.borrow_graph.add_strong_field_borrow(
            code_id,
            parent,
            Label::Field(struct_id, offset),
            child,
        )
    }

    fn add_local_borrow(&mut self, code_id: AttrId, local: TempIndex, id: RefID) {
        self.borrow_graph.add_strong_field_borrow(
            code_id,
            self.frame_root(),
            Label::Local(local),
            id,
        )
    }

    fn add_resource_borrow(
        &mut self,
        code_id: AttrId,
        struct_id: QualifiedId<StructId>,
        id: RefID,
    ) {
        self.borrow_graph.add_weak_field_borrow(
            code_id,
            self.frame_root(),
            Label::Global(struct_id),
            id,
        )
    }

    /// Checks if local is borrowed
    fn is_local_borrowed(&self, local: TempIndex) -> bool {
        self.borrow_graph
            .has_consistent_borrows(self.frame_root(), Some(Label::Local(local)))
    }

    /// Checks if local is mutably borrowed
    fn is_local_mutably_borrowed(&self, local: TempIndex) -> bool {
        self.borrow_graph
            .has_consistent_mutable_borrows(self.frame_root(), Some(Label::Local(local)))
    }

    /// Checks if global is borrowed
    fn is_global_borrowed(&self, struct_id: QualifiedId<StructId>) -> bool {
        self.borrow_graph
            .has_consistent_borrows(self.frame_root(), Some(Label::Global(struct_id)))
    }

    /// Checks if global is mutably borrowed
    fn is_global_mutably_borrowed(&self, struct_id: QualifiedId<StructId>) -> bool {
        self.borrow_graph
            .has_consistent_mutable_borrows(self.frame_root(), Some(Label::Global(struct_id)))
    }

    /// Returns all currently active global borrow edges.
    fn global_borrow_edges(
        &self,
    ) -> impl Iterator<Item = (AttrId, QualifiedId<StructId>, RefID)> + use<> {
        self.borrow_graph
            .out_edges(self.frame_root())
            .into_iter()
            .flat_map(|(code_id, labels, _strong, target)| {
                labels.into_iter().filter_map(move |l| {
                    if let Label::Global(struct_id) = l {
                        Some((code_id, struct_id, target))
                    } else {
                        None
                    }
                })
            })
    }
}

// ================================================================================================
// Lifetime Analysis

/// A structure providing context information for operations during lifetime analysis.
/// This encapsulates the function target which is analyzed, giving also access to
/// the global model. Live var annotations are attached which are evaluated during
/// analysis.
struct LifeTimeAnalysis<'env> {
    /// The function target being analyzed
    target: &'env FunctionTarget<'env>,
    /// The live-var annotation extracted from a previous phase
    live_var_annotation: &'env LiveVarAnnotation,
}

/// A structure encapsulating, in addition to the analysis context, context
/// about the current instruction step being processed.
struct LifetimeAnalysisStep<'env, 'state> {
    /// The analysis context
    parent: &'env LifeTimeAnalysis<'env>,
    /// The attribute id at the code offset
    attr_id: AttrId,
    /// Lifetime information at the given code offset
    alive: &'env LiveVarInfoAtCodeOffset,
    /// Mutable reference to the analysis state
    state: &'state mut LifetimeState,
}

impl LifeTimeAnalysis<'_> {
    fn new_step<'a>(
        &'a self,
        code_offset: CodeOffset,
        attr_id: AttrId,
        state: &'a mut LifetimeState,
    ) -> LifetimeAnalysisStep<'a, 'a> {
        let alive = self
            .live_var_annotation
            .get_live_var_info_at(code_offset)
            .expect("live var info");
        LifetimeAnalysisStep {
            parent: self,
            attr_id,
            alive,
            state,
        }
    }
}

// -------------------------------------------------------------------------------------------------
// Analysing, Diagnosing, and Primitives

impl LifetimeAnalysisStep<'_, '_> {
    /// Get the location associated with bytecode attribute.
    fn loc(&self, id: AttrId) -> Loc {
        self.target().get_bytecode_loc(id)
    }

    /// Returns the location of the current instruction
    fn cur_loc(&self) -> Loc {
        self.loc(self.attr_id)
    }

    /// Gets a string for a local to be displayed in error messages
    fn display(&self, local: TempIndex) -> String {
        self.target().get_local_name_for_error_message(local)
    }

    /// Display a label path.
    fn display_path(&self, path: &[Label]) -> String {
        path.iter()
            .rev()
            .map(|l| l.display(self.target()))
            .join(" via ")
    }

    /// Returns "<prefix>`<name>` " if local has name, otherwise empty.
    fn display_name_or_empty(&self, prefix: &str, local: TempIndex) -> String {
        self.target()
            .get_local_name_opt(local)
            .map(|s| format!("{}`{}`", prefix, s))
            .unwrap_or_default()
    }

    /// Get the type associated with local.
    fn ty(&self, local: TempIndex) -> &Type {
        self.target().get_local_type(local)
    }

    /// Get expected reference. If the value is not a reference, report as a bug.
    fn expect_ref(&self, value: AbstractValue) -> RefID {
        if let Some(id) = value.ref_id() {
            id
        } else {
            self.global_env()
                .diag(Severity::Bug, &self.cur_loc(), "expected reference");
            self.state.frame_root()
        }
    }

    /// Returns true if the local is used after this program point.
    fn used_after(
        &self,
        src: TempIndex,
        remaining_srcs: &[TempIndex],
        dests: &[TempIndex],
    ) -> bool {
        // is used if in the after set and not assigned in this instruction
        self.alive.after.contains_key(&src) && !dests.contains(&src)
            // ... or is appearing again in the remaining sources
            || remaining_srcs.contains(&src)
    }

    /// Moves or copies the value in the source, depending on whether it is used after. If the
    /// source is borrowed it cannot be moved and will always be copied. Returns
    /// the resulting abstract value.
    fn move_or_copy(
        &mut self,
        src: TempIndex,
        remaining_srcs: &[TempIndex],
        dests: &[TempIndex],
    ) -> AbstractValue {
        if !self.state.is_local_borrowed(src) && !self.used_after(src, remaining_srcs, dests) {
            self.move_(src)
        } else {
            self.copy(src)
        }
    }

    /// Calls move or copy and expects the value to be a reference.
    fn move_or_copy_ref(
        &mut self,
        src: TempIndex,
        remaining_srcs: &[TempIndex],
        dests: &[TempIndex],
    ) -> RefID {
        let value = self.move_or_copy(src, remaining_srcs, dests);
        self.expect_ref(value)
    }

    /// Moves the value out of the source.
    fn move_(&mut self, src: TempIndex) -> AbstractValue {
        let old_value = std::mem::replace(&mut self.state.locals[src], AbstractValue::NonReference);
        match old_value {
            AbstractValue::Reference(id) => AbstractValue::Reference(id),
            AbstractValue::NonReference => {
                if self.state.is_local_borrowed(src) {
                    self.error_with_hints(
                        self.cur_loc(),
                        format!("cannot move {} which is still borrowed", self.display(src)),
                        "move attempted here",
                        self.borrow_info_for_local(src).into_iter(),
                    )
                }
                AbstractValue::NonReference
            },
        }
    }

    /// Copies the value from the source.
    fn copy(&mut self, src: TempIndex) -> AbstractValue {
        match self.state.locals[src] {
            AbstractValue::Reference(id) => {
                let new_id = self.state.new_ref(self.state.borrow_graph.is_mutable(id));
                self.state.add_copy(self.attr_id, id, new_id);
                AbstractValue::Reference(new_id)
            },
            AbstractValue::NonReference => {
                if self.state.is_local_mutably_borrowed(src) {
                    self.error_with_hints(
                        self.cur_loc(),
                        format!(
                            "cannot copy {} which is still mutably borrowed",
                            self.display(src)
                        ),
                        "copy attempted here",
                        self.borrow_info_for_local(src).into_iter(),
                    )
                }
                AbstractValue::NonReference
            },
        }
    }

    /// Drops the value in the source.
    fn drop(&mut self, src: TempIndex) {
        match self.state.locals[src] {
            AbstractValue::Reference(_id) => {
                self.state.release_local(src);
            },
            AbstractValue::NonReference => {
                if self.state.is_local_borrowed(src) {
                    self.error_with_hints(
                        self.cur_loc(),
                        format!("cannot drop {} which is still borrowed", self.display(src)),
                        "dropped here",
                        self.borrow_info_for_local(src).into_iter(),
                    )
                }
            },
        }
    }

    /// Replaces the value in the source, dropping an older value.
    fn replace(&mut self, src: TempIndex, value: AbstractValue) {
        self.drop(src);
        self.state.locals[src] = value;
    }

    /// Reports an error together with hints
    fn error_with_hints(
        &self,
        loc: impl AsRef<Loc>,
        msg: impl AsRef<str>,
        primary: impl AsRef<str>,
        hints: impl Iterator<Item = (Loc, String)>,
    ) {
        self.global_env().diag_with_primary_and_labels(
            Severity::Error,
            loc.as_ref(),
            msg.as_ref(),
            primary.as_ref(),
            hints.collect(),
        )
    }

    #[inline]
    fn global_env(&self) -> &GlobalEnv {
        self.target().global_env()
    }

    #[inline]
    fn target(&self) -> &FunctionTarget<'_> {
        self.parent.target
    }

    fn borrow_info_for_global(&self, struct_id: QualifiedId<StructId>) -> Vec<(Loc, String)> {
        let mut result = vec![];
        self.collect_borrow_info(
            &mut result,
            self.state.frame_root(),
            Some(Label::Global(struct_id)),
        );
        self.collect_usage_info(&mut result, |id| {
            self.state
                .borrow_graph
                .is_borrowed_via(id, &Label::Global(struct_id))
        });
        result
    }

    fn borrow_info_for_local(&self, local: TempIndex) -> Vec<(Loc, String)> {
        let mut result = vec![];
        self.collect_borrow_info(
            &mut result,
            self.state.frame_root(),
            Some(Label::Local(local)),
        );
        self.collect_usage_info(&mut result, |id| {
            self.state
                .borrow_graph
                .is_borrowed_via(id, &Label::Local(local))
        });
        result
    }

    fn borrow_info_for_ref(&self, id: RefID, for_label: Option<Label>) -> Vec<(Loc, String)> {
        let mut result = vec![];
        self.collect_borrow_info(&mut result, id, for_label);
        self.collect_usage_info(&mut result, |other_id| {
            self.state.borrow_graph.is_derived_from(other_id, id)
        });
        result
    }

    fn collect_borrow_info(
        &self,
        result: &mut Vec<(Loc, String)>,
        id: RefID,
        for_label: Option<Label>,
    ) {
        result.extend(
            self.state
                .borrow_graph
                .out_edges(id)
                .into_iter()
                .filter_map(|(code_id, path, _, target)| {
                    let loc = self.target().get_bytecode_loc(code_id);
                    let mut_str = if self.state.borrow_graph.is_mutable(target) {
                        "mutably "
                    } else {
                        ""
                    };
                    if path.is_empty() && for_label.is_none() {
                        Some((loc, format!("previously {}borrowed here", mut_str)))
                    } else if for_label.is_none() || path.contains(&for_label.unwrap()) {
                        Some((
                            loc,
                            format!(
                                "{} previously {}borrowed here",
                                self.display_path(&path),
                                mut_str
                            ),
                        ))
                    } else {
                        None
                    }
                }),
        )
    }

    /// Collect usage information for temporaries alive after this program point and
    /// involved in borrowing as defined by predicate.
    fn collect_usage_info(
        &self,
        result: &mut Vec<(Loc, String)>,
        predicate: impl Fn(RefID) -> bool,
    ) {
        let cands = self
            .state
            .locals
            .iter()
            .enumerate()
            .filter_map(|(temp, value)| {
                value
                    .ref_id()
                    .and_then(|id| if predicate(id) { Some(temp) } else { None })
            });
        result.extend(cands.filter_map(|temp| {
            self.alive.after.get(&temp).map(|info| {
                (
                    info.usage_locations().iter().next().unwrap().clone(),
                    format!(
                        "conflicting reference{} used here",
                        self.display_name_or_empty(" ", temp)
                    ),
                )
            })
        }))
    }

    /// Checks whether a function potentially accesses a global resource which is
    /// currently borrowed.
    fn check_global_access(&mut self, fun_id: QualifiedInstId<FunId>) {
        let fun = self.global_env().get_function(fun_id.to_qualified_id());
        if self.parent.target.func_env.module_env.get_id() != fun_id.module_id
            || fun.is_native()
            || fun.is_inline()
        {
            // Not function in the same module, a native function, or inline
            return;
        }
        let empty_acquires = BTreeSet::new();
        let acquires = fun.get_acquired_structs().unwrap_or(&empty_acquires);

        for (_code_id, struct_id, target) in self.state.global_borrow_edges() {
            let is_mut = self.state.borrow_graph.is_mutable(target);
            if struct_id.module_id == fun.module_env.get_id() && acquires.contains(&struct_id.id) {
                // Try to find the location of the access declaration via the access specifier
                // list.
                let access_origin_hint = fun
                    .get_access_specifiers()
                    .unwrap_or_default()
                    .iter()
                    .find_map(|s| {
                        if s.kind == AccessSpecifierKind::LegacyAcquires
                            && matches!(&s.resource.1,
                    ResourceSpecifier::Resource(s) if s.to_qualified_id() == struct_id)
                        {
                            Some(vec![(s.loc.clone(), "`acquires` declared here".to_owned())])
                        } else {
                            None
                        }
                    })
                    .unwrap_or_else(|| {
                        vec![(
                            fun.get_id_loc(),
                            "`acquires` of this function was inferred".to_owned(),
                        )]
                    });
                self.error_with_hints(
                    self.cur_loc(),
                    format!(
                        "function acquires global `{}` which is currently {}borrowed",
                        self.global_env().display(&struct_id),
                        if is_mut { "mutably " } else { "" }
                    ),
                    "function called here",
                    self.borrow_info_for_global(struct_id)
                        .into_iter()
                        .chain(access_origin_hint),
                )
            }
        }
    }
}

// -------------------------------------------------------------------------------------------------
// Program Steps

impl LifetimeAnalysisStep<'_, '_> {
    fn assign(&mut self, dest: TempIndex, src: TempIndex, kind: AssignKind) {
        if src != dest {
            self.drop(dest);
        }
        let value = match kind {
            AssignKind::Move => self.move_(src),
            AssignKind::Copy => self.copy(src),
            AssignKind::Inferred => self.move_or_copy(src, &[], &[dest]),
            AssignKind::Store => panic!("unexpected assign kind"),
        };
        self.replace(dest, value)
    }

    fn freeze_ref(&mut self, explicit: bool, dest: TempIndex, src: TempIndex) {
        let id = self.move_or_copy_ref(src, &[], &[dest]);
        if !self.state.borrow_graph.is_freezable(id, None) {
            self.error_with_hints(
                self.cur_loc(),
                format!(
                    "cannot freeze {} which is still mutably borrowed",
                    self.display(dest)
                ),
                if explicit {
                    "frozen here"
                } else {
                    "implicitly frozen here"
                },
                self.borrow_info_for_ref(id, None).into_iter(),
            )
        }
        let frozen_id = self.state.new_ref(false);
        self.state.add_copy(self.attr_id, id, frozen_id);
        self.state.release(id);
        self.replace(dest, AbstractValue::Reference(frozen_id))
    }

    fn borrow_local(&mut self, dest: TempIndex, src: TempIndex) {
        let is_mut = self.ty(dest).is_mutable_reference();
        // nothing to check in case borrow is mutable since the frame cannot have an full borrow/
        // epsilon outgoing edge
        if !is_mut && self.state.is_local_mutably_borrowed(src) {
            self.error_with_hints(
                self.cur_loc(),
                format!(
                    "cannot immutably borrow {} which is already mutably borrowed",
                    self.display(dest)
                ),
                "borrow attempted here",
                self.borrow_info_for_local(src).into_iter(),
            )
        }
        let new_id = self.state.new_ref(is_mut);
        self.state.add_local_borrow(self.attr_id, src, new_id);
        self.replace(dest, AbstractValue::Reference(new_id))
    }

    fn borrow_field(
        &mut self,
        dest: TempIndex,
        src: TempIndex,
        struct_id: QualifiedInstId<StructId>,
        offset: usize,
    ) {
        let is_mut = self.ty(dest).is_mutable_reference();
        let id = self.move_or_copy_ref(src, &[], &[dest]);
        let field_label = Label::Field(struct_id.to_qualified_id(), offset);
        if is_mut && self.state.borrow_graph.has_full_borrows(id) {
            self.error_with_hints(
                self.cur_loc(),
                format!(
                    "cannot mutably borrow {} of {} which is already borrowed",
                    field_label.display(self.target()),
                    self.display(dest)
                ),
                "borrow attempted here",
                self.borrow_info_for_ref(id, None).into_iter(),
            )
        } else if !is_mut && !self.state.borrow_graph.is_readable(id, Some(field_label)) {
            self.error_with_hints(
                self.cur_loc(),
                format!(
                    "cannot borrow {} of {} which is already mutably borrowed",
                    field_label.display(self.target()),
                    self.display(dest)
                ),
                "borrow attempted here",
                self.borrow_info_for_ref(id, Some(field_label)).into_iter(),
            )
        }
        let new_id = self.state.new_ref(is_mut);
        self.state.add_field_borrow(
            self.attr_id,
            id,
            struct_id.to_qualified_id(),
            offset,
            new_id,
        );
        self.state.release(id);
        self.replace(dest, AbstractValue::Reference(new_id))
    }

    /// Process a borrow global instruction.
    fn borrow_global(
        &mut self,
        struct_id: QualifiedInstId<StructId>,
        dest: TempIndex,
        src: TempIndex,
    ) {
        // Traditional Move semantics does not distinguish type instantiations, but this could
        // be easily generalized.
        let struct_id = struct_id.to_qualified_id();
        let is_mut = self.ty(dest).is_mutable_reference();
        if is_mut && self.state.is_global_borrowed(struct_id) {
            self.error_with_hints(
                self.cur_loc(),
                format!(
                    "cannot mutably borrow `{}` since it is already borrowed",
                    self.global_env().get_struct(struct_id).get_full_name_str()
                ),
                "mutable borrow attempted here",
                self.borrow_info_for_global(struct_id).into_iter(),
            )
        } else if self.state.is_global_mutably_borrowed(struct_id) {
            self.error_with_hints(
                self.cur_loc(),
                format!(
                    "cannot borrow `{}` since it is already mutably borrowed",
                    self.global_env().get_struct(struct_id).get_full_name_str()
                ),
                "borrow attempted here",
                self.borrow_info_for_global(struct_id).into_iter(),
            )
        }
        let _address_value = self.move_or_copy(src, &[], &[dest]);
        let new_id = self.state.new_ref(is_mut);
        self.state
            .add_resource_borrow(self.attr_id, struct_id, new_id);
        self.replace(dest, AbstractValue::Reference(new_id))
    }

    fn call_operation(&mut self, oper: Operation, dests: &[TempIndex], srcs: &[TempIndex]) {
        // If this is a Move function call, check access of resources.
        if let Operation::Function(mid, fid, inst) = oper {
            self.check_global_access(mid.qualified_inst(fid, inst))
        }
        // Transfer arguments, remembering what we have been borrowed from
        let mut all_references_to_borrow_from = BTreeSet::new();
        let mut mutable_references_to_borrow_from = BTreeSet::new();
        for (pos, src) in srcs.iter().enumerate() {
            let value = self.move_or_copy(*src, &srcs[pos + 1..], dests);
            if let Some(id) = value.ref_id() {
                if self.state.borrow_graph.is_mutable(id) {
                    self.check_transfer(*src, id);
                    mutable_references_to_borrow_from.insert(id);
                }
                all_references_to_borrow_from.insert(id);
            }
        }

        // Create references for return values
        for dest in dests {
            let ty = self.ty(*dest);
            if ty.is_mutable_reference() {
                let id = self.state.new_ref(true);
                for parent in &mutable_references_to_borrow_from {
                    self.state.add_borrow(self.attr_id, *parent, id);
                }
                self.replace(*dest, AbstractValue::Reference(id))
            } else if ty.is_reference() {
                let id = self.state.new_ref(false);
                for parent in &all_references_to_borrow_from {
                    self.state.add_borrow(self.attr_id, *parent, id);
                }
                self.replace(*dest, AbstractValue::Reference(id))
            }
        }

        // Release all input references
        for id in all_references_to_borrow_from {
            self.state.release(id)
        }
    }

    fn check_transfer(&mut self, src: TempIndex, id: RefID) {
        if !self.state.borrow_graph.is_writable(id) {
            self.error_with_hints(
                self.cur_loc(),
                format!(
                    "cannot transfer mutable {} since it is borrowed",
                    self.display(src)
                ),
                "transfer attempted here",
                self.borrow_info_for_ref(id, None).into_iter(),
            )
        }
    }

    fn move_from(&mut self, dest: TempIndex, resource: &QualifiedInstId<StructId>, src: TempIndex) {
        let struct_id = resource.to_qualified_id();
        let _address = self.move_or_copy(src, &[], &[dest]);
        if self.state.is_global_borrowed(struct_id) {
            self.error_with_hints(
                self.cur_loc(),
                format!(
                    "cannot extract `{}` which is still borrowed",
                    self.global_env().display(&struct_id)
                ),
                "extraction attempted here",
                self.borrow_info_for_global(struct_id).into_iter(),
            )
        }
        self.replace(dest, AbstractValue::NonReference)
    }

    fn return_(&mut self, _instr: &Bytecode, srcs: &[TempIndex]) {
        // Move all source values
        let mut refs = vec![];
        for src in srcs {
            let value = self.move_(*src);
            if let Some(id) = value.ref_id() {
                if self.state.borrow_graph.is_mutable(id) {
                    self.check_transfer(*src, id);
                }
                refs.push(id)
            }
        }

        // Release all locals still active
        for local in self.state.locals_range() {
            self.state.release_local(local)
        }

        // Check that no local or global is still borrowed
        for (_, path, _, _) in self.state.borrow_graph.out_edges(self.state.frame_root()) {
            self.error_with_hints(
                self.cur_loc(),
                format!(
                    "cannot return a reference derived from {} since it is not based on a parameter",
                    self.display_path(&path)
                ),
                "return attempted here",
                self.borrow_info_for_ref(self.state.frame_root(), path.first().cloned()).into_iter()
            )
        }

        // Release the returned references
        refs.into_iter().for_each(|id| self.state.release(id))
    }

    fn read_ref(&mut self, dest: TempIndex, src: TempIndex) {
        let id = self.move_or_copy_ref(src, &[], &[dest]);
        if !self.state.borrow_graph.is_readable(id, None) {
            self.error_with_hints(
                self.cur_loc(),
                format!(
                    "cannot read {} since it is mutably borrowed",
                    self.display(src)
                ),
                "read attempted here",
                self.borrow_info_for_ref(id, None).into_iter(),
            )
        }
        self.state.release(id);
        self.replace(dest, AbstractValue::NonReference)
    }

    fn write_ref(&mut self, dest: TempIndex, src: TempIndex) {
        let _value = self.move_or_copy(src, &[], &[]);
        let id = self.move_or_copy_ref(dest, &[], &[]);
        if !self.state.borrow_graph.is_writable(id) {
            self.error_with_hints(
                self.cur_loc(),
                format!("cannot write {} since it is borrowed", self.display(dest)),
                "write attempted here",
                self.borrow_info_for_ref(id, None).into_iter(),
            )
        }
        self.state.release(id)
    }

    fn branch(&mut self, cond: TempIndex) {
        let _value = self.move_or_copy(cond, &[], &[]);
    }

    fn comparison(&mut self, dest: TempIndex, srcs: &[TempIndex]) {
        // Difference in comparison to regular call is that we allow mutable references
        // without write check.
        let mut refs = vec![];
        for (pos, src) in srcs.iter().enumerate() {
            if let Some(id) = self.move_or_copy(*src, &srcs[pos + 1..], &[dest]).ref_id() {
                refs.push(id)
            }
        }
        refs.into_iter().for_each(|id| self.state.release(id));
        self.replace(dest, AbstractValue::NonReference)
    }
}

// -------------------------------------------------------------------------------------------------
// Transfer Function

impl TransferFunctions for LifeTimeAnalysis<'_> {
    type State = LifetimeState;

    const BACKWARD: bool = false;

    /// Transfer function for given bytecode.
    fn execute(&self, state: &mut Self::State, instr: &Bytecode, code_offset: CodeOffset) {
        use Bytecode::*;

        // Construct step context
        let mut step = self.new_step(code_offset, instr.get_attr_id(), state);

        // Process the instruction
        match instr {
            Assign(_, dest, src, kind) => {
                step.assign(*dest, *src, *kind);
            },
            Ret(_, srcs) => step.return_(instr, srcs),
            Branch(_, _, _, src) => step.branch(*src),
            Call(_, dests, oper, srcs, _) => {
                use Operation::*;
                match oper {
                    BorrowLoc => {
                        step.borrow_local(dests[0], srcs[0]);
                    },
                    BorrowGlobal(mid, sid, inst) => {
                        step.borrow_global(
                            mid.qualified_inst(*sid, inst.clone()),
                            dests[0],
                            srcs[0],
                        );
                    },
                    BorrowField(mid, sid, inst, field_offs) => {
                        let (dest, src) = (dests[0], srcs[0]);
                        let qid = mid.qualified_inst(*sid, inst.clone());
                        step.borrow_field(dest, src, qid, *field_offs);
                    },
                    BorrowVariantField(mid, sid, _variants, inst, field_offs) => {
                        let (dest, src) = (dests[0], srcs[0]);
                        let qid = mid.qualified_inst(*sid, inst.clone());
                        step.borrow_field(dest, src, qid, *field_offs);
                    },
                    ReadRef => step.read_ref(dests[0], srcs[0]),
                    WriteRef => step.write_ref(srcs[0], srcs[1]),
                    FreezeRef(explicit) => step.freeze_ref(*explicit, dests[0], srcs[0]),
                    MoveFrom(mid, sid, inst) => {
                        step.move_from(dests[0], &mid.qualified_inst(*sid, inst.clone()), srcs[0])
                    },
                    Eq | Neq | Le | Lt | Ge | Gt => step.comparison(dests[0], srcs),
                    _ => step.call_operation(oper.clone(), dests, srcs),
                }
            },
            _ => {},
        }

        // Some instructions may not have released inputs, do so now if they aren't used
        // longer. The release operation is idempotent.
        for local in step.state.locals_range() {
            if !step.alive.after.contains_key(&local) {
                step.state.release_local(local);
            }
        }
        state.canonicalize()
    }
}

/// Instantiate the data flow analysis framework based on the transfer function
impl DataflowAnalysis for LifeTimeAnalysis<'_> {}

// ===============================================================================
// Processor

pub struct ReferenceSafetyProcessor {}

impl FunctionTargetProcessor for ReferenceSafetyProcessor {
    fn process(
        &self,
        _targets: &mut FunctionTargetsHolder,
        fun_env: &FunctionEnv,
        mut data: FunctionData,
        _scc_opt: Option<&[FunctionEnv]>,
    ) -> FunctionData {
        if fun_env.is_native() {
            return data;
        }
        let target = FunctionTarget::new(fun_env, &data);
        let live_var_annotation = target
            .get_annotations()
            .get::<LiveVarAnnotation>()
            .expect("livevar annotation");
        let analyzer = LifeTimeAnalysis {
            target: &target,
            live_var_annotation,
        };
        let code = target.get_bytecode();
        let cfg = StacklessControlFlowGraph::new_forward(code);
        let state = LifetimeState::new(&target);
        let state_map = analyzer.analyze_function(state, target.get_bytecode(), &cfg);
        let state_map_per_instr = analyzer.state_per_instruction_with_default(
            state_map,
            target.get_bytecode(),
            &cfg,
            |before, after| {
                LifetimeInfoAtCodeOffset::new(Rc::new(before.clone()), Rc::new(after.clone()))
            },
        );
        let annotation = LifetimeAnnotation(state_map_per_instr);
        data.annotations.set(annotation, true);
        data
    }

    fn name(&self) -> String {
        "ReferenceSafetyProcessor".to_owned()
    }
}

impl LifetimeInfo for LifetimeState {
    fn borrow_kind(&self, temp: TempIndex) -> Option<ReferenceKind> {
        if self.is_local_mutably_borrowed(temp) {
            Some(ReferenceKind::Mutable)
        } else if self.is_local_borrowed(temp) {
            Some(ReferenceKind::Immutable)
        } else {
            None
        }
    }

    fn display(&self, target: &FunctionTarget) -> Option<String> {
        Some(self.display(target).to_string())
    }
}

// ===============================================================================
// Display

struct LabelDisplay<'a>(&'a FunctionTarget<'a>, &'a Label, /*raw*/ bool);

impl Label {
    fn display<'a>(&'a self, fun: &'a FunctionTarget) -> LabelDisplay<'a> {
        LabelDisplay(fun, self, false)
    }

    fn display_raw<'a>(&'a self, fun: &'a FunctionTarget) -> LabelDisplay<'a> {
        LabelDisplay(fun, self, false) // TODO: turn on in different commit
    }
}

impl Display for LabelDisplay<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self.1 {
            Label::Local(local) if /*raw*/self.2 => write!(f, "$t{}", local),
            Label::Local(local) => write!(f, "{}", self.0.get_local_name_for_error_message(*local)),
            Label::Global(struct_id) => {
                let struct_env = self.0.global_env().get_struct(*struct_id);
                write!(
                    f,
                    "{} `{}`",
                    if struct_env.has_variants() {
                        "enum"
                    } else {
                        "struct"
                    },
                    self.0
                        .global_env()
                        .get_struct(*struct_id)
                        .get_full_name_str()
                )
            },
            Label::Field(struct_id, offset) => {
                let struct_env = self.0.global_env().get_struct(*struct_id);
                let name = if struct_env.has_variants() {
                    // Find the name of any field with this offset
                    struct_env
                        .get_fields()
                        .find(|f| f.get_offset() == *offset)
                        .unwrap()
                        .get_name()
                } else {
                    struct_env.get_field_by_offset(*offset).get_name()
                };
                write!(f, "field `{}`", name.display(self.0.symbol_pool()))
            },
        }
    }
}

struct LifetimeStateDisplay<'a>(&'a FunctionTarget<'a>, &'a LifetimeState);

impl LifetimeState {
    fn display<'a>(&'a self, fun: &'a FunctionTarget) -> LifetimeStateDisplay<'a> {
        LifetimeStateDisplay(fun, self)
    }
}

impl Display for LifetimeStateDisplay<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let fmt_ref_id = |id: RefID| {
            if self.1.frame_root() == id {
                "#root".to_owned()
            } else {
                format!("#{}", id.number())
            }
        };
        writeln!(
            f,
            "refs: [{}]",
            self.1
                .locals
                .iter()
                .enumerate()
                .filter_map(|(idx, val)| val.ref_id().map(|id| format!(
                    "$t{} => {}",
                    idx,
                    fmt_ref_id(id)
                )))
                .join(", ")
        )?;
        let all_refs = self.1.borrow_graph.all_refs();
        if all_refs.len() == 1 && all_refs.first().unwrap() == &self.1.frame_root() {
            // If there is only the frame root in the graph, it is trivial, skip
            // printing this.
            return Ok(());
        }
        for id in all_refs {
            writeln!(f, "{}", fmt_ref_id(id))?;
            let out_edges = self.1.borrow_graph.out_edges(id);
            if out_edges.is_empty() {
                writeln!(f, "  <no edges>")?
            } else {
                for (code_id, path, is_strong, target) in self.1.borrow_graph.out_edges(id) {
                    let mut_str = if self.1.borrow_graph.is_mutable(target) {
                        " (mut)"
                    } else {
                        ""
                    };
                    writeln!(
                        f,
                        "  {}>{} {} via [{}] {}",
                        if is_strong { "=" } else { "-" },
                        mut_str,
                        fmt_ref_id(target),
                        path.iter().map(|l| l.display_raw(self.0)).join(", "),
                        self.0
                            .get_bytecode_loc(code_id)
                            .display_line_only(self.0.global_env()),
                    )?
                }
            }
        }
        Ok(())
    }
}
