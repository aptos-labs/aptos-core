// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! Implements memory safety analysis.
//!
//! NOTE: this implementation is experimental and currently not used.
//!
//! Prerequisite: livevar annotation is available by performing liveness analysis.
//!
//! This is an intra functional, forward-directed data flow analysis over the domain
//! of what we call a *borrow graph*. The borrow graph tracks the creation of references from
//! root memory locations and derivation of other references, by recording an edge for each
//! borrow relation. For example, if `s` is the memory location of a struct, then
//! `&s.f` is represented by a node which is derived from `s`, and those two nodes are
//! connected by an edge labeled with `.f`. In the below example, we have `&s.g` stored
//! in `r1` and `&s.f` stored in `r2` (edges in the graph should be read as arrows pointing
//! downwards):
//!
//! ```text
//!              s
//!              | &
//!          .g / \ .f
//!            r1 r2
//! ```
//!
//! Borrow graphs do not come into a normalized form, thus different graphs can represent the
//! same borrow relations. For example, this graph is equivalent to the above. It has what
//! we call an _implicit choice_, that is the choice between alternatives is down
//! after a borrow step:
//!
//! ```text
//!              s
//!           & / \ &
//!          .g |  | .f
//!            r1 r2
//! ```
//!
//! In general, the graph is a DAG. Joining of nodes represents branching in the code. For
//! example, the graph below depicts that `r` can either be `&s.f` or `&s.g`:
//!
//! ```text
//!              s
//!           & / \ &
//!          .g \ / .f
//!              r
//! ```
//!
//! Together with the borrow graph, pointing from temporaries to graph nodes is maintained at
//! each program point. These represent the currently alive references into the borrowed data.
//! All the _parents_ of those nodes are indirectly borrowed as well. For example, in the
//! graph above, `r` is active and `s` is as well because it is indirectly borrowed by `r`.
//! When the temporaries pointing into the borrow graph are not longer alive, we perform a
//! clean up step and also release any parents not longer needed. For more details of
//! this mechanics, see the comments in the code.
//!
//! The safety analysis essentially evaluates each instruction under the viewpoint of the current
//! active borrow graph at the program point, to detect any conditions of non-safety. This
//! includes specifically the following rules:
//!
//! 1. A local which is borrowed (i.e. points to a node in the graph) cannot be overwritten.
//! 2. A local which is borrowed cannot be moved.
//! 3. References returned from a function call must be derived from parameters
//! 4. Before any call to a user function, or before reading or writing a reference,
//!    the borrow graph must be _safe_ w.r.t. the arguments. Notice specifically, that for
//!    a series of other instructions (loading and moving temporaries around, borrowing fields)
//!    safety is not enforced. This is important to allow construction of more complex
//!    borrow graphs, where the intermediate steps are not safe.
//!
//! To understand the concept of a _safe_ borrow graph consider that edges have a notion of being
//! disjoint. For instance, field selection `s.f` and `s.g` constructs two references into `s` which
//! can safely co-exist because there is no overlap. Consider further a path `p` being a sequence
//! of borrow steps (edges) in the graph from a root to a leaf node. For two paths `p1` and `p2`,
//! _diverging edges_, `(e1, e2) = diverging(p1, p2)`, are a pair of edges where the paths differ
//! after some non-empty common prefix, and do not have a common node where they join again. Here is
//! an example of two paths with diverging edges:
//!
//! ```text
//!              s
//!        &mut / \ &mut
//!      call f |  | call g
//!            r1  r2
//! ```
//!
//! Here is another example where, while edges differ, they do not diverge because the paths later
//! join again. Notice that this is a result of different execution paths from code
//! like `let r = if (c) f(&mut s) else g(&mut s)`:
//!
//! ```text
//!              s
//!        &mut / \ &mut
//!      call f |  | call g
//!             \  /
//!              r
//! ```
//!
//! Given this definition, a graph is called *safe w.r.t. a set of temporaries `temps`*
//! under the following conditions:
//!
//! a. Any path which does not end in `nodes(temps)` is safe and considered out of scope,
//!    where `nodes(temps)` denotes the nodes which are associated with the given `temps`.
//! b. For any two paths `p` and `q`, `q != p`, and any pair of diverging edges `e1` and `e2`, if
//!    any of those edges is mut, the other needs to be disjoint. This basically states that one
//!    cannot have `&x.f` and `&mut x.f` coexist in a safe graph. However, `&x.f` and `&mut x.g`
//!    is fine.
//! c. For any path `p`, if the last edge is mut, `p` must not be a prefix of any other path. This
//!    basically states that mutable reference in `temps` must be exclusive and cannot
//!    have other borrows.
//! d. For all identical paths in the graph (recall that because of indirect choices, we can
//!    have the same path appearing multiple times in the graph), if the last edge is mut, then
//!    the set of temporaries associated with those paths must be a singleton. This basically
//!    states that the same mutable reference in `temps` cannot be used twice.

use crate::{
    pipeline::{
        livevar_analysis_processor::{LiveVarAnnotation, LiveVarInfoAtCodeOffset},
        reference_safety::{LifetimeAnnotation, LifetimeInfo, LifetimeInfoAtCodeOffset},
    },
    Experiment, Options,
};
use abstract_domain_derive::AbstractDomain;
use codespan_reporting::diagnostic::Severity;
use itertools::Itertools;
use log::{debug, log_enabled, Level};
use move_binary_format::file_format::CodeOffset;
use move_model::{
    ast::{AccessSpecifierKind, TempIndex},
    model::{FieldId, FunId, FunctionEnv, GlobalEnv, Loc, Parameter, QualifiedInstId, StructId},
    symbol::Symbol,
    ty::{ReferenceKind, Type},
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
    cmp::Ordering,
    collections::{btree_map, BTreeMap, BTreeSet},
    fmt::{Display, Formatter},
    iter,
    rc::Rc,
};

const DEBUG: bool = false;

// ===============================================================================
// Memory Safety Analysis

// -------------------------------------------------------------------------------------------------
// Program Analysis Domain

/// The program analysis domain used with the data flow analysis framework.
#[derive(Clone, Default, Debug)]
pub struct LifetimeState {
    /// Contains the borrow graph at the current program point, which consists of a set of `LifetimeNode` values
    /// which are labeled by `LifetimeLabel`. This contains exactly those nodes reachable
    /// as parents or children of the node labels used in the below maps and grows and shrinks from
    /// program point to program point.
    graph: MapDomain<LifetimeLabel, LifetimeNode>,
    /// A map from temporaries to labels, for those temporaries which have an associated node in the graph.
    /// If a local is originally borrowed, it will point from `temp` to a node with the `MemoryLocation::Local(temp)`.
    /// If a local is a reference derived from a location, it will point to a node with `MemoryLocation::Derived`.
    temp_to_label_map: BTreeMap<TempIndex, LifetimeLabel>,
    /// A map from globals to labels. Represents root states of the active graph.
    global_to_label_map: BTreeMap<QualifiedInstId<StructId>, LifetimeLabel>,
    /// A map indicating which nodes have been derived from the given set of temporaries.
    /// For example, if we have `label <- borrow_field(f)(src)`, then `label -> src` will be in
    /// this map. This map is used to deal with a quirk of v1 borrow semantics which allows
    /// a temporary which was used to derive a node to be used after the borrow again, but
    /// does not allow the same thing with a temporary which contains a copy of this reference.
    /// Once we update the v1 bytecode verifier, this should go away, because there is no safety
    /// reason to not allow the copy.
    derived_from: BTreeMap<LifetimeLabel, BTreeSet<TempIndex>>,
}

/// Represents a node of the borrow graph.
#[derive(AbstractDomain, Clone, Debug, PartialEq, Eq)]
struct LifetimeNode {
    /// Memory locations associated with this node. This is a set as a result of joins.
    locations: SetDomain<MemoryLocation>,
    /// Outgoing edges to children.
    children: SetDomain<BorrowEdge>,
    /// Backlinks to parents.
    parents: SetDomain<LifetimeLabel>,
}

/// A label for a lifetime node.
#[derive(PartialOrd, Ord, PartialEq, Eq, Clone, Copy, Debug)]
struct LifetimeLabel(u64);

/// A memory location, either a global in storage, a local on the stack, an external from parameter, or
/// a derived portion of it.
#[derive(PartialOrd, Ord, PartialEq, Eq, Clone, Debug)]
enum MemoryLocation {
    /// The underlying memory is in global storage.
    Global(QualifiedInstId<StructId>),
    /// The underlying memory is a local on the stack.
    Local(TempIndex),
    /// The underlying memory is some external memory referenced by a function parameter
    External,
    /// Derives from underlying memory as defined by incoming edges. This is used to represent the
    /// result of a field select or function call.
    Derived,
}

/// Represents an edge in the borrow graph. The source of the edge is implicit in the ownership
/// of the edge by a LifetimeNode through its children field
#[derive(PartialOrd, Ord, PartialEq, Eq, Clone, Debug)]
struct BorrowEdge {
    /// The kind of borrow edge.
    kind: BorrowEdgeKind,
    /// A location associated with the borrow edge.
    loc: Loc,
    /// Target of the edge.
    target: LifetimeLabel,
}

