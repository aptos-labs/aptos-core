// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::prep::{
    function::FunctionDecl,
    ident::FunctionIdent,
    model::{ability_set_candidates, Model},
    typing::{
        ComplexType, TypeBase, TypeItem, TypeMode, TypeRef, TypeSubstitution, TypeTag,
        TypeUnification,
    },
};
use itertools::Itertools;
use log::trace;
use move_core_types::ability::AbilitySet;
use petgraph::{
    algo::is_cyclic_directed,
    stable_graph::{NodeIndex, StableGraph},
    Direction,
};
use std::{
    collections::{BTreeMap, BTreeSet},
    fmt::Display,
    mem,
    time::{Duration, Instant},
};

/// Datatype node
#[derive(Debug, Clone, Ord, PartialOrd, Eq, PartialEq)]
pub enum DatatypeItem {
    Base(ComplexType),
    ImmRef(ComplexType),
    MutRef(ComplexType),
}

impl DatatypeItem {
    fn from_type_item(ty: &TypeItem) -> Option<Self> {
        let converted = match ty {
            TypeItem::Base(t) => match TypeMode::convert(t) {
                TypeMode::Simple(_) => return None,
                TypeMode::Complex(complex_ty) => Self::Base(complex_ty),
            },
            TypeItem::ImmRef(t) => match TypeMode::convert(t) {
                TypeMode::Simple(_) => return None,
                TypeMode::Complex(complex_ty) => Self::ImmRef(complex_ty),
            },
            TypeItem::MutRef(t) => match TypeMode::convert(t) {
                TypeMode::Simple(_) => return None,
                TypeMode::Complex(complex_ty) => Self::MutRef(complex_ty),
            },
        };
        Some(converted)
    }

    fn as_type_item(&self) -> TypeItem {
        match self {
            Self::Base(t) => TypeItem::Base(t.revert()),
            Self::ImmRef(t) => TypeItem::ImmRef(t.revert()),
            Self::MutRef(t) => TypeItem::MutRef(t.revert()),
        }
    }
}

/// Function instantiation node
#[derive(Debug, Clone, Ord, PartialOrd, Eq, PartialEq)]
pub struct FunctionInst {
    pub ident: FunctionIdent,
    pub type_args: Vec<TypeBase>,
}

impl Display for FunctionInst {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.type_args.is_empty() {
            write!(f, "{}", self.ident)
        } else {
            let inst = self.type_args.iter().join(", ");
            write!(f, "{}<{inst}>", self.ident)
        }
    }
}

/// A node in the flow graph
#[derive(Debug, Clone, Ord, PartialOrd, Eq, PartialEq)]
pub enum FlowGraphNode {
    Function(FunctionInst),
    Datatype(DatatypeItem),
}

/// An edge in the flow graph
#[derive(Debug, Clone, Ord, PartialOrd, Eq, PartialEq)]
pub enum FlowGraphEdge {
    Use(usize),
    Def(usize),
    Copy,
    Deref,
    Freeze,
    ImmBorrow,
    MutBorrow,
    VectorToElement,
    ElementToVector,
}

/// Builder for flow graphs
const MAX_DERIVED_GRAPHS_PER_PROCESS: usize = 4096;
const MAX_EXTERNAL_PROVIDER_MATCHES_PER_DATATYPE: usize = 64;

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
enum ProcessLimit {
    GraphBudget,
    TimeBudget,
}

impl ProcessLimit {
    fn as_str(self) -> &'static str {
        match self {
            Self::GraphBudget => "graph_budget",
            Self::TimeBudget => "time_budget",
        }
    }
}

pub struct GraphBuilder<'a> {
    // registries
    model: &'a Model,
    // how long the call trace can be
    max_trace_depth: usize,
    // how many times a function can be called in a trace
    max_call_repetition: usize,
    // graph exploration state
    trace: Vec<(FunctionInst, Vec<DatatypeItem>)>,
    // soft cap to keep one pathological function from monopolizing script generation
    remaining_graph_budget: usize,
    max_process_time: Option<Duration>,
    process_deadline: Option<Instant>,
    process_limit_hit: Option<ProcessLimit>,
}

impl<'a> GraphBuilder<'a> {
    /// Initialize the analyzer
    pub fn new(
        model: &'a Model,
        max_trace_depth: usize,
        max_call_repetition: usize,
        max_process_time: Option<Duration>,
    ) -> Self {
        Self {
            model,
            max_trace_depth,
            max_call_repetition,
            trace: vec![],
            remaining_graph_budget: 0,
            max_process_time,
            process_deadline: None,
            process_limit_hit: None,
        }
    }

    fn reset_process_budget(&mut self) {
        self.remaining_graph_budget = MAX_DERIVED_GRAPHS_PER_PROCESS;
        self.process_deadline = self
            .max_process_time
            .map(|duration| Instant::now() + duration);
        self.process_limit_hit = None;
    }

    fn budget_exhausted(&mut self) -> bool {
        if self.remaining_graph_budget == 0 {
            self.process_limit_hit
                .get_or_insert(ProcessLimit::GraphBudget);
            true
        } else {
            false
        }
    }

