// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Implements memory safety analysis.
//!
//! This is an intra functional, forward-directed data flow analysis over the domain
//! of what we call a *borrow graph*. The borrow graph tracks the creation of references from
//! root memory locations and derivation of other references, by recording an edge for each
//! borrow relation. For example, if `s` is the memory location of a struct, then
//! `&s.f` is represented by a node which is derived from `s`, and those two nodes are
//! connected by an edge labeled with `f`. The borrow graph is a DAG (acyclic).
//!
//! Together with the borrow graph, a mapping from temporaries to graph nodes is maintained at
//! each program point. These represent the currently active references into the borrowed data.
//! All the _parents_ of those nodes are indirectly borrowed as well. Consider again `s` a
//! struct, and assume ` let $t = &s.f` a field selection, then `$t` points to the node
//! representing the reference to the field. However, the original `s` from which the field
//! was selected, is also (indirectly) borrowed, as described by the borrow graph. When
//! any of the temporaries pointing into the borrow graph go out of scope, we perform a
//! clean up step and also release any parents not longer needed.
//!
//! The safety analysis essentially evaluates each instruction under the viewpoint of the current
//! active borrow graph at the program point, to detect any conditions of non-safety. This
//! includes specifically the following rules:
//!
//! (a) If immutable references to a location exist at a given program point, no mutable
//!     references can co-exist.
//! (b) Only one mutable reference can exist at time for the same location.
//! (c) A location which is mutable or immutably borrowed cannot be updated, and the value in it
//!     cannot be moved out. Here with location we mean a temporary value on the stack or a global
//!     resource.
//! (d) References returned from a function call must be derived from parameters
//!
//! Hereby, selection of fields leads to independent 'sub-locations': the above rules are
//! applying independently for such sub-locations. Thus one can have `&s.f` and `&mut s.g` at the
//! same time.

use crate::{
    pipeline::livevar_analysis_processor::{LiveVarAnnotation, LiveVarInfoAtCodeOffset},
    Experiment, Options,
};
use codespan_reporting::diagnostic::Severity;
use im::ordmap::Entry;
use itertools::Itertools;
use move_binary_format::file_format::CodeOffset;
use move_model::{
    ast::TempIndex,
    model::{FieldId, FunctionEnv, Loc, QualifiedInstId, StructId},
    ty::Type,
};
use move_stackless_bytecode::{
    dataflow_analysis::{DataflowAnalysis, TransferFunctions},
    dataflow_domains::{AbstractDomain, JoinResult, MapDomain, SetDomain},
    function_target::{FunctionData, FunctionTarget},
    function_target_pipeline::{FunctionTargetProcessor, FunctionTargetsHolder},
    stackless_bytecode::{AssignKind, AttrId, Bytecode, Operation},
    stackless_control_flow_graph::StacklessControlFlowGraph,
};
use std::{
    collections::{btree_map, BTreeMap, BTreeSet},
    fmt::{Display, Formatter},
};

// ===============================================================================
// Memory Safety Analysis

// -------------------------------------------------------------------------------------------------
// Program Analysis Domain

/// The borrow graph consists of a set of `LifetimeNode` values which are labeled by
/// `LifetimeLabel`. Each node carries information about an associated
/// `MemoryLocation`, as well as the children of the node, given by list of `BorrowEdge`s.
/// The node also has backlinks to its parents, given by a set of `LifetimeLabel`, for
/// more flexible navigation through the (acyclic) graph.
#[derive(Clone, Debug, PartialEq, Eq)]
struct LifetimeNode {
    /// Memory location associated with this node.
    location: MemoryLocation,
    /// Outgoing edges to children.
    children: SetDomain<BorrowEdge>,
    /// Backlinks to parents.
    parents: SetDomain<LifetimeLabel>,
}

/// A label for a lifetime node.
#[derive(PartialOrd, Ord, PartialEq, Eq, Clone, Debug)]
enum LifetimeLabel {
    Basic(u64),
    Derived(Box<LifetimeLabel>, Box<LifetimeLabel>),
}

/// A memory location, either a global in storage or a local on the stack.
#[derive(PartialOrd, Ord, PartialEq, Eq, Clone, Debug)]
enum MemoryLocation {
    /// The associated memory is in global storage.
    Global(QualifiedInstId<StructId>),
    /// The associated memory is a local on the stack.
    Local(TempIndex),
    /// The associated memory has been reused, but the old value is still bound here to this node.
    /// This happens after an update to the location.
    Replaced,
}

/// Represents an edge in the borrow graph.
#[derive(PartialOrd, Ord, PartialEq, Eq, Clone, Debug)]
struct BorrowEdge {
    /// The kind of borrow edge.
    kind: BorrowEdgeKind,
    /// Whether this edge is a mutable borrow
    is_mut: bool,
    /// A location associated with the borrow edge. For Skip edges (see below), no location may be
    /// present.
    loc: Option<Loc>,
    /// Target of the edge.
    target: LifetimeLabel,
}

/// The different type of edges.
#[derive(PartialOrd, Ord, PartialEq, Eq, Clone, Debug)]
enum BorrowEdgeKind {
    /// Borrows the local at the MemoryLocation in the source node.
    BorrowLocal,
    /// Borrows the global at the MemoryLocation in the source node.
    BorrowGlobal,
    /// Borrows a field from a reference.
    BorrowField(FieldId),
    /// Calls an operation, where the incoming references are used to derive outgoing references.
    Call(Operation),
    /// The `Skip` edge is used for graph composition and glues two graph nodes together via
    /// connecting them. For more details see the implementation of the `LifetimeDomain`
    /// join operator.
    Skip,
}

impl LifetimeLabel {
    /// Creates a new, unique and stable, life time label based on a code offset and
    /// a qualifier to distinguish multiple labels at the same code point.
    /// Since the program analysis could run fixpoint loops, we need to ensure that
    /// these labels are the same in each iteration.
    fn new(code_offset: CodeOffset, qualifier: u8) -> LifetimeLabel {
        LifetimeLabel::Basic(((code_offset as u64) << 8) | (qualifier as u64))
    }

    /// Creates a new, unique and stable, lifetime label from two other labels.
    /// This exploits the fact that code offsets are 16 bits, so we can shift
    /// the one lifetime label and bitor it with the other.
    fn derive(label1: LifetimeLabel, label2: LifetimeLabel) -> LifetimeLabel {
        LifetimeLabel::Derived(Box::new(label1), Box::new(label2))
    }
}