/// The different type of edges. Each edge caries a boolean indicating whether it is a mutating borrow.
#[derive(PartialOrd, Ord, PartialEq, Eq, Clone, Debug)]
enum BorrowEdgeKind {
    /// Borrows the local at the MemoryLocation in the source node.
    BorrowLocal(bool),
    /// Borrows the global at the MemoryLocation in the source node. Since the address
    /// from which we borrow can be different for each call, they are distinguished by code offset
    /// -- two borrow_global edges are never the same.
    BorrowGlobal(bool, CodeOffset),
    /// Borrows a field from a reference.
    BorrowField(bool, FieldId),
    /// Calls an operation, where the incoming references are used to derive outgoing references. Similar
    /// as for BorrowGlobal, a call offset is used to distinguish different results of calls.
    Call(bool, Operation, CodeOffset),
    /// Freezes a mutable reference.
    Freeze,
}

impl BorrowEdgeKind {
    fn is_mut(&self) -> bool {
        use BorrowEdgeKind::*;
        match self {
            BorrowLocal(is_mut)
            | BorrowGlobal(is_mut, _)
            | BorrowField(is_mut, _)
            | Call(is_mut, _, _) => *is_mut,
            Freeze => false,
        }
    }

    /// Returns true if any of the edge kinds in the set is mut
    fn any_is_mut(kinds: &BTreeSet<BorrowEdgeKind>) -> bool {
        kinds.iter().any(|k| k.is_mut())
    }

    /// Determines whether the region derived from this edge has overlap with the region
    /// of the other edge. Overlap can only be excluded for field edges.
    fn could_overlap(&self, other: &BorrowEdgeKind) -> bool {
        use BorrowEdgeKind::*;
        match (self, other) {
            (BorrowField(_, field1), BorrowField(_, field2)) => field1 == field2,
            _ => true,
        }
    }

    /// Returns true if there is any overlap between the edges in the two sets.
    fn any_could_overlap(set1: &BTreeSet<BorrowEdgeKind>, set2: &BTreeSet<BorrowEdgeKind>) -> bool {
        set1.iter()
            .any(|k1| set2.iter().any(|k2| k1.could_overlap(k2)))
    }
}

impl LifetimeLabel {
    /// Creates a new, unique and stable, life time label based on a code offset and
    /// a qualifier to distinguish multiple labels at the same code point.
    /// Since the program analysis could run fixpoint loops, we need to ensure that
    /// these labels are the same in each iteration.
    fn new_from_code_offset(code_offset: CodeOffset, qualifier: u8) -> LifetimeLabel {
        LifetimeLabel(((code_offset as u64) << 8) | (qualifier as u64))
    }

    /// Creates a globally unique label from a counter. These are disjoint from those
    /// from code labels.
    fn new_from_counter(count: u32) -> LifetimeLabel {
        // code offset = 16 bits, qualifier 8 bits
        LifetimeLabel(((count + 1) as u64) << 24)
    }
}

impl BorrowEdge {
    /// Shortcut to create an edge.
    fn new(kind: BorrowEdgeKind, loc: Loc, target: LifetimeLabel) -> Self {
        Self { kind, loc, target }
    }
}

impl BorrowEdgeKind {}

impl AbstractDomain for LifetimeState {
    /// The join operator of the dataflow analysis domain.
    ///
    /// Joining of lifetime states is easy for the borrow graph, as we can simply join the node representations
    /// using the same label. This is consistent because each label is constructed from the program point.
    /// However, if it comes to the mappings of globals/temporaries to labels, we need to unify distinct labels of the
    /// two states. Consider `$t1 -> @1` in one state and `$t1 -> @2` in another state, then we need to unify
    /// the states under labels `@1` and `@2` into one, and renames any occurrence of the one label by the other.
    fn join(&mut self, other: &Self) -> JoinResult {
        // Join the graph
        let mut change = self.graph.join(&other.graph);
        self.check_graph_consistency();

        // A label renaming map resulting from joining lifetime nodes.
        let mut renaming: BTreeMap<LifetimeLabel, LifetimeLabel> = BTreeMap::new();

        let mut new_temp_to_label_map = std::mem::take(&mut self.temp_to_label_map);
        change = change.combine(self.join_label_map(
            &mut new_temp_to_label_map,
            &other.temp_to_label_map,
            &mut renaming,
        ));
        let mut new_global_to_label_map = std::mem::take(&mut self.global_to_label_map);
        change = change.combine(self.join_label_map(
            &mut new_global_to_label_map,
            &other.global_to_label_map,
            &mut renaming,
        ));
        self.temp_to_label_map = new_temp_to_label_map;
        self.global_to_label_map = new_global_to_label_map;

        if !renaming.is_empty() {
            Self::rename_labels_in_graph(&renaming, &mut self.graph);
            Self::rename_labels_in_map(&renaming, &mut self.temp_to_label_map);
            Self::rename_labels_in_map(&renaming, &mut self.global_to_label_map);
            change = JoinResult::Changed;
        }
        self.check_graph_consistency();
        change
    }
}

impl LifetimeState {
    /// Joins two maps with labels in their range. For overlapping keys pointing to different labels,
    /// the nodes behind the labels in the graph are joined, and the label in the `other_map` is
    /// replaced by the given one in `map`. This functions remembers (but does not yet apply)
    /// the replaced labels in the `renaming` map.
    fn join_label_map<A: Clone + Ord>(
        &mut self,
        map: &mut BTreeMap<A, LifetimeLabel>,
        other_map: &BTreeMap<A, LifetimeLabel>,
        renaming: &mut BTreeMap<LifetimeLabel, LifetimeLabel>,
    ) -> JoinResult {
        let mut change = JoinResult::Unchanged;
        for (k, other_label) in other_map {
            match map.entry(k.clone()) {
                btree_map::Entry::Vacant(entry) => {
                    entry.insert(*other_label);
                    change = JoinResult::Changed;
                },
                btree_map::Entry::Occupied(entry) => {
                    let label = entry.get();
                    if label != other_label {
                        // Merge other node into this one, and add renaming of label.
                        let other_copy = self.node(other_label).clone(); // can't mut and read same time
                        self.node_mut(label).join(&other_copy);
                        renaming.insert(*other_label, *label);
                        change = JoinResult::Changed;
                    }
                },
            }
        }
        change
    }

    fn rename_label(renaming: &BTreeMap<LifetimeLabel, LifetimeLabel>, label: &mut LifetimeLabel) {
        // Apply renaming transitively -- it likely cannot occur right now but perhaps in the future.
        let mut visited = BTreeSet::new();
        while let Some(actual) = renaming.get(label) {
            assert!(visited.insert(*label), "renaming must be acyclic");
            *label = *actual
        }
    }

    fn rename_labels_in_map<A: Clone + Ord>(
        renaming: &BTreeMap<LifetimeLabel, LifetimeLabel>,
        map: &mut BTreeMap<A, LifetimeLabel>,
    ) {
        for label in map.values_mut() {
            Self::rename_label(renaming, label)
        }
    }

    fn rename_labels_in_graph(
        renaming: &BTreeMap<LifetimeLabel, LifetimeLabel>,
        graph: &mut MapDomain<LifetimeLabel, LifetimeNode>,
    ) {
        graph.update_values(|node| {
            let mut new_edges = SetDomain::default();
            for mut edge in std::mem::take(&mut node.children).into_iter() {
                Self::rename_label(renaming, &mut edge.target);
                new_edges.insert(edge);
            }
            node.children = new_edges;
            Self::rename_labels_in_set(renaming, &mut node.parents)
        });
        // Delete any nodes which are renamed
        for l in renaming.keys() {
            graph.remove(l);
        }
    }

    fn rename_labels_in_set(
        renaming: &BTreeMap<LifetimeLabel, LifetimeLabel>,
        set: &mut SetDomain<LifetimeLabel>,
    ) {
        *set = set
            .iter()
            .cloned()
            .map(|mut l| {
                Self::rename_label(renaming, &mut l);
                l
            })
            .collect();
    }

    /// Checks, at or above debug level, that
    /// - for any nodes `v, u` in the borrow graph, `v` is has parent/child `u` iff `u` has child/parent `v`
    /// - all labels in the label maps are in the graph
    fn check_graph_consistency(&self) {
        if log_enabled!(Level::Debug) {
            self.debug_print("before check");
            for (l, n) in self.graph.iter() {
                for e in n.children.iter() {
                    assert!(
                        self.graph.contains_key(&e.target),
                        "{} child not in graph",
                        e.target
                    );
                    assert!(
                        self.node(&e.target).parents.contains(l),
                        "{} is not included as a parent in {}",
                        l,
                        e.target
                    )
                }
                for p in n.parents.iter() {
                    assert!(self.graph.contains_key(p), "{} parent not in graph", p);
                    assert!(
                        self.node(p).children.iter().any(|e| &e.target == l),
                        "{} no a child of {}",
                        l,
                        p
                    )
                }
            }
            for l in self
                .temp_to_label_map
                .values()
                .chain(self.global_to_label_map.values())
            {
                assert!(
                    self.graph.contains_key(l),
                    "{} is in label map but not in graph",
                    l
                )
            }
        }
    }