    fn deadline_exceeded(&mut self) -> bool {
        let exceeded = self
            .process_deadline
            .is_some_and(|deadline| Instant::now() >= deadline);
        if exceeded {
            self.process_limit_hit
                .get_or_insert(ProcessLimit::TimeBudget);
        }
        exceeded
    }

    fn exploration_stopped(&mut self) -> bool {
        self.deadline_exceeded() || self.budget_exhausted()
    }

    pub fn process_limit_hit(&self) -> Option<&'static str> {
        self.process_limit_hit.map(ProcessLimit::as_str)
    }

    fn extend_with_budget(&mut self, out: &mut Vec<FlowGraph>, incoming: Vec<FlowGraph>) {
        for graph in incoming {
            if self.exploration_stopped() {
                break;
            }
            self.remaining_graph_budget -= 1;
            out.push(graph);
        }
    }

    /// Analyze a function declaration and build flow graphs
    pub fn process(&mut self, decl: &FunctionDecl, type_args: &[TypeBase]) -> Vec<FlowGraph> {
        self.reset_process_budget();

        // construct the generics map
        let mut generics = BTreeMap::new();
        for param in type_args {
            match param {
                TypeBase::Param { index, abilities } => {
                    let exists = generics.insert(*index, *abilities);
                    assert!(exists.is_none());
                },
                _ => panic!("expected type parameter only"),
            }
        }

        // initialize the base graph and start analysis
        let base = FlowGraph::new(generics);
        let derived = self.add_call(base, decl, type_args, None);

        // sanity check
        assert!(self.trace.is_empty());

        // return all derived graphs
        derived
    }

    /// Add a call to a specific function instantiation into the base graph
    fn add_call(
        &mut self,
        mut base: FlowGraph,
        decl: &FunctionDecl,
        type_args: &[TypeBase],
        tail_node: Option<(NodeIndex, usize)>,
    ) -> Vec<FlowGraph> {
        if self.exploration_stopped() {
            return vec![];
        }
        trace!(
            "{}{}<{}>",
            "  ".repeat(self.trace.len()),
            decl.ident,
            type_args.iter().map(|t| t.to_string()).join(", ")
        );

        // shortcut if we have reached max trace depth
        if self.trace.len() >= self.max_trace_depth {
            return vec![];
        }

        // check if we have seen this instantiation enough times
        let func_inst = FunctionInst {
            ident: decl.ident.clone(),
            type_args: type_args.to_vec(),
        };
        if self.trace.iter().filter(|(f, _)| f == &func_inst).count() >= self.max_call_repetition {
            return vec![];
        }

        // now add this instantiation (and a new stack) to the trace and a node to the graph
        self.trace.push((func_inst.clone(), vec![]));
        let call_node = base
            .graph
            .add_node(FlowGraphNode::Function(func_inst.clone()));

        // add the tail node if requested
        if let Some((tail_node, ret_idx)) = tail_node {
            base.graph
                .add_edge(call_node, tail_node, FlowGraphEdge::Def(ret_idx));
        }

        // instantiate the function parameters
        let params_inst = decl
            .parameters
            .iter()
            .map(|t| {
                self.model
                    .datatype_registry
                    .instantiate_type_ref(t, type_args)
            })
            .collect_vec();

        // analyze the parameters
        let mut worklist = vec![];
        for (idx, ty) in params_inst.iter().enumerate() {
            let dt = match DatatypeItem::from_type_item(ty) {
                Some(item) => item,
                None => continue,
            };
            worklist.push((idx, dt));
        }

        let mut candidates = vec![base];
        for (idx, dt) in worklist {
            if self.exploration_stopped() {
                break;
            }
            for graph in mem::take(&mut candidates) {
                if self.exploration_stopped() {
                    break;
                }
                let results = self.add_arg(graph, call_node, idx, dt.clone());
                self.extend_with_budget(&mut candidates, results);
            }
        }

        // pop the trace and run some sanity check
        let (last_inst, stack) = self.trace.pop().expect("item in trace");
        assert_eq!(last_inst, func_inst);
        assert!(stack.is_empty());

        // return all candidates
        candidates
    }

    /// Add an argument node to the graph
    fn add_arg(
        &mut self,
        mut base: FlowGraph,
        call_node: NodeIndex,
        arg_index: usize,
        arg_type: DatatypeItem,
    ) -> Vec<FlowGraph> {
        // register a new node for this argument
        let arg_node = base.graph.add_node(FlowGraphNode::Datatype(arg_type));

        // add an edge from the argument node to the call
        base.graph
            .add_edge(arg_node, call_node, FlowGraphEdge::Use(arg_index));

        // construct the exploration plan based on the item type
        let candidates = self.plan_for_datatype(base, arg_node);
        let (_, stack) = self.trace.last().expect("item in trace");
        assert!(stack.is_empty());

        // done
        candidates
    }

    /// Plan for ways that a datatype can be provided
    fn plan_for_datatype(&mut self, base: FlowGraph, dt_node: NodeIndex) -> Vec<FlowGraph> {
        if self.exploration_stopped() {
            return vec![];
        }
        // lookup the datatype item
        let dt_type = match base.graph.node_weight(dt_node).unwrap() {
            FlowGraphNode::Datatype(t) => t.clone(),
            _ => panic!("expected datatype node"),
        };

        trace!(
            "{}+ {}",
            "  ".repeat(self.trace.len()),
            dt_type.as_type_item()
        );

        // check if we have already planned for this datatype in the stack
        let (_, stack) = self.trace.last_mut().expect("item in trace");
        if stack.contains(&dt_type) {
            return vec![];
        }
        stack.push(dt_type.clone());

        // initialize the plan
        let mut plan = vec![];

        // solve for providers first
        let provider_candidates = self.probe_datatype_providers(&base, dt_node, &dt_type);
        self.extend_with_budget(&mut plan, provider_candidates);

        // construct the exploration plan based on the item type
        match &dt_type {
            DatatypeItem::Base(ty) => {
                let ty_base = ty.revert();

                // batch: deref
                if ty_base.abilities().has_copy() {
                    // batch: deref imm ref
                    let mut new_graph = base.clone();
                    let src_type = DatatypeItem::ImmRef(ty.clone());
                    let src_node = new_graph.graph.add_node(FlowGraphNode::Datatype(src_type));
                    new_graph
                        .graph
                        .add_edge(src_node, dt_node, FlowGraphEdge::Deref);
                    let recursive = self.plan_for_datatype(new_graph, src_node);
                    self.extend_with_budget(&mut plan, recursive);

                    // batch: deref mut ref
                    let mut new_graph = base.clone();
                    let src_type = DatatypeItem::MutRef(ty.clone());
                    let src_node = new_graph.graph.add_node(FlowGraphNode::Datatype(src_type));
                    new_graph
                        .graph
                        .add_edge(src_node, dt_node, FlowGraphEdge::Deref);
                    let recursive = self.plan_for_datatype(new_graph, src_node);
                    self.extend_with_budget(&mut plan, recursive);
                }

                // case analysis based on the complex type
                match ty {
                    ComplexType::Datatype { .. } | ComplexType::Param { .. } => {
                        let mut new_graph = base.clone();
                        let src_type = DatatypeItem::Base(ComplexType::Vector {
                            element: Box::new(ty.clone()),
                        });
                        let src_node = new_graph.graph.add_node(FlowGraphNode::Datatype(src_type));
                        new_graph
                            .graph
                            .add_edge(src_node, dt_node, FlowGraphEdge::VectorToElement);
                        let recursive = self.plan_for_datatype(new_graph, src_node);
                        self.extend_with_budget(&mut plan, recursive);
                    },
                    ComplexType::Vector { element } => {
                        let mut new_graph = base.clone();
                        let src_type = DatatypeItem::Base(element.as_ref().clone());
                        let src_node = new_graph.graph.add_node(FlowGraphNode::Datatype(src_type));
                        new_graph
                            .graph
                            .add_edge(src_node, dt_node, FlowGraphEdge::ElementToVector);
                        let recursive = self.plan_for_datatype(new_graph, src_node);
                        self.extend_with_budget(&mut plan, recursive);
                    },
                }
            },
            DatatypeItem::ImmRef(inner) => {
                // batch: freeze
                let mut new_graph = base.clone();
                let src_type = DatatypeItem::MutRef(inner.clone());
                let src_node = new_graph.graph.add_node(FlowGraphNode::Datatype(src_type));
                new_graph
                    .graph
                    .add_edge(src_node, dt_node, FlowGraphEdge::Freeze);
                let recursive = self.plan_for_datatype(new_graph, src_node);
                self.extend_with_budget(&mut plan, recursive);

                // batch: imm borrow
                let mut new_graph = base.clone();
                let src_type = DatatypeItem::Base(inner.clone());
                let src_node = new_graph.graph.add_node(FlowGraphNode::Datatype(src_type));
                new_graph
                    .graph
                    .add_edge(src_node, dt_node, FlowGraphEdge::ImmBorrow);
                let recursive = self.plan_for_datatype(new_graph, src_node);
                self.extend_with_budget(&mut plan, recursive);
            },
            DatatypeItem::MutRef(inner) => {
                // batch: mut borrow
                let mut new_graph = base.clone();
                let src_type = DatatypeItem::Base(inner.clone());
                let src_node = new_graph.graph.add_node(FlowGraphNode::Datatype(src_type));
                new_graph
                    .graph
                    .add_edge(src_node, dt_node, FlowGraphEdge::MutBorrow);
                let recursive = self.plan_for_datatype(new_graph, src_node);
                self.extend_with_budget(&mut plan, recursive);
            },
        }

        // pop the stack and sanity check
        let (_, stack) = self.trace.last_mut().expect("item in trace");
        let last_analyzed = stack.pop().expect("item in stack");
        assert_eq!(last_analyzed, dt_type);

        // done
        plan
    }

    /// Probe for potential providers for a given datatype
    fn probe_datatype_providers(
        &mut self,
        base: &FlowGraph,
        dt_node: NodeIndex,
        dt_type: &DatatypeItem,
    ) -> Vec<FlowGraph> {
        if self.exploration_stopped() {
            return vec![];
        }
        // sanity check˝
        assert!(!is_cyclic_directed(&base.graph));

        // ways of providing the datatype
        let mut candidates = vec![];
        let internal = self.probe_internal(base, dt_node, dt_type);
        self.extend_with_budget(&mut candidates, internal);
        let external = self.probe_external(base, dt_node, dt_type);
        self.extend_with_budget(&mut candidates, external);
        if dt_type.as_type_item().abilities().has_copy() {
            let copyable = self.probe_copyable(base, dt_node, dt_type);
            self.extend_with_budget(&mut candidates, copyable);
        }
        candidates
    }

    /// Probe functions already registered in the graph for potential providers
    fn probe_internal(
        &mut self,
        base: &FlowGraph,
        target_node: NodeIndex,
        target_type: &DatatypeItem,
    ) -> Vec<FlowGraph> {
        let target_item = target_type.as_type_item();

        let mut candidates = vec![];
        for node in base.graph.node_indices() {
            if self.exploration_stopped() {
                break;
            }
            if self.exploration_stopped() {
                break;
            }
            let func_inst = match base.graph.node_weight(node).unwrap() {
                FlowGraphNode::Datatype(_) => continue,
                FlowGraphNode::Function(f) => f,
            };

            // only check internal function when it does not create a loop in the graph
            let mut trial = base.graph.clone();
            trial.add_edge(
                node,
                target_node,
                FlowGraphEdge::Def(0), // dummy value as a placeholder
            );
            if is_cyclic_directed(&trial) {
                continue;
            }

            // check outgoing edges for unused return values
            let mut used_returns = BTreeSet::new();
            for edge in base.graph.edges_directed(node, Direction::Outgoing) {
                match edge.weight() {
                    FlowGraphEdge::Def(idx) => {
                        let inserted = used_returns.insert(*idx);
                        assert!(inserted);
                    },
                    _ => panic!("unexpected outgoing edge from function node"),
                }
            }

            // try to unify the return type with the target datatype
            let func_decl = self.model.function_registry.lookup_decl(&func_inst.ident);
            for (idx, ty_base) in func_decl.return_sig.iter().enumerate() {
                if self.exploration_stopped() {
                    break;
                }
                if used_returns.contains(&idx) {
                    continue;
                }

                // probe for datatype
                let ty_item = self
                    .model
                    .datatype_registry
                    .instantiate_type_ref(ty_base, &func_inst.type_args);

                // check for match via type unification
                let unified = match (&ty_item, &target_item) {
                    (TypeItem::Base(lhs), TypeItem::Base(rhs))
                    | (TypeItem::ImmRef(lhs), TypeItem::ImmRef(rhs))
                    | (TypeItem::MutRef(lhs), TypeItem::MutRef(rhs)) => {
                        let mut unifier = TypeUnification::new(&base.generics);
                        if unifier.unify(lhs, rhs).is_none() {
                            continue;
                        }
                        unifier.finish()
                    },
                    _ => continue,
                };

                // found a candidate
                let mut new_graph = base.instantiate(&unified);

                // after instantiation, nodes may have been removed (datatype resolved to
                // simple type, or function became dangling); skip if either endpoint is gone
                if new_graph.graph.node_weight(node).is_none()
                    || new_graph.graph.node_weight(target_node).is_none()
                {
                    continue;
                }

                new_graph
                    .graph
                    .add_edge(node, target_node, FlowGraphEdge::Def(idx));
                candidates.push(new_graph);
            }
        }
        candidates
    }

    /// Bring new functions from the model to provide the target datatype
    fn probe_external(
        &mut self,
        base: &FlowGraph,
        target_node: NodeIndex,
        target_type: &DatatypeItem,
    ) -> Vec<FlowGraph> {
        // get the type base
        let target_item = target_type.as_type_item();

        // iterate over all function declarations and try to find providers
        let mut candidates = vec![];
        let mut external_matches = 0usize;
        'decl_loop: for decl in self
            .model
            .function_registry
            .iter_decls()
            .filter(|decl| decl.kind.is_external_provider_candidate())
            .sorted_by_key(|decl| {
                (
                    decl.kind.external_provider_rank(),
                    decl.parameters.len(),
                    decl.generics.len(),
                    decl.ident.clone(),
                )
            })
        {
            if self.exploration_stopped() {
                break;
            }
            for (idx, ret_ty) in decl.return_sig.iter().enumerate() {
                if self.exploration_stopped() {
                    break;
                }
                let unified = match (ret_ty, &target_item) {
                    (TypeRef::Base(tag), TypeItem::Base(base))
                    | (TypeRef::ImmRef(tag), TypeItem::ImmRef(base))
                    | (TypeRef::MutRef(tag), TypeItem::MutRef(base)) => {
                        // try to unify them
                        let mut unifier = TypeSubstitution::new(&decl.generics);
                        if !unifier.unify(tag, base) {
                            continue;
                        }
                        unifier.finish()
                    },
                    _ => continue,
                };

                // found a candidate, create new function instantiations
                external_matches += 1;
                if external_matches > MAX_EXTERNAL_PROVIDER_MATCHES_PER_DATATYPE {
                    break 'decl_loop;
                }

                let mut next_param_index = base.generics.len();
                let mut per_ty_arg_insts = vec![];
                let mut new_generics_pos = BTreeSet::new();
                for (pos, (unified_ty, constraint)) in
                    unified.into_iter().zip(decl.generics.iter()).enumerate()
                {
                    match unified_ty {
                        Some(base) => per_ty_arg_insts.push(vec![base]),
                        None => {
                            let param_insts = ability_set_candidates(*constraint)
                                .into_iter()
                                .map(|abilities| TypeBase::Param {
                                    index: next_param_index,
                                    abilities,
                                })
                                .collect();
                            per_ty_arg_insts.push(param_insts);
                            next_param_index += 1;
                            new_generics_pos.insert(pos);
                        },
                    }
                }

                // instantiate all combinations
                for type_args in per_ty_arg_insts.into_iter().multi_cartesian_product() {
                    if self.exploration_stopped() {
                        break;
                    }
                    let mut new_graph = base.clone();

                    // register new generics to the new graph
                    for pos in &new_generics_pos {
                        let ty_arg = &type_args[*pos];
                        match ty_arg {
                            TypeBase::Param { index, abilities } => {
                                let exists = new_graph.generics.insert(*index, *abilities);
                                assert!(exists.is_none());
                            },
                            _ => panic!("expected type parameter only"),
                        }
                    }

                    // add the call based on the new graph
                    let results =
                        self.add_call(new_graph, decl, &type_args, Some((target_node, idx)));
                    self.extend_with_budget(&mut candidates, results);
                }
            }
        }
        candidates
    }

    /// Probe datatypes already exists in the graph for potential providers
    fn probe_copyable(
        &mut self,
        base: &FlowGraph,
        target_node: NodeIndex,
        target_type: &DatatypeItem,
    ) -> Vec<FlowGraph> {
        let mut candidates = vec![];
        for node in base.graph.node_indices() {
            match base.graph.node_weight(node).unwrap() {
                FlowGraphNode::Datatype(t) => {
                    // only copy from the same datatype
                    if t != target_type {
                        continue;
                    }
                },
                FlowGraphNode::Function(_) => continue,
            };

            // prevent a chain of copies
            let node_is_copied = base
                .graph
                .edges_directed(node, Direction::Incoming)
                .next()
                .is_some_and(|e| matches!(e.weight(), FlowGraphEdge::Copy));
            if node_is_copied {
                continue;
            }

            // only copy when it does not create a loop in the graph
            let mut trial = base.graph.clone();
            trial.add_edge(node, target_node, FlowGraphEdge::Copy);
            if is_cyclic_directed(&trial) {
                continue;
            }

            // found a candidate
            let mut new_graph = base.clone();
            new_graph
                .graph
                .add_edge(node, target_node, FlowGraphEdge::Copy);
            candidates.push(new_graph);
        }
        candidates
    }

    /// Check if a graph is feasible
    pub fn is_feasible(&self, graph: &FlowGraph) -> bool {
        graph.check_integrity();

        // the graph is not feasible if either of the following holds:
        // - more than one signer is needed
        // - at least one return value of a function is not dropped nor used
        // - a non-droppable base-typed datatype node is only borrowed, never consumed
        let mut signer_count = 0;
        for node_idx in graph.graph.node_indices() {
            match graph.graph.node_weight(node_idx).unwrap() {
                FlowGraphNode::Function(func) => {
                    let decl = self.model.function_registry.lookup_decl(&func.ident);
                    let mut provided_complex_params = BTreeSet::new();
                    for edge in graph.graph.edges_directed(node_idx, Direction::Incoming) {
                        match edge.weight() {
                            FlowGraphEdge::Use(param) => {
                                if provided_complex_params.insert(*param) == false {
                                    return false;
                                }
                            },
                            _ => unreachable!(
                                "unexpected incoming edge of non-use kind for a function node"
                            ),
                        }
                    }

                    // Truncated exploration can return graphs that are still acyclic but do
                    // not provide all complex arguments required by this call.
                    let expected_complex_params = decl
                        .parameters
                        .iter()
                        .enumerate()
                        .filter_map(|(idx, param)| {
                            let instantiated = self
                                .model
                                .datatype_registry
                                .instantiate_type_ref(param, &func.type_args);
                            DatatypeItem::from_type_item(&instantiated).map(|_| idx)
                        })
                        .collect::<BTreeSet<_>>();
                    if provided_complex_params != expected_complex_params {
                        return false;
                    }

                    for param in &decl.parameters {
                        if matches!(param, TypeRef::Base(TypeTag::Signer)) {
                            signer_count += 1;
                            if signer_count > 1 {
                                return false;
                            }
                        }
                    }

                    // find which return indices are consumed via Def edges
                    let mut used_returns = BTreeSet::new();
                    for edge in graph.graph.edges_directed(node_idx, Direction::Outgoing) {
                        if let FlowGraphEdge::Def(idx) = edge.weight() {
                            used_returns.insert(*idx);
                        }
                    }

                    // only check drop ability on unused return values
                    for (idx, ty_decl) in decl.return_sig.iter().enumerate() {
                        if used_returns.contains(&idx) {
                            continue;
                        }
                        let ret_ty = self
                            .model
                            .datatype_registry
                            .instantiate_type_ref(ty_decl, &func.type_args);
                        if !ret_ty.abilities().has_drop() {
                            return false;
                        }
                    }
                },
                FlowGraphNode::Datatype(dt) => {
                    // only base-typed (non-reference) nodes can have drop issues
                    if !matches!(dt, DatatypeItem::Base(_)) {
                        continue;
                    }
                    if dt.as_type_item().abilities().has_drop() {
                        continue;
                    }

                    // non-droppable base value: check it has at least one consuming edge
                    let has_consuming_edge = graph
                        .graph
                        .edges_directed(node_idx, Direction::Outgoing)
                        .any(|e| {
                            matches!(
                                e.weight(),
                                FlowGraphEdge::Use(_)
                                    | FlowGraphEdge::VectorToElement
                                    | FlowGraphEdge::ElementToVector
                            )
                        });
                    if !has_consuming_edge {
                        return false;
                    }
                },
            }
        }

        // done
        true
    }
}

