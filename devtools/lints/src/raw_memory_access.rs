// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! `raw_memory_access`: Rust's built-in raw memory accessors are banned;
//! raw memory access must go through `MemAddr` instead.
//!
//! The Mono Move VM confines raw pointer arithmetic and dereference to one
//! crate (`mono-move-mem`) so that every memory access has a single place to
//! attach checks. In every other VM crate, this lint rejects:
//!
//! - raw pointer types wherever they are written (`*const T`, `*mut T`, in
//!   signatures, fields, locals, casts, turbofish);
//! - method calls whose receiver is a raw pointer (`p.add(1)`, `p.read()`),
//!   which catches a pointer that is never spelled, e.g. `v.as_mut_ptr().add(1)`;
//! - the free-function forms: `core::ptr::{copy, copy_nonoverlapping, read,
//!   read_unaligned, write, write_unaligned, write_bytes, swap,
//!   swap_nonoverlapping}` and `core::slice::{from_raw_parts, from_raw_parts_mut}`.
//!
//! Which crates it applies to is configured in `dylint.toml`; the memory
//! crate and the allocator below it are exempt there.

use crate::scope::Scope;
use clippy_utils::diagnostics::span_lint_and_help;
use rustc_hir::{self as hir, def::Res, AmbigArg, Expr, ExprKind};
use rustc_lint::{LateContext, LateLintPass};
use rustc_middle::ty;
use rustc_session::{declare_lint, impl_lint_pass};
use rustc_span::Span;

declare_lint! {
    /// ### What it does
    ///
    /// Bans Rust's built-in raw memory accessors: raw pointer types, method
    /// calls on raw pointers, and the `core::ptr` /
    /// `core::slice::from_raw_parts` family.
    ///
    /// ### Why is this bad?
    ///
    /// Raw memory access in the VM must go through `mono_move_mem::MemAddr`
    /// so runtime checks can be attached in one place. A raw pointer
    /// anywhere else is an access the checks cannot see.
    ///
    /// ### Example
    ///
    /// ```rust,ignore
    /// let p: *mut u8 = buf.as_mut_ptr();
    /// unsafe { p.add(8).write(0) };
    /// ```
    ///
    /// Use instead:
    ///
    /// ```rust,ignore
    /// let a = MemAddr::from_mut(&mut buf);
    /// unsafe { a.add(8).store::<u8>(0) };
    /// ```
    pub RAW_MEMORY_ACCESS,
    Deny,
    "raw pointer or raw memory operation outside the memory crate"
}

pub struct RawMemoryAccess {
    scope: Scope,
    applies: Option<bool>,
    /// Spans already reported, so one expression is reported once even when
    /// several hooks see it (a cast both writes the type and is an expression).
    reported: Vec<Span>,
}

impl_lint_pass!(RawMemoryAccess => [RAW_MEMORY_ACCESS]);

impl RawMemoryAccess {
    pub fn new(scope: Scope) -> Self {
        Self {
            scope,
            applies: None,
            reported: Vec::new(),
        }
    }

    fn applies(&mut self, cx: &LateContext<'_>) -> bool {
        *self.applies.get_or_insert_with(|| self.scope.applies(cx))
    }

    fn report(&mut self, cx: &LateContext<'_>, span: Span, what: &str) {
        if self
            .reported
            .iter()
            .any(|r| r.contains(span) || span.contains(*r))
        {
            return;
        }
        self.reported.push(span);
        span_lint_and_help(
            cx,
            RAW_MEMORY_ACCESS,
            span,
            format!("{what} in a crate where raw memory access is banned"),
            None,
            "Rust's native raw memory accessors are banned here; raw memory access \
             must go through `mono_move_mem::MemAddr` instead",
        );
    }
}

/// The `ptr` / `slice` functions that perform raw memory operations, as
/// `(module, name)` under either `core` or `std`.
const RAW_FNS: &[(&str, &str)] = &[
    ("ptr", "copy"),
    ("ptr", "copy_nonoverlapping"),
    ("ptr", "read"),
    ("ptr", "read_unaligned"),
    ("ptr", "read_volatile"),
    ("ptr", "write"),
    ("ptr", "write_unaligned"),
    ("ptr", "write_volatile"),
    ("ptr", "write_bytes"),
    ("ptr", "swap"),
    ("ptr", "swap_nonoverlapping"),
    ("ptr", "replace"),
    ("slice", "from_raw_parts"),
    ("slice", "from_raw_parts_mut"),
];

/// Whether `def_id` is one of [`RAW_FNS`]: a free function whose definition
/// path is `core::<module>::<name>` or `std::<module>::<name>`.
fn is_raw_fn(cx: &LateContext<'_>, def_id: rustc_hir::def_id::DefId) -> Option<String> {
    let path = cx.tcx.def_path_str(def_id);
    let mut parts = path.split("::");
    let krate = parts.next()?;
    if krate != "core" && krate != "std" {
        return None;
    }
    let module = parts.next()?;
    let name = parts.next()?;
    if parts.next().is_some() {
        return None;
    }
    RAW_FNS
        .iter()
        .any(|(m, n)| *m == module && *n == name)
        .then(|| path)
}

impl<'tcx> LateLintPass<'tcx> for RawMemoryAccess {
    /// Raw pointer types as written.
    fn check_ty(&mut self, cx: &LateContext<'tcx>, ty: &'tcx hir::Ty<'tcx, AmbigArg>) {
        if !self.applies(cx) {
            return;
        }
        if let hir::TyKind::Ptr(_) = ty.kind {
            self.report(cx, ty.span, "raw pointer type");
        }
    }

    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx Expr<'tcx>) {
        if !self.applies(cx) {
            return;
        }
        match expr.kind {
            // `p.add(1)`, `p.read()`, `p.cast::<T>()`: a method whose
            // receiver is a raw pointer, however the pointer was obtained.
            ExprKind::MethodCall(segment, receiver, ..) => {
                let recv_ty = cx.typeck_results().expr_ty_adjusted(receiver).peel_refs();
                if let ty::RawPtr(..) = recv_ty.kind() {
                    self.report(
                        cx,
                        expr.span,
                        &format!("method `{}` called on a raw pointer", segment.ident),
                    );
                }
            },
            // `ptr::copy_nonoverlapping(..)`, `slice::from_raw_parts(..)`.
            ExprKind::Call(callee, _) => {
                if let ExprKind::Path(qpath) = &callee.kind
                    && let Res::Def(_, def_id) = cx.qpath_res(qpath, callee.hir_id)
                {
                    if let Some(path) = is_raw_fn(cx, def_id) {
                        self.report(cx, expr.span, &format!("call to `{path}`"));
                    }
                }
            },
            // `expr as *mut T` writes the type; `check_ty` reports that span,
            // so nothing to add here.
            _ => {},
        }
    }
}