impl BorrowEdge {
    /// Shortcut to create an edge.
    fn new(kind: BorrowEdgeKind, is_mut: bool, loc: Option<Loc>, target: LifetimeLabel) -> Self {
        Self {
            kind,
            is_mut,
            loc,
            target,
        }
    }
}

impl BorrowEdgeKind {
    /// Determines whether the region derived from this edge has overlap with the region
    /// of the other edge. Overlap can only be excluded for field edges.
    fn overlaps(&self, other: &BorrowEdgeKind) -> bool {
        use BorrowEdgeKind::*;
        match (self, other) {
            (BorrowField(field1), BorrowField(field2)) => field1 == field2,
            _ => true,
        }
    }
}

/// The program analysis domain used with the data flow analysis framework.
///
/// This structure and its components need to implement `AbstractDomain` so they
/// can be used by the data flow analysis framework. This trait defines a `join`
/// which is used to merge information from multiple incoming paths during data
/// flow analysis.
#[derive(Clone, Default, Debug)]
pub struct LifetimeState {
    /// Contains the borrow graph at the current program point.
    graph: MapDomain<LifetimeLabel, LifetimeNode>,
    /// A map from locals to labels. Represents root states of the active graph.
    local_to_label_map: BTreeMap<TempIndex, LifetimeLabel>,
    /// A map from globals to labels. Represents root states of the active graph.
    global_to_label_map: BTreeMap<QualifiedInstId<StructId>, LifetimeLabel>,
    /// Contains the set of variables whose values have been moved to somewhere else.
    moved: SetDomain<TempIndex>,
}

impl AbstractDomain for LifetimeNode {
    fn join(&mut self, other: &Self) -> JoinResult {
        self.children
            .join(&other.children)
            .combine(self.parents.join(&other.parents))
    }
}

impl AbstractDomain for LifetimeState {
    /// The join operator of the dataflow analysis domain. This calls into `join_label_map`
    /// which does the work of graph gluing.
    fn join(&mut self, other: &Self) -> JoinResult {
        let mut change = self.graph.join(&other.graph);

        let mut new_local_to_label_map = std::mem::take(&mut self.local_to_label_map);
        change = change.combine(self.join_label_map(
            &mut new_local_to_label_map,
            &other.local_to_label_map,
            |temp| MemoryLocation::Local(*temp),
        ));
        let mut new_global_to_label_map = std::mem::take(&mut self.global_to_label_map);
        change = change.combine(self.join_label_map(
            &mut new_global_to_label_map,
            &other.global_to_label_map,
            |id| MemoryLocation::Global(id.clone()),
        ));
        self.local_to_label_map = new_local_to_label_map;
        self.global_to_label_map = new_global_to_label_map;

        change = change.combine(self.moved.join(&other.moved));
        change
    }
}

impl LifetimeState {
    /// Joins two maps with labels in their range. For overlapping keys, a new `Skip`
    /// node is created and configured as a child of both label nodes, gluing the
    /// graphs together.
    fn join_label_map<A: Clone + Ord>(
        &mut self,
        map: &mut BTreeMap<A, LifetimeLabel>,
        other_map: &BTreeMap<A, LifetimeLabel>,
        mk_location: impl Fn(&A) -> MemoryLocation,
    ) -> JoinResult {
        let mut change = JoinResult::Unchanged;
        for (k, other_label) in other_map {
            match map.entry(k.clone()) {
                btree_map::Entry::Vacant(entry) => {
                    entry.insert(other_label.clone());
                    change = JoinResult::Changed;
                },
                btree_map::Entry::Occupied(mut entry) => {
                    let label = entry.get().clone();
                    if &label != other_label && self.node(&label) != self.node(other_label) {
                        // Create a new intermediate node and make it child of the other ones.
                        let new_label = LifetimeLabel::derive(label.clone(), other_label.clone());
                        self.new_node(new_label.clone(), mk_location(entry.key()));
                        entry.insert(new_label.clone());
                        // Determine mutability from the other nodes and add a `Skip` edge
                        let is_mut = self
                            .node(&label)
                            .children
                            .iter()
                            .chain(self.node(other_label).children.iter())
                            .any(|e| e.is_mut);
                        let skip_edge =
                            BorrowEdge::new(BorrowEdgeKind::Skip, is_mut, None, new_label.clone());
                        self.add_edge(&label, skip_edge.clone());
                        self.add_edge(other_label, skip_edge);
                        change = JoinResult::Changed;
                    }
                },
            }
        }
        change
    }
}

impl LifetimeState {
    /// Creates a new node with the given label and location information.
    fn new_node(&mut self, assigned_label: LifetimeLabel, location: MemoryLocation) {
        self.graph.insert(assigned_label, LifetimeNode {
            location,
            children: Default::default(),
            parents: Default::default(),
        });
    }

    /// Returns reference to node.
    fn node(&self, label: &LifetimeLabel) -> &LifetimeNode {
        &self.graph[label]
    }

    /// Returns mutable reference to node.
    fn node_mut(&mut self, label: &LifetimeLabel) -> &mut LifetimeNode {
        &mut self.graph[label]
    }

    /// Returns an iteration of child edges of given node.
    fn children(&self, label: &LifetimeLabel) -> impl Iterator<Item = &BorrowEdge> {
        self.node(label).children.iter()
    }

    /// Returns true if given node has no children
    fn is_leaf(&self, label: &LifetimeLabel) -> bool {
        self.node(label).children.is_empty()
    }

    /// Iterates the effective child edges of a node. For incoming or outgoing Skip edges,
    /// parent and target children are also included. This reflects that Skip edges represent
    /// aliases, like when one local is assigned to another.
    fn effective_children(&self, label: &LifetimeLabel) -> Vec<(LifetimeLabel, &BorrowEdge)> {
        // The walk upwards and downwards the graph can run into cycles, henceforth a
        // `visited` set is needed.
        self.effective_children_walk(label, &mut BTreeSet::new())
    }