/// A database that holds information on how to construct datatypes via function calls
#[derive(Clone)]
pub struct FlowGraph {
    pub graph: StableGraph<FlowGraphNode, FlowGraphEdge>,
    pub generics: BTreeMap<usize, AbilitySet>,
}

impl FlowGraph {
    /// Initialize the flow graph to an empty state
    fn new(generics: BTreeMap<usize, AbilitySet>) -> Self {
        Self {
            graph: StableGraph::new(),
            generics,
        }
    }

    /// Instantiate the graph with a type substitution
    fn instantiate(&self, inst: &BTreeMap<usize, TypeBase>) -> Self {
        let mut new_graph = self.graph.clone();

        // instantiate datatype nodes: first compute new values, then apply
        let node_indices: Vec<_> = new_graph.node_indices().collect();
        let mut updates = vec![];
        for node in &node_indices {
            match new_graph.node_weight(*node).unwrap() {
                FlowGraphNode::Datatype(t) => {
                    let instantiated = DatatypeItem::from_type_item(&Self::instantiate_type_item(
                        &t.as_type_item(),
                        inst,
                    ));
                    updates.push((*node, instantiated));
                },
                FlowGraphNode::Function(_) => continue,
            }
        }
        let mut removal = vec![];
        for (node, update) in updates {
            match update {
                None => {
                    removal.push(node);
                },
                Some(instantiated) => {
                    if let FlowGraphNode::Datatype(t) = new_graph.node_weight_mut(node).unwrap() {
                        *t = instantiated;
                    }
                },
            }
        }

        // record function nodes that are already isolated (no edges) before
        // removing datatype nodes -- these include the root function node which
        // should never be removed
        let already_isolated: BTreeSet<_> = new_graph
            .node_indices()
            .filter(|node| {
                matches!(
                    new_graph.node_weight(*node).unwrap(),
                    FlowGraphNode::Function(_)
                ) && new_graph.edges_directed(*node, Direction::Incoming).count() == 0
                    && new_graph.edges_directed(*node, Direction::Outgoing).count() == 0
            })
            .collect();

        // remove nodes that instantiate to simple types
        for node in removal {
            new_graph.remove_node(node);
        }

        // remove function nodes that BECAME dangling due to their connected
        // datatype nodes being removed (instantiated to simple types).
        // Do NOT remove function nodes that were already isolated -- the root
        // function node (the target we are fuzzing) naturally has no edges when
        // all its parameters are simple types and its return value is unused.
        let dangling: Vec<_> = new_graph
            .node_indices()
            .filter(|node| {
                matches!(
                    new_graph.node_weight(*node).unwrap(),
                    FlowGraphNode::Function(_)
                ) && new_graph.edges_directed(*node, Direction::Incoming).count() == 0
                    && new_graph.edges_directed(*node, Direction::Outgoing).count() == 0
                    && !already_isolated.contains(node)
            })
            .collect();
        for node in dangling {
            new_graph.remove_node(node);
        }

        // instantiate function nodes
        for node in new_graph.node_weights_mut() {
            match node {
                FlowGraphNode::Function(f) => {
                    for ty_arg in f.type_args.iter_mut() {
                        *ty_arg = Self::instantiate_type_base(ty_arg, inst);
                    }
                },
                FlowGraphNode::Datatype(_) => continue,
            }
        }

        // done
        Self {
            graph: new_graph,
            generics: self.generics.clone(),
        }
    }

