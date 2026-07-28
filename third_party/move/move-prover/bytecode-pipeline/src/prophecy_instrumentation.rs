// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Instrumentation of mutable references using the prophecy (RustHorn/Creusot) model —
//! the default counterpart of `MemoryInstrumentationProcessor` (`--path-refs` selects
//! the legacy model, see `pipeline_factory.rs`). Instead of `WriteBack`/`IsParent`
//! instructions propagating a mutation back along a statically known borrow path, it
//! inserts:
//!
//!   - `ProphecyBorrow(lender, edge)` after each borrow creation: eagerly installs the
//!     child's prophecy (its final value) into the lender; and
//!   - `Resolve(ref)` at each point a reference *binding* ends: fulfills the prophecy
//!     (`assume current == final`).
//!
//! A binding ends (1) at its live-range exit — unconditionally, not gated on the
//! borrow graph's in-use notion: after its last use a reference can only have been
//! relinked to a child's fresh prophecy, so resolving chains the pending obligations
//! even while children live; (2) at a redefinition of its temp (death by shadowing,
//! see `redefinition_resolves`); and (3) at the merged-exit move of `NormalizeExits`,
//! a rename which inherits the prophecy (see there).
//!
//! Every in-body `&mut`-to-`&mut` assignment is a *reborrow* — fresh prophecy for the
//! destination, source relinked (see the translator's `Assign` arm) — so `&mut` copies
//! are stateful: `ReachingDefProcessor` does not alias-propagate them in this pipeline,
//! `LambdaSpecInferenceProcessor` normalizes its own copy, and the loop-target
//! analysis sees the relink through `Bytecode::modifies`.
//!
//! Observations (in-code spec blocks and loop invariants) must see the *current* value
//! of a borrowed lender, not the installed prophecy: each is bracketed by
//! `ProphecySyncCurrent`/`ProphecySyncFinal` markers over the live borrow sites,
//! guarded by per-site path flags (see `SyncSite`). Sites cover the syntactic borrow
//! instructions, `&mut` assignments, and the native vector/table `borrow_mut` calls;
//! observing a lender of any other call-derived borrow is rejected
//! (`check_observation`).
//!
//! Example: `let r = &mut v[i]; *r = 9` lowers (conceptually) to
//!
//! ```text
//!   var x;                                         // fresh prophecy
//!   r := $Mutation{curr: ReadVec(v, i), final: x}; // borrow &mut v[i]
//!   v := $UpdateVec(v, i, x);                      // x eagerly installed at v[i]
//!   r := $Mutation{curr: 9, ..r};                  // *r = 9
//!   assume r->final == r->curr;                    // resolve at r's death: pins x == 9
//! ```
//!
//! Calls under a live global borrow get a re-pin (`call_repins`): a callee may havoc
//! the borrowed resource's memory, severing the eager link, and since no non-aborting
//! callee can change an exclusively borrowed slot, the slot is restored after the
//! call. Loop-head havocs (from `loop_analysis`, which runs later) get no re-pin: a
//! borrow held across a back-edge is sound but leaves its lender unconstrained unless
//! the loop invariants characterize it. A `&mut` derived through a function value
//! (`BorrowEdge::Invoke`) is not supported; the translator rejects it with an error.
//!
//! See `doc/dev/prophecies/prophecy_model.md` for the design and rationale.

use crate::memory_instrumentation::Instrumenter;
use move_binary_format::file_format::CodeOffset;
use move_model::{
    ast::{ConditionKind, Exp},
    exp_generator::ExpGenerator,
    model::{FunctionEnv, QualifiedInstId, StructId},
    ty::{PrimitiveType, Type, BOOL_TYPE},
};
use move_stackless_bytecode::{
    borrow_analysis::{BorrowAnnotation, BorrowInfo, WriteBackAction},
    function_data_builder::FunctionDataBuilder,
    function_target::FunctionData,
    function_target_pipeline::{FunctionTargetProcessor, FunctionTargetsHolder},
    stackless_bytecode::{
        AssignKind, AttrId, BorrowEdge, BorrowNode, Bytecode, Constant, IndexEdgeKind, Operation,
    },
};
use std::collections::{BTreeMap, BTreeSet};

pub struct ProphecyInstrumentationProcessor {}

impl ProphecyInstrumentationProcessor {
    pub fn new() -> Box<Self> {
        Box::new(ProphecyInstrumentationProcessor {})
    }
}

