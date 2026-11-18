// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! The checks on calls, which are the fourth pass: the struct is never
//! hidden from the lint, and a function that holds a cell makes no call
//! that could borrow it again.

use super::{
    body::{BodyInfo, Callee},
    types::{is_erased, type_of},
    util::report,
    Analysis,
};
use rustc_hir::def::DefKind;
use rustc_middle::ty::{self, Ty};

impl<'tcx> Analysis<'_, 'tcx> {
    /// The checks on each call of one body.
    pub(super) fn check_calls(&self, info: &BodyInfo<'tcx>) {
        let tcx = self.cx.tcx;
        let held = &info.sites;
        let held_names = || self.cells.names(tcx, held);
        let held_notes = || {
            held.iter()
                .map(|(cell, span)| {
                    (
                        *span,
                        format!("{} is reborrowed here", self.cells.name(tcx, *cell)),
                    )
                })
                .collect::<Vec<_>>()
        };
        for call in &info.calls {
            // Check, in any body: the struct is not hidden from the lint,
            // by a closure argument or by a generic or `impl` parameter. (A
            // trait object is caught where the value is coerced.)
            let bad_closure = call.closure_args.iter().any(|c| {
                self.may_borrow.get(c).is_some_and(|set| !set.is_empty())
                    || self.cells.reaches(tcx, type_of(tcx, c.to_def_id()))
            });
            if bad_closure {
                report(
                    self.cx,
                    call.hir_id,
                    call.span,
                    "a closure that can reach a cell struct is passed as an argument; call it \
                     directly or pass the components it needs instead"
                        .to_string(),
                    &[],
                );
                continue;
            }
            if let Callee::Local(callee) = call.callee
                && matches!(tcx.def_kind(callee), DefKind::Fn | DefKind::AssocFn)
            {
                let sig = tcx.fn_sig(callee).skip_binder().skip_binder();
                let hidden = call.reaching.iter().any(|operand| {
                    sig.inputs()
                        .get(operand.position)
                        .is_some_and(|declared| self.hidden_in(*declared, operand.passed_as))
                });
                if hidden {
                    report(
                        self.cx,
                        call.hir_id,
                        call.span,
                        format!(
                            "a value that can reach a cell struct is passed to a generic or \
                             trait-object parameter of {}; what the callee calls on it cannot be \
                             followed",
                            call.callee.describe(tcx)
                        ),
                        &[],
                    );
                    continue;
                }
            }
            if held.is_empty() {
                continue;
            }
            // Checks while a cell is held: the callee may not borrow a held
            // cell; opaque code is not handed the struct; no call on a trait
            // object, type parameter or function value, unless the receiver
            // is a field of the struct.
            let problem = match &call.callee {
                Callee::Local(callee) => {
                    let overlap = held
                        .iter()
                        .filter(|(cell, _)| {
                            self.may_borrow
                                .get(callee)
                                .is_some_and(|s| s.contains(cell))
                        })
                        .map(|(cell, _)| self.cells.name(tcx, *cell))
                        .collect::<Vec<_>>();
                    (!overlap.is_empty()).then(|| {
                        format!(
                            "{} may reborrow {}, which this function already holds",
                            call.callee.describe(tcx),
                            overlap.join(", ")
                        )
                    })
                },
                Callee::External(_) | Callee::Dynamic(_) | Callee::Value if call.leaks() => {
                    Some(format!(
                        "a cell struct reaches {}, which has no body in this crate, while {} is \
                         held",
                        call.callee.describe(tcx),
                        held_names()
                    ))
                },
                Callee::Dynamic(_) if !call.through_field => Some(format!(
                    "{} is called on a type parameter or trait object while {} is held; the \
                     callee may reach the struct through its caller",
                    call.callee.describe(tcx),
                    held_names()
                )),
                Callee::Value if !call.through_field => Some(format!(
                    "a call through a function value while {} is held; the callee may reach the \
                     struct through its caller",
                    held_names()
                )),
                Callee::External(_) | Callee::Dynamic(_) | Callee::Value => None,
            };
            if let Some(problem) = problem {
                report(self.cx, call.hir_id, call.span, problem, &held_notes());
            }
        }
    }

    /// Whether an operand of type `actual`, passed to a parameter declared
    /// as `declared`, is hidden from the lint: the part of it that reaches
    /// the struct lines up with a type parameter, `dyn` or `impl` in
    /// `declared`. A type parameter needs no trait bound to count: the
    /// callee can still hand the value to a function value.
    pub(super) fn hidden_in(&self, declared: Ty<'tcx>, actual: Ty<'tcx>) -> bool {
        let tcx = self.cx.tcx;
        match (declared.kind(), actual.kind()) {
            (ty::Param(_), _) => self.cells.reaches(tcx, actual),
            (ty::Ref(_, declared, _), ty::Ref(_, actual, _))
            | (ty::RawPtr(declared, _), ty::RawPtr(actual, _))
            | (ty::Array(declared, _), ty::Array(actual, _))
            | (ty::Slice(declared), ty::Slice(actual)) => self.hidden_in(*declared, *actual),
            (ty::Adt(declared_adt, declared_args), ty::Adt(actual_adt, actual_args))
                if declared_adt.did() == actual_adt.did() =>
            {
                let through_arg = declared_args
                    .types()
                    .zip(actual_args.types())
                    .any(|(declared, actual)| self.hidden_in(declared, actual));
                // The struct is reached through a field that depends on a
                // type parameter without being one, such as `F::Out`.
                through_arg
                    || (is_erased(tcx, declared)
                        && self.cells.reaches(tcx, actual)
                        && !self.cells.reaches(tcx, declared))
            },
            (ty::Tuple(declared), ty::Tuple(actual)) if declared.len() == actual.len() => declared
                .iter()
                .zip(actual.iter())
                .any(|(declared, actual)| self.hidden_in(declared, actual)),
            // Anything else: hidden if `declared` contains anything erased.
            _ => is_erased(tcx, declared) && self.cells.reaches(tcx, actual),
        }
    }
}