    /// Implementation of `effective_children` with visited set.
    fn effective_children_walk(
        &self,
        label: &LifetimeLabel,
        visited: &mut BTreeSet<LifetimeLabel>,
    ) -> Vec<(LifetimeLabel, &BorrowEdge)> {
        if !visited.insert(label.clone()) {
            return vec![];
        }
        let mut result = vec![];
        for e in self.children(label) {
            if e.kind == BorrowEdgeKind::Skip {
                result.append(&mut self.effective_children_walk(&e.target, visited));
            } else {
                result.push((label.clone(), e))
            }
        }
        for (parent, e) in self.parent_edges(label) {
            if e.kind == BorrowEdgeKind::Skip {
                result.append(&mut self.effective_children_walk(&parent, visited))
            } // else: the parent edge is not a child
        }
        result
    }

    /// Returns true if there are no effective children.
    fn is_effective_leaf(&self, label: &LifetimeLabel) -> bool {
        self.effective_children(label).is_empty()
    }

    /// Gets the label associated with a local, if it has effective children.
    fn label_for_local_with_children(&self, temp: TempIndex) -> Option<&LifetimeLabel> {
        self.label_for_local(temp)
            .filter(|l| !self.is_effective_leaf(l))
    }

    /// Gets the label associated with a global, if it has effective children.
    fn label_for_global_with_children(
        &self,
        resource: &QualifiedInstId<StructId>,
    ) -> Option<LifetimeLabel> {
        self.label_for_global(resource)
            .filter(|l| !self.is_effective_leaf(l))
    }

    /// Returns true if the node has outgoing (effective) mut edges.
    fn has_mut_edges(&self, label: &LifetimeLabel) -> bool {
        self.effective_children(label).iter().any(|(_, e)| e.is_mut)
    }

    /// Gets the label associated with a local.
    fn label_for_local(&self, temp: TempIndex) -> Option<&LifetimeLabel> {
        self.local_to_label_map.get(&temp)
    }

    /// If label for local exists, return it, otherwise create a new node.
    fn make_local(
        &mut self,
        temp: TempIndex,
        code_offset: CodeOffset,
        qualifier: u8,
    ) -> LifetimeLabel {
        if let Some(label) = self.local_to_label_map.get(&temp) {
            label.clone()
        } else {
            let label = LifetimeLabel::new(code_offset, qualifier);
            self.new_node(label.clone(), MemoryLocation::Local(temp));
            self.local_to_label_map.insert(temp, label.clone());
            label
        }
    }

    /// Gets the label associated with a global.
    #[allow(unused)]
    fn label_for_global(&self, global: &QualifiedInstId<StructId>) -> Option<LifetimeLabel> {
        self.global_to_label_map.get(global).cloned()
    }

    /// If label for global exists, return it, otherwise create a new one.
    fn make_global(
        &mut self,
        struct_id: QualifiedInstId<StructId>,
        code_offset: CodeOffset,
        qualifier: u8,
    ) -> LifetimeLabel {
        if let Some(label) = self.global_to_label_map.get(&struct_id) {
            label.clone()
        } else {
            let label = LifetimeLabel::new(code_offset, qualifier);
            self.new_node(label.clone(), MemoryLocation::Global(struct_id.clone()));
            self.global_to_label_map.insert(struct_id, label.clone());
            label
        }
    }

    /// Adds an edge to the graph.
    fn add_edge(&mut self, label: &LifetimeLabel, edge: BorrowEdge) {
        let child = edge.target.clone();
        self.node_mut(label).children.insert(edge);
        self.node_mut(&child).parents.insert(label.clone());
    }

    /// Drops a node. The parents are recursively dropped if their children go down to
    /// zero. Collects the locations of the removed nodes.
    fn drop_node(
        &mut self,
        label: &LifetimeLabel,
        alive: &BTreeSet<TempIndex>,
        removed: &mut BTreeSet<MemoryLocation>,
    ) {
        match self.graph.entry(label.clone()) {
            Entry::Occupied(entry) => {
                let current: LifetimeNode = entry.remove();
                debug_assert!(current.children.is_empty());
                removed.insert(current.location);
                for parent in current.parents.iter() {
                    let node = self.node_mut(parent);
                    // Remove the dropped node from the children list.
                    let children = std::mem::take(&mut node.children);
                    node.children = children
                        .into_iter()
                        .filter(|e| &e.target != label)
                        .collect();
                    // Decide whether the parent node should be dropped as well
                    if let MemoryLocation::Local(temp) = &node.location {
                        if alive.contains(temp) {
                            // Do not drop this node, since it is referenced from a temp
                            continue;
                        }
                    }
                    if node.children.is_empty() {
                        self.drop_node(parent, alive, removed)
                    }
                }
            },
            Entry::Vacant(_) => {
                panic!("inconsistent borrow graph")
            },
        }
    }

    /// Releases graph resources related to the local, for example, since the local
    /// is overwritten or not longer used.
    fn release_local(&mut self, temp: TempIndex, alive: &BTreeSet<TempIndex>) {
        if let Some(label) = self.label_for_local(temp).cloned() {
            if self.is_leaf(&label) {
                let mut removed = BTreeSet::new();
                self.drop_node(&label, alive, &mut removed);
                for location in removed {
                    use MemoryLocation::*;
                    match location {
                        Local(temp) => {
                            self.local_to_label_map.remove(&temp);
                        },
                        Global(qid) => {
                            self.global_to_label_map.remove(&qid);
                        },
                        Replaced => {},
                    }
                }
            }
        }
    }

    /// Replaces a local, as result of an assignment. The current
    /// node associated with the local is released and then
    /// a new node for the local is created.
    fn replace_local(
        &mut self,
        temp: TempIndex,
        alive: &BTreeSet<TempIndex>,
        code_offset: CodeOffset,
        qualifier: u8,
    ) -> LifetimeLabel {
        self.release_local(temp, alive);
        if let Some(label) = self.label_for_local(temp).cloned() {
            // Set the location to be 'replaced'. That means while the node logically still
            // exists, it is not longer associated with a specific temporary.
            self.node_mut(&label).location = MemoryLocation::Replaced;
            self.local_to_label_map.remove(&temp);
        }
        self.make_local(temp, code_offset, qualifier)
    }