    /// Utility: instantiate a `TypeBase` with mapping
    fn instantiate_type_base(ty: &TypeBase, inst: &BTreeMap<usize, TypeBase>) -> TypeBase {
        match ty {
            TypeBase::Bool => TypeBase::Bool,
            TypeBase::U8 => TypeBase::U8,
            TypeBase::I8 => TypeBase::I8,
            TypeBase::U16 => TypeBase::U16,
            TypeBase::I16 => TypeBase::I16,
            TypeBase::U32 => TypeBase::U32,
            TypeBase::I32 => TypeBase::I32,
            TypeBase::U64 => TypeBase::U64,
            TypeBase::I64 => TypeBase::I64,
            TypeBase::U128 => TypeBase::U128,
            TypeBase::I128 => TypeBase::I128,
            TypeBase::U256 => TypeBase::U256,
            TypeBase::I256 => TypeBase::I256,
            TypeBase::Bitvec => TypeBase::Bitvec,
            TypeBase::String => TypeBase::String,
            TypeBase::Address => TypeBase::Address,
            TypeBase::Signer => TypeBase::Signer,
            TypeBase::Vector { element } => TypeBase::Vector {
                element: Box::new(Self::instantiate_type_base(element, inst)),
            },
            TypeBase::Datatype {
                ident,
                type_args,
                abilities,
            } => TypeBase::Datatype {
                ident: ident.clone(),
                type_args: type_args
                    .iter()
                    .map(|t| Self::instantiate_type_base(t, inst))
                    .collect(),
                abilities: *abilities,
            },
            TypeBase::Param { index, abilities } => match inst.get(index) {
                None => TypeBase::Param {
                    index: *index,
                    abilities: *abilities,
                },
                Some(replacement) => replacement.clone(),
            },
            TypeBase::ObjectKnown {
                ident,
                type_args,
                abilities,
            } => TypeBase::ObjectKnown {
                ident: ident.clone(),
                type_args: type_args
                    .iter()
                    .map(|t| Self::instantiate_type_base(t, inst))
                    .collect(),
                abilities: *abilities,
            },
            TypeBase::ObjectParam { index, abilities } => match inst.get(index) {
                None => TypeBase::ObjectParam {
                    index: *index,
                    abilities: *abilities,
                },
                Some(replacement) => match replacement {
                    TypeBase::Param { index, abilities } => TypeBase::ObjectParam {
                        index: *index,
                        abilities: *abilities,
                    },
                    TypeBase::Datatype {
                        ident,
                        type_args,
                        abilities,
                    } => TypeBase::ObjectKnown {
                        ident: ident.clone(),
                        type_args: type_args.clone(),
                        abilities: *abilities,
                    },
                    _ => panic!("invalid replacement for object param"),
                },
            },
            TypeBase::Function {
                params,
                returns,
                abilities,
            } => TypeBase::Function {
                params: params
                    .iter()
                    .map(|t| Self::instantiate_type_item(t, inst))
                    .collect(),
                returns: returns
                    .iter()
                    .map(|t| Self::instantiate_type_item(t, inst))
                    .collect(),
                abilities: *abilities,
            },
        }
    }

