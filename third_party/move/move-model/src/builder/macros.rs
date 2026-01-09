// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Module for expanding macros, as `assert!(cond, code)`. This are expanded to
//! the input AST before type checking.  We also allow `assert!(cond)`, for Move 2,
//! which generates the "well-known" abort code `UNSPECIFIED_ABORT_CODE`.

use crate::{
    builder::exp_builder::ExpTranslator, well_known::UNSPECIFIED_ABORT_CODE, LanguageVersion,
};
use legacy_move_compiler::{
    expansion::ast::{Address, Exp, Exp_, LValue, LValue_, ModuleAccess_, ModuleIdent_, Value_},
    parser::ast::{BinOp_, CallKind, ModuleName},
    shared::NumericalAddress,
};
use move_core_types::account_address::AccountAddress;
use move_ir_types::location::{sp, Loc, Spanned};
use move_symbol_pool::Symbol;
use std::fmt::Display;

/// Maximum number of arguments we can format using `std::string_utils::format<N>`.
const MAX_ARGS: usize = 5;

#[derive(Copy, Clone)]
enum AssertKind {
    Eq,
    Ne,
}

impl AssertKind {
    fn op(&self) -> &str {
        match self {
            AssertKind::Eq => "==",
            AssertKind::Ne => "!=",
        }
    }
}

impl Display for AssertKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AssertKind::Eq => write!(f, "assert_eq!"),
            AssertKind::Ne => write!(f, "assert_ne!"),
        }
    }
}

impl From<AssertKind> for BinOp_ {
    fn from(kind: AssertKind) -> Self {
        match kind {
            AssertKind::Eq => Self::Eq,
            AssertKind::Ne => Self::Neq,
        }
    }
}