    /// Returns an iterator of the edges which are leading into this node.
    fn parent_edges<'a>(
        &'a self,
        label: &'a LifetimeLabel,
    ) -> impl Iterator<Item = (LifetimeLabel, &'a BorrowEdge)> + '_ {
        self.node(label).parents.iter().flat_map(move |parent| {
            self.children(parent)
                .filter(move |edge| &edge.target == label)
                .map(|e| (parent.clone(), e))
        })
    }

    /// Returns the roots of this node, that is those nodes which have no parents.
    fn roots(&self, label: &LifetimeLabel) -> BTreeSet<LifetimeLabel> {
        let mut roots = BTreeSet::new();
        let mut todo = self.node(label).parents.iter().cloned().collect::<Vec<_>>();
        while let Some(l) = todo.pop() {
            let mut parents = self.node(&l).parents.iter().cloned().collect::<Vec<_>>();
            if parents.is_empty() {
                // Found a root
                roots.insert(l);
            } else {
                // Explore parents
                todo.append(&mut parents)
            }
        }
        roots
    }
}

impl LifetimeState {
    /// Returns the locals borrowed
    pub fn borrowed_locals(&self) -> impl Iterator<Item = TempIndex> + '_ {
        self.local_to_label_map.keys().cloned()
    }

    /// Checks if the given local is borrowed
    pub fn is_borrowed(&self, temp: TempIndex) -> bool {
        self.borrowed_locals().contains(&temp)
    }
}

// -------------------------------------------------------------------------------------------------
// Lifetime Analysis

/// Used to distinguish how a local is read
#[derive(Clone, Copy, PartialEq, Eq)]
enum ReadMode {
    /// The local is moved
    Move,
    /// The local is copied
    Copy,
    /// The local is transferred as an argument to another function
    Argument,
}

/// A structure providing context information for operations during lifetime analysis.
/// This encapsulates the function target which is analyzed, giving also access to
/// the global model. Live var annotations are attached which are evaluated during
/// analysis.
struct LifeTimeAnalysis<'env> {
    target: &'env FunctionTarget<'env>,
    live_var_annotation: &'env LiveVarAnnotation,
    // If true, any errors generated by this analysis will be suppressed
    suppress_errors: bool,
}

impl<'env> LifeTimeAnalysis<'env> {
    // ---------------------------------------------------------------------------------------------
    // Diagnosis

    fn check_mut_edge(
        &self,
        state: &LifetimeState,
        label: &LifetimeLabel,
        edge: &BorrowEdge,
    ) -> Vec<(Loc, String)> {
        let mut diags = vec![];
        // There must be no overlapping child.
        for (_, e) in state.effective_children(label) {
            if e.kind.overlaps(&edge.kind) {
                if let Some(diag) = self.borrow_edge_info("previous ", state, false, e) {
                    diags.push(diag)
                } else {
                    debug_assert!(false, "unexpect Skip edge")
                }
            }
        }
        diags
    }

    fn check_immut_edge(
        &self,
        state: &LifetimeState,
        label: &LifetimeLabel,
        edge: &BorrowEdge,
    ) -> Vec<(Loc, String)> {
        let mut diags = vec![];
        // There must be no overlapping mutable child.
        for (_, e) in state.effective_children(label) {
            if e.is_mut && e.kind.overlaps(&edge.kind) {
                if let Some(diag) = self.borrow_edge_info("previous ", state, true, e) {
                    diags.push(diag)
                } else {
                    debug_assert!(false, "unexpected Skip edge")
                }
            }
        }
        diags
    }

    fn check_and_add_edge(
        &self,
        state: &mut LifetimeState,
        label: &LifetimeLabel,
        edge: BorrowEdge,
        _alive: &LiveVarInfoAtCodeOffset,
    ) {
        debug_assert_ne!(edge.kind, BorrowEdgeKind::Skip);
        let msg_for_source = || {
            match &state.node(label).location {
                MemoryLocation::Global(resource) => {
                    format!("global `{}`", self.target.global_env().display(resource))
                },
                MemoryLocation::Local(temp) => self.display(*temp),
                MemoryLocation::Replaced => {
                    // We do not create a new edge starting from a replace location.
                    panic!("unexpected location for new edge")
                },
            }
        };
        if edge.is_mut {
            let diags = self.check_mut_edge(state, label, &edge);
            if !diags.is_empty() {
                self.error_with_hints(
                    edge.loc.as_ref().expect("only Skip edge has no location"),
                    format!(
                        "cannot mutably borrow {} since other references exists",
                        msg_for_source()
                    ),
                    "mutable borrow attempted here",
                    diags.into_iter(),
                )
            }
        } else {
            let diags = self.check_immut_edge(state, label, &edge);
            if !diags.is_empty() {
                self.error_with_hints(
                    edge.loc.as_ref().expect("only Skip edge has no location"),
                    format!(
                        "cannot immutably borrow {} since other mutable references exist",
                        msg_for_source()
                    ),
                    "immutable borrow attempted here",
                    diags.into_iter(),
                )
            }
        }
        state.add_edge(label, edge)
    }

    fn error_with_hints(
        &self,
        loc: &Loc,
        msg: impl AsRef<str>,
        primary: impl AsRef<str>,
        hints: impl Iterator<Item = (Loc, String)>,
    ) {
        if !self.suppress_errors {
            self.target.global_env().diag_with_primary_and_labels(
                Severity::Error,
                loc,
                msg.as_ref(),
                primary.as_ref(),
                hints.collect(),
            )
        }
    }

    fn borrow_info(
        &self,
        state: &LifetimeState,
        label: &LifetimeLabel,
        only_mut: bool,
        alive: &LiveVarInfoAtCodeOffset,
    ) -> Vec<(Loc, String)> {
        let primary_edges = state
            .effective_children(label)
            .iter()
            .cloned()
            .collect::<BTreeSet<_>>();
        let mut secondary_edges = BTreeSet::new();
        for (_, edge) in primary_edges.iter() {
            if let MemoryLocation::Local(temp) = &state.graph[&edge.target].location {
                // Include secondary edge only if it is a local which has gone out of
                // scope. In this case, the user may wonder why they get the error
                // message, and showing derived references helps. Otherwise derived
                // references may be noisy.
                if !alive.before.contains_key(temp) {
                    secondary_edges.extend(state.effective_children(&edge.target))
                }
            }
        }
        primary_edges
            .into_iter()
            .filter_map(|(_, e)| self.borrow_edge_info("previous ", state, only_mut, e))
            .chain(
                secondary_edges
                    .into_iter()
                    .filter_map(|(_, e)| self.borrow_edge_info("used by ", state, only_mut, e)),
            )
            .collect()
    }