impl FunctionTargetProcessor for ProphecyInstrumentationProcessor {
    fn process(
        &self,
        _targets: &mut FunctionTargetsHolder,
        func_env: &FunctionEnv,
        mut data: FunctionData,
        _scc_opt: Option<&[FunctionEnv]>,
    ) -> FunctionData {
        if func_env.no_verified_bytecode() {
            return data;
        }
        let borrow_annotation = data
            .annotations
            .remove::<BorrowAnnotation>()
            .expect("borrow annotation");
        let builder = FunctionDataBuilder::new(func_env, data);
        let mut instr = ProphecyInstrumenter {
            builder,
            borrow_annotation: &borrow_annotation,
            sites: Vec::new(),
            global_only_children: BTreeMap::new(),
            saved_addrs: BTreeMap::new(),
            unsynced_children: BTreeSet::new(),
        };
        let code = std::mem::take(&mut instr.builder.data.code);
        instr.global_only_children = compute_global_only_children(&code);
        instr.compute_sync_sites(&code);
        instr.unsynced_children = instr.compute_unsynced_children(&code);
        // Path flags are false until their borrow site runs; a site whose flag was
        // never set (a path that borrowed elsewhere or not at all) contributes no
        // sync.
        instr.builder.set_loc(instr.builder.fun_env.get_loc());
        for site in &instr.sites {
            let flag = site.flag;
            instr
                .builder
                .emit_with(|id| Bytecode::Load(id, flag, Constant::Bool(false)));
        }
        for (offset, bytecode) in code.into_iter().enumerate() {
            instr.instrument(offset as CodeOffset, bytecode);
        }
        // Publish the saved-address temps for downstream processors
        // (`loop_analysis` addresses globally-rooted refs through them).
        instr.builder.data.prophecy_saved_addrs = instr.saved_addrs.clone();
        instr.builder.data
    }

    fn name(&self) -> String {
        "prophecy_instr".to_string()
    }
}

/// A borrow site eligible for observation syncs: the place a lender received its
/// eager prophecy update. Around an observation (an in-code spec or a loop
/// invariant), the site's lender is temporarily given the child's *current* value
/// and afterwards restored to the child's *final* value (the eager state), each
/// guarded by the site's path flag.
struct SyncSite {
    /// Code offset of the borrow (or reborrow assign) in the original code.
    offset: CodeOffset,
    /// The borrowing reference.
    child: usize,
    /// The lender receiving the sync.
    lender: BorrowNode,
    /// The borrow edge from lender to child (mirrors the eager update's edge).
    edge: BorrowEdge,
    /// Path flag: true exactly when this site is the child's reaching borrow.
    flag: usize,
    /// The site's dynamic selector as `(operand at the site, saved temp)`: the borrow
    /// address for a `GlobalRoot` lender, the element index/key for an `Index` edge.
    /// Saved into the fresh temp at the site, immune to later reassignment of the
    /// operand local.
    selector: Option<(usize, usize)>,
}

struct ProphecyInstrumenter<'a> {
    builder: FunctionDataBuilder<'a>,
    borrow_annotation: &'a BorrowAnnotation,
    /// All borrow sites, in code order. Observation syncs are emitted for the
    /// sites whose child is live at the observation, deepest (latest) site first,
    /// so a chain's parents pick up their children's synced values.
    sites: Vec<SyncSite>,
    /// Reference temps every one of whose definitions is a `BorrowGlobal` of one memory,
    /// mapped to that memory. Only such single-origin children get the call re-pin (see
    /// `call_repins`): their saved-address copy always matches the reaching borrow on
    /// every path. A mixed-origin temp (e.g. borrowing a global on one branch and a
    /// local on the other) is skipped — re-pinning it could write through a stale
    /// address — leaving the incomplete-but-sound unpinned slot across calls.
    global_only_children: BTreeMap<usize, QualifiedInstId<StructId>>,
    /// Saved address temp per global-borrow child, assigned at every `BorrowGlobal` of
    /// the child so the reaching definition of the address always matches the reaching
    /// borrow. Serves the observation syncs for every child, and the call re-pin for
    /// the single-origin children in `global_only_children`.
    saved_addrs: BTreeMap<usize, usize>,
    /// `&mut` temps with a definition that is not a borrow site: a reference
    /// materialized as a call result (through an opaque callee or a custom native)
    /// whose structure is not statically known. No sync exists for such a borrow,
    /// so an observation of one of its lenders is rejected (`check_observation`).
    unsynced_children: BTreeSet<usize>,
}