    /// Utility: instantiate a `TypeItem` with mapping
    fn instantiate_type_item(ty: &TypeItem, inst: &BTreeMap<usize, TypeBase>) -> TypeItem {
        match ty {
            TypeItem::Base(t) => TypeItem::Base(Self::instantiate_type_base(t, inst)),
            TypeItem::ImmRef(t) => TypeItem::ImmRef(Self::instantiate_type_base(t, inst)),
            TypeItem::MutRef(t) => TypeItem::MutRef(Self::instantiate_type_base(t, inst)),
        }
    }

    /// Check graph integrity
    pub fn check_integrity(&self) {
        // the graph should always be acyclic
        assert!(!is_cyclic_directed(&self.graph));

        // check by node types
        let mut root = None;
        for node in self.graph.node_indices() {
            match self.graph.node_weight(node).unwrap() {
                FlowGraphNode::Datatype(_) => {
                    // all datatype nodes should have exactly one incoming edge
                    let incoming = self.graph.edges_directed(node, Direction::Incoming).count();
                    assert_eq!(incoming, 1);

                    // outgoing: exactly one primary edge, but a copyable node may also
                    // serve as the source of Copy edges to other datatype nodes
                    let outgoing = self.graph.edges_directed(node, Direction::Outgoing).count();
                    assert!(outgoing >= 1);

                    // there is at most one primary outgoing edges (all others should be Copy)
                    let non_copy_count = self
                        .graph
                        .edges_directed(node, Direction::Outgoing)
                        .filter(|e| !matches!(e.weight(), FlowGraphEdge::Copy))
                        .count();
                    assert_eq!(non_copy_count, 1);
                },
                FlowGraphNode::Function(_) => {
                    // there should be one and only one function node without outgoing edges
                    let outgoing = self.graph.edges_directed(node, Direction::Outgoing).count();
                    if outgoing == 0 {
                        assert!(root.is_none());
                        root = Some(node);
                    }
                },
            }
        }
        assert!(root.is_some());
    }