    fn borrow_edge_info(
        &self,
        prefix: &str,
        _state: &LifetimeState,
        only_mut: bool,
        e: &BorrowEdge,
    ) -> Option<(Loc, String)> {
        if e.is_mut || !only_mut {
            if let Some(loc) = &e.loc {
                use BorrowEdgeKind::*;
                let mut_prefix = if e.is_mut { "mutable " } else { "" };
                return Some((
                    loc.clone(),
                    format!("{}{}{}", prefix, mut_prefix, match &e.kind {
                        BorrowLocal => "local borrow",
                        BorrowGlobal => "global borrow",
                        BorrowField(..) => "field borrow",
                        Call(..) => "call result",
                        Skip => return None,
                    },),
                ));
            }
        }
        None
    }

    // ---------------------------------------------------------------------------------------------
    // Program Steps

    /// Process an assign instruction. This checks whether the source is currently borrowed and
    /// rejects a move if so. Constructs a `Skip` edge in the borrow graph if references
    /// are assigned.
    fn assign(
        &self,
        state: &mut LifetimeState,
        code_offset: CodeOffset,
        id: AttrId,
        dest: TempIndex,
        src: TempIndex,
        kind: AssignKind,
        alive: &LiveVarInfoAtCodeOffset,
    ) {
        // Check validness
        let mode = if kind == AssignKind::Move {
            ReadMode::Move
        } else {
            ReadMode::Copy
        };
        self.check_read_local(state, id, src, mode, alive);
        self.check_write_local(state, id, dest, alive);
        let ty = self.ty(src);
        if ty.is_reference() {
            // Track reference in the graph as a Skip edge.
            let loc = self.loc(id);
            let label = state.make_local(src, code_offset, 0);
            let child = state.replace_local(dest, &alive.after_set(), code_offset, 1);
            state.add_edge(
                &label,
                BorrowEdge::new(
                    BorrowEdgeKind::Skip,
                    ty.is_mutable_reference(),
                    Some(loc),
                    child,
                ),
            );
        }
        // Track whether the variable content is moved
        if kind == AssignKind::Move {
            state.moved.insert(src);
        }
        state.moved.remove(&dest);
    }

    /// Check validness of reading a local. The read is not allowed if the local is borrowed,
    /// depending on whether it is read or written. Returns true if valid.
    fn check_read_local(
        &self,
        state: &LifetimeState,
        id: AttrId,
        local: TempIndex,
        read_mode: ReadMode,
        alive: &LiveVarInfoAtCodeOffset,
    ) -> bool {
        if let Some(label) = state.label_for_local_with_children(local) {
            let ty = self.ty(local);
            let loc = self.loc(id);
            if ty.is_reference() {
                if ty.is_mutable_reference() {
                    match read_mode {
                        ReadMode::Move | ReadMode::Copy => {
                            let (op_str, verb_str) = if read_mode == ReadMode::Move {
                                ("move", "moved")
                            } else {
                                ("copy", "copied")
                            };
                            self.error_with_hints(
                                &loc,
                                format!(
                                    "cannot {} mutable reference in {} which is still borrowed",
                                    op_str,
                                    self.display(local)
                                ),
                                format!("{} here", verb_str),
                                self.borrow_info(state, label, false, alive).into_iter(),
                            );
                            false
                        },
                        ReadMode::Argument => {
                            self.error_with_hints(
                                &loc,
                                format!(
                                    "cannot pass mutable reference in {}, which is still borrowed, as function argument",
                                    self.display(local)
                                ),
                                "passed here",
                                self.borrow_info(state, label, false, alive).into_iter(),
                            );
                            false
                        },
                    }
                } else {
                    // immutable reference always ok
                    true
                }
            } else {
                match read_mode {
                    ReadMode::Copy => {
                        // Mutable borrow is not allowed
                        if state.has_mut_edges(label) {
                            self.error_with_hints(
                                &loc,
                                format!(
                                    "cannot copy {} which is still mutable borrowed",
                                    self.display(local)
                                ),
                                "copied here",
                                self.borrow_info(state, label, true, alive).into_iter(),
                            );
                            false
                        } else {
                            true
                        }
                    },
                    ReadMode::Move => {
                        // Any borrow not allowed
                        self.error_with_hints(
                            &loc,
                            format!(
                                "cannot move {} which is still borrowed",
                                self.display(local)
                            ),
                            "moved here",
                            self.borrow_info(state, label, false, alive).into_iter(),
                        );
                        false
                    },
                    ReadMode::Argument => {
                        // Mutable borrow not allowed
                        if state.has_mut_edges(label) {
                            self.error_with_hints(
                                &loc,
                                format!(
                                    "cannot pass {} which is still mutably \
                                    borrowed as function argument",
                                    self.display(local)
                                ),
                                "passed here",
                                self.borrow_info(state, label, false, alive).into_iter(),
                            );
                            false
                        } else {
                            true
                        }
                    },
                }
            }
        } else {
            true
        }
    }

    /// Check whether a local can be written. This is only allowed if no borrowed references exist.
    fn check_write_local(
        &self,
        state: &LifetimeState,
        id: AttrId,
        local: TempIndex,
        alive: &LiveVarInfoAtCodeOffset,
    ) {
        if let Some(label) = state.label_for_local_with_children(local) {
            let ty = self.ty(local);
            if !ty.is_reference() {
                // The destination is currently borrowed and cannot be assigned
                let loc = self.loc(id);
                self.error_with_hints(
                    &loc,
                    format!("cannot assign to borrowed {}", self.display(local)),
                    "attempted to assign here",
                    self.borrow_info(state, label, false, alive).into_iter(),
                )
            }
        } /* local not borrowed */
    }

    /// Process a borrow local instruction. This checks whether the borrow is allowed and
    /// constructs a borrow edge.
    fn borrow_local(
        &self,
        state: &mut LifetimeState,
        code_offset: CodeOffset,
        id: AttrId,
        dest: TempIndex,
        src: TempIndex,
        alive: &LiveVarInfoAtCodeOffset,
    ) {
        let label = state.make_local(src, code_offset, 0);
        let child = state.replace_local(dest, &alive.after_set(), code_offset, 1);
        let loc = self.loc(id);
        let is_mut = self.ty(dest).is_mutable_reference();
        self.check_and_add_edge(
            state,
            &label,
            BorrowEdge::new(BorrowEdgeKind::BorrowLocal, is_mut, Some(loc), child),
            alive,
        );
        state.moved.remove(&dest);
    }