impl ProphecyInstrumenter<'_> {
    fn instrument(&mut self, code_offset: CodeOffset, bytecode: Bytecode) {
        if let Bytecode::Prop(attr_id, _, exp) = &bytecode {
            let attr_id = *attr_id;
            self.check_observation(code_offset, attr_id, exp);
            // Bracket the observation with syncs so it sees each borrowed lender's
            // current value, not the installed prophecy (covers any read form,
            // including spec-function bodies). Loop invariants are recorded instead —
            // they must stay consecutive at the loop header — and
            // `LoopAnalysisProcessor` emits the same brackets wherever it asserts or
            // assumes them.
            let pairs = self.sync_pairs_at(code_offset);
            if self.builder.data.loop_invariants.contains(&attr_id) {
                if !pairs.is_empty() {
                    let entries = pairs
                        .iter()
                        .map(|i| {
                            let site = &self.sites[*i];
                            (
                                site.lender.clone(),
                                site.edge.clone(),
                                site.child,
                                site.flag,
                                site.selector.map(|(_, saved)| saved),
                            )
                        })
                        .collect();
                    self.builder
                        .data
                        .loop_invariant_prophecy_syncs
                        .insert(attr_id, entries);
                }
            } else {
                self.builder.set_loc_from_attr(attr_id);
                self.emit_syncs(&pairs, true);
                self.builder.emit(bytecode.clone());
                self.emit_syncs(&pairs, false);
                // A reference may still die exactly at this assertion; resolve it as usual.
                self.prophecy_instrumentation(code_offset, &bytecode);
                return;
            }
        }
        // A (re)definition of a `&mut` temp ends its previous binding (shadowing);
        // resolve it first — a redefined temp stays live across the redefinition, so
        // the live-exit resolves never fire for it. At a first definition the assume
        // relates the components of an unconstrained local: unobservable. A call dest
        // aliasing a `&mut` operand is not a shadowing (the binding continues).
        self.redefinition_resolves(&bytecode);
        // Re-pins must land after the call itself (on its non-abort continuation),
        // which the branching dispatch below cannot provide: a call with an abort
        // action counts as branching, so its instrumentation is emitted before it.
        let repin_attr = match &bytecode {
            Bytecode::Call(attr_id, _, Operation::Function(..) | Operation::Invoke, _, _) => {
                Some(*attr_id)
            },
            _ => None,
        };
        if bytecode.is_branching()
            || matches!(bytecode, Bytecode::Call(_, _, Operation::Drop, _, _))
        {
            self.prophecy_instrumentation(code_offset, &bytecode);
            self.builder.emit(bytecode);
        } else {
            self.builder.emit(bytecode.clone());
            self.prophecy_instrumentation(code_offset, &bytecode);
        }
        if let Some(attr_id) = repin_attr {
            self.call_repins(code_offset, attr_id);
        }
        // Maintain the borrow site's path flag and saved address on the site's
        // non-abort continuation: the flag becomes true, sibling sites of the same
        // child become false (the reaching borrow is now this one).
        self.site_updates(code_offset);
    }

    /// Whether the reference is connected to a `&mut` parameter purely through
    /// `Direct` (assignment) edges in the borrow graph, i.e. it is an alias of the
    /// parameter's own borrow rather than a borrow derived from it. The graph may
    /// contain cycles (see `BorrowInfo::is_in_use`), hence the visited set.
    fn is_direct_alias_of_param(&self, info: &BorrowInfo, temp: usize) -> bool {
        let param_count = self.builder.get_target().get_parameter_count();
        let mut visited = BTreeSet::new();
        let mut todo = vec![BorrowNode::Reference(temp)];
        while let Some(node) = todo.pop() {
            if !visited.insert(node.clone()) {
                continue;
            }
            if let BorrowNode::Reference(idx) = node {
                if idx < param_count {
                    return true;
                }
            }
            for (parent, edge) in info.get_incoming(&node) {
                if matches!(edge, BorrowEdge::Direct) {
                    todo.push(parent.clone());
                }
            }
        }
        false
    }

    /// Resolve the previous binding of every `&mut` temp this instruction
    /// (re)defines, before the instruction executes. See `instrument` for why.
    fn redefinition_resolves(&mut self, bytecode: &Bytecode) {
        use Operation::*;
        let redefined: Vec<usize> = match bytecode {
            Bytecode::Assign(_, dest, _, _) => vec![*dest],
            Bytecode::Call(_, dests, op, srcs, _) => match op {
                BorrowLoc
                | BorrowField(..)
                | BorrowVariantField(..)
                | BorrowGlobal(..)
                | Function(..)
                | Invoke => dests
                    .iter()
                    .filter(|dest| !srcs.contains(dest))
                    .copied()
                    .collect(),
                _ => vec![],
            },
            _ => vec![],
        };
        for dest in redefined {
            if self
                .builder
                .get_target()
                .get_local_type(dest)
                .is_mutable_reference()
            {
                self.builder.emit_with(|id| {
                    Bytecode::Call(id, vec![], Operation::Resolve, vec![dest], None)
                });
            }
        }
    }

    /// The sites whose child is live at the given offset, deepest (latest) first,
    /// so that syncing a chain installs children's values before their parents
    /// read them.
    fn sync_pairs_at(&self, code_offset: CodeOffset) -> Vec<usize> {
        let Some(borrow_at) = self.borrow_annotation.get_borrow_info_at(code_offset) else {
            return vec![];
        };
        let before = &borrow_at.before;
        self.sites
            .iter()
            .enumerate()
            .rev()
            .filter(|(_, site)| before.is_in_use(&BorrowNode::Reference(site.child)))
            .map(|(i, _)| i)
            .collect()
    }

    /// Emit the sync (`current == true`) or restore (`current == false`)
    /// instructions for the given sites.
    fn emit_syncs(&mut self, pairs: &[usize], current: bool) {
        use Bytecode::Call;
        for i in pairs {
            let site = &self.sites[*i];
            let op = if current {
                Operation::ProphecySyncCurrent(site.lender.clone(), site.edge.clone())
            } else {
                Operation::ProphecySyncFinal(site.lender.clone(), site.edge.clone())
            };
            let mut srcs = vec![site.child, site.flag];
            if let Some((_, saved)) = site.selector {
                srcs.push(saved);
            }
            self.builder
                .emit_with(|id| Call(id, vec![], op, srcs, None));
        }
    }

    /// Emit the flag/saved-selector maintenance for a borrow site at this offset.
    fn site_updates(&mut self, code_offset: CodeOffset) {
        let updates: Vec<(usize, Option<(usize, usize)>, Vec<usize>)> = self
            .sites
            .iter()
            .enumerate()
            .filter(|(_, site)| site.offset == code_offset)
            .map(|(i, site)| {
                let siblings = self
                    .sites
                    .iter()
                    .enumerate()
                    .filter(|(j, other)| *j != i && other.child == site.child)
                    .map(|(_, other)| other.flag)
                    .collect();
                (site.flag, site.selector, siblings)
            })
            .collect();
        for (flag, selector, siblings) in updates {
            if let Some((sel_src, saved)) = selector {
                // The selector operand may be a user local reassigned while the
                // borrow lives; the copy taken here always matches this borrow.
                self.builder
                    .emit_with(|id| Bytecode::Assign(id, saved, sel_src, AssignKind::Copy));
            }
            self.builder
                .emit_with(|id| Bytecode::Load(id, flag, Constant::Bool(true)));
            for sibling in siblings {
                self.builder
                    .emit_with(|id| Bytecode::Load(id, sibling, Constant::Bool(false)));
            }
        }
    }

    /// Enumerate the borrow sites (see `SyncSite`), allocating their path flags
    /// and the saved selector temps.
    fn compute_sync_sites(&mut self, code: &[Bytecode]) {
        use Operation::*;
        for (offset, bytecode) in code.iter().enumerate() {
            let offset = offset as CodeOffset;
            match bytecode {
                Bytecode::Call(_, dests, op, srcs, _) if !dests.is_empty() && !srcs.is_empty() => {
                    let (lender, edge, selector) = match op {
                        BorrowLoc => (self.borrow_node(srcs[0]), BorrowEdge::Direct, None),
                        BorrowField(mid, sid, inst, field) => (
                            self.borrow_node(srcs[0]),
                            BorrowEdge::Field(
                                mid.qualified_inst(*sid, inst.to_owned()),
                                None,
                                *field,
                            ),
                            None,
                        ),
                        BorrowVariantField(mid, sid, variants, inst, field) => (
                            self.borrow_node(srcs[0]),
                            BorrowEdge::Field(
                                mid.qualified_inst(*sid, inst.to_owned()),
                                Some(variants.clone()),
                                *field,
                            ),
                            None,
                        ),
                        BorrowGlobal(mid, sid, inst) => {
                            let saved = match self.saved_addrs.get(&dests[0]) {
                                Some(saved) => *saved,
                                None => {
                                    let saved = self
                                        .builder
                                        .new_temp(Type::Primitive(PrimitiveType::Address));
                                    self.saved_addrs.insert(dests[0], saved);
                                    saved
                                },
                            };
                            (
                                BorrowNode::GlobalRoot(mid.qualified_inst(*sid, inst.clone())),
                                BorrowEdge::Direct,
                                Some((srcs[0], saved)),
                            )
                        },
                        Function(..) => {
                            // A native element borrow (`vector`/`table` `borrow_mut`),
                            // recognized by the `Index` edge its borrow summary installs
                            // from the container operand to the result; the selector
                            // operand is saved at the site like a global site's borrow
                            // address. Other call-derived borrows have no statically
                            // known structure and get no site; observations over their
                            // lenders are rejected (see `check_observation`).
                            let Some(borrow_at) = self.borrow_annotation.get_borrow_info_at(offset)
                            else {
                                continue;
                            };
                            let kind = borrow_at
                                .after
                                .get_incoming(&BorrowNode::Reference(dests[0]))
                                .into_iter()
                                .find_map(|(parent, edge)| match edge {
                                    BorrowEdge::Index(
                                        kind @ (IndexEdgeKind::Vector | IndexEdgeKind::Table),
                                    ) if *parent == BorrowNode::Reference(srcs[0])
                                        && srcs.len() >= 2 =>
                                    {
                                        Some(kind.clone())
                                    },
                                    _ => None,
                                });
                            let Some(kind) = kind else { continue };
                            let selector_ty =
                                self.builder.get_target().get_local_type(srcs[1]).to_owned();
                            let saved = self.builder.new_temp(selector_ty);
                            (
                                BorrowNode::Reference(srcs[0]),
                                BorrowEdge::Index(kind),
                                Some((srcs[1], saved)),
                            )
                        },
                        _ => continue,
                    };
                    let flag = self.builder.new_temp(BOOL_TYPE.clone());
                    self.sites.push(SyncSite {
                        offset,
                        child: dests[0],
                        lender,
                        edge,
                        flag,
                        selector,
                    });
                },
                Bytecode::Assign(_, dest, src, _) => {
                    let target = self.builder.get_target();
                    if target.get_local_type(*dest).is_mutable_reference()
                        && target.get_local_type(*src).is_mutable_reference()
                    {
                        let flag = self.builder.new_temp(BOOL_TYPE.clone());
                        self.sites.push(SyncSite {
                            offset,
                            child: *dest,
                            lender: BorrowNode::Reference(*src),
                            edge: BorrowEdge::Direct,
                            flag,
                            selector: None,
                        });
                    }
                },
                _ => {},
            }
        }
    }

    /// The `&mut` temps with a definition that is not a borrow site (see
    /// `unsynced_children`). A call dest that is also a source continues an
    /// existing binding through the call (`x := f(x)`) and keeps its sites.
    fn compute_unsynced_children(&self, code: &[Bytecode]) -> BTreeSet<usize> {
        let site_defs: BTreeSet<(CodeOffset, usize)> =
            self.sites.iter().map(|s| (s.offset, s.child)).collect();
        let target = self.builder.get_target();
        let mut result = BTreeSet::new();
        for (offset, bc) in code.iter().enumerate() {
            let defs: Vec<usize> = match bc {
                Bytecode::Assign(_, dest, _, _) => vec![*dest],
                Bytecode::Call(_, dests, _, srcs, _) => dests
                    .iter()
                    .filter(|dest| !srcs.contains(dest))
                    .copied()
                    .collect(),
                _ => vec![],
            };
            for dest in defs {
                if target.get_local_type(dest).is_mutable_reference()
                    && !site_defs.contains(&(offset as CodeOffset, dest))
                {
                    result.insert(dest);
                }
            }
        }
        result
    }

    /// Reject an observation that reads a lender of a live borrow without a sync
    /// site (see `unsynced_children`): the observation cannot be corrected to the
    /// borrow's current value and would silently read the installed prophecy.
    fn check_observation(&self, code_offset: CodeOffset, attr_id: AttrId, exp: &Exp) {
        if self.unsynced_children.is_empty() {
            return;
        }
        let Some(borrow_at) = self.borrow_annotation.get_borrow_info_at(code_offset) else {
            return;
        };
        let before = &borrow_at.before;
        let env = self.builder.global_env();
        let used_temps = exp.used_temporaries();
        let used_memory: BTreeSet<QualifiedInstId<StructId>> = exp
            .used_memory(env)
            .into_iter()
            .filter_map(|(mem, label)| label.is_none().then_some(mem))
            .collect();
        for child in &self.unsynced_children {
            let node = BorrowNode::Reference(*child);
            if before.get_incoming(&node).is_empty() || !before.is_in_use(&node) {
                continue;
            }
            // Walk the transitive lenders; reading any of them (or an intermediate
            // reference other than the borrow itself) observes the prophecy.
            let mut visited = BTreeSet::new();
            let mut todo = vec![node];
            let mut observed = false;
            while let Some(n) = todo.pop() {
                if !visited.insert(n.clone()) {
                    continue;
                }
                observed = match &n {
                    BorrowNode::Reference(t) => t != child && used_temps.contains(t),
                    BorrowNode::LocalRoot(t) => used_temps.contains(t),
                    BorrowNode::GlobalRoot(mem) => used_memory.contains(mem),
                    BorrowNode::ReturnPlaceholder(..) => false,
                };
                if observed {
                    break;
                }
                todo.extend(before.get_incoming(&n).into_iter().map(|(p, _)| p.clone()));
            }
            if observed {
                env.error(
                    &self.builder.get_loc(attr_id),
                    "cannot observe a location while it is mutably borrowed through a \
                     function call (prophecy reference model): the borrow's structure is \
                     not statically known here. Assert over the returned reference \
                     instead, or use `--path-refs`",
                );
                return;
            }
        }
    }

    /// After a call, restore the eager update of every live global borrow
    /// (`mem := ResourceUpdate(mem, addr, child->f)`): the callee may havoc the
    /// borrowed memory, and Move's borrow rules (`acquires`, dispatch reentrancy
    /// check) guarantee no non-aborting callee changes an exclusively borrowed slot.
    /// An assignment rather than an assumption, so a callee spec that (impossibly)
    /// constrains the slot is overridden, not turned into a false assumption.
    fn call_repins(&mut self, code_offset: CodeOffset, attr_id: AttrId) {
        use Bytecode::Call;
        use Operation::ProphecyRepin;
        let Some(borrow_at) = self.borrow_annotation.get_borrow_info_at(code_offset) else {
            return;
        };
        let after = &borrow_at.after;
        let repins: Vec<(usize, QualifiedInstId<StructId>, usize)> = self
            .saved_addrs
            .iter()
            .filter(|(child, _)| after.is_in_use(&BorrowNode::Reference(**child)))
            .filter_map(|(child, saved)| {
                // Only single-origin children are re-pinned (see `global_only_children`);
                // the spec-read guard uses the saved address for every child, but writing
                // through it requires the reaching borrow to be a global of one memory.
                self.global_only_children
                    .get(child)
                    .map(|mem| (*child, mem.clone(), *saved))
            })
            .collect();
        if repins.is_empty() {
            return;
        }
        self.builder.set_loc_from_attr(attr_id);
        for (child, mem, saved) in repins {
            self.builder
                .emit_with(|id| Call(id, vec![], ProphecyRepin(mem), vec![child, saved], None));
        }
    }

    fn prophecy_instrumentation(&mut self, code_offset: CodeOffset, bytecode: &Bytecode) {
        use Bytecode::Call;
        use Operation::*;
        let Some(borrow_at) = self.borrow_annotation.get_borrow_info_at(code_offset) else {
            return;
        };
        let before = &borrow_at.before;
        let after = &borrow_at.after;
        let param_count = self.builder.get_target().get_parameter_count();

        // Borrow creation: keep the data-invariant bracketing (UnpackRef) and add the
        // eager lender update (ProphecyBorrow) for supported borrow forms.
        if let Call(attr_id, dests, op, srcs, _) = bytecode {
            // Lender node and edge for the eager update. Global borrows do their eager
            // memory update directly in the translator (the address is live there), so
            // they get no creation-time ProphecyBorrow; their global invariant is
            // asserted at the resolve marker instead.
            let edge = match op {
                BorrowLoc => Some((self.borrow_node(srcs[0]), BorrowEdge::Direct)),
                BorrowField(mid, sid, inst, field) => Some((
                    self.borrow_node(srcs[0]),
                    BorrowEdge::Field(mid.qualified_inst(*sid, inst.to_owned()), None, *field),
                )),
                BorrowVariantField(mid, sid, variants, inst, field) => Some((
                    self.borrow_node(srcs[0]),
                    BorrowEdge::Field(
                        mid.qualified_inst(*sid, inst.to_owned()),
                        Some(variants.clone()),
                        *field,
                    ),
                )),
                _ => None,
            };
            if matches!(
                op,
                BorrowLoc | BorrowField(..) | BorrowGlobal(..) | BorrowVariantField(..)
            ) {
                let node = BorrowNode::Reference(dests[0]);
                let in_use = after.is_in_use(&node);
                let ty = self
                    .builder
                    .get_target()
                    .get_local_type(dests[0])
                    .to_owned();
                if self.is_pack_ref_ty(&ty) && in_use {
                    self.builder.set_loc_from_attr(*attr_id);
                    self.builder
                        .emit_with(|id| Call(id, vec![], UnpackRef, vec![dests[0]], None));
                }
                if let Some((lender, edge)) = edge {
                    if in_use {
                        self.builder.set_loc_from_attr(*attr_id);
                        self.builder.emit_with(|id| {
                            Call(
                                id,
                                vec![],
                                ProphecyBorrow(lender, edge),
                                vec![dests[0]],
                                None,
                            )
                        });
                    }
                }
            }
        }

        // Reference reborrow `dest := src`: no marker is needed — the translator's
        // `Assign` arm performs the full reborrow itself (fresh prophecy for `dest`,
        // source relinked) for every `&mut`-to-`&mut` assignment present at this
        // stage. (The merged-exit moves `NormalizeExits` creates later are renames,
        // marked in `FunctionData::prophecy_rename_assigns`.)

        // A returned `&mut` parameter reaches the caller twice — as the result and as
        // its own out-value (the calling convention returns every `&mut` parameter) —
        // sharing one prophecy; resolving both would pin it to two values. The
        // translator's `Ret` handling re-borrows the returned parameter (fresh
        // prophecy for the result, out-value relinked), path-correctly per exit
        // branch, so no caller-side tie is emitted here.

        // Dying references: resolve the prophecy of every reference that dies here. A
        // single dying leaf carries its whole ancestor chain (`action.src` for each
        // write-back action is a body reference in the chain), so we resolve all of
        // them, deduped. Resolution emits `assume current == final`, so the order is
        // irrelevant. Mutable reference parameters are resolved at the call boundary
        // (C4), not here; they only get a data-invariant assertion.
        let attr_id = bytecode.get_attr_id();
        self.builder.set_loc_from_attr(attr_id);
        let mut to_resolve: Vec<usize> = vec![];
        let mut seen: BTreeSet<usize> = BTreeSet::new();
        let mut global_roots: Vec<(QualifiedInstId<StructId>, usize)> = vec![];
        // Local roots whose dying chain mutated them and whose type has a data invariant.
        // We assert that invariant on the resolved *root local* (not on the reference, as
        // the static model does): the root holds a well-defined value on every path — the
        // mutated branch's final value, or a not-taken branch's valid initial value — so a
        // conditional reborrow does not assert an invariant on an uninitialized reference.
        let mut pack_root_locals: Vec<usize> = vec![];
        // References directly borrowing a global resource (the `src` of a dying
        // chain's write-back into a `GlobalRoot`): at resolution the resource is
        // logically stored back, so its data invariant is asserted deeply on the
        // reference — as the static model does at its write-back — covering invariants
        // nested in resource fields, which the root-local check does not reach.
        let mut pack_refs: Vec<usize> = vec![];
        // Conditional-commit intermediates enclosing a dying leaf, whose own data
        // invariants may be uncheckable; rejected below unless covered by a pack
        // (a candidate may be another dying node's packed leaf).
        let mut reject_candidates: BTreeSet<usize> = BTreeSet::new();
        // Resolve at live-range exit, not gated on the borrow graph's in-use notion:
        // after its last use a reference can only have been relinked to a child's
        // fresh prophecy, so `assume v == f` chains the obligations even while
        // children live. Gating on in-use would drop the obligation of a lender
        // exiting before its child (the move-out/store-back pattern around a receiver
        // call), since the dying-node walk never revisits skipped nodes. Parameters
        // resolve at the call boundary instead.
        for node in before.live_nodes().iter() {
            if after.live_nodes().contains(node) {
                continue;
            }
            if let BorrowNode::Reference(idx) = node {
                if *idx >= param_count
                    && self
                        .builder
                        .get_target()
                        .get_local_type(*idx)
                        .is_mutable_reference()
                    && seen.insert(*idx)
                {
                    to_resolve.push(*idx);
                }
            }
        }
        // The `&mut` operands of a `Ret` at this offset. Their prophecies get the
        // variant-gated `ResolveReturn`, and a returned parameter (directly or through
        // assignment copies) has its data invariant asserted on the returned alias.
        let returned_refs: BTreeSet<usize> = if let Bytecode::Ret(_, rets) = bytecode {
            rets.iter()
                .filter(|&&t| {
                    self.builder
                        .get_target()
                        .get_local_type(t)
                        .is_mutable_reference()
                })
                .copied()
                .collect()
        } else {
            BTreeSet::new()
        };
        let returned_param_aliases: BTreeSet<usize> = returned_refs
            .iter()
            .filter(|&&t| t < param_count)
            .copied()
            .collect();
        for (node, ancestors) in before.dying_nodes(after) {
            let dying_idx = if let BorrowNode::Reference(idx) = node {
                if idx < param_count {
                    let target = self.builder.get_target();
                    let ty = target.get_local_type(idx);
                    // A returned parameter's invariant is asserted on the returned
                    // alias instead (see the returned-alias pack below).
                    if self.is_pack_ref_ty(ty) && !returned_param_aliases.contains(&idx) {
                        self.builder
                            .emit_with(|id| Call(id, vec![], PackRefDeep, vec![idx], None));
                    }
                    continue;
                }
                Some(idx)
            } else {
                None
            };
            // Computed on demand for conditional global commits; identical for
            // every chain of this dying node.
            let mut fork_base: Option<usize> = None;
            for chain in &ancestors {
                for action in chain {
                    if seen.insert(action.src) {
                        to_resolve.push(action.src);
                    }
                }
                // The action writing into the root (a local or global). Record the global
                // root for its global invariant, and the root local for its data invariant.
                if let Some(last) = chain.last() {
                    match &last.dst {
                        BorrowNode::GlobalRoot(mem) => {
                            // One commit marker per (memory, resource-writing ref):
                            // the ref anchors the marker to its defining `BorrowGlobal`
                            // site(s), where update invariants' `old(mem)` snapshots
                            // are placed. Simultaneously dying borrows of one memory
                            // get separate commits, each checked against its own
                            // pre-state.
                            let entry = (mem.clone(), last.src);
                            if !global_roots.contains(&entry) {
                                global_roots.push(entry);
                            }
                            // Assert data invariants on a reference that is
                            // initialized on every path reaching this death: for an
                            // unconditional (single-chain) borrow, the resource
                            // reference itself, as the static model does at its
                            // write-back. For a conditional multi-chain borrow, the
                            // highest reference below the borrow fork (the leaf and
                            // its unique-parent ancestors are definitely assigned;
                            // the fork's branches are not). Its deep pack covers
                            // every enclosing level up to the fork; a level above
                            // the fork is path-specific, so an own data invariant
                            // there cannot be asserted and is rejected below.
                            if Some(last.src) == dying_idx || ancestors.len() == 1 {
                                if !pack_refs.contains(&last.src) {
                                    pack_refs.push(last.src);
                                }
                            } else if let Some(leaf) = dying_idx {
                                let base = *fork_base.get_or_insert_with(|| {
                                    self.definite_pack_base(&ancestors, leaf)
                                });
                                if !pack_refs.contains(&base) {
                                    pack_refs.push(base);
                                }
                                let mut seen_base = false;
                                let mut encloses_base = false;
                                for action in chain {
                                    if !seen_base {
                                        if action.src == base {
                                            seen_base = true;
                                        } else {
                                            continue;
                                        }
                                    }
                                    if encloses_base {
                                        reject_candidates.insert(action.src);
                                    }
                                    if !matches!(action.edge, BorrowEdge::Direct) {
                                        encloses_base = true;
                                    }
                                }
                            }
                        },
                        BorrowNode::LocalRoot(local_idx) => {
                            let ty = self
                                .builder
                                .get_target()
                                .get_local_type(*local_idx)
                                .to_owned();
                            if self.is_pack_ref_ty(&ty) && !pack_root_locals.contains(local_idx) {
                                pack_root_locals.push(*local_idx);
                            }
                        },
                        BorrowNode::Reference(..) | BorrowNode::ReturnPlaceholder(..) => {},
                    }
                }
            }
        }
        // A returned `&mut` is finalized only when this function is verified
        // standalone — the translator gates `ResolveReturn` on the variant; inlined,
        // the caller resolves it where it dies. Chain intermediates are not `Ret`
        // operands and keep plain `Resolve`. Returning a `&mut` parameter (directly,
        // or via the compiler's return copies) ends the function's contribution to
        // that borrow, so its data invariant is asserted — on the returned alias,
        // which holds the real current value in both variants, not on the parameter,
        // whose slot was relinked to the alias's unresolved prophecy at the copy. A
        // returned *derived* borrow asserts nothing here; only the caller resolves it.
        for &ret in &returned_refs {
            let ty = self.builder.get_target().get_local_type(ret).to_owned();
            if self.is_pack_ref_ty(&ty)
                && self.is_direct_alias_of_param(before, ret)
                && !pack_refs.contains(&ret)
            {
                pack_refs.push(ret);
            }
        }
        for idx in to_resolve {
            let op = if returned_refs.contains(&idx) {
                ResolveReturn
            } else {
                Resolve
            };
            self.builder
                .emit_with(|id| Call(id, vec![], op, vec![idx], None));
        }
        for temp in reject_candidates {
            if !pack_refs.contains(&temp) {
                self.reject_unchecked_invariant(attr_id, temp);
            }
        }
        // After the prophecies are resolved, assert the data invariant of every root local
        // mutated through a body borrow (it now holds the final value) and of every
        // resolved global resource. Without this a data-invariant violation through a
        // body borrow would go undetected.
        for idx in pack_root_locals.into_iter().chain(pack_refs) {
            self.builder
                .emit_with(|id| Call(id, vec![], PackRefDeep, vec![idx], None));
        }
        // Mark each finalized global resource so the global invariant analysis asserts the
        // global invariant on the resolved value. The source anchors the marker to the
        // borrow it commits.
        for (mem, child) in global_roots {
            self.builder
                .emit_with(|id| Call(id, vec![], ProphecyCommitGlobal(mem), vec![child], None));
        }
    }

    /// The highest reference through which *every* borrow chain of the dying leaf
    /// passes — the definite pack base: it is initialized on every path reaching
    /// this death, and after the leaf's resolve it holds the committed value of its
    /// subtree, covering every level below the chains' divergence. Multiple chains
    /// arise both from genuinely conditional borrows and from sequential borrows
    /// whose temps the compiler reuses or copies; the chains re-converge exactly
    /// where the divergence sits below one lender. (Chains never list an in-use
    /// reference as a source, so the base is not mid-mutation.)
    fn definite_pack_base(&self, ancestors: &[Vec<WriteBackAction>], leaf: usize) -> usize {
        let mut common: Vec<usize> = ancestors
            .first()
            .map(|chain| chain.iter().map(|action| action.src).collect())
            .unwrap_or_default();
        for chain in ancestors.iter().skip(1) {
            let srcs: BTreeSet<usize> = chain.iter().map(|action| action.src).collect();
            common.retain(|temp| srcs.contains(temp));
        }
        common.last().copied().unwrap_or(leaf)
    }

    /// Reject a conditional global commit whose skipped intermediate's struct type
    /// itself declares data invariants: the intermediate reference is uninitialized
    /// on the paths that borrowed elsewhere, so there is no path-independent value
    /// to assert them on, and the leaf pack does not reach an enclosing struct's
    /// own invariant.
    fn reject_unchecked_invariant(&self, attr_id: AttrId, temp: usize) {
        let ty = self.builder.get_target().get_local_type(temp).to_owned();
        if let Type::Struct(mid, sid, _) = ty.skip_reference() {
            let env = self.builder.global_env();
            let struct_env = env.get_module(*mid).into_struct(*sid);
            if struct_env
                .get_spec()
                .conditions
                .iter()
                .any(|c| matches!(c.kind, ConditionKind::StructInvariant))
            {
                env.error(
                    &self.builder.get_loc(attr_id),
                    &format!(
                        "data invariant on `{}` cannot be checked here: the struct is \
                         mutated through a conditional borrow of a global resource, \
                         which the prophecy reference model does not yet support. Use \
                         `--path-refs` or restructure the borrow",
                        struct_env.get_full_name_str(),
                    ),
                );
            }
        }
    }

    fn borrow_node(&self, idx: usize) -> BorrowNode {
        let target = self.builder.get_target();
        if target.get_local_type(idx).is_reference() {
            BorrowNode::Reference(idx)
        } else {
            BorrowNode::LocalRoot(idx)
        }
    }

    fn is_pack_ref_ty(&self, ty: &Type) -> bool {
        Instrumenter::is_pack_ref_ty_(ty, self.builder.global_env())
    }
}

/// Reference temps every one of whose definitions is a `BorrowGlobal` of one memory,
/// mapped to that memory (see `ProphecyInstrumenter::global_only_children`).
fn compute_global_only_children(code: &[Bytecode]) -> BTreeMap<usize, QualifiedInstId<StructId>> {
    let mut origins: BTreeMap<usize, Option<QualifiedInstId<StructId>>> = BTreeMap::new();
    let mut record = |temp: usize, origin: Option<QualifiedInstId<StructId>>| {
        origins
            .entry(temp)
            .and_modify(|o| {
                if *o != origin {
                    *o = None;
                }
            })
            .or_insert(origin);
    };
    for bytecode in code {
        let origin =
            if let Bytecode::Call(_, _, Operation::BorrowGlobal(mid, sid, inst), _, _) = bytecode {
                Some(mid.qualified_inst(*sid, inst.clone()))
            } else {
                None
            };
        for (i, dest) in bytecode.dests().into_iter().enumerate() {
            record(dest, if i == 0 { origin.clone() } else { None });
        }
    }
    origins
        .into_iter()
        .filter_map(|(temp, origin)| origin.map(|mem| (temp, mem)))
        .collect()
}