    fn debug_print(&self, header: &str) {
        if DEBUG && log_enabled!(Level::Debug) {
            let mut header = header.to_owned();
            for (l, n) in self.graph.iter() {
                debug!(
                    "{} {} {:?} -> {}  (<- {})",
                    header,
                    l,
                    n.locations,
                    n.children
                        .iter()
                        .map(|e| format!("{}", e.target))
                        .join(", "),
                    n.parents.iter().map(|l| format!("{}", l)).join(", ")
                );
                header = (0..header.len()).map(|_| ' ').collect();
            }
            debug!(
                "{} {}",
                header,
                self.temp_to_label_map
                    .iter()
                    .map(|(k, v)| format!("$t{} = {}", k, v))
                    .join(", ")
            )
        }
    }
}

// -------------------------------------------------------------------------------------------------
// Working with LifetimeState

impl LifetimeState {
    /// Creates a new node with the given label and location information.
    fn new_node(&mut self, assigned_label: LifetimeLabel, location: MemoryLocation) {
        self.graph.insert(assigned_label, LifetimeNode {
            locations: iter::once(location).collect(),
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

    /// Returns true if the given label is an ancestor of the other. This is transitive and reflexive.
    fn is_ancestor(&self, label: &LifetimeLabel, descendant: &LifetimeLabel) -> bool {
        label == descendant
            || self
                .children(label)
                .any(|e| self.is_ancestor(&e.target, descendant))
    }

    /// Returns an iteration of child edges of given node.
    fn children(&self, label: &LifetimeLabel) -> impl Iterator<Item = &BorrowEdge> {
        self.node(label).children.iter()
    }

    /// Returns true if the node has incoming mut edges.
    fn is_mut(&self, label: &LifetimeLabel) -> bool {
        self.parent_edges(label).any(|(_, e)| e.kind.is_mut())
    }

    /// Returns the children of the given nodes, grouped into hyper edges. A hyper edge
    /// is constituted by a set of edge kinds and associated list of edges. Each hyper edge
    /// represents an abstract borrow operation.
    ///
    /// 1) All edges which lead into the same node are considered to be part of the same hyper
    /// edge. Consider:
    ///
    /// ```text
    ///           \     /
    ///         e1 \   / e2
    ///             \ /
    ///              n
    /// ```
    /// This forms a hyper edge `{e1.kind, e2.kind} -> [e1, e2]`. Both edges have to be in the same
    /// group because `n` has a 'weak' borrow history, it can either stem from `e1` or `e2`.
    ///
    /// 2) For all other edges not leading into the same node, they are grouped according
    /// their kind. Consider:
    ///
    /// ```text
    ///            |    |
    ///         e1 |    | e2
    ///            |    |
    ///           n1   n2
    /// ```
    /// If `kind == e1.kind == e2.kind`, this forms a hyper edge `{kind} -> [e1, e2]`, otherwise
    /// it will be two independent hyper edges `{e1.kind} -> [e1]` and `{e2.kind} -> [e2]`. The
    /// former reflects that the edges of the same kind are the same abstract borrow operation,
    /// independent of the number of edges involved.
    fn group_children_into_hyper_edges(
        &self,
        labels: &BTreeSet<LifetimeLabel>,
    ) -> BTreeMap<BTreeSet<BorrowEdgeKind>, Vec<&BorrowEdge>> {
        // First compute map from target nodes to edges, allowing to identify weak edges.
        let mut target_to_incoming: BTreeMap<LifetimeLabel, Vec<&BorrowEdge>> = BTreeMap::new();
        for edge in labels.iter().flat_map(|l| self.children(l)) {
            target_to_incoming
                .entry(edge.target)
                .or_default()
                .push(edge)
        }
        // Now compute the result.
        let mut result: BTreeMap<BTreeSet<BorrowEdgeKind>, Vec<&BorrowEdge>> = BTreeMap::new();
        for (_, mut edges) in target_to_incoming {
            let key = edges
                .iter()
                .map(|e| e.kind.clone())
                .collect::<BTreeSet<_>>();
            result.entry(key).or_default().append(&mut edges);
        }
        result
    }

    /// Returns true if given node has no children
    fn is_leaf(&self, label: &LifetimeLabel) -> bool {
        self.node(label).children.is_empty()
    }

    /// Returns true of this edge leads to a mutable leaf. A mutable edge
    /// can lead to an immutable leaf via a freeze edge, that is why
    /// a transitive check is necessary.
    fn is_mut_path(&self, edge: &BorrowEdge) -> bool {
        if self.is_leaf(&edge.target) {
            edge.kind.is_mut()
        } else {
            self.children(&edge.target).any(|e| self.is_mut_path(e))
        }
    }

    /// Gets the label associated with a local, if it has children.
    fn label_for_temp_with_children(&self, temp: TempIndex) -> Option<&LifetimeLabel> {
        self.label_for_temp(temp).filter(|l| !self.is_leaf(l))
    }

    /// Gets the label associated with a global, if it has children.
    fn label_for_global_with_children(
        &self,
        resource: &QualifiedInstId<StructId>,
    ) -> Option<&LifetimeLabel> {
        self.label_for_global(resource).filter(|l| !self.is_leaf(l))
    }

    /// Returns true if the node has outgoing mut edges.
    fn has_mut_edges(&self, label: &LifetimeLabel) -> bool {
        self.children(label).any(|e| e.kind.is_mut())
    }

    /// Gets the label associated with a local.
    fn label_for_temp(&self, temp: TempIndex) -> Option<&LifetimeLabel> {
        self.temp_to_label_map.get(&temp)
    }

    /// If label for local exists, return it, otherwise create a new node. The code offset and qualifier are
    /// used to create a lifetime label if needed. 'root' indicates whether is a label for an actual memory
    /// root (like a local, external, or global) instead of a reference.
    fn make_temp(
        &mut self,
        temp: TempIndex,
        code_offset: CodeOffset,
        qualifier: u8,
        root: bool,
    ) -> LifetimeLabel {
        self.make_temp_from_label_fun(
            temp,
            || LifetimeLabel::new_from_code_offset(code_offset, qualifier),
            root,
        )
    }

    /// More general version as above where the label to be created, if needed, is specified
    /// by a function.
    fn make_temp_from_label_fun(
        &mut self,
        temp: TempIndex,
        from_label: impl Fn() -> LifetimeLabel,
        root: bool,
    ) -> LifetimeLabel {
        if let Some(label) = self.temp_to_label_map.get(&temp) {
            *label
        } else {
            let label = from_label();
            self.new_node(
                label,
                if root {
                    MemoryLocation::Local(temp)
                } else {
                    MemoryLocation::Derived
                },
            );
            self.temp_to_label_map.insert(temp, label);
            label
        }
    }

    /// Gets the label associated with a global.
    fn label_for_global(&self, global: &QualifiedInstId<StructId>) -> Option<&LifetimeLabel> {
        self.global_to_label_map.get(global)
    }

    /// If label for global exists, return it, otherwise create a new one.
    fn make_global(
        &mut self,
        struct_id: QualifiedInstId<StructId>,
        code_offset: CodeOffset,
        qualifier: u8,
    ) -> LifetimeLabel {
        if let Some(label) = self.global_to_label_map.get(&struct_id) {
            *label
        } else {
            let label = LifetimeLabel::new_from_code_offset(code_offset, qualifier);
            self.new_node(label, MemoryLocation::Global(struct_id.clone()));
            self.global_to_label_map.insert(struct_id, label);
            label
        }
    }

    /// Adds an edge to the graph.
    fn add_edge(&mut self, label: LifetimeLabel, edge: BorrowEdge) {
        let child = edge.target;
        self.node_mut(&label).children.insert(edge);
        self.node_mut(&child).parents.insert(label);
    }

    /// Drops a leaf node. The parents are recursively dropped if their children go down to
    /// zero. Collects the locations of the dropped nodes. Gets passed the set of
    /// labels which are currently in use and pointed to from outside of the graph.
    fn drop_leaf_node(
        &mut self,
        label: &LifetimeLabel,
        in_use: &BTreeSet<LifetimeLabel>,
        removed: &mut BTreeSet<MemoryLocation>,
    ) {
        if in_use.contains(label) {
            return;
        }
        // Remove any information about temporaries used to derive this node.
        self.derived_from.remove(label);
        // Rempve the node from the graph.
        if let Some(node) = self.graph.remove(label) {
            debug_assert!(node.children.is_empty());
            removed.extend(node.locations.iter().cloned());
            for parent in node.parents.iter() {
                let node = self.node_mut(parent);
                // Remove the dropped node from the children list.
                let children = std::mem::take(&mut node.children);
                node.children = children
                    .into_iter()
                    .filter(|e| &e.target != label)
                    .collect();
                // Drop the parent as well if it is now a leaf
                if node.children.is_empty() {
                    self.drop_leaf_node(parent, in_use, removed)
                }
            }
        }
    }

    /// Returns a map from labels which are used by temporaries to the set which are using them.
    fn leaves(&self) -> BTreeMap<LifetimeLabel, BTreeSet<TempIndex>> {
        let mut map: BTreeMap<LifetimeLabel, BTreeSet<TempIndex>> = BTreeMap::new();
        for (temp, label) in &self.temp_to_label_map {
            map.entry(*label).or_default().insert(*temp);
        }
        map
    }

    /// Releases graph resources for a reference in temporary.
    fn release_ref(&mut self, temp: TempIndex) {
        if let Some(label) = self.temp_to_label_map.remove(&temp) {
            if self.is_leaf(&label) {
                // We can drop the underlying node, as there are no borrows out, and
                // it is not mapped from another temp.
                let in_use = self.leaves().keys().cloned().collect();
                let mut indirectly_removed = BTreeSet::new();
                self.drop_leaf_node(&label, &in_use, &mut indirectly_removed);
                // Remove memory locations no longer borrowed.
                for location in indirectly_removed {
                    use MemoryLocation::*;
                    match location {
                        Local(temp) => {
                            self.temp_to_label_map.remove(&temp);
                        },
                        Global(qid) => {
                            self.global_to_label_map.remove(&qid);
                        },
                        External | Derived => {},
                    }
                }
            }
        }
        self.check_graph_consistency()
    }

    /// Replaces a reference in a temporary, as result of an assignment. The current
    /// node associated with the ref is released and then a new node is created and
    /// returned.
    fn replace_ref(
        &mut self,
        temp: TempIndex,
        code_offset: CodeOffset,
        qualifier: u8,
    ) -> LifetimeLabel {
        self.release_ref(temp);
        // Temp might not be released if it is still borrowed, so remove from the map
        self.temp_to_label_map.remove(&temp);
        let label = self.make_temp(temp, code_offset, qualifier, false);
        self.check_graph_consistency();
        label
    }

    /// Move a reference from source to destination. This moves the LifetimeLabel over to the new temp.
    fn move_ref(&mut self, dest: TempIndex, src: TempIndex) {
        let Some(label) = self.temp_to_label_map.remove(&src) else {
            return;
        };
        self.temp_to_label_map.insert(dest, label);
        self.check_graph_consistency()
    }

    /// Copies a reference from source to destination. This create a new lifetime node and clones the edges
    /// leading into the node associated with the source reference.
    fn copy_ref(&mut self, dest: TempIndex, src: TempIndex) {
        if let Some(label) = self.label_for_temp(src).cloned() {
            self.temp_to_label_map.insert(dest, label);
            self.mark_derived_from(label, src)
        }
    }

    /// Marks the node with label to be derived from temporary.
    fn mark_derived_from(&mut self, label: LifetimeLabel, temp: TempIndex) {
        self.derived_from.entry(label).or_default().insert(temp);
    }

    /// Gets the set of active temporaries from which nodes are derived.
    fn derived_temps(&self) -> BTreeSet<TempIndex> {
        self.derived_from.values().flatten().cloned().collect()
    }

    /// Returns an iterator of the edges which are leading into this node.
    #[allow(unused)]
    fn parent_edges<'a>(
        &'a self,
        label: &'a LifetimeLabel,
    ) -> impl Iterator<Item = (LifetimeLabel, &'a BorrowEdge)> + 'a {
        self.node(label).parents.iter().flat_map(move |parent| {
            self.children(parent)
                .filter(move |edge| &edge.target == label)
                .map(|e| (*parent, e))
        })
    }

    /// Returns the roots of this node, that is those ancestors which have no parents.
    fn roots(&self, label: &LifetimeLabel) -> BTreeSet<LifetimeLabel> {
        let mut roots = BTreeSet::new();
        let mut todo = self.node(label).parents.iter().cloned().collect::<Vec<_>>();
        if todo.is_empty() {
            // Node is already root
            roots.insert(*label);
        } else {
            let mut done = BTreeSet::new();
            while let Some(l) = todo.pop() {
                if !done.insert(l) {
                    continue;
                }
                let node = self.node(&l);
                if node.parents.is_empty() {
                    // Found a root
                    roots.insert(l);
                } else {
                    // Explore parents
                    todo.extend(node.parents.iter().cloned())
                }
            }
        }
        roots
    }

    /// Returns the transitive children of this node.
    fn transitive_children(&self, label: &LifetimeLabel) -> BTreeSet<LifetimeLabel> {
        // Helper function to collect the target nodes of the children.
        let get_children =
            |label: &LifetimeLabel| self.node(label).children.iter().map(|e| e.target);
        let mut result = BTreeSet::new();
        result.insert(*label);
        let mut todo = get_children(label).collect::<Vec<_>>();
        while let Some(l) = todo.pop() {
            if !result.insert(l) {
                continue;
            }
            todo.extend(get_children(&l));
        }
        result
    }
}

// -------------------------------------------------------------------------------------------------
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
    // If true, any errors generated by this analysis will be suppressed
    suppress_errors: bool,
}

/// A structure encapsulating, in addition to the analysis context, context
/// about the current instruction step being processed.
struct LifetimeAnalysisStep<'env, 'state> {
    /// The analysis context
    parent: &'env LifeTimeAnalysis<'env>,
    /// The code offset
    code_offset: CodeOffset,
    /// The attribute id at the code offset
    attr_id: AttrId,
    /// Lifetime information at the given code offset
    alive: &'env LiveVarInfoAtCodeOffset,
    /// Mutable reference to the analysis state
    state: &'state mut LifetimeState,
}

/// Used to distinguish how a local is read
#[derive(Clone, Copy, PartialEq, Eq)]
enum ReadMode {
    /// The local is moved
    Move,
    /// The local is copied
    Copy,
    /// The local is transferred as an argument to another function
    Argument,
    /// The local is used as a branch condition
    BranchCondition,
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
            code_offset,
            attr_id,
            alive,
            state,
        }
    }
}

