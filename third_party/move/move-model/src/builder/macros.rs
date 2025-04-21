// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Module for expanding macros, as `assert!(cond, code)`. This are expanded to
//! the input AST before type checking.  We also allow `assert!(cond)`, for Move 2,
//! which generates the "well-known" abort code `UNSPECIFIED_ABORT_CODDE`,
//! which is `(0xD8CA26CBD9BE << 16)`.

use crate::{
    builder::exp_builder::ExpTranslator, well_known::UNSPECIFIED_ABORT_CODE, LanguageVersion,
};
use move_compiler::expansion::ast as EA;
use move_ir_types::location::{sp, Loc, Spanned};

impl<'env, 'translator, 'module_translator> ExpTranslator<'env, 'translator, 'module_translator> {
    pub fn expand_macro(&self, loc: Loc, name: &str, args: &Spanned<Vec<EA::Exp>>) -> EA::Exp {
        // Currently, there is only the assert! macro, and no user definable ones.
        let expansion_ = match name {
            "assert" => self.expand_assert(loc, args),
            _ => {
                self.error(&self.to_loc(&loc), &format!("unknown macro `{}`", name));
                EA::Exp_::UnresolvedError
            },
        };
        sp(loc, expansion_)
    }

    fn expand_assert(&self, loc: Loc, args: &Spanned<Vec<EA::Exp>>) -> EA::Exp_ {
        let (cond, abort_code) = match args.value.len() {
            1 => {
                self.check_language_version(
                    &self.to_loc(&loc),
                    "single-argument `assert!` macro",
                    LanguageVersion::V2_0,
                );
                (
                    args.value[0].clone(),
                    sp(
                        loc,
                        EA::Exp_::Value(sp(loc, EA::Value_::U64(UNSPECIFIED_ABORT_CODE))),
                    ),
                )
            },
            2 => (args.value[0].clone(), args.value[1].clone()),
            _ => {
                self.error(
                    &self.to_loc(&args.loc),
                    "assert macro must have one or two arguments",
                );
                return EA::Exp_::UnresolvedError;
            },
        };
        EA::Exp_::IfElse(
            Box::new(cond),
            Box::new(sp(loc, EA::Exp_::Unit { trailing: false })),
            Box::new(sp(loc, EA::Exp_::Abort(Box::new(abort_code)))),
        )
    }
}
