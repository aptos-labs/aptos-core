// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Instrumentation of mutable references using the prophecy (RustHorn/Creusot) model.
//!
//! This processor is the prophecy-model counterpart of `MemoryInstrumentationProcessor`.
//! Under `--prophecy-refs` it replaces that processor in the pipeline (see
//! `pipeline_factory.rs`). Instead of inserting `WriteBack`/`IsParent` instructions that
//! propagate a mutation back up a statically known borrow path at end of scope, it inserts:
//!
//!   - a `ProphecyBorrow(lender, edge)` right after each borrow creation, which eagerly
//!     installs the child's prophecy (its final value) into the lender; and
//!   - a `Resolve(ref)` at each point a reference dies, which fulfills the prophecy
//!     (`assume current == final`).
//!
//! Combined with the path-free `$Mutation(current, final)` datatype, this flows a mutation
//! back to its lender without a static write-back path. For example, mutating a vector
//! element and overwriting it with a value, `let r = &mut v[i]; *r = 9`, lowers
//! (conceptually, with `v` standing for the lender vector) to:
//!
//! ```text
//!   var x;                          // fresh var
//!   r := $Mutation{curr: ReadVec(v, i), final: x}; // borrow &mut v[i]
//!   v := $UpdateVec(v, i, x);       // prophecy x eagerly installed at v[i]
//!   r := $Mutation{curr: 9, ..r};   // *r = 9
//!   assume r->final == r->curr;     // resolve at r's death: pins x == 9
//! ```
//!
//! Also see `doc/dev/prophecy_model.md`.
//!
//! Supported borrow forms: local-root borrows, field borrows on a reference, vector
//! indices (via the native `borrow_mut` template), and global roots. A global root is
//! updated eagerly in the translator and its invariant is asserted at a resolve-time
//! `ProphecyBorrow(GlobalRoot)` marker. Function-value borrows (`Invoke`) and the
//! inter-procedural boundary are handled in later commits; functions using them are not
//! yet expressible in this model and are exercised only by opt-in tests that avoid them.

use crate::memory_instrumentation::Instrumenter;
use move_binary_format::file_format::CodeOffset;
use move_model::{
    ast::{Exp, ExpData, Operation as AstOperation, RewriteResult},
    exp_generator::ExpGenerator,
    model::{FieldId, FunctionEnv, GlobalEnv, ModuleId, NodeId, QualifiedInstId, StructId},
    ty::Type,
};
use move_stackless_bytecode::{
    borrow_analysis::{BorrowAnnotation, BorrowInfo},
    function_data_builder::FunctionDataBuilder,
    function_target::FunctionData,
    function_target_pipeline::{FunctionTargetProcessor, FunctionTargetsHolder},
    stackless_bytecode::{BorrowEdge, BorrowNode, Bytecode, Operation},
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
            local_child: BTreeMap::new(),
            field_child: BTreeMap::new(),
            global_borrows: Vec::new(),
        };
        let code = std::mem::take(&mut instr.builder.data.code);
        for (offset, bytecode) in code.into_iter().enumerate() {
            instr.instrument(offset as CodeOffset, bytecode);
        }
        instr.builder.data
    }

    fn name(&self) -> String {
        "prophecy_instr".to_string()
    }
}

struct ProphecyInstrumenter<'a> {
    builder: FunctionDataBuilder<'a>,
    borrow_annotation: &'a BorrowAnnotation,
    /// Borrowed value local -> the reference borrowing it (`let c = &mut x`). Lets an
    /// in-code spec read the borrow's current value `*c` instead of the lender's prophecy.
    local_child: BTreeMap<usize, usize>,
    /// (parent reference temp, module, struct, field offset) -> the reference borrowing
    /// that field (`let c = &mut r.field`). Field-precise, so simultaneously borrowed
    /// fields of one reference stay distinct.
    field_child: BTreeMap<(usize, ModuleId, StructId, usize), usize>,
    /// Each global borrow `let c = &mut T[addr]`, as (memory, child reference, address
    /// temp). An in-code spec read of `T[a]` is guarded against these so it observes `*c`
    /// when `a` equals a borrowed address.
    global_borrows: Vec<(QualifiedInstId<StructId>, usize, usize)>,
}