// -------------------------------------------------------------------------------------------------
// Analysing and Diagnosing

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

    /// Display a non-empty set of temps. This prefers the first printable representative, if any
    fn display_set(&self, set: &BTreeSet<TempIndex>) -> String {
        if let Some(temp) = set
            .iter()
            .find(|t| self.target().get_local_name_opt(**t).is_some())
        {
            self.display(*temp)
        } else {
            self.display(*set.first().expect("non empty"))
        }
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

    /// Returns true if the local is a reference.
    fn is_ref(&self, local: TempIndex) -> bool {
        self.ty(local).is_reference()
    }

    /// Check validness of reading a local.
    fn check_read_local(&self, local: TempIndex, read_mode: ReadMode) -> bool {
        if self.is_ref(local) {
            // Always valid
            return true;
        }
        if let Some(label) = self.state.label_for_temp_with_children(local) {
            let loc = self.cur_loc();
            let usage_info = || self.usage_info(label, |t| t != &local);
            match read_mode {
                ReadMode::Copy => {
                    // Mutable borrow is not allowed
                    if self.state.has_mut_edges(label) {
                        self.error_with_hints(
                            loc,
                            format!(
                                "cannot copy {} which is still mutably borrowed",
                                self.display(local)
                            ),
                            "copied here",
                            self.borrow_info(label, |e| e.kind.is_mut())
                                .into_iter()
                                .chain(usage_info()),
                        );
                        false
                    } else {
                        true
                    }
                },
                ReadMode::Move => {
                    // Any borrow not allowed
                    self.error_with_hints(
                        loc,
                        format!(
                            "cannot move {} which is still borrowed",
                            self.display(local)
                        ),
                        "moved here",
                        self.borrow_info(label, |_| true)
                            .into_iter()
                            .chain(usage_info()),
                    );
                    false
                },
                ReadMode::Argument => {
                    // Mutable borrow not allowed
                    if self.state.has_mut_edges(label) {
                        self.error_with_hints(
                            loc,
                            format!(
                                "cannot pass {} which is still mutably \
                                    borrowed as function argument",
                                self.display(local)
                            ),
                            "passed here",
                            self.borrow_info(label, |_| true)
                                .into_iter()
                                .chain(usage_info()),
                        );
                        false
                    } else {
                        true
                    }
                },
                ReadMode::BranchCondition => {
                    // Mutable borrow not allowed
                    if self.state.has_mut_edges(label) {
                        self.error_with_hints(
                            loc,
                            format!(
                                "cannot use {} which is still mutably \
                                    borrowed as branch condition",
                                self.display(local)
                            ),
                            "used in this context",
                            self.borrow_info(label, |_| true)
                                .into_iter()
                                .chain(usage_info()),
                        );
                        false
                    } else {
                        true
                    }
                },
            }
        } else {
            true
        }
    }

    /// Check whether a local can be written. This is only allowed if no borrowed references exist.
    fn check_write_local(&self, local: TempIndex) {
        if self.is_ref(local) {
            // Always valid
            return;
        }
        if let Some(label) = self.state.label_for_temp_with_children(local) {
            // The destination is currently borrowed and cannot be assigned
            self.error_with_hints(
                self.cur_loc(),
                format!("cannot assign to borrowed {}", self.display(local)),
                "attempted to assign here",
                self.borrow_info(label, |_| true)
                    .into_iter()
                    .chain(self.usage_info(label, |t| t != &local)),
            )
        }
    }

    /// Marks in the borrow state that the inputs of an instruction have been consumed. At
    /// this point all references which are not alive after this program point can be
    /// released. Notice that this must be called before a check_write_local can be
    /// performed. This function is idempotent for a given program step.
    fn release_refs_not_alive_after(&mut self) {
        for temp in self
            .state
            .temp_to_label_map
            .keys()
            .cloned()
            .collect::<Vec<_>>()
        {
            if self.is_ref(temp) && !self.alive.after.contains_key(&temp) {
                self.state.release_ref(temp)
            }
        }
    }

    /// Check whether the borrow graph is 'safe' w.r.t a set of `exclusive_temps`. Those temporaries
    /// are used as a list of arguments to a function call and need to follow borrow rules of
    /// exclusive access, as discussed at the beginning of this file.
    ///
    /// To effectively check the path-oriented conditions of safety here, we need to deal with the fact
    /// that graphs have non-explicit choice nodes, for example:
    ///
    /// ```text
    ///                 s
    ///            &mut /\ &mut
    ///             .f /  \ .g
    ///              r1    r2
    /// ```
    ///
    /// The diverging edges `.f` and `.g` are not directly visible. In order to deal with this, we construct a
    /// _hyper graph_ on the fly as follows:
    ///
    /// 1. The root nodes are the singleton sets with all the root nodes for the given temporaries.
    /// 2. The hyper edges are grouped into those of the same edge kind. Hence, two `&mut` edges
    ///    like in the example above become one hyper edge. The successor state of the hyper edge
    ///    is the union of all the targets of the edges grouped together.
    ///
    /// If we walk this graph now from the root to the leaves, we can determine safety by directly comparing
    /// hyper edge siblings.
    fn check_borrow_safety(&mut self, exclusive_temps_vec: &[TempIndex]) {
        // Make a set out of the temporaries to check.
        let exclusive_temps = exclusive_temps_vec.iter().cloned().collect::<BTreeSet<_>>();

        // Check direct duplicates if needed.
        if exclusive_temps.len() != exclusive_temps_vec.len() {
            for (i, temp) in exclusive_temps_vec.iter().enumerate() {
                if self.ty(*temp).is_mutable_reference()
                    && exclusive_temps_vec[i + 1..].contains(temp)
                {
                    self.exclusive_access_direct_dup_error(*temp)
                }
            }
        }
        let filtered_leaves = self
            .state
            .leaves()
            .into_iter()
            .filter_map(|(l, mut ts)| {
                ts = ts.intersection(&exclusive_temps).cloned().collect();
                if !ts.is_empty() {
                    Some((l, ts))
                } else {
                    None
                }
            })
            .collect::<BTreeMap<_, _>>();
        // Initialize root hyper nodes
        let mut hyper_nodes: BTreeSet<BTreeSet<LifetimeLabel>> = BTreeSet::new();
        for filtered_leaf in filtered_leaves.keys() {
            for root in self.state.roots(filtered_leaf) {
                hyper_nodes.insert(iter::once(root).collect());
            }
        }
        let mut edges_reported: BTreeSet<BTreeSet<&BorrowEdge>> = BTreeSet::new();
        // Continue to process hyper nodes
        while let Some(hyper) = hyper_nodes.pop_first() {
            let hyper_edges = self.state.group_children_into_hyper_edges(&hyper);
            // Check 2-wise combinations of hyper edges for issues. This discovers cases where edges
            // conflict because of mutability.
            for mut perm in hyper_edges.iter().combinations(2) {
                let (kinds1, edges1) = perm.pop().unwrap();
                let (kinds2, edges2) = perm.pop().unwrap();
                if DEBUG {
                    debug!(
                        "{}[{}] vs {}[{}]",
                        kinds1
                            .iter()
                            .map(|k| k.display(self.target()).to_string())
                            .join("|"),
                        edges1
                            .iter()
                            .map(|e| e.display(self.target(), true).to_string())
                            .join(","),
                        kinds2
                            .iter()
                            .map(|k| k.display(self.target()).to_string())
                            .join("|"),
                        edges2
                            .iter()
                            .map(|e| e.display(self.target(), true).to_string())
                            .join(","),
                    );
                }
                if (BorrowEdgeKind::any_is_mut(kinds1) || BorrowEdgeKind::any_is_mut(kinds2))
                    && BorrowEdgeKind::any_could_overlap(kinds1, kinds2)
                {
                    for (e1, e2) in edges1.iter().cartesian_product(edges2.iter()) {
                        if e1 == e2 || !edges_reported.insert([*e1, *e2].into_iter().collect()) {
                            continue;
                        }
                        // If the diverging edges have common transitive children they result from
                        // joining of conditional branches and are allowed. See also discussion in the file
                        // comment.
                        // NOTE: we may do this more efficiently using a lazy algorithm similar as LCA graph
                        // algorithms as the first common children we find is enough. However, since we
                        // expect the graph of small size, this seems not be too important.
                        // CONJECTURE: it is sufficient here to just check for an intersection. If there is any
                        // common child, then for any later divergences when following the edges, we will do
                        // this check again.
                        if !self
                            .state
                            .transitive_children(&e1.target)
                            .is_disjoint(&self.state.transitive_children(&e2.target))
                        {
                            continue;
                        }
                        self.diverging_edge_error(hyper.first().unwrap(), e1, e2, &filtered_leaves)
                    }
                }
            }
            // Now go over each hyper edge and if they target a leaf node check for conditions
            for (_, edges) in hyper_edges {
                let mut mapped_temps = BTreeSet::new();
                let mut targets = BTreeSet::new();
                for edge in edges {
                    let target = edge.target;
                    targets.insert(target);
                    if edge.kind.is_mut() {
                        if let Some(ts) = filtered_leaves.get(&target) {
                            let mut inter = ts
                                .intersection(&exclusive_temps)
                                .cloned()
                                .collect::<BTreeSet<_>>();
                            if !inter.is_empty() {
                                if !self.state.is_leaf(&target) {
                                    // A mut leaf node must have exclusive access
                                    self.exclusive_access_borrow_error(&target, &inter)
                                }
                                mapped_temps.append(&mut inter)
                            }
                        }
                    }
                }
                if mapped_temps.len() > 1 {
                    // We cannot associate the same mut node with more than one local
                    self.exclusive_access_indirect_dup_error(&hyper, &mapped_temps)
                }
                hyper_nodes.insert(targets);
            }
        }
        // Temps containing mut refs alive after this program point and referring to nodes
        // in the exclusive set are not allowed in v1 borrow semantics unless they are used to
        // derive the exclusive nodes. Consider
        // `let r = &mut s; let r1 = r; let x = &mut r.f; *x; *r1`: this is not allowed in v1.
        // In contrast, `let r = &mut s; let x = &mut r.f; *x; *r` *is* allowed. The reason
        // is that `x` is derived from `r` but not (for the first example) from `r1`.
        let derived = self.state.derived_temps();
        for mut_alive_after in self.alive.after.keys().cloned().filter(|t| {
            self.ty(*t).is_mutable_reference()
                && !exclusive_temps.contains(t)
                && !derived.contains(t)
        }) {
            if let Some(label) = self.state.label_for_temp(mut_alive_after) {
                if let Some(conflict) = filtered_leaves.keys().find(|exclusive_label| {
                    self.state.is_mut(exclusive_label)
                        && self.state.is_ancestor(label, exclusive_label)
                }) {
                    self.exclusive_access_borrow_error(
                        conflict,
                        filtered_leaves.get(conflict).unwrap(),
                    )
                }
            }
        }
    }

    /// Reports an error about a diverging edge. See condition (b) in the file header documentation.
    fn diverging_edge_error<'a>(
        &self,
        label: &LifetimeLabel,
        mut edge: &'a BorrowEdge,
        mut other_edge: &'a BorrowEdge,
        leaves: &BTreeMap<LifetimeLabel, BTreeSet<TempIndex>>,
    ) {
        // Order edges for better error message: the later one in the text should be flagged as error.
        if edge.loc.cmp(&other_edge.loc) == Ordering::Less {
            (other_edge, edge) = (edge, other_edge)
        }
        let (temps, temp_str) = match leaves.get(label) {
            Some(temps) if !temps.is_empty() => (
                temps.clone(),
                format!(
                    "{} ",
                    self.target()
                        .get_local_name_for_error_message(*temps.iter().next().unwrap())
                ),
            ),
            _ => (BTreeSet::new(), "".to_string()),
        };
        let (action, attempt) = if edge.kind.is_mut() {
            ("mutably", "mutable")
        } else {
            ("immutably", "immutable")
        };
        let reason = if other_edge.kind.is_mut() {
            "mutable references exist"
        } else {
            "immutable references exist"
        };
        let mut info = self.borrow_info(label, |e| e != edge);
        info.push((self.cur_loc(), "requirement enforced here".to_string()));
        self.error_with_hints(
            &edge.loc,
            format!(
                "cannot {action} borrow {what}since {reason}",
                action = action,
                what = temp_str,
                reason = reason
            ),
            format!("{} borrow attempted here", attempt),
            info.into_iter()
                .chain(self.usage_info(&other_edge.target, |t| !temps.contains(t))),
        );
    }

    /// Reports an error about exclusive access requirement for borrows. See
    /// safety condition (c) in the file header documentation.
    fn exclusive_access_borrow_error(&self, label: &LifetimeLabel, temps: &BTreeSet<TempIndex>) {
        self.error_with_hints(
            self.cur_loc(),
            format!(
                "mutable reference in {} requires exclusive access but is borrowed",
                self.display_set(temps)
            ),
            "requirement enforced here",
            self.borrow_info(label, |_| true)
                .into_iter()
                .chain(self.usage_info(label, |t| !temps.contains(t))),
        )
    }

    /// Reports an error about exclusive access requirement for duplicate usage. See safety
    /// condition (d) in the file header documentation. This handles the case were the
    /// same node is used by multiple temps
    fn exclusive_access_indirect_dup_error(
        &self,
        labels: &BTreeSet<LifetimeLabel>,
        temps: &BTreeSet<TempIndex>,
    ) {
        debug_assert!(temps.len() > 1);
        let ts = temps.iter().take(2).collect_vec();
        self.error_with_hints(
            self.cur_loc(),
            format!(
                "same mutable reference in {} is also used in other {} in argument list",
                self.display(*ts[0]),
                self.display(*ts[1])
            ),
            "requirement enforced here",
            labels.iter().flat_map(|l| self.borrow_info(l, |_| true)),
        )
    }

    /// Reports an error about exclusive access requirement for duplicate usage. See safety
    /// condition (d) in the file header documentation. This handles the case were the
    /// same local is used multiple times.
    fn exclusive_access_direct_dup_error(&self, temp: TempIndex) {
        self.error_with_hints(
            self.cur_loc(),
            format!(
                "same mutable reference in {} is used again in argument list",
                self.display(temp),
            ),
            "requirement enforced here",
            iter::empty(),
        )
    }

    /// Reports an error together with hints
    fn error_with_hints(
        &self,
        loc: impl AsRef<Loc>,
        msg: impl AsRef<str>,
        primary: impl AsRef<str>,
        hints: impl Iterator<Item = (Loc, String)>,
    ) {
        if !self.parent.suppress_errors {
            self.global_env().diag_with_primary_and_labels(
                Severity::Error,
                loc.as_ref(),
                msg.as_ref(),
                primary.as_ref(),
                hints.collect(),
            )
        }
    }

    #[inline]
    fn global_env(&self) -> &GlobalEnv {
        self.target().global_env()
    }

    #[inline]
    fn target(&self) -> &FunctionTarget {
        self.parent.target
    }

    /// Produces borrow hints for the given node in the graph, for error messages.
    fn borrow_info(
        &self,
        label: &LifetimeLabel,
        filter: impl Fn(&BorrowEdge) -> bool,
    ) -> Vec<(Loc, String)> {
        let leaves = self.state.leaves();
        let primary_edges = self
            .state
            .children(label)
            .filter(|e| filter(e))
            .cloned()
            .collect::<BTreeSet<_>>();
        let mut secondary_edges = BTreeSet::new();
        for edge in primary_edges.iter() {
            // Only include the secondary edge if the primary target is not longer in use. This gives a user an
            // additional hint only if needed.
            if !leaves.contains_key(&edge.target) {
                secondary_edges.extend(self.state.children(&edge.target));
            }
        }
        primary_edges
            .into_iter()
            .map(|e| self.borrow_edge_info("previous ", &e))
            .chain(
                secondary_edges
                    .into_iter()
                    .map(|e| self.borrow_edge_info("used by ", e)),
            )
            .collect::<Vec<_>>()
    }

    fn borrow_edge_info(&self, prefix: &str, e: &BorrowEdge) -> (Loc, String) {
        use BorrowEdgeKind::*;
        let mut_prefix = if e.kind.is_mut() { "mutable " } else { "" };
        (
            e.loc.clone(),
            format!("{}{}{}", prefix, mut_prefix, match &e.kind {
                BorrowLocal(_) => "local borrow",
                BorrowGlobal(..) => "global borrow",
                BorrowField(..) => "field borrow",
                Call(..) => "call result",
                Freeze => "freeze",
            },),
        )
    }

    /// Produces usage information for temporaries involved in the current borrow graph.
    fn usage_info(
        &self,
        label: &LifetimeLabel,
        filter: impl Fn(&TempIndex) -> bool,
    ) -> Vec<(Loc, String)> {
        // Collect the candidates to display. These are temporaries which are alive _after_ this program point
        // and which are in the same path in the graph (i.e. or parent and child of each other).
        let mut cands = vec![];
        for (temp, leaf) in self.state.temp_to_label_map.iter() {
            if self.is_ref(*temp)
                && self.alive.after.contains_key(temp)
                && (self.state.is_ancestor(label, leaf) || self.state.is_ancestor(leaf, label))
                && filter(temp)
            {
                let mut done = false;
                for (ct, cl) in cands.iter_mut() {
                    if self.state.is_ancestor(cl, leaf) {
                        // This leaf is a better proof of the problem as it is derived from the other, replace
                        *ct = *temp;
                        *cl = *leaf;
                        done = true;
                        break;
                    } else if self.state.is_ancestor(leaf, cl) {
                        // The existing one is better than the new one
                        done = true;
                        break;
                    }
                }
                if !done {
                    cands.push((*temp, *leaf))
                }
            }
        }
        // Now compute display
        let mut infos = vec![];
        for (temp, _) in cands {
            if let Some(info) = self.alive.after.get(&temp) {
                for loc in info.usage_locations().into_iter() {
                    infos.push((
                        loc,
                        format!(
                            "conflicting reference{} used here",
                            self.display_name_or_empty(" ", temp)
                        ),
                    ))
                }
            }
        }
        infos
    }
}

