// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Check: a function that holds a cell has no parameter that could carry a
//! callback from its caller.

use super::{
    body::Callee,
    types::{erased_kind, find_in_ty, type_of},
    util::report,
    Analysis,
};
use rustc_hir::{def::DefKind, def_id::LocalDefId};
use rustc_middle::ty::{self, Ty};

impl<'tcx> Analysis<'_, 'tcx> {
    /// Check: a function that holds a cell has no parameter, `self` and
    /// closure captures included, of generic, trait-object or opaque type.
    /// Such a value could call back into the struct.
    pub(super) fn check_signature(&self, owner: LocalDefId) {
        let tcx = self.cx.tcx;
        let def_id = owner.to_def_id();
        if self.bodies[&owner].sites.is_empty() {
            return;
        }
        let inputs = match tcx.def_kind(def_id) {
            DefKind::Fn | DefKind::AssocFn => {
                let sig = tcx.fn_sig(def_id).skip_binder().skip_binder();
                sig.inputs().to_vec()
            },
            DefKind::Closure => {
                let ty::Closure(_, args) = type_of(tcx, def_id).kind() else {
                    return;
                };
                // What a closure captures comes from outside, like a parameter.
                let closure = args.as_closure();
                let mut inputs = closure.sig().skip_binder().inputs().to_vec();
                inputs.extend(closure.upvar_tys());
                inputs
            },
            _ => return,
        };
        for input in inputs {
            if let Some(what) = self.opaque_in(input) {
                report(
                    self.cx,
                    tcx.local_def_id_to_hir_id(owner),
                    tcx.def_span(owner),
                    format!(
                        "in {}, a parameter has {what}, which could carry a route back to the \
                         struct while a cell is held",
                        Callee::Local(owner).describe(tcx)
                    ),
                    &[],
                );
            }
        }
    }

    /// What a value of `ty` can contain that the lint cannot see through, if
    /// anything: a type parameter, `dyn` or `impl`.
    pub(super) fn opaque_in(&self, ty: Ty<'tcx>) -> Option<String> {
        let tcx = self.cx.tcx;
        find_in_ty(tcx, self.cells, false, ty, |ty| {
            if matches!(ty.kind(), ty::Param(_)) {
                Some(format!("the generic type `{ty}`"))
            } else {
                erased_kind(tcx, ty).map(str::to_string)
            }
        })
        .unwrap_or_else(|_| Some(format!("a type too large to analyse (`{ty}`)")))
    }
}
