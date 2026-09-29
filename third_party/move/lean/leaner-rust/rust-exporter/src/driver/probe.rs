// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! Executable M0 capability inventory for the pinned Rustc Public revision.
//!
//! This is deliberately not an exchange format. It exercises the public
//! queries needed by the mapper and prints a deterministic diagnostic summary;
//! the only detached artifact will remain LeanerIR `RawUnit` JSON.

use rustc_public::{
    crate_def::CrateDef,
    mir::{AggregateKind, BinOp, Body, Rvalue, StatementKind, TerminatorKind, UnwindAction},
    ty::{AssocKind, GenericParamDefKind, RigidTy, Ty, TyKind},
};
use std::{
    collections::BTreeSet,
    fmt::{self, Display, Formatter},
};

#[derive(Default)]
pub(super) struct Report {
    bodies: usize,
    generic_bodies: usize,
    basic_blocks: usize,
    statements: usize,
    source_files: BTreeSet<String>,
    generic_parameters: usize,
    trait_declarations: usize,
    trait_methods: usize,
    trait_predicates: usize,
    trait_implementations: usize,
    direct_callees: BTreeSet<String>,
    switch_terminators: usize,
    drop_terminators: usize,
    cleanup_edges: usize,
    borrow_rvalues: usize,
    raw_pointer_types: usize,
    raw_pointer_operations: usize,
    unsupported_inline_asm: usize,
}

impl Display for Report {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "M0 probe bodies={} generic_bodies={} blocks={} statements={} \
             generic_parameters={} traits={} trait_methods={} trait_predicates={} \
             trait_impls={} direct_calls={} switches={} drops={} cleanup_edges={} \
             borrows={} raw_pointer_types={} raw_pointer_operations={} inline_asm={} \
             sources={:?} callees={:?}",
            self.bodies,
            self.generic_bodies,
            self.basic_blocks,
            self.statements,
            self.generic_parameters,
            self.trait_declarations,
            self.trait_methods,
            self.trait_predicates,
            self.trait_implementations,
            self.direct_callees.len(),
            self.switch_terminators,
            self.drop_terminators,
            self.cleanup_edges,
            self.borrow_rvalues,
            self.raw_pointer_types,
            self.raw_pointer_operations,
            self.unsupported_inline_asm,
            self.source_files,
            self.direct_callees,
        )
    }
}

pub(super) fn collect() -> Report {
    let mut report = Report::default();
    let local_crate = rustc_public::local_crate();

    for function in local_crate.fn_defs() {
        report.generic_parameters += function
            .generics_of()
            .params
            .iter()
            .filter(|parameter| !matches!(parameter.kind, GenericParamDefKind::Lifetime))
            .count();
    }

    for trait_def in local_crate.trait_decls() {
        let declaration = rustc_public::ty::TraitDef::declaration(&trait_def);
        report.trait_declarations += 1;
        report.trait_predicates += declaration.explicit_predicates_of().predicates.len();
        report.trait_methods += trait_def
            .associated_items()
            .iter()
            .filter(|item| matches!(item.kind, AssocKind::Fn { .. }))
            .count();
    }
    report.trait_implementations = local_crate.trait_impls().len();

    for item in rustc_public::all_local_items() {
        let Some(body) = item.body() else { continue };
        report.bodies += 1;
        report.generic_bodies += usize::from(item.requires_monomorphization());
        observe_source(&mut report, item.span().get_filename());
        observe_body(&mut report, &body);
    }

    report
}

fn observe_source(report: &mut Report, filename: String) {
    report.source_files.insert(filename);
}

fn observe_body(report: &mut Report, body: &Body) {
    report.basic_blocks += body.blocks.len();
    observe_source(report, body.span.get_filename());
    for declaration in body.locals() {
        observe_type(report, declaration.ty);
        observe_source(report, declaration.span.get_filename());
    }
    for block in &body.blocks {
        report.statements += block.statements.len();
        for statement in &block.statements {
            observe_source(report, statement.source_info.span.get_filename());
            if let StatementKind::Assign(_, rvalue) = &statement.kind {
                observe_rvalue(report, rvalue);
            }
        }
        observe_source(report, block.terminator.source_info.span.get_filename());
        observe_terminator(report, body, &block.terminator.kind);
    }
}