// -------------------------------------------------------------------------------------------------
// Program Steps

impl LifetimeAnalysisStep<'_, '_> {
    /// Process an assign instruction. This checks whether the source is currently borrowed and
    /// rejects a move if so.
    fn assign(&mut self, dest: TempIndex, src: TempIndex, kind: AssignKind) {
        // Check validness
        let mode = if kind == AssignKind::Move {
            ReadMode::Move
        } else {
            ReadMode::Copy
        };
        if self.is_ref(src) {
            match kind {
                AssignKind::Move => self.state.move_ref(dest, src),
                AssignKind::Copy => self.state.copy_ref(dest, src),
                AssignKind::Inferred => {
                    if self.state.label_for_temp_with_children(src).is_none()
                        && !self.alive.after.contains_key(&src)
                    {
                        self.state.move_ref(dest, src)
                    } else {
                        self.state.copy_ref(dest, src)
                    }
                },
                AssignKind::Store => panic!("unexpected assign kind"),
            }
        } else {
            self.check_read_local(src, mode);
            self.release_refs_not_alive_after();
            self.check_write_local(dest);
        }
    }

    /// Process a borrow local instruction.
    fn borrow_local(&mut self, dest: TempIndex, src: TempIndex) {
        let label = self.state.make_temp(src, self.code_offset, 0, true);
        let child = self.state.replace_ref(dest, self.code_offset, 1);
        let loc = self.cur_loc();
        let is_mut = self.ty(dest).is_mutable_reference();
        self.state.add_edge(
            label,
            BorrowEdge::new(BorrowEdgeKind::BorrowLocal(is_mut), loc, child),
        );
    }

