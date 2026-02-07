// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Module for expanding macros. Supported macros:
//! - `assert!`
//! - `assert_eq!`
//! - `assert_ne!`
//!
//! These macros are expanded to the input AST before type checking.
//!
//! ## `assert!` macro
//! Supported forms:
//! - `assert!(cond)` - aborts with well-known code `UNSPECIFIED_ABORT_CODE`
//! - `assert!(cond, exp)` - aborts with provided expression (either u64 or vector<u8>)
//! - `assert!(cond, fmt, arg1, ..., argN)` - aborts with formatted message (1 ≤ N ≤ 4)
//!
//! ## `assert_eq!` and `assert_ne!` macros
//! Supported forms:
//! - `assert_eq!(left, right)` - aborts with default message
//! - `assert_eq!(left, right, message)` - aborts with custom message
//! - `assert_eq!(left, right, fmt, arg1, ..., argN)` - aborts with formatted message (1 ≤ N ≤ 4)
//! - `assert_ne!` supports the same forms as `assert_eq!`
//!
//! ## Version requirements
//! - `assert!(cond)` requires Move 2
//! - `assert!(cond, fmt, arg1, ..., argN)` requires Move 2.4
//! - `assert_eq!` and `assert_ne!` require Move 2.4

use crate::{
    builder::exp_builder::ExpTranslator,
    well_known::{
        INTO_BYTES_FUNCTION_NAME, STRING_MODULE, STRING_UTILS_MODULE, UNSPECIFIED_ABORT_CODE,
        UTF8_FUNCTION_NAME,
    },
    LanguageVersion,
};
use legacy_move_compiler::{
    expansion::ast::{
        Address, Exp, Exp_, LValue, LValue_, ModuleAccess_, ModuleIdent_, SequenceItem_, Value_,
    },
    parser::ast::{BinOp_, CallKind, ModuleName},
    shared::NumericalAddress,
};
use move_core_types::account_address::AccountAddress;
use move_ir_types::location::{sp, Loc, Spanned};
use std::{collections::VecDeque, fmt::Display};

/// Maximum total number of arguments for string formatting, including the format string itself.
/// Note that `string_utils::format<N>` takes N + 1 arguments: the format string + N format arguments.
/// Currently, the last supported function is `format4`, which takes 5 total arguments, hence this is 5.
const MAX_FORMAT_ARGS: usize = 5;

#[derive(Copy, Clone)]
enum AssertKind {
    Eq,
    Ne,
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
    /// Note that, while the macro does not explicitly enforce this constraint, this will only
    /// compile when `exp` is a `u64` (an abort code) or a `vector<u8>` (an abort message).
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