    /// Collected function parameters that are actually involved in the graph
    fn involved_parameters(&self) -> BTreeSet<usize> {
        let mut params = BTreeSet::new();
        for node in self.graph.node_weights() {
            match node {
                FlowGraphNode::Function(f) => {
                    for ty_arg in &f.type_args {
                        ty_arg.involved_parameters(&mut params);
                    }
                },
                FlowGraphNode::Datatype(t) => {
                    t.as_type_item().involved_parameters(&mut params);
                },
            }
        }
        params
    }

    /// Compact the generics in the graph to a contiguous sequence starting from 0
    pub fn compact_generics(&self) -> Self {
        let mut new_generics = BTreeMap::new();
        let mut compact_inst = BTreeMap::new();
        for (index, param) in self.involved_parameters().into_iter().enumerate() {
            let abilities = self.generics.get(&param).unwrap();
            new_generics.insert(index, *abilities);
            compact_inst.insert(param, TypeBase::Param {
                index,
                abilities: *abilities,
            });
        }

        let mut new_graph = self.instantiate(&compact_inst);
        new_graph.generics = new_generics;
        new_graph
    }
}

#[cfg(test)]
mod tests {
    use super::{DatatypeItem, FlowGraph, FlowGraphNode, FunctionInst, GraphBuilder};
    use crate::{
        deps::PkgKind,
        prep::{
            datatype::DatatypeRegistry,
            function::{FunctionDecl, FunctionRegistry},
            ident::FunctionIdent,
            model::Model,
            typing::{ComplexType, TypeBase, TypeItem, TypeRef, TypeTag},
        },
    };
    use move_core_types::{
        ability::{Ability, AbilitySet},
        account_address::AccountAddress,
        identifier::Identifier,
    };
    use std::collections::BTreeMap;