    /// Process a borrow global instruction.
    fn borrow_global(&mut self, struct_: QualifiedInstId<StructId>, dest: TempIndex) {
        let label = self.state.make_global(struct_.clone(), self.code_offset, 0);
        let child = self.state.replace_ref(dest, self.code_offset, 1);
        let loc = self.cur_loc();
        let is_mut = self.ty(dest).is_mutable_reference();
        self.state.add_edge(
            label,
            BorrowEdge::new(
                BorrowEdgeKind::BorrowGlobal(is_mut, self.code_offset),
                loc,
                child,
            ),
        );
    }

    /// Process a borrow field instruction.
    fn borrow_field(
        &mut self,
        struct_: QualifiedInstId<StructId>,
        variant: Option<Symbol>,
        field_offs: &usize,
        dest: TempIndex,
        src: TempIndex,
    ) {
        let label = self.state.make_temp(src, self.code_offset, 0, false);
        let child = self.state.replace_ref(dest, self.code_offset, 1);
        self.state.mark_derived_from(child, src);
        let loc = self.cur_loc();
        let struct_env = self.global_env().get_struct(struct_.to_qualified_id());
        let field_id = struct_env
            .get_field_by_offset_optional_variant(variant, *field_offs)
            .get_id();
        let is_mut = self.ty(dest).is_mutable_reference();
        self.state.add_edge(
            label,
            BorrowEdge::new(BorrowEdgeKind::BorrowField(is_mut, field_id), loc, child),
        );
        // In v1, borrow safety is enforced even when `dest` is not used after this
        // program point, AND when `label` has an outgoing call edge. However, our
        // `check_borrow_safety` implementation will (correctly) not trigger an error since
        // the borrowed reference is never used. To simulate v1 behavior, we check for those
        // conditions and produce an error ad-hoc.
        if is_mut
            && !self.alive.after.contains_key(&dest)
            && self
                .state
                .children(&label)
                .any(|e| matches!(&e.kind, BorrowEdgeKind::Call(..)))
        {
            self.error_with_hints(
                self.cur_loc(),
                format!(
                    "cannot mutably borrow field of {} since references derived from a call exist",
                    self.display(src),
                ),
                "mutable borrow attempted here",
                self.borrow_info(&label, |_| true)
                    .into_iter()
                    .chain(self.usage_info(&label, |_| true)),
            )
        }
    }