        let abort_arg = match rest.len() {
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
                if check_string_literal(&rest[0]).is_some() {
                    self.check_format_string(&rest[0], 0);
                }
                rest[0].clone()
            },
            n if n <= MAX_FORMAT_ARGS => {
                // assert!(cond, fmt, arg1, ..., argN)
                self.check_language_version(
                    &self.to_loc(&loc),
                    "`assert!` macro with string formatting",
                    LanguageVersion::V2_4,
                );
                self.check_format_string(&rest[0], n - 1);
                self.call_into_bytes(
                    loc,
                    self.call_format(loc, rest[0].clone(), rest[1..].to_vec()),
                )
            },
            _ => {
                self.error(
                    &self.to_loc(&args.loc),
                    &format!(
                        "Macro `assert!` cannot take more than {} arguments",
                        MAX_FORMAT_ARGS + 1
                    ),
                );
                return Exp_::UnresolvedError;
            },
        };

        Exp_::IfElse(
            Box::new(cond.clone()),
            Box::new(sp(loc, Exp_::Unit { trailing: false })),
            Box::new(sp(loc, Exp_::Abort(Box::new(abort_arg)))),
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
    /// let ($left, $right) = (left, right);
    /// if ($left == $right) {
    ///     ()
    /// } else {
    ///     abort string::into_bytes(string_utils::format2(<assertion_failed_message>, $left, $right))
    /// }
    /// ```
    ///
    /// Three arguments:
    /// ```move
    /// assert_eq!(left, right, message)
    /// ```
    /// expands to:
    /// ```move
    /// let ($left, $right) = (left, right);
    /// if ($left == $right) {
    ///     ()
    /// } else {
    ///     abort string::into_bytes(string_utils::format3(<assertion_failed_message>, string::utf8(message), $left, $right))
    /// }
    /// ```
    ///
    /// More than three arguments:
    /// ```move
    /// assert_eq!(left, right, fmt, arg1, ..., argN) // for 1 ≤ N ≤ 4
    /// ```
    /// expands to:
    /// ```move
    /// let ($left, $right) = (left, right);
    /// if ($left == $right) {
    ///     ()
    /// } else {
    ///     abort string::into_bytes(string_utils::format3(<assertion_failed_message>, string_utils::format<N>(&fmt, arg1, ..., argN), $left, $right))
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

        let (left_lvalue, left) = Self::make_binding(loc, "$left");
        let (right_lvalue, right) = Self::make_binding(loc, "$right");

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
                self.call_into_bytes(
                    loc,
                    self.call_format(loc, assertion_failed_message, vec![left, right]),
                )
            },
            1 => {
                // assert_eq!(left, right, bytes)
                let assertion_failed_message = Self::assertion_failed_message(loc, kind, true);
                let message = self.call_utf8(loc, rest[0].clone());
                self.call_into_bytes(
                    loc,
                    self.call_format(loc, assertion_failed_message, vec![message, left, right]),
                )
            },
            n if n <= MAX_FORMAT_ARGS => {
                // assert_eq!(left, right, fmt, arg1, ..., argN)
                let assertion_failed_message = Self::assertion_failed_message(loc, kind, true);
                self.check_format_string(&rest[0], n - 1);
                let message = self.call_format(loc, rest[0].clone(), rest[1..].to_vec());
                self.call_into_bytes(
                    loc,
                    self.call_format(loc, assertion_failed_message, vec![message, left, right]),
                )
            },
            _ => {
                self.error(
                    &self.to_loc(&args.loc),
                    &format!(
                        "Macro `{}` cannot take more than {} arguments",
                        kind,
                        MAX_FORMAT_ARGS + 2,
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

        let binding = sp(
            loc,
            SequenceItem_::Bind(lvalues, sp(loc, Exp_::ExpList(operands.to_vec()))),
        );

        Exp_::Block(VecDeque::from_iter([
            binding,
            sp(loc, SequenceItem_::Seq(assert)),
        ]))
    }

    /// Calls `std::string_utils::format<N>(&fmt, arg1, ..., argN)` for 1 ≤ N ≤ 4.
    fn call_format(&self, loc: Loc, fmt: Exp, args: Vec<Exp>) -> Exp {
        let n = args.len();
        debug_assert!((1..MAX_FORMAT_ARGS).contains(&n));
        let borrow_fmt = sp(loc, Exp_::Borrow(false, Box::new(fmt)));
        self.call_stdlib_function(
            loc,
            STRING_UTILS_MODULE,
            &format!("format{}", n),
            std::iter::once(borrow_fmt).chain(args).collect(),
        )
    }

    /// Calls `std::string::into_bytes(s)`.
    fn call_into_bytes(&self, loc: Loc, s: Exp) -> Exp {
        self.call_stdlib_function(loc, STRING_MODULE, INTO_BYTES_FUNCTION_NAME, vec![s])
    }

    /// Calls `std::string::utf8(bytes)`.
    fn call_utf8(&self, loc: Loc, bytes: Exp) -> Exp {
        self.call_stdlib_function(loc, STRING_MODULE, UTF8_FUNCTION_NAME, vec![bytes])
    }

    /// Calls a standard library function `std::module_name::function_name(args)`.
    fn call_stdlib_function(
        &self,
        loc: Loc,
        module_name: &str,
        function_name: &str,
        args: Vec<Exp>,
    ) -> Exp {
        if !self.check_stdlib_module(loc, module_name) {
            return sp(loc, Exp_::UnresolvedError);
        }

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

    /// Checks that the given standard library module is available in the environment.
    /// If not, reports an error and returns `false`.
    fn check_stdlib_module(&self, loc: Loc, module_name_str: &str) -> bool {
        let module_name = crate::ast::ModuleName::new(
            crate::ast::Address::Numerical(AccountAddress::ONE),
            self.env().symbol_pool().make(module_name_str),
        );
        match self.env().find_module(&module_name) {
            Some(_) => true,
            None => {
                self.env().error(
                    &self.to_loc(&loc),
                    &format!("Cannot find `{}` module", module_name_str),
                );
                false
            },
        }
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
        let op = match kind {
            AssertKind::Eq => "==",
            AssertKind::Ne => "!=",
        };
        let str = if args {
            format!("assertion `left {op} right` failed: {{}}\n  left: {{}}\n right: {{}}")
        } else {
            format!("assertion `left {op} right` failed\n  left: {{}}\n right: {{}}")
        };
        Self::string_literal(loc, str)
    }

    fn string_literal(loc: Loc, str: String) -> Exp {
        sp(
            loc,
            Exp_::Value(sp(loc, Value_::Bytearray(str.into_bytes()))),
        )
    }

    fn check_format_string(&self, exp: &Exp, args: usize) {
        let Some(bytes) = check_string_literal(exp) else {
            self.error(&self.to_loc(&exp.loc), "Expected a string literal");
            return;
        };

        // Check that the format string is valid and count the number of placeholders.
        let placeholders = match count_placeholders(bytes) {
            Ok(n) => n,
            Err(err) => {
                self.error(&self.to_loc(&exp.loc), &err.to_string());
                return;
            },
        };

        // Check that the number of placeholders matches the number of arguments.
        if placeholders != args {
            self.error(
                &self.to_loc(&exp.loc),
                &format!(
                    "Format string has {} placeholders, but {} arguments were provided",
                    placeholders, args
                ),
            );
        }
    }
}

/// Checks that the expression is a string literal.
/// If so, returns the byte array. Otherwise, returns `None`.
fn check_string_literal(exp: &Exp) -> Option<&Vec<u8>> {
    if let Exp_::Value(val) = &exp.value
        && let Value_::Bytearray(bytes) = &val.value
    {
        Some(bytes)
    } else {
        None
    }
}

enum BraceError {
    UnmatchedOpening,
    UnmatchedClosing,
    InvalidPlaceholder,
}

impl Display for BraceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BraceError::UnmatchedOpening => write!(f, "Unmatched '{{' in format string"),
            BraceError::UnmatchedClosing => write!(f, "Unmatched '}}' in format string"),
            BraceError::InvalidPlaceholder => write!(f, "Invalid placeholder in format string"),
        }
    }
}

/// Counts the number of valid `{}` placeholders in a format string.
///
/// Literal braces must be escaped: `{{` becomes `{` and `}}` becomes `}`.
/// Any other brace sequence is invalid (e.g., `{foo}`, unmatched braces).
///
/// Returns an error if the format string contains unmatched or invalid braces.
fn count_placeholders(bytes: &[u8]) -> Result<usize, BraceError> {
    let mut i = 0;
    let mut count = 0;

    while i < bytes.len() {
        match bytes[i] {
            b'{' => {
                if i + 1 >= bytes.len() {
                    return Err(BraceError::UnmatchedOpening);
                }

                match bytes[i + 1] {
                    b'{' => {
                        // Escaped '{'
                        i += 2;
                    },
                    b'}' => {
                        // Valid "{}" placeholder
                        count += 1;
                        i += 2;
                    },
                    _ => {
                        return Err(BraceError::InvalidPlaceholder);
                    },
                }
            },
            b'}' => {
                if i + 1 < bytes.len() && bytes[i + 1] == b'}' {
                    // Escaped '}'
                    i += 2;
                } else {
                    return Err(BraceError::UnmatchedClosing);
                }
            },
            _ => {
                i += 1;
            },
        }
    }

    Ok(count)
}