fn observe_type(report: &mut Report, ty: Ty) {
    match ty.kind() {
        TyKind::RigidTy(RigidTy::RawPtr(referent, _)) => {
            report.raw_pointer_types += 1;
            observe_type(report, referent);
        },
        TyKind::RigidTy(RigidTy::Ref(_, referent, _))
        | TyKind::RigidTy(RigidTy::Slice(referent))
        | TyKind::RigidTy(RigidTy::Pat(referent, _)) => observe_type(report, referent),
        TyKind::RigidTy(RigidTy::Array(element, _)) => observe_type(report, element),
        TyKind::RigidTy(RigidTy::Tuple(elements)) => {
            for element in elements {
                observe_type(report, element);
            }
        },
        TyKind::RigidTy(RigidTy::Adt(_, arguments))
        | TyKind::RigidTy(RigidTy::FnDef(_, arguments))
        | TyKind::RigidTy(RigidTy::Closure(_, arguments))
        | TyKind::RigidTy(RigidTy::Coroutine(_, arguments))
        | TyKind::RigidTy(RigidTy::CoroutineClosure(_, arguments))
        | TyKind::RigidTy(RigidTy::CoroutineWitness(_, arguments)) => {
            for argument in arguments.0 {
                if let Some(ty) = argument.ty() {
                    observe_type(report, *ty);
                }
            }
        },
        TyKind::RigidTy(
            RigidTy::Bool
            | RigidTy::Char
            | RigidTy::Int(_)
            | RigidTy::Uint(_)
            | RigidTy::Float(_)
            | RigidTy::Foreign(_)
            | RigidTy::Str
            | RigidTy::FnPtr(_)
            | RigidTy::Dynamic(_, _)
            | RigidTy::Never,
        )
        | TyKind::Alias(_, _)
        | TyKind::Param(_)
        | TyKind::Bound(_, _) => {},
    }
}

fn observe_rvalue(report: &mut Report, rvalue: &Rvalue) {
    match rvalue {
        Rvalue::AddressOf(_, _) => report.raw_pointer_operations += 1,
        Rvalue::Ref(_, _, _) | Rvalue::Reborrow(_, _, _) => report.borrow_rvalues += 1,
        Rvalue::Cast(_, _, target) if target.kind().is_raw_ptr() => {
            report.raw_pointer_operations += 1;
        },
        Rvalue::BinaryOp(BinOp::Offset, _, _)
        | Rvalue::CheckedBinaryOp(BinOp::Offset, _, _)
        | Rvalue::Aggregate(AggregateKind::RawPtr(_, _), _) => {
            report.raw_pointer_operations += 1;
        },
        Rvalue::Aggregate(_, _)
        | Rvalue::BinaryOp(_, _, _)
        | Rvalue::Cast(_, _, _)
        | Rvalue::CheckedBinaryOp(_, _, _)
        | Rvalue::CopyForDeref(_)
        | Rvalue::Discriminant(_)
        | Rvalue::Len(_)
        | Rvalue::Repeat(_, _)
        | Rvalue::ThreadLocalRef(_)
        | Rvalue::UnaryOp(_, _)
        | Rvalue::Use(_, _) => {},
    }
}

fn observe_terminator(report: &mut Report, body: &Body, terminator: &TerminatorKind) {
    match terminator {
        TerminatorKind::SwitchInt { .. } => report.switch_terminators += 1,
        TerminatorKind::Drop { unwind, .. } => {
            report.drop_terminators += 1;
            observe_unwind(report, unwind);
        },
        TerminatorKind::Call { func, unwind, .. } => {
            observe_unwind(report, unwind);
            if let Ok(ty) = func.ty(body.locals())
                && let TyKind::RigidTy(RigidTy::FnDef(definition, _)) = ty.kind()
            {
                report.direct_callees.insert(definition.name());
            }
        },
        TerminatorKind::Assert { unwind, .. } => observe_unwind(report, unwind),
        TerminatorKind::InlineAsm { unwind, .. } => {
            report.unsupported_inline_asm += 1;
            observe_unwind(report, unwind);
        },
        TerminatorKind::Goto { .. }
        | TerminatorKind::Resume
        | TerminatorKind::Abort
        | TerminatorKind::Return
        | TerminatorKind::Unreachable => {},
    }
}

fn observe_unwind(report: &mut Report, unwind: &UnwindAction) {
    if matches!(unwind, UnwindAction::Cleanup(_)) {
        report.cleanup_edges += 1;
    }
}