    /// Process a function call. For now, we implement standard Move semantics, where
    /// 1) every output immutable reference is a child of all input references;
    /// 2) every output mutable reference is a child of all input mutable references,
    /// because mutable references cannot be derived from immutable references.
    /// Here would be the point where to
    /// evaluate lifetime modifiers in future language versions.
    fn call_operation(&mut self, oper: Operation, dests: &[TempIndex], srcs: &[TempIndex]) {
        // If this a function call, check acquires conditions for global borrows.
        if let Operation::Function(mid, fid, inst) = &oper {
            self.check_global_access(mid.qualified_inst(*fid, inst.clone()))
        }
        // Check validness of arguments
        for src in srcs {
            self.check_read_local(*src, ReadMode::Argument);
        }
        // Now draw edges
        // 1) from all reference sources to all immutable reference destinations.
        // 2) from all mutable reference sources to all mutable reference destinations.
        let dest_labels = dests
            .iter()
            .filter(|d| self.ty(**d).is_reference())
            .collect::<Vec<_>>()
            .into_iter()
            .enumerate()
            .map(|(i, t)| (*t, self.state.replace_ref(*t, self.code_offset, i as u8)))
            .collect::<BTreeMap<_, _>>();
        let src_qualifier_offset = dest_labels.len();
        let loc = self.cur_loc();
        for dest in dests {
            let dest_ty = self.ty(*dest).clone();
            if dest_ty.is_reference() {
                for (i, src) in srcs.iter().enumerate() {
                    let src_ty = self.ty(*src);
                    if src_ty.is_reference() {
                        // dest does not rely on src if
                        // dest is a mutable reference while src is not
                        if dest_ty.is_mutable_reference() && !src_ty.is_mutable_reference() {
                            continue;
                        }
                        let label = self.state.make_temp(
                            *src,
                            self.code_offset,
                            (src_qualifier_offset + i) as u8,
                            false,
                        );
                        let child = &dest_labels[dest];
                        self.state.mark_derived_from(*child, *src);
                        self.state.add_edge(
                            label,
                            BorrowEdge::new(
                                BorrowEdgeKind::Call(
                                    dest_ty.is_mutable_reference(),
                                    oper.clone(),
                                    self.code_offset,
                                ),
                                loc.clone(),
                                *child,
                            ),
                        )
                    }
                }
            }
        }
        // Check whether destinations can be written.
        self.release_refs_not_alive_after();
        for dest in dests {
            self.check_write_local(*dest)
        }
    }

    /// Checks whether a function potentially accesses a global resource which is
    /// currently borrowed.
    fn check_global_access(&mut self, fun_id: QualifiedInstId<FunId>) {
        let fun = self.global_env().get_function(fun_id.to_qualified_id());
        let specifiers = fun.get_access_specifiers().unwrap_or(&[]);

        for (global, label) in &self.state.global_to_label_map {
            let is_mut = self.state.children(label).any(|e| e.kind.is_mut());
            // We are only checking positive specifiers, as negatives say nothing
            // about what is accessed.
            for spec in specifiers.iter().filter(|s| !s.negated) {
                if spec
                    .resource
                    .1
                    .matches(self.global_env(), &fun_id.inst, global)
                    // For mut global borrows, no access is allowed at all. For
                    // non-mut, write access is not allowed.
                    // TODO: needs to be updated to use acquired resources instead
                    //   access specifiers (see v3 code).
                    && (is_mut || spec.kind.subsumes(&AccessSpecifierKind::Writes))
                {
                    self.error_with_hints(
                        self.cur_loc(),
                        format!(
                            "function {} global `{}` which is currently {}borrowed",
                            spec.kind,
                            self.global_env().display(global),
                            if is_mut { "mutably " } else { "" }
                        ),
                        "function called here",
                        self.borrow_info(label, |_| true)
                            .into_iter()
                            .chain(iter::once((
                                spec.loc.clone(),
                                "access declared here".to_owned(),
                            ))),
                    )
                }
            }
        }
    }

    /// Process a FreezeRef instruction.
    ///
    /// Freezes have specific conditions in the v1 borrow semantics as also implemented
    /// by the bytecode verifier which require some ad-hoc treatment. (In a new borrow
    /// semantics we may want to investigate to relax them, because they appear unnecessary
    /// strict).
    ///
    /// When a reference is frozen, there must not exist any other mutable reference
    /// pointing to the same location. It is, however, ok if the currently
    /// frozen reference is used later again. This seems to be over-restrictive
    /// since it shouldn't matter whether copies of the mutable reference exist
    /// as long as they aren't passed at the same time as an argument to a function,
    /// i.e. as long as they do not create aliasing.
    ///
    /// The above condition can be violated in two situations:
    ///
    /// a. The frozen reference has siblings which also mutably borrow the same parent.
    /// b. There exists a mutable reference, alive after this program point, which
    ///    borrows the same location but is not derived from the reference we are
    ///    freezing.
    fn freeze_ref(
        &mut self,
        code_offset: CodeOffset,
        explicit: bool,
        dest: TempIndex,
        src: TempIndex,
    ) {
        let label = *self.state.label_for_temp(src).expect("label for reference");
        let target = self.state.replace_ref(dest, code_offset, 0);
        self.state.add_edge(label, BorrowEdge {
            kind: BorrowEdgeKind::Freeze,
            loc: self.cur_loc(),
            target,
        });
        if let Some(label) = self.state.label_for_temp(src) {
            // Handle case (a): search for any siblings which mutably borrow the same
            // parent.
            let qualifier = if explicit { "" } else { "implicitly " };
            for (parent, edge) in self.state.parent_edges(label) {
                for sibling_edge in self.state.children(&parent) {
                    if &sibling_edge.target == label
                        || sibling_edge.kind == edge.kind
                            && matches!(edge.kind, BorrowEdgeKind::Call(..))
                        || !sibling_edge.kind.could_overlap(&edge.kind)
                    {
                        // The sibling edge is harmless if
                        // (a) it is not actually a sibling but leads to the same target
                        // (b) the kind is the same and stems from a call This happens e.g. for a
                        //     call which returns multiple references, as in `(r1, r2) = foo(r)`,
                        //     then even though we have different edges with different targets,
                        //     they stem from the same borrow.
                        // (c) if the sibling has no overlap, as in `&mut r.f1` and `&mut r.f2`.
                        continue;
                    }
                    if self.state.is_mut_path(sibling_edge) {
                        self.error_with_hints(
                            self.cur_loc(),
                            format!(
                                "cannot {}freeze {}  since multiple mutable references exist",
                                qualifier,
                                self.display(src)
                            ),
                            format!("{}frozen here", qualifier),
                            vec![
                                self.borrow_edge_info("originating ", edge),
                                self.borrow_edge_info("conflicting ", sibling_edge),
                            ]
                            .into_iter(),
                        )
                    }
                }
            }
            // Handle case (b): check whether there is any alive mutable reference
            // which overlaps with the frozen reference.
            let derived = self.state.derived_temps();
            for (temp, other_label) in self.state.temp_to_label_map.iter() {
                if temp == &src || !self.ty(*temp).is_mutable_reference() || derived.contains(temp)
                {
                    continue;
                }
                // Apart from the same memory location, locations mutably borrowed from label also need to be included
                if other_label == label
                    || self.state.transitive_children(label).contains(other_label)
                {
                    // Compute all visible usages at leaves to show the conflict.
                    // It is not enough to just show the usage of `temp`, because the
                    // actual usage might be something derived from it, and `temp`
                    // is no longer used.
                    let leaves = self.state.leaves();
                    let mut show: BTreeSet<(bool, Loc)> = BTreeSet::new();
                    let mut todo = vec![*other_label];
                    while let Some(l) = todo.pop() {
                        if let Some(temps) = leaves.get(&l) {
                            show.extend(
                                temps
                                    .iter()
                                    .map(|t| {
                                        self.alive
                                            .after
                                            .get(t)
                                            .map(|i| {
                                                i.usage_locations()
                                                    .iter()
                                                    .map(|l| (true, l.clone()))
                                                    .collect::<BTreeSet<_>>()
                                            })
                                            .unwrap_or_default()
                                    })
                                    .concat(),
                            )
                        } else {
                            for e in self.state.children(&l) {
                                show.insert((false, e.loc.clone()));
                                todo.push(e.target)
                            }
                        }
                    }
                    self.error_with_hints(
                        self.cur_loc(),
                        format!(
                            "cannot {}freeze {} since other mutable usages for this reference exist",
                            qualifier,
                            self.display(src),
                        ),
                        format!("{}frozen here", qualifier),
                        show.into_iter().map(|(is_leaf, loc)| {
                            (
                                loc,
                                if is_leaf { "used here" } else { "derived here" }.to_string(),
                            )
                        }),
                    )
                }
            }
        }
    }

    /// Process a MoveFrom instruction.
    fn move_from(&mut self, dest: TempIndex, resource: &QualifiedInstId<StructId>, src: TempIndex) {
        self.check_read_local(src, ReadMode::Argument);
        self.release_refs_not_alive_after();
        self.check_write_local(dest);
        if let Some(label) = self.state.label_for_global_with_children(resource) {
            self.error_with_hints(
                self.cur_loc(),
                format!(
                    "cannot extract resource `{}` which is still borrowed",
                    self.global_env().display(resource)
                ),
                "extracted here",
                self.borrow_info(label, |_| true).into_iter(),
            )
        }
    }