    fn param(index: usize, abilities: AbilitySet) -> ComplexType {
        ComplexType::Param { index, abilities }
    }

    fn function(name: &str) -> FunctionIdent {
        FunctionIdent::from_function_tuple(
            AccountAddress::ONE,
            Identifier::new("m").unwrap(),
            Identifier::new(name).unwrap(),
        )
    }

    fn model_with_decl(decl: FunctionDecl) -> Model {
        let mut function_registry = FunctionRegistry::new();
        function_registry.insert_for_test(decl);
        Model {
            datatype_registry: DatatypeRegistry::new(),
            function_registry,
        }
    }

    #[test]
    fn test_datatype_item_from_type_item_filters_simple_types() {
        assert!(DatatypeItem::from_type_item(&TypeItem::Base(TypeBase::Bool)).is_none());
        assert_eq!(
            DatatypeItem::from_type_item(&TypeItem::ImmRef(TypeBase::Param {
                index: 2,
                abilities: AbilitySet::PRIMITIVES,
            })),
            Some(DatatypeItem::ImmRef(param(2, AbilitySet::PRIMITIVES)))
        );
    }

    #[test]
    fn test_flow_graph_instantiate_removes_nodes_that_become_simple() {
        let mut graph = FlowGraph::new(BTreeMap::from([(0, AbilitySet::PRIMITIVES)]));
        graph
            .graph
            .add_node(FlowGraphNode::Datatype(DatatypeItem::Base(param(
                0,
                AbilitySet::PRIMITIVES,
            ))));

        let instantiated = graph.instantiate(&BTreeMap::from([(0, TypeBase::U64)]));
        assert_eq!(instantiated.graph.node_count(), 0);
        assert!(instantiated.generics.contains_key(&0));
    }