impl ExpTranslator<'_, '_, '_> {
    pub fn expand_macro(&self, loc: Loc, name: &str, args: &Spanned<Vec<Exp>>) -> Exp {
        // Currently, there are only built-in macros, and no user definable ones.
        let expansion_ = match name {
            "assert" => self.expand_assert(loc, args),
            "assert_eq" => self.expand_assert_eq(loc, args),
            "assert_ne" => self.expand_assert_ne(loc, args),
            _ => {
                self.error(&self.to_loc(&loc), &format!("unknown macro `{}`", name));
                Exp_::UnresolvedError
            },
        };
        sp(loc, expansion_)
    }

    /// The macro `assert!` has the following forms:
    ///
    /// Single argument:
    /// ```move
    /// assert!(cond)
    /// ```
    /// expands to:
    /// ```move
    /// if (cond) {
    ///     ()
    /// } else {
    ///     abort <default_abort_code>
    /// }
    /// ```
    ///
    /// Two arguments:
    /// ```move
    /// assert!(cond, exp)
    /// ```
    /// expands to:
    /// ```move
    /// if (cond) {
    ///     ()
    /// } else {
    ///     abort exp
    /// }
    /// ```
    ///
    /// More than two arguments:
    /// ```move
    /// assert!(cond, fmt, arg1, ..., argN) // 1 ≤ N ≤ 4
    /// ```
    /// expands to:
    /// ```move
    /// if (cond) {
    ///     ()
    /// } else {
    ///     abort string::into_bytes(string_utils::format<N>(&fmt, arg1, ..., argN))
    /// }
    /// ```
    fn expand_assert(&self, loc: Loc, args: &Spanned<Vec<Exp>>) -> Exp_ {
        if args.value.is_empty() {
            self.error(
                &self.to_loc(&args.loc),
                "Macro `assert!` must have at least one argument",
            );
            return Exp_::UnresolvedError;
        }

        let cond = &args.value[0];
        let rest = &args.value[1..];

        let e = match rest.len() {
            0 => {
                // assert!(cond)
                self.check_language_version(
                    &self.to_loc(&loc),
                    "single-argument `assert!` macro",
                    LanguageVersion::V2_0,
                );
                sp(
                    loc,
                    Exp_::Value(sp(loc, Value_::U64(UNSPECIFIED_ABORT_CODE))),
                )
            },
            1 => {
                // assert!(cond, exp)
                rest[0].clone()
            },
            n if n <= MAX_ARGS => {
                // assert!(cond, fmt, arg1, ..., argN)
                self.check_language_version(
                    &self.to_loc(&loc),
                    "`assert!` macro with string formatting",
                    LanguageVersion::V2_4,
                );
                Self::into_bytes(loc, Self::format(loc, rest[0].clone(), rest[1..].to_vec()))
            },
            _ => {
                self.error(
                    &self.to_loc(&args.loc),
                    &format!(
                        "Macro `assert!` cannot take more than {} arguments",
                        MAX_ARGS + 1
                    ),
                );
                return Exp_::UnresolvedError;
            },
        };

        Exp_::IfElse(
            Box::new(cond.clone()),
            Box::new(sp(loc, Exp_::Unit { trailing: false })),
            Box::new(sp(loc, Exp_::Abort(Box::new(e)))),
        )
    }

    /// The macro `assert_eq!` has the following forms:
    ///
    /// Two arguments:
    /// ```move
    /// assert_eq!(left, right)
    /// ```
    /// expands to:
    /// ```move
    /// match ((left, right)) {
    ///     (_left, _right) => {
    ///         if (_left == _right) {
    ///             ()
    ///         } else {
    ///             abort string::into_bytes(string_utils::format2(<assertion_failed_message>, _left, _right))
    ///         }
    ///     }
    /// }
    /// ```
    ///
    /// Three arguments:
    /// ```move
    /// assert_eq!(left, right, message)
    /// ```
    /// expands to:
    /// ```move
    /// match ((left, right)) {
    ///     (_left, _right) => {
    ///         if (_left == _right) {
    ///             ()
    ///         } else {
    ///             abort string::into_bytes(string_utils::format3(<assertion_failed_message>, string::utf8(message), _left, _right))
    ///         }
    ///     }
    /// }
    /// ```
    ///
    /// More than three arguments:
    /// ```move
    /// assert_eq!(left, right, fmt, arg1, ..., argN) // for 1 ≤ N ≤ 4
    /// ```
    /// expands to:
    /// ```move
    /// match ((left, right)) {
    ///     (_left, _right) => {
    ///         if (_left == _right) {
    ///             ()
    ///         } else {
    ///             abort string::into_bytes(string_utils::format3(<assertion_failed_message>, string_utils::format<N>(&fmt, arg1, ..., argN), _left, _right))
    ///         }
    ///     }
    /// }
    /// ```
    fn expand_assert_eq(&self, loc: Loc, args: &Spanned<Vec<Exp>>) -> Exp_ {
        self.expand_assert_inner(loc, args, AssertKind::Eq)
    }

    /// Same as `assert_eq!` but uses `!=` instead of `==`.
    fn expand_assert_ne(&self, loc: Loc, args: &Spanned<Vec<Exp>>) -> Exp_ {
        self.expand_assert_inner(loc, args, AssertKind::Ne)
    }

    fn expand_assert_inner(&self, loc: Loc, args: &Spanned<Vec<Exp>>, kind: AssertKind) -> Exp_ {
        self.check_language_version(
            &self.to_loc(&loc),
            &format!("`{}` macro", kind),
            LanguageVersion::V2_4,
        );

        if args.value.len() < 2 {
            self.error(
                &self.to_loc(&args.loc),
                &format!("Macro `{}` must have at least two arguments", kind),
            );
            return Exp_::UnresolvedError;
        }

        let operands = &args.value[0..2];
        let rest = &args.value[2..];

        let discriminator = sp(loc, Exp_::ExpList(operands.to_vec()));

        let (left_lvalue, left) = Self::make_binding(loc, "_left");
        let (right_lvalue, right) = Self::make_binding(loc, "_right");

        let lvalues = sp(loc, vec![left_lvalue, right_lvalue]);

        let cond = sp(
            loc,
            Exp_::BinopExp(
                Box::new(left.clone()),
                sp(loc, BinOp_::from(kind)),
                Box::new(right.clone()),
            ),
        );

        let e = match rest.len() {
            0 => {
                // assert_eq!(left, right)
                let assertion_failed_message = Self::assertion_failed_message(loc, kind, false);
                Self::into_bytes(
                    loc,
                    Self::format(loc, assertion_failed_message, vec![left, right]),
                )
            },
            1 => {
                // assert_eq!(left, right, bytes)
                let assertion_failed_message = Self::assertion_failed_message(loc, kind, true);
                let message = Self::utf8(loc, rest[0].clone());
                Self::into_bytes(
                    loc,
                    Self::format(loc, assertion_failed_message, vec![message, left, right]),
                )
            },
            n if n <= MAX_ARGS => {
                // assert_eq!(left, right, fmt, arg1, ..., argN)
                let assertion_failed_message = Self::assertion_failed_message(loc, kind, true);
                let message = Self::format(loc, rest[0].clone(), rest[1..].to_vec());
                Self::into_bytes(
                    loc,
                    Self::format(loc, assertion_failed_message, vec![message, left, right]),
                )
            },
            _ => {
                self.error(
                    &self.to_loc(&args.loc),
                    &format!(
                        "Macro `{}` cannot take more than {} arguments",
                        kind,
                        MAX_ARGS + 2,
                    ),
                );
                return Exp_::UnresolvedError;
            },
        };

        let assert = sp(
            loc,
            Exp_::IfElse(
                Box::new(cond),
                Box::new(sp(loc, Exp_::Unit { trailing: false })),
                Box::new(sp(loc, Exp_::Abort(Box::new(e)))),
            ),
        );

        Exp_::Match(Box::new(discriminator), vec![sp(
            loc,
            (lvalues, None, assert),
        )])
    }

    /// Calls `std::string_utils::format<N>(&fmt, arg1, ..., argN)` for 1 ≤ N ≤ 4.
    fn format(loc: Loc, fmt: Exp, args: Vec<Exp>) -> Exp {
        let n = args.len();
        debug_assert!((1..MAX_ARGS).contains(&n));
        let borrow_fmt = sp(loc, Exp_::Borrow(false, Box::new(fmt)));
        Self::call_std_function(
            loc,
            "string_utils",
            format!("format{}", n),
            std::iter::once(borrow_fmt).chain(args).collect(),
        )
    }

    /// Calls `std::string::into_bytes(s)`.
    fn into_bytes(loc: Loc, s: Exp) -> Exp {
        Self::call_std_function(loc, "string", "into_bytes", vec![s])
    }

    /// Calls `std::string::utf8(bytes)`.
    fn utf8(loc: Loc, bytes: Exp) -> Exp {
        Self::call_std_function(loc, "string", "utf8", vec![bytes])
    }

    /// Calls a standard library function `std::module_name::function_name(args)`.
    fn call_std_function(
        loc: Loc,
        module_name: impl Into<Symbol>,
        function_name: impl Into<Symbol>,
        args: Vec<Exp>,
    ) -> Exp {
        let address = sp(
            loc,
            NumericalAddress::from_account_address(AccountAddress::ONE),
        );
        let module_name = sp(loc, module_name.into());
        let module_ident = sp(
            loc,
            ModuleIdent_::new(Address::Numerical(None, address), ModuleName(module_name)),
        );

        let function_name = sp(loc, function_name.into());

        let module_access = sp(
            loc,
            ModuleAccess_::ModuleAccess(module_ident, function_name, None),
        );

        sp(
            loc,
            Exp_::Call(module_access, CallKind::Regular, None, sp(loc, args)),
        )
    }

    /// Creates a binding for a fresh local variable and an expression to read it.
    ///
    /// Returns:
    /// - an `LValue` to be used in a pattern (e.g. `match` binding), and
    /// - an `Exp` to refer to the bound variable in expressions.
    fn make_binding(loc: Loc, symbol: &str) -> (LValue, Exp) {
        let module_access = sp(loc, ModuleAccess_::Name(sp(loc, symbol.into())));
        let lvalue = sp(loc, LValue_::Var(module_access.clone(), None));
        let exp = sp(loc, Exp_::Name(module_access, None));
        (lvalue, exp)
    }

    fn assertion_failed_message(loc: Loc, kind: AssertKind, args: bool) -> Exp {
        let op = kind.op();
        let str = if args {
            format!("assertion `left {op} right` failed: {{}}\n  left: {{}}\n right: {{}}")
        } else {
            format!("assertion `left {op} right` failed\n  left: {{}}\n right: {{}}")
        };
        Self::string_value(loc, str)
    }

    fn string_value(loc: Loc, str: String) -> Exp {
        sp(
            loc,
            Exp_::Value(sp(loc, Value_::Bytearray(str.into_bytes()))),
        )
    }
}