    /// Process a return instruction.
    fn return_(&mut self, srcs: &[TempIndex]) {
        for src in srcs {
            if self.ty(*src).is_reference() {
                // Need to check whether this reference is derived from a local which is not a
                // a parameter
                if let Some(label) = self.state.label_for_temp(*src) {
                    for root in self.state.roots(label) {
                        for location in self.state.node(&root).locations.iter() {
                            match location {
                                MemoryLocation::Global(resource) => self.error_with_hints(
                                    self.cur_loc(),
                                    format!(
                                        "cannot return a reference derived from global `{}`",
                                        self.global_env().display(resource)
                                    ),
                                    "returned here",
                                    self.borrow_info(&root, |_| true).into_iter(),
                                ),
                                MemoryLocation::Local(local) => {
                                    if *local >= self.target().get_parameter_count() {
                                        self.error_with_hints(
                                            self.cur_loc(),
                                            format!(
                                                "cannot return a reference derived from {} since it is not a parameter",
                                                self.display(*local)
                                            ),
                                            "returned here",
                                            self.borrow_info(&root, |_| true).into_iter(),
                                        )
                                    }
                                },
                                MemoryLocation::External | MemoryLocation::Derived => {},
                            }
                        }
                    }
                }
            }
        }
    }

    /// Process a ReadRef instruction.
    fn read_ref(&mut self, dest: TempIndex, src: TempIndex) {
        debug_assert!(self.is_ref(src));
        self.release_refs_not_alive_after();
        self.check_write_local(dest);
        self.check_read_local(src, ReadMode::Argument);
    }

    /// Process a WriteRef instruction.
    fn write_ref(&mut self, dest: TempIndex, src: TempIndex) {
        self.check_read_local(src, ReadMode::Argument);
        if let Some(label) = self.state.label_for_temp_with_children(dest) {
            self.error_with_hints(
                self.cur_loc(),
                format!(
                    "cannot write to reference in {} which is still borrowed",
                    self.display(dest)
                ),
                "written here",
                self.borrow_info(label, |_| true).into_iter(),
            )
        }
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

        // Preprocessing: check borrow safety of the currently active borrow graph for
        // selected instructions.
        #[allow(clippy::single_match)]
        match instr {
            // Call operations which can take references
            Call(_, _, oper, srcs, ..) => match oper {
                Operation::ReadRef
                | Operation::WriteRef
                | Operation::Function(..)
                | Operation::Eq
                | Operation::Neq => {
                    let exclusive_refs = srcs
                        .iter()
                        .filter(|t| step.is_ref(**t))
                        .cloned()
                        .collect_vec();
                    step.check_borrow_safety(&exclusive_refs)
                },
                _ => {},
            },
            Ret(_, srcs) => {
                let exclusive_refs = srcs
                    .iter()
                    .filter(|t| step.is_ref(**t))
                    .cloned()
                    .collect_vec();
                step.check_borrow_safety(&exclusive_refs)
            },
            Assign(_, _, src, _) if step.ty(*src).is_mutable_reference() => {
                step.check_borrow_safety(&[*src])
            },
            _ => {},
        }

        // Process the instruction
        match instr {
            Assign(_, dest, src, kind) => {
                step.assign(*dest, *src, *kind);
            },
            Ret(_, srcs) => step.return_(srcs),
            Branch(_, _, _, src) => {
                step.check_read_local(*src, ReadMode::BranchCondition);
            },
            Call(_, dests, oper, srcs, _) => {
                use Operation::*;
                match oper {
                    BorrowLoc => {
                        step.borrow_local(dests[0], srcs[0]);
                    },
                    BorrowGlobal(mid, sid, inst) => {
                        step.borrow_global(mid.qualified_inst(*sid, inst.clone()), dests[0]);
                    },
                    BorrowField(mid, sid, inst, field_offs) => {
                        let (dest, src) = (dests[0], srcs[0]);
                        step.borrow_field(
                            mid.qualified_inst(*sid, inst.clone()),
                            None,
                            field_offs,
                            dest,
                            src,
                        );
                    },
                    BorrowVariantField(mid, sid, variants, inst, field_offs) => {
                        let (dest, src) = (dests[0], srcs[0]);

                        step.borrow_field(
                            mid.qualified_inst(*sid, inst.clone()),
                            // Use one representative variant
                            Some(variants[0]),
                            field_offs,
                            dest,
                            src,
                        );
                    },
                    ReadRef => step.read_ref(dests[0], srcs[0]),
                    WriteRef => step.write_ref(srcs[0], srcs[1]),
                    FreezeRef(explicit) => {
                        step.freeze_ref(code_offset, *explicit, dests[0], srcs[0])
                    },
                    MoveFrom(mid, sid, inst) => {
                        step.move_from(dests[0], &mid.qualified_inst(*sid, inst.clone()), srcs[0])
                    },
                    _ => step.call_operation(oper.clone(), dests, srcs),
                }
            },
            _ => {},
        }

        // Some instructions may not have released inputs, do so now. The operation
        // is idempotent.
        step.release_refs_not_alive_after()
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
        let suppress_errors = !fun_env
            .module_env
            .env
            .get_extension::<Options>()
            .unwrap_or_default()
            .experiment_on(Experiment::REPORT_ERRORS_REF_SAFETY);
        let analyzer = LifeTimeAnalysis {
            target: &target,
            live_var_annotation,
            suppress_errors,
        };
        let code = target.get_bytecode();
        let cfg = StacklessControlFlowGraph::new_forward(code);
        let mut state = LifetimeState::default();
        let mut label_counter: u32 = 0;
        for (i, Parameter(_, ty, loc)) in fun_env.get_parameters().into_iter().enumerate() {
            if ty.is_reference() {
                let label = LifetimeLabel::new_from_counter(label_counter);
                label_counter += 1;
                state.new_node(label, MemoryLocation::External);
                let target = state.make_temp_from_label_fun(
                    i,
                    || LifetimeLabel::new_from_counter(label_counter),
                    false,
                );
                label_counter += 1;
                state.add_edge(label, BorrowEdge {
                    kind: BorrowEdgeKind::BorrowLocal(ty.is_mutable_reference()),
                    loc,
                    target,
                })
            }
        }
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
        self.label_for_temp_with_children(temp)
            .map(|label| ReferenceKind::from_is_mut(self.is_mut(label)))
    }

    fn display(&self, target: &FunctionTarget) -> Option<String> {
        Some(self.display(target).to_string())
    }
}

// ===============================================================================================
// Display

struct BorrowEdgeDisplay<'a>(&'a FunctionTarget<'a>, &'a BorrowEdge, bool);
impl Display for BorrowEdgeDisplay<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let edge = &self.1;
        write!(f, "{}", edge.kind.display(self.0))?;
        let display_child = self.2;
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

struct BorrowEdgeKindDisplay<'a>(&'a FunctionTarget<'a>, &'a BorrowEdgeKind);
impl Display for BorrowEdgeKindDisplay<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        use BorrowEdgeKind::*;
        let mut_str = if self.1.is_mut() { "mut" } else { "imm" };
        match &self.1 {
            BorrowLocal(_) => write!(f, "borrow_{}", mut_str),
            BorrowGlobal(_, offs) => write!(f, "borrow_global_{}@{}", mut_str, offs),
            BorrowField(_, field_id) => write!(
                f,
                "borrow_{}.{}",
                mut_str,
                field_id.symbol().display(self.0.symbol_pool()),
            ),
            Call(_, _, offs) => write!(f, "call_{}@{}", mut_str, offs),
            Freeze => write!(f, "freeze"),
        }
    }
}

impl BorrowEdgeKind {
    fn display<'a>(&'a self, target: &'a FunctionTarget) -> BorrowEdgeKindDisplay<'a> {
        BorrowEdgeKindDisplay(target, self)
    }
}

impl Display for LifetimeLabel {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "@{:X}", self.0)
    }
}

struct MemoryLocationDisplay<'a>(&'a FunctionTarget<'a>, &'a MemoryLocation);
impl Display for MemoryLocationDisplay<'_> {
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
            External => write!(f, "external"),
            Derived => write!(f, "derived"),
        }
    }
}
impl MemoryLocation {
    fn display<'a>(&'a self, fun: &'a FunctionTarget) -> MemoryLocationDisplay<'a> {
        MemoryLocationDisplay(fun, self)
    }
}

struct LifetimeNodeDisplay<'a>(&'a FunctionTarget<'a>, &'a LifetimeNode);
impl Display for LifetimeNodeDisplay<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}[",
            self.1.locations.iter().map(|l| l.display(self.0)).join(",")
        )?;
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

struct LifetimeStateDisplay<'a>(&'a FunctionTarget<'a>, &'a LifetimeState);
impl Display for LifetimeStateDisplay<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let LifetimeState {
            graph,
            temp_to_label_map,
            global_to_label_map,
            derived_from,
        } = &self.1;
        let pool = self.0.global_env().symbol_pool();
        writeln!(
            f,
            "graph: {}",
            graph.to_string(|k| k.to_string(), |v| v.display(self.0).to_string())
        )?;
        writeln!(
            f,
            "locals: {{{}}}",
            temp_to_label_map
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
            "globals: {{{}}}",
            global_to_label_map
                .iter()
                .map(|(str, label)| format!("{}={}", self.0.global_env().display(str), label))
                .join(",")
        )?;
        if !derived_from.is_empty() {
            writeln!(
                f,
                "derived-from: {}",
                derived_from
                    .iter()
                    .map(|(l, ts)| {
                        format!(
                            "{}={}",
                            l,
                            ts.iter()
                                .map(|t| self.0.get_local_raw_name(*t).display(pool).to_string())
                                .join(",")
                        )
                    })
                    .join(",")
            )?
        }
        Ok(())
    }
}

impl LifetimeState {
    fn display<'a>(&'a self, fun: &'a FunctionTarget) -> LifetimeStateDisplay<'a> {
        LifetimeStateDisplay(fun, self)
    }
}