    #[test]
    fn test_flow_graph_compact_generics_reindexes_used_parameters() {
        let mut graph = FlowGraph::new(BTreeMap::from([
            (2, AbilitySet::PRIMITIVES),
            (5, AbilitySet::EMPTY.add(Ability::Store)),
            (8, AbilitySet::EMPTY.add(Ability::Key)),
        ]));
        graph
            .graph
            .add_node(FlowGraphNode::Datatype(DatatypeItem::Base(param(
                5,
                AbilitySet::EMPTY.add(Ability::Store),
            ))));
        graph
            .graph
            .add_node(FlowGraphNode::Datatype(DatatypeItem::Base(
                ComplexType::Vector {
                    element: Box::new(param(2, AbilitySet::PRIMITIVES)),
                },
            )));

        let compact = graph.compact_generics();
        assert_eq!(compact.generics.into_iter().collect::<Vec<_>>(), vec![
            (0, AbilitySet::PRIMITIVES),
            (1, AbilitySet::EMPTY.add(Ability::Store)),
        ]);
        let nodes = compact.graph.node_weights().cloned().collect::<Vec<_>>();
        assert!(
            nodes.contains(&FlowGraphNode::Datatype(DatatypeItem::Base(param(
                1,
                AbilitySet::EMPTY.add(Ability::Store),
            ))))
        );
        assert!(nodes.contains(&FlowGraphNode::Datatype(DatatypeItem::Base(
            ComplexType::Vector {
                element: Box::new(param(0, AbilitySet::PRIMITIVES)),
            },
        ))));
    }

    #[test]
    fn test_is_feasible_rejects_missing_complex_param_provider() {
        let ident = function("target");
        let model = model_with_decl(FunctionDecl {
            ident: ident.clone(),
            generics: vec![AbilitySet::PRIMITIVES],
            parameters: vec![TypeRef::Base(TypeTag::Param(0))],
            return_sig: vec![],
            kind: PkgKind::Primary,
            is_entry: true,
        });
        let mut graph = FlowGraph::new(BTreeMap::from([(0, AbilitySet::PRIMITIVES)]));
        graph.graph.add_node(FlowGraphNode::Function(FunctionInst {
            ident,
            type_args: vec![TypeBase::Param {
                index: 0,
                abilities: AbilitySet::PRIMITIVES,
            }],
        }));

        let builder = GraphBuilder::new(&model, 4, 1, None);
        assert_eq!(builder.is_feasible(&graph), false);
    }

    #[test]
    fn test_is_feasible_accepts_simple_only_root_call() {
        let ident = function("simple");
        let model = model_with_decl(FunctionDecl {
            ident: ident.clone(),
            generics: vec![],
            parameters: vec![TypeRef::Base(TypeTag::U64)],
            return_sig: vec![],
            kind: PkgKind::Primary,
            is_entry: true,
        });
        let mut graph = FlowGraph::new(BTreeMap::new());
        graph.graph.add_node(FlowGraphNode::Function(FunctionInst {
            ident,
            type_args: vec![],
        }));

        let builder = GraphBuilder::new(&model, 4, 1, None);
        assert_eq!(builder.is_feasible(&graph), true);
    }
}