    /// Process a borrow global instruction. This checks whether the borrow is allowed and
    /// constructs a borrow edge.
    fn borrow_global(
        &self,
        state: &mut LifetimeState,
        code_offset: CodeOffset,
        id: AttrId,
        struct_: QualifiedInstId<StructId>,
        dest: TempIndex,
        alive: &LiveVarInfoAtCodeOffset,
    ) {
        let label = state.make_global(struct_.clone(), code_offset, 0);
        let child = state.replace_local(dest, &alive.after_set(), code_offset, 1);
        let loc = self.loc(id);
        let is_mut = self.ty(dest).is_mutable_reference();
        self.check_and_add_edge(
            state,
            &label,
            BorrowEdge::new(BorrowEdgeKind::BorrowGlobal, is_mut, Some(loc), child),
            alive,
        );
        state.moved.remove(&dest);
    }

    /// Process a borrow field instruction. This checks whether the borrow is allowed and
    /// constructs a borrow edge.
    fn borrow_field(
        &self,
        state: &mut LifetimeState,
        code_offset: CodeOffset,
        id: AttrId,
        struct_: QualifiedInstId<StructId>,
        field_offs: &usize,
        dest: TempIndex,
        src: TempIndex,
        alive: &LiveVarInfoAtCodeOffset,
    ) {
        let label = state.make_local(src, code_offset, 0);
        let child = state.replace_local(dest, &alive.after_set(), code_offset, 1);
        let loc = self.loc(id);
        let is_mut = self.ty(dest).is_mutable_reference();
        let struct_env = self
            .target
            .global_env()
            .get_struct(struct_.to_qualified_id());
        let field_id = struct_env.get_field_by_offset(*field_offs).get_id();
        self.check_and_add_edge(
            state,
            &label,
            BorrowEdge::new(
                BorrowEdgeKind::BorrowField(field_id),
                is_mut,
                Some(loc),
                child,
            ),
            alive,
        );
        state.moved.remove(&dest);
    }

    /// Process a function call. For now we implement standard Move semantics, where every
    /// output reference is a child of all input references. Here would be the point where to
    //  evaluate lifetime modifiers in future language versions.
    fn call_operation(
        &self,
        state: &mut LifetimeState,
        code_offset: CodeOffset,
        id: AttrId,
        oper: Operation,
        dests: &[TempIndex],
        srcs: &[TempIndex],
        alive: &LiveVarInfoAtCodeOffset,
    ) {
        // Check validness of transferring arguments into function.
        let mut src_check_failed = BTreeSet::new();
        for src in srcs {
            if !self.check_read_local(state, id, *src, ReadMode::Argument, alive) {
                src_check_failed.insert(*src);
            }
        }
        // Next check whether we can assign to the destinations.
        for dest in dests {
            self.check_write_local(state, id, *dest, alive)
        }
        // Now draw edges from all reference sources to all reference destinations.
        let dest_labels = dests
            .iter()
            .filter(|d| self.ty(**d).is_reference())
            .enumerate()
            .map(|(i, t)| {
                (
                    *t,
                    state.replace_local(*t, &alive.after_set(), code_offset, i as u8),
                )
            })
            .collect::<BTreeMap<_, _>>();
        let src_qualifier_offset = dest_labels.len();
        let loc = self.loc(id);
        for dest in dests {
            let dest_ty = self.ty(*dest);
            if dest_ty.is_reference() {
                for (i, src) in srcs.iter().enumerate() {
                    // Only check the edge if src check succeeded, otherwise we get confusing
                    // double errors on the same location.
                    if src_check_failed.contains(src) {
                        continue;
                    }
                    let src_ty = self.ty(*src);
                    if src_ty.is_reference() {
                        let label =
                            state.make_local(*src, code_offset, (src_qualifier_offset + i) as u8);
                        let child = &dest_labels[dest];
                        self.check_and_add_edge(
                            state,
                            &label,
                            BorrowEdge::new(
                                BorrowEdgeKind::Call(oper.clone()),
                                dest_ty.is_mutable_reference(),
                                Some(loc.clone()),
                                child.clone(),
                            ),
                            alive,
                        )
                    }
                }
            }
        }
        // All sources are moved into a call
        state.moved.extend(srcs.iter().cloned());
        for dest in dests {
            state.moved.remove(dest);
        }
    }

    /// Process a MoveFrom instruction.
    fn move_from(
        &self,
        state: &mut LifetimeState,
        id: AttrId,
        dest: TempIndex,
        resource: &QualifiedInstId<StructId>,
        src: TempIndex,
        alive: &LiveVarInfoAtCodeOffset,
    ) {
        self.check_read_local(state, id, src, ReadMode::Argument, alive);
        self.check_write_local(state, id, dest, alive);
        if let Some(label) = state.label_for_global_with_children(resource) {
            self.error_with_hints(
                &self.loc(id),
                format!(
                    "cannot extract resource `{}` which is still borrowed",
                    self.target.global_env().display(resource)
                ),
                "extracted here",
                self.borrow_info(state, &label, false, alive).into_iter(),
            )
        }
        state.moved.insert(src);
        state.moved.remove(&dest);
    }

    /// Process a return instruction.
    fn return_(
        &self,
        state: &mut LifetimeState,
        id: AttrId,
        srcs: &[TempIndex],
        alive: &LiveVarInfoAtCodeOffset,
    ) {
        for src in srcs {
            if self.ty(*src).is_reference() {
                // Need to check whether this reference is derived from a local which is not a
                // a parameter
                if let Some(label) = state.label_for_local(*src) {
                    for root in state.roots(label) {
                        match &state.node(&root).location {
                            MemoryLocation::Global(resource) => self.error_with_hints(
                                &self.loc(id),
                                format!(
                                    "cannot return a reference derived from global `{}`",
                                    self.target.global_env().display(resource)
                                ),
                                "returned here",
                                self.borrow_info(state, &root, false, alive).into_iter(),
                            ),
                            MemoryLocation::Local(local) => {
                                if *local >= self.target.get_parameter_count() {
                                    self.error_with_hints(
                                        &self.loc(id),
                                        format!(
                                            "cannot return a reference derived from {} since it is not a parameter",
                                            self.display(*local)
                                        ),
                                        "returned here",
                                        self.borrow_info(state, &root, false, alive).into_iter(),
                                    )
                                }
                            },
                            MemoryLocation::Replaced => {},
                        }
                    }
                }
            }
        }
        state.moved.extend(srcs.iter().cloned())
    }