impl ProphecyInstrumenter<'_> {
    fn instrument(&mut self, code_offset: CodeOffset, bytecode: Bytecode) {
        self.track_borrow(&bytecode);
        if matches!(bytecode, Bytecode::Prop(..))
            && !self
                .builder
                .data
                .loop_invariants
                .contains(&bytecode.get_attr_id())
        {
            // An in-code spec assertion must observe the current value of a borrowed local,
            // field, or global, not the havoced prophecy the eager update installed in the
            // lender. Rewrite the assertion to read the borrow's current value `*child`
            // directly. Loop invariants are excluded: they must stay consecutive at the loop
            // header and are handled by `LoopAnalysisProcessor`.
            self.spec_expr_rewrite(code_offset, bytecode);
            return;
        }
        if bytecode.is_branching()
            || matches!(bytecode, Bytecode::Call(_, _, Operation::Drop, _, _))
        {
            self.prophecy_instrumentation(code_offset, &bytecode);
            self.builder.emit(bytecode);
        } else {
            self.builder.emit(bytecode.clone());
            self.prophecy_instrumentation(code_offset, &bytecode);
        }
    }

    /// Record each active borrow so an in-code spec can later read the borrow's current
    /// value `*child` in place of the lender (which holds the prophecy). Consumed by
    /// `spec_expr_rewrite`.
    fn track_borrow(&mut self, bytecode: &Bytecode) {
        use Bytecode::Call;
        use Operation::*;
        if let Call(_, dests, op, srcs, _) = bytecode {
            if dests.is_empty() || srcs.is_empty() {
                return;
            }
            match op {
                BorrowLoc => {
                    self.local_child.insert(srcs[0], dests[0]);
                },
                BorrowField(mid, sid, _inst, field) => {
                    self.field_child
                        .insert((srcs[0], *mid, *sid, *field), dests[0]);
                },
                BorrowVariantField(mid, sid, _variants, _inst, field) => {
                    self.field_child
                        .insert((srcs[0], *mid, *sid, *field), dests[0]);
                },
                BorrowGlobal(mid, sid, inst) => {
                    let mem = mid.qualified_inst(*sid, inst.clone());
                    self.global_borrows.push((mem, dests[0], srcs[0]));
                },
                _ => {},
            }
        }
    }

    /// Emit an in-code spec assertion, rewriting reads of currently-borrowed locations to
    /// read the borrow's current value `*child` instead of the lender (which holds the
    /// prophecy). An expression rewrite only; it emits no additional instructions.
    fn spec_expr_rewrite(&mut self, code_offset: CodeOffset, bytecode: Bytecode) {
        let annotation = self.borrow_annotation;
        let Some(borrow_at) = annotation.get_borrow_info_at(code_offset) else {
            self.builder.emit(bytecode);
            return;
        };
        let before = &borrow_at.before;
        let new_bytecode = if let Bytecode::Prop(attr_id, kind, exp) = &bytecode {
            let new_exp = self.rewrite_prop_exp(exp.clone(), before);
            Bytecode::Prop(*attr_id, *kind, new_exp)
        } else {
            bytecode
        };
        self.builder.emit(new_bytecode.clone());
        // A reference may still die exactly at this assertion; resolve it as usual.
        self.prophecy_instrumentation(code_offset, &new_bytecode);
    }

    /// Rewrite a spec expression so each read of a borrowed location yields the current
    /// value `*child`. A spec names the lender (`x`, `r.field`, `T[addr]`), but while it is
    /// borrowed the lender holds the prophecy `f`, not the current value; the current value
    /// is only available through the borrowing reference. We therefore identify the borrowing
    /// reference for each lender the spec reads and redirect the read to `*child`. Reads
    /// inside `old(..)` / labeled memory are left untouched (they observe the pre-state
    /// snapshot, which the eager update never changed).
    fn rewrite_prop_exp(&self, exp: Exp, before: &BorrowInfo) -> Exp {
        let env = self.builder.global_env();
        let mut rewriter = |e: Exp| -> RewriteResult {
            enum Act {
                Fence,
                Replace(Exp),
                Descend,
            }
            let act = match e.as_ref() {
                ExpData::Call(_, AstOperation::Old, _)
                | ExpData::Call(_, AstOperation::Global(Some(_)), _)
                | ExpData::Call(_, AstOperation::Exists(Some(_)), _) => Act::Fence,
                ExpData::Temporary(id, x) => match self.local_current_value(env, before, *x, *id) {
                    Some(repl) => Act::Replace(repl),
                    None => Act::Descend,
                },
                ExpData::Call(_, AstOperation::Select(mid, sid, fid), args) => {
                    match self.field_current_value(env, before, *mid, *sid, *fid, args) {
                        Some(repl) => Act::Replace(repl),
                        None => Act::Descend,
                    }
                },
                ExpData::Call(_, AstOperation::SelectVariants(mid, sid, fids), args) => {
                    match fids.first().and_then(|fid| {
                        self.field_current_value(env, before, *mid, *sid, *fid, args)
                    }) {
                        Some(repl) => Act::Replace(repl),
                        None => Act::Descend,
                    }
                },
                ExpData::Call(id, AstOperation::Global(None), args) => {
                    match self.global_current_value(env, before, *id, args) {
                        Some(repl) => Act::Replace(repl),
                        None => Act::Descend,
                    }
                },
                _ => Act::Descend,
            };
            match act {
                Act::Fence => RewriteResult::Rewritten(e),
                Act::Replace(repl) => RewriteResult::Rewritten(repl),
                Act::Descend => RewriteResult::Unchanged(e),
            }
        };
        ExpData::rewrite(exp, &mut rewriter)
    }

    /// `*child` for a value local `x` (read at `node_id`) that is currently borrowed.
    fn local_current_value(
        &self,
        env: &GlobalEnv,
        before: &BorrowInfo,
        x: usize,
        node_id: NodeId,
    ) -> Option<Exp> {
        if env.get_node_type(node_id).is_reference() {
            return None;
        }
        let child = *self.local_child.get(&x)?;
        before
            .is_in_use(&BorrowNode::Reference(child))
            .then(|| self.builder.mk_temporary(child))
    }

    /// `*child` for a field `r.field` (`r` = `args[0]`) that is currently borrowed.
    fn field_current_value(
        &self,
        env: &GlobalEnv,
        before: &BorrowInfo,
        mid: ModuleId,
        sid: StructId,
        fid: FieldId,
        args: &[Exp],
    ) -> Option<Exp> {
        let ExpData::Temporary(_, r) = args.first()?.as_ref() else {
            return None;
        };
        let offset = env
            .get_module(mid)
            .into_struct(sid)
            .get_field(fid)
            .get_offset();
        let child = *self.field_child.get(&(*r, mid, sid, offset))?;
        before
            .is_in_use(&BorrowNode::Reference(child))
            .then(|| self.builder.mk_temporary(child))
    }

    /// Guard a resource read `T[addr]` (node `node_id`, address `args[0]`) against every
    /// live global borrow of `T`: it reads `*child` when `addr` equals a borrowed address
    /// and the underlying resource value otherwise.
    fn global_current_value(
        &self,
        env: &GlobalEnv,
        before: &BorrowInfo,
        node_id: NodeId,
        args: &[Exp],
    ) -> Option<Exp> {
        let value_ty = env.get_node_type(node_id);
        let Type::Struct(mid, sid, inst) = &value_ty else {
            return None;
        };
        let mem = mid.qualified_inst(*sid, inst.clone());
        let addr = args.first()?.clone();
        let mut result: Option<Exp> = None;
        for (m, child, borrowed_addr) in &self.global_borrows {
            if *m != mem || !before.is_in_use(&BorrowNode::Reference(*child)) {
                continue;
            }
            let base = result.take().unwrap_or_else(|| {
                ExpData::Call(node_id, AstOperation::Global(None), args.to_vec()).into_exp()
            });
            let cond = self
                .builder
                .mk_eq(addr.clone(), self.builder.mk_temporary(*borrowed_addr));
            let current = self.builder.mk_temporary(*child);
            let ite_id = self.builder.new_node(value_ty.clone(), None);
            result = Some(ExpData::IfElse(ite_id, cond, current, base).into_exp());
        }
        result
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

        // Reference reborrow `dest := src` (the `Assign` adds a `Direct` edge `src -> dest`
        // in the borrow graph). Emit the eager update for that edge, mirroring the static
        // model's `WriteBack(Reference(src), Direct)(dest)` in the dying chain. Without it,
        // the reborrowed temp `dest` carries the mutation while `src` (which shares `dest`'s
        // prophecy) is resolved with its stale value, forcing the shared prophecy to two
        // values and pruning the path (a silently lost mutation).
        if let Bytecode::Assign(attr_id, dest, src, _) = bytecode {
            let target = self.builder.get_target();
            if target.get_local_type(*dest).is_mutable_reference()
                && target.get_local_type(*src).is_mutable_reference()
                && after.is_in_use(&BorrowNode::Reference(*dest))
            {
                self.builder.set_loc_from_attr(*attr_id);
                let lender = BorrowNode::Reference(*src);
                self.builder.emit_with(|id| {
                    Call(
                        id,
                        vec![],
                        ProphecyBorrow(lender, BorrowEdge::Direct),
                        vec![*dest],
                        None,
                    )
                });
            }
        }

        // A `&mut` parameter returned from this function (directly as in
        // `fun identity(x): &mut _ { x }`, or conditionally as in
        // `fun pick(c, a, b): &mut _ { if c {a} else {b} }`) aliases the result at the caller:
        // the prover's calling convention also returns each `&mut` parameter as an out-value,
        // so the chosen parameter is handed back both as the result and as its own out-value,
        // sharing one prophecy. Resolving both would force that prophecy to two values. This is
        // reconciled where the returned value is materialized — the translator's `Ret`
        // handling re-borrows a returned `&mut` parameter, giving the result a fresh prophecy
        // and relinking the parameter's out-value to it. That site is path-correct (it sees the
        // parameter actually returned on each branch), which a path-free caller cannot
        // determine when several parameters qualify, so no caller-side tie is emitted here.

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
        let mut global_roots: Vec<BorrowNode> = vec![];
        // Local roots whose dying chain mutated them and whose type has a data invariant.
        // We assert that invariant on the resolved *root local* (not on the reference, as
        // the static model does): the root holds a well-defined value on every path — the
        // mutated branch's final value, or a not-taken branch's valid initial value — so a
        // conditional reborrow does not assert an invariant on an uninitialized reference.
        let mut pack_root_locals: Vec<usize> = vec![];
        // References that directly borrow a global resource (the `src` of a dying chain's
        // write-back into a `GlobalRoot`). When such a borrow is resolved the resource is
        // logically stored back, so its data invariant must hold. We assert it deeply on the
        // reference — exactly as the static model does at the resource write-back — which
        // covers invariants on values nested in the resource's fields (e.g. a struct kept in
        // a table), a case the root-local check below does not reach. The assert is a no-op
        // for invariant-free resources.
        let mut pack_refs: Vec<usize> = vec![];
        for (node, ancestors) in before.dying_nodes(after) {
            if let BorrowNode::Reference(idx) = node {
                if idx < param_count {
                    let target = self.builder.get_target();
                    let ty = target.get_local_type(idx);
                    if self.is_pack_ref_ty(ty) {
                        self.builder
                            .emit_with(|id| Call(id, vec![], PackRefDeep, vec![idx], None));
                    }
                    continue;
                }
            }
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
                        BorrowNode::GlobalRoot(..) => {
                            if !global_roots.contains(&last.dst) {
                                global_roots.push(last.dst.clone());
                            }
                            if ancestors.len() == 1 && !pack_refs.contains(&last.src) {
                                pack_refs.push(last.src);
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
        // A returned `&mut` is, by Move's rules, derived from a `&mut` parameter; it is not
        // committed in the inlined body (the caller resolves it where it dies, after the
        // caller's own write). It is finalized only when verifying this function standalone
        // — the translator gates `ResolveReturn` on the variant. Intermediate links in a
        // returned ref's borrow chain are not `Ret` operands, so they keep plain `Resolve`.
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
        for idx in to_resolve {
            let op = if returned_refs.contains(&idx) {
                ResolveReturn
            } else {
                Resolve
            };
            self.builder
                .emit_with(|id| Call(id, vec![], op, vec![idx], None));
        }
        // After the prophecies are resolved, assert the data invariant of every root local
        // mutated through a body borrow (it now holds the final value). Without this a
        // data-invariant violation through a body borrow would go undetected.
        for local_idx in pack_root_locals {
            self.builder
                .emit_with(|id| Call(id, vec![], PackRefDeep, vec![local_idx], None));
        }
        for ref_idx in pack_refs {
            self.builder
                .emit_with(|id| Call(id, vec![], PackRefDeep, vec![ref_idx], None));
        }
        // Mark each finalized global resource so the global invariant analysis asserts the
        // global invariant on the resolved value.
        for root in global_roots {
            self.builder.emit_with(|id| {
                Call(
                    id,
                    vec![],
                    ProphecyBorrow(root, BorrowEdge::Direct),
                    vec![],
                    None,
                )
            });
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