    /// Process a ReadRef instruction. In contrast to `self.check_read_local`, this needs
    /// to check the value behind the reference.
    fn read_ref(
        &self,
        state: &mut LifetimeState,
        id: AttrId,
        dest: TempIndex,
        src: TempIndex,
        alive: &LiveVarInfoAtCodeOffset,
    ) {
        self.check_write_local(state, id, dest, alive);
        if let Some(label) = state.label_for_local_with_children(src) {
            if state.has_mut_edges(label) {
                self.error_with_hints(
                    &self.loc(id),
                    format!(
                        "cannot dereference {} which is still mutable borrowed",
                        self.display(src)
                    ),
                    "dereferenced here",
                    self.borrow_info(state, label, true, alive).into_iter(),
                )
            }
        }
        state.moved.insert(src);
        state.moved.remove(&dest);
    }

    /// Process a WriteRef instruction. In contrast to `self.check_write_local`, this needs
    /// to check the value behind the reference.
    fn write_ref(
        &self,
        state: &mut LifetimeState,
        id: AttrId,
        dest: TempIndex,
        src: TempIndex,
        alive: &LiveVarInfoAtCodeOffset,
    ) {
        self.check_read_local(state, id, src, ReadMode::Argument, alive);
        if let Some(label) = state.label_for_local_with_children(dest) {
            self.error_with_hints(
                &self.loc(id),
                format!(
                    "cannot write to reference in {} which is still borrowed",
                    self.display(dest)
                ),
                "written here",
                self.borrow_info(state, label, false, alive).into_iter(),
            )
        }
        state.moved.insert(src);
        // The destination variable is not overridden, only what it is pointing to, so
        // no removal from moved
    }

    /// Get the location associated with bytecode attribute.
    fn loc(&self, id: AttrId) -> Loc {
        self.target.get_bytecode_loc(id)
    }

    /// Gets a string for a local to be displayed in error messages
    fn display(&self, local: TempIndex) -> String {
        self.target.get_local_name_for_error_message(local)
    }

    /// Get the type associated with local.
    fn ty(&self, local: TempIndex) -> &Type {
        self.target.get_local_type(local)
    }
}

impl<'env> TransferFunctions for LifeTimeAnalysis<'env> {
    type State = LifetimeState;

    const BACKWARD: bool = false;

    /// Transfer function for given bytecode.
    fn execute(&self, state: &mut Self::State, instr: &Bytecode, code_offset: CodeOffset) {
        use Bytecode::*;
        let alive = self
            .live_var_annotation
            .get_live_var_info_at(code_offset)
            .expect("livevar annotation");
        // Before processing the instruction, release all temps in the label map
        // which are not longer alive at this point.
        let alive_temps = alive.before_set();
        for temp in state.local_to_label_map.keys().cloned().collect::<Vec<_>>() {
            if !alive_temps.contains(&temp) {
                state.release_local(temp, &alive_temps)
            }
        }
        match instr {
            Load(_, dest, _) => {
                state.moved.remove(dest);
            },
            Assign(id, dest, src, kind) => {
                self.assign(state, code_offset, *id, *dest, *src, *kind, alive);
            },
            Ret(id, srcs) => self.return_(state, *id, srcs, alive),
            Call(id, dests, oper, srcs, _) => {
                use Operation::*;
                match oper {
                    BorrowLoc => {
                        self.borrow_local(state, code_offset, *id, dests[0], srcs[0], alive);
                    },
                    BorrowGlobal(mid, sid, inst) => {
                        self.borrow_global(
                            state,
                            code_offset,
                            *id,
                            mid.qualified_inst(*sid, inst.clone()),
                            dests[0],
                            alive,
                        );
                    },
                    BorrowField(mid, sid, inst, field_offs) => {
                        let (dest, src) = (dests[0], srcs[0]);
                        self.borrow_field(
                            state,
                            code_offset,
                            *id,
                            mid.qualified_inst(*sid, inst.clone()),
                            field_offs,
                            dest,
                            src,
                            alive,
                        );
                    },
                    ReadRef => self.read_ref(state, *id, dests[0], srcs[0], alive),
                    WriteRef => self.write_ref(state, *id, srcs[0], srcs[1], alive),
                    MoveFrom(mid, sid, inst) => self.move_from(
                        state,
                        *id,
                        dests[0],
                        &mid.qualified_inst(*sid, inst.clone()),
                        srcs[0],
                        alive,
                    ),
                    _ => self.call_operation(
                        state,
                        code_offset,
                        *id,
                        oper.clone(),
                        dests,
                        srcs,
                        alive,
                    ),
                }
            },
            _ => {},
        }
        // After processing, release any locals which are dying at this program point.
        // Variables which are introduced in this step but not alive after need to be released as well, as they
        // are not in the before set.
        let after_set = alive.after_set();
        for released in alive.before.keys().chain(
            instr
                .dests()
                .iter()
                .filter(|t| !alive.before.contains_key(t)),
        ) {
            if !after_set.contains(released) {
                state.release_local(*released, &after_set)
            }
        }
    }
}

/// Instantiate the data flow analysis framework based on the transfer function
impl<'env> DataflowAnalysis for LifeTimeAnalysis<'env> {}

// ===============================================================================
// Processor

pub struct ReferenceSafetyProcessor {}

/// Annotation produced by this processor
#[derive(Clone, Debug)]
pub struct LifetimeAnnotation(BTreeMap<CodeOffset, LifetimeInfoAtCodeOffset>);

impl LifetimeAnnotation {
    /// Returns information for code offset.
    pub fn get_info_at(&self, code_offset: CodeOffset) -> &LifetimeInfoAtCodeOffset {
        self.0.get(&code_offset).expect("lifetime info")
    }
}

/// Annotation present at each code offset
#[derive(Debug, Clone, Default)]
pub struct LifetimeInfoAtCodeOffset {
    pub before: LifetimeState,
    pub after: LifetimeState,
}

/// Public functions on lifetime info
impl LifetimeInfoAtCodeOffset {
    /// Returns the locals which are released at the give code offset since they are not borrowed
    /// any longer. Notice that this is only for locals which are actually borrowed, other
    /// locals being released need to be determined from livevar analysis results.
    pub fn released_temps(&self) -> impl Iterator<Item = TempIndex> + '_ {
        self.before
            .local_to_label_map
            .keys()
            .filter(|t| !self.after.local_to_label_map.contains_key(t))
            .cloned()
    }

    /// Returns true if the value in the variable has been moved at this program point.
    pub fn is_moved(&self, temp: TempIndex) -> bool {
        self.after.moved.contains(&temp)
    }
}

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
        let suppress_errors = fun_env
            .module_env
            .env
            .get_extension::<Options>()
            .unwrap_or_default()
            .experiment_on(Experiment::NO_SAFETY);
        let analyzer = LifeTimeAnalysis {
            target: &target,
            live_var_annotation,
            suppress_errors,
        };
        let code = target.get_bytecode();
        let cfg = StacklessControlFlowGraph::new_forward(code);
        let state_map =
            analyzer.analyze_function(LifetimeState::default(), target.get_bytecode(), &cfg);
        let mut state_map_per_instr = analyzer.state_per_instruction(
            state_map,
            target.get_bytecode(),
            &cfg,
            |before, after| LifetimeInfoAtCodeOffset {
                before: before.clone(),
                after: after.clone(),
            },
        );
        // For dead code, there may be holes in the map. Identify those and populate with default so
        // that each code offset actually has an annotation.
        for offset in 0..code.len() {
            state_map_per_instr
                .entry(offset as CodeOffset)
                .or_insert_with(LifetimeInfoAtCodeOffset::default);
        }
        let annotation = LifetimeAnnotation(state_map_per_instr);
        data.annotations.set(annotation, true);
        data
    }

    fn name(&self) -> String {
        "MemorySafetyProcessor".to_owned()
    }
}

// ===============================================================================================
// Display

impl ReferenceSafetyProcessor {
    /// Registers annotation formatter at the given function target. This is for debugging and
    /// testing.
    pub fn register_formatters(target: &FunctionTarget) {
        target.register_annotation_formatter(Box::new(format_lifetime_annotation))
    }
}

struct BorrowEdgeDisplay<'a>(&'a FunctionTarget<'a>, &'a BorrowEdge, bool);
impl<'a> Display for BorrowEdgeDisplay<'a> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let edge = &self.1;
        let display_child = self.2;
        use BorrowEdgeKind::*;
        (match &edge.kind {
            BorrowLocal => write!(f, "borrow({})", edge.is_mut),
            BorrowGlobal => write!(f, "borrow_global({})", edge.is_mut),
            BorrowField(_) => write!(f, "borrow_field({})", edge.is_mut),
            Call(_) => write!(f, "call({})", edge.is_mut),
            Skip => f.write_str("skip"),
        })?;
        if display_child {
            write!(f, " -> {}", edge.target)
        } else {
            Ok(())
        }
    }
}
impl BorrowEdge {
    fn display<'a>(
        &'a self,
        target: &'a FunctionTarget,
        display_child: bool,
    ) -> BorrowEdgeDisplay<'a> {
        BorrowEdgeDisplay(target, self, display_child)
    }
}

impl Display for LifetimeLabel {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            LifetimeLabel::Basic(n) => write!(f, "L{}", n),
            LifetimeLabel::Derived(l1, l2) => write!(f, "{}::{}", l1, l2),
        }
    }
}

struct MemoryLocationDisplay<'a>(&'a FunctionTarget<'a>, &'a MemoryLocation);
impl<'a> Display for MemoryLocationDisplay<'a> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        use MemoryLocation::*;
        let env = self.0.global_env();
        match self.1 {
            Global(qid) => write!(f, "global<{}>", env.display(qid)),
            Local(temp) => write!(
                f,
                "local({})",
                env.display(&self.0.get_local_raw_name(*temp))
            ),
            Replaced => write!(f, "replaced"),
        }
    }
}
impl MemoryLocation {
    fn display<'a>(&'a self, fun: &'a FunctionTarget) -> MemoryLocationDisplay<'a> {
        MemoryLocationDisplay(fun, self)
    }
}

struct LifetimeNodeDisplay<'a>(&'a FunctionTarget<'a>, &'a LifetimeNode);
impl<'a> Display for LifetimeNodeDisplay<'a> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}[", self.1.location.display(self.0))?;
        f.write_str(
            &self
                .1
                .children
                .iter()
                .map(|e| e.display(self.0, true).to_string())
                .join(","),
        )?;
        f.write_str("]")
    }
}
impl LifetimeNode {
    fn display<'a>(&'a self, fun: &'a FunctionTarget) -> LifetimeNodeDisplay<'a> {
        LifetimeNodeDisplay(fun, self)
    }
}

struct LifetimeDomainDisplay<'a>(&'a FunctionTarget<'a>, &'a LifetimeState);
impl<'a> Display for LifetimeDomainDisplay<'a> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let LifetimeState {
            graph,
            local_to_label_map,
            global_to_label_map,
            moved,
        } = &self.1;
        let pool = self.0.global_env().symbol_pool();
        writeln!(
            f,
            "graph: {}",
            graph.to_string(|k| k.to_string(), |v| v.display(self.0).to_string())
        )?;
        writeln!(
            f,
            "local_to_label: {{{}}}",
            local_to_label_map
                .iter()
                .map(|(temp, label)| format!(
                    "{}={}",
                    self.0.get_local_raw_name(*temp).display(pool),
                    label
                ))
                .join(",")
        )?;
        writeln!(
            f,
            "global_to_label: {{{}}}",
            global_to_label_map
                .iter()
                .map(|(str, label)| format!("{}={}", self.0.global_env().display(str), label))
                .join(",")
        )?;
        writeln!(
            f,
            "moved: {{{}}}",
            moved
                .iter()
                .map(|t| self.0.get_local_raw_name(*t).display(pool).to_string())
                .join(",")
        )
    }
}
impl LifetimeState {
    fn display<'a>(&'a self, fun: &'a FunctionTarget) -> LifetimeDomainDisplay<'a> {
        LifetimeDomainDisplay(fun, self)
    }
}

fn format_lifetime_annotation(
    target: &FunctionTarget<'_>,
    code_offset: CodeOffset,
) -> Option<String> {
    if let Some(LifetimeAnnotation(map)) = target.get_annotations().get::<LifetimeAnnotation>() {
        if let Some(at) = map.get(&code_offset) {
            return Some(at.before.display(target).to_string());
        }
    }
    None
}
