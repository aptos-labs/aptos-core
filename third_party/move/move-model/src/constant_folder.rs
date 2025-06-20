// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! Struct `ConstantFolder` implements `ExpRewriterFunctions` to try constant-folding on an
//! expression after types are finalized.  Usual entry point is `ConstantFolder::rewrite_exp`,
//! although functions `fold_binary_exp`, `fold_unary_exp`, and `fold_tuple` should work on arguments
//! which are `ExpData::Value` expressions with appropriate types set (in the `GlobalEnv`) for
//! `NodeId` values.
//!
//! Example usage for constant declarations:
//!    // Contextual code to show how types might be filled in:
//!    let mut et = ExpTranslator::new(...);
//!    let exp = et.translate_exp(...).into_exp();
//!    et.finalize_types();
//!    let mut reasons = Vec::new();
//!    if exp.is_valid_for_constant(&global_env, &mut reasons) {
//!        // This is the actual constant folding part:
//!        let constant_folder = ConstantFolder::new(&global_env, true);
//!        let rewritten: Exp = constant_folder.rewrite_exp(exp);
//!    }

//! The current implementation handles most built-in operators with constant parameters.
//! Operators not yet handled that match `ast::Operation::is_builtin_op`:
//! TODO:
//! | Index
//! | Slice
//! | Range
//! | Implies
//! | Iff
//! | Identical
//! | Len

use crate::{
    ast::{Exp, ExpData, Operation, Value},
    exp_rewriter::ExpRewriterFunctions,
    model::{GlobalEnv, Loc, NodeId},
    ty::{PrimitiveType, Type, TypeDisplay, TypeDisplayContext},
};
use codespan_reporting::diagnostic::Severity;
use core::ops::{BitAnd, BitOr, BitXor, Rem, Shl, Shr};
use num::{BigInt, ToPrimitive, Zero};

pub struct ConstantFolder<'env> {
    env: &'env GlobalEnv,
    type_display_ctxt: TypeDisplayContext<'env>,
    in_constant_declaration: bool,
    error_status: bool, // saw an error already in the current expression
}

impl<'env> ConstantFolder<'env> {
    /// Set the `GlobalEnv` to use.  This is used to obtain code `Loc` for diagnostics,
    /// for obtaining expression and argument types, and to generate diagnostics..
    pub fn new(env: &'env GlobalEnv, in_constant_declaration: bool) -> Self {
        Self {
            env,
            type_display_ctxt: env.get_type_display_ctx(),
            error_status: false,
            in_constant_declaration,
        }
    }

    // clear folder error status and return old status
    fn clear_error_status(&mut self) -> bool {
        let old_value = self.error_status;
        self.error_status = false;
        old_value
    }

    // merge `old_status` with folder error status.
    fn merge_old_error_status(&mut self, old_status: bool) {
        self.error_status = self.error_status || old_status;
    }

    fn constant_folding_error<T, F>(&mut self, id: NodeId, error_msg_generator: F) -> Option<T>
    where
        F: FnOnce(&mut Self) -> String,
    {
        if self.in_constant_declaration && !self.error_status {
            let loc = self.env.get_node_loc(id);
            self.env.error(
                &loc,
                &format!(
                    "Invalid expression in `const`. {}",
                    error_msg_generator(self),
                ),
            );
            self.error_status = true;
        };
        None
    }

    // Return `Some(val.clone())` iff `val` is in bounds of type `pty`.
    fn type_bound_bigint(val: &BigInt, pty: &PrimitiveType) -> Option<BigInt> {
        Some(val.clone())
            .filter(|val| {
                if let Some(min_val) = &pty.get_min_value() {
                    val >= min_val
                } else {
                    true
                }
            })
            .filter(|val| {
                if let Some(max_val) = &pty.get_max_value() {
                    max_val >= val
                } else {
                    true
                }
            })
    }

    // Helper to display a type in error messages
    fn display_type<'a>(&'a self, ty: &'a Type) -> TypeDisplay<'a> {
        ty.display(&self.type_display_ctxt)
    }

    /// Try constant folding of a non-tuple unary operation `oper` applied to argument `arg0`,
    /// returning `Some(exp)` where `exp` is a ExpData::Value(id, ..)` expression if constant
    /// folding is possible.  Operation result type may be obtained from `id`.
    ///
    /// Returns `None` and emitting diagnostic messages (referencing code corresponding to`id`)
    /// if the specified operation cannot be folded to a constant.
    ///
    /// Argument expressions and `id` may need to be fully typed for success.
    pub fn fold_unary_exp(&mut self, id: NodeId, oper: &Operation, arg0: &Exp) -> Option<Exp> {
        use ExpData::Value as V;
        use Operation as O;
        use PrimitiveType::Bool as PTBool;
        use Type::Primitive as T;
        use Value::{Bool, Number};

        if let V(arg0_id, val0) = arg0.as_ref() {
            let arg0_type = self.env.get_node_type(*arg0_id);
            let result_type = self.env.get_node_type(id);
            match (oper, val0, &arg0_type, &result_type) {
                (O::Not, Bool(b), T(PTBool), T(PTBool)) => Some(V(id, Bool(!b)).into_exp()),
                (O::Not, _, T(PTBool), T(PTBool)) =>
                    self.constant_folding_error(
                        id,
                        |_| "Argument to `!` is not a constant".to_owned(),
                    ),
                (O::Not, _, _, _) =>
                    self.constant_folding_error(id,
                                                |aself: &mut Self|
                                                format!(
                                                    "Expected bool types for argument \
                                                     and result of operator `Not` (`!`) but found `{}` and `{}`",
                                                    aself.display_type(&arg0_type),
                                                    aself.display_type(&result_type)
                                                )),
                (O::Cast, Number(val0_bigint), T(_arg0_pty), T(result_pty)) => {
                    if arg0_type.is_number() && result_type.is_number() {
                        Self::type_bound_bigint(val0_bigint, result_pty)
                            .map(|result_val| V(id, Number(result_val)).into_exp())
                            .or_else(|| {
                                self.constant_folding_error(
                                    id,
                                    |aself: &mut Self| format!(
                                        "Cast argument value `{}` out of range for type `{}`",
                                        val0_bigint,
                                        aself.display_type(&result_type)))
                            })
                    } else {
                        self.constant_folding_error(
                            id,
                            |aself: &mut Self| format!(
                                "Expected numeric types for argument and result \
                                 of cast (`as`) but found `{}` and `{}`",
                                aself.display_type(&arg0_type),
                                aself.display_type(&result_type)
                            ),
                        )
                    }
                },
                (O::Cast, _, _, _) => self.constant_folding_error(id, |_| {
                    "Argument to cast operation (`as`) is not foldable \
                     to a numeric constant."
                        .to_owned()
                }),
                _ => self.constant_folding_error(id, |_|
                                                 "Unary expression not foldable to constant".to_owned()),
            }
        } else {
            self.constant_folding_error(id, |_| {
                "Parameter to unary expression not foldable to constant".to_owned()
            })
        }
    }

    fn binop_num<F>(
        &mut self,
        binop_name: &str,
        binop_fun: F,
        id: NodeId,
        result_pty: &PrimitiveType,
        val0: &BigInt,
        val1: &BigInt,
    ) -> Option<Exp>
    where
        F: Fn(&BigInt, &BigInt) -> Option<BigInt>,
    {
        binop_fun(val0, val1)
            .and_then(|val| Self::type_bound_bigint(&val, result_pty))
            .map(|val| ExpData::Value(id, Value::Number(val)).into_exp())
            .or_else(|| {
                self.constant_folding_error(id, |aself: &mut Self| {
                    format!(
                        "Operator `{}` result value out of range for `{}`",
                        binop_name,
                        aself.display_type(&Type::Primitive(*result_pty)),
                    )
                    .to_owned()
                })
            })
    }

    fn checked_rem(a: &BigInt, b: &BigInt) -> Option<BigInt> {
        if b != &BigInt::zero() {
            Some(a.rem(b))
        } else {
            None
        }
    }

    /// Check the RHS of a shift: `rhs` should be less than result type size.
    fn shift_rhs_check(
        &mut self,
        rhs: &BigInt,
        result_ty_size: &BigInt,
        id: NodeId,
        binop_name: &str,
    ) -> Option<()> {
        if rhs < result_ty_size {
            Some(())
        } else {
            self.constant_folding_error(id, |_| {
                format!(
                    "Operand on the right is too large for `{}`, should be less than `{}`",
                    binop_name, result_ty_size
                )
                .to_owned()
            })
        }
    }

    fn checked_shl(a: &BigInt, b: &BigInt) -> Option<BigInt> {
        b.to_u16().map(|b| a.shl(b))
    }

    fn checked_shr(a: &BigInt, b: &BigInt) -> Option<BigInt> {
        b.to_u16().map(|b| a.shr(b))
    }

    // number of bits or 256 if undefined
    fn ptype_num_bits_bigint(ptype: &PrimitiveType) -> BigInt {
        ptype
            .get_num_bits()
            .map(BigInt::from)
            .unwrap_or_else(|| BigInt::from(256))
    }

    /// Try constant folding of a non-tuple binary operation `oper` applied to arguments `arg0` and
    /// `arg1`, returning `Some(exp)` where `exp` is a ExpData::Value(id, ..)` expression if
    /// constant folding is possible.  Operation result type may be obtained from `id`.
    ///
    /// Returns `None` and emitting diagnostic messages (referencing code corresponding to`id`)
    /// if the specified operation cannot be folded to a constant.
    ///
    /// Argument expressions and `id` may need to be fully typed for success.
    pub fn fold_binary_exp(
        &mut self,
        id: NodeId,
        oper: &Operation,
        arg0: &Exp,
        arg1: &Exp,
    ) -> Option<Exp> {
        use ExpData::Value as V;
        use Operation as O;
        use PrimitiveType::Bool as PTBool;
        use Type::Primitive as T;
        use Value::{Bool, Number};

        if let (V(_, val0), V(_, val1)) = (arg0.as_ref(), arg1.as_ref()) {
            let result_type = self.env.get_node_type(id);

            if let (Number(val0), Number(val1), T(result_pty)) = (&val0, &val1, &result_type) {
                // Get the name of a binary operator, is only used when `oper` is a binary operator.
                let name = || oper.to_string_if_binop().expect("binop");
                // Binops with numeric arguments and result.
                match oper {
                    O::Add => {
                        self.binop_num(name(), BigInt::checked_add, id, result_pty, val0, val1)
                    },
                    O::Sub => {
                        self.binop_num(name(), BigInt::checked_sub, id, result_pty, val0, val1)
                    },
                    O::Mul => {
                        self.binop_num(name(), BigInt::checked_mul, id, result_pty, val0, val1)
                    },
                    O::Div => {
                        self.binop_num(name(), BigInt::checked_div, id, result_pty, val0, val1)
                    },
                    O::Mod => self.binop_num(name(), Self::checked_rem, id, result_pty, val0, val1),
                    O::Shl => {
                        // result_pty should be same size as arg0
                        let arg0_size = Self::ptype_num_bits_bigint(result_pty);
                        self.shift_rhs_check(val1, &arg0_size, id, name())
                            .and_then(|_| {
                                self.binop_num(
                                    name(),
                                    Self::checked_shl,
                                    id,
                                    result_pty,
                                    val0,
                                    val1,
                                )
                            })
                    },
                    O::Shr => {
                        // result_pty should be same size as arg0
                        let arg0_size = Self::ptype_num_bits_bigint(result_pty);
                        self.shift_rhs_check(val1, &arg0_size, id, name())
                            .and_then(|_| {
                                self.binop_num(
                                    name(),
                                    Self::checked_shr,
                                    id,
                                    result_pty,
                                    val0,
                                    val1,
                                )
                            })
                    },
                    O::BitAnd => Some(V(id, Number(val0.bitand(val1))).into_exp()),
                    O::BitOr => Some(V(id, Number(val0.bitor(val1))).into_exp()),
                    O::Xor => Some(V(id, Number(val0.bitxor(val1))).into_exp()),
                    O::Lt => Some(V(id, Bool(val0 < val1)).into_exp()),
                    O::Gt => Some(V(id, Bool(val0 > val1)).into_exp()),
                    O::Le => Some(V(id, Bool(val0 <= val1)).into_exp()),
                    O::Ge => Some(V(id, Bool(val0 >= val1)).into_exp()),
                    O::Eq => Some(V(id, Bool(val0 == val1)).into_exp()),
                    O::Neq => Some(V(id, Bool(val0 != val1)).into_exp()),
                    _ => self.constant_folding_error(id, |_| {
                        "Binary expression with numeric parameters not foldable to constant"
                            .to_owned()
                    }),
                }
            } else if let (Bool(val0), Bool(val1), T(PTBool)) = (&val0, &val1, &result_type) {
                // Binops with Boolean arguments and result.
                match oper {
                    O::And => Some(V(id, Bool(*val0 && *val1)).into_exp()),
                    O::Or => Some(V(id, Bool(*val0 || *val1)).into_exp()),
                    O::Eq => Some(V(id, Bool(*val0 == *val1)).into_exp()),
                    O::Neq => Some(V(id, Bool(*val0 != *val1)).into_exp()),
                    _ => {
                        return self.constant_folding_error(id, |_| {
                            "Binary expression with boolean parameters and result \
                             not foldable to constant"
                                .to_owned()
                        })
                    },
                }
            } else {
                match oper {
                    O::Eq => val0
                        .equivalent(val1)
                        .map(|equivalence| V(id, Bool(equivalence)).into_exp()),
                    O::Neq => val0
                        .equivalent(val1)
                        .map(|equivalence| V(id, Bool(!equivalence)).into_exp()),
                    _ => {
                        if oper.is_binop() {
                            self.constant_folding_error(id, |_| {
                                "Constant folding failed due to incomplete evaluation".to_owned()
                            })
                        } else {
                            self.constant_folding_error(id, |_| {
                                "Unknown binary expression in `const`".to_owned()
                            })
                        }
                    },
                }
            }
        } else {
            self.constant_folding_error(id, |_| {
                "Binary expression arguments not both foldable to constant".to_owned()
            })
        }
    }

    /// Try constant folding of a vector of arguments represented as a slice of `Exp`.
    ///
    /// If every operand is already a constant literal, then return `Some(exp)` where `exp`
    /// is an `ExpData::Value(id, ..)` expression representing a `Vector` of values.
    ///
    /// Returns `None` and emitting diagnostic messages (referencing code corresponding to`id`)
    /// if the specified operation cannot be folded to a constant.
    ///
    /// Argument expression `node_id` values and `id` may need to be fully typed in `self.env`
    /// for success.
    pub fn fold_vector_exp<T>(
        &mut self,
        id: NodeId,
        constructor: T,
        oper_name: &str,
        args: &[Exp],
    ) -> Option<Exp>
    where
        T: FnOnce(Vec<Value>) -> Value,
    {
        let mut reasons: Vec<(Loc, String)> = Vec::new();
        let mut vec_result: Vec<Value> = Vec::new();
        for (idx, exp) in args.iter().enumerate() {
            if let ExpData::Value(_, val) = exp.as_ref() {
                if reasons.is_empty() {
                    vec_result.push(val.clone())
                }
            } else {
                // arg doesn't work.
                let id = exp.node_id();
                let loc = self.env.get_node_loc(id);
                reasons.push((
                    loc,
                    format!("List element {} not (foldable to) a constant value", idx),
                ));
            }
        }
        if reasons.is_empty() {
            Some(ExpData::Value(id, constructor(vec_result)).into_exp())
        } else {
            if self.in_constant_declaration {
                self.env.diag_with_labels(
                    Severity::Error,
                    &self.env.get_node_loc(id),
                    &format!("{} operand list not constant", oper_name),
                    reasons,
                );
            };
            None
        }
    }
}

impl ExpRewriterFunctions for ConstantFolder<'_> {
    fn rewrite_call(&mut self, id: NodeId, oper: &Operation, args: &[Exp]) -> Option<Exp> {
        if matches!(oper, Operation::Tuple) {
            if self.in_constant_declaration {
                self.fold_vector_exp(id, Value::Tuple, "tuple", args)
            } else {
                None
            }
        } else if matches!(oper, Operation::Vector) {
            // Note that creating an empty vector is less gas than `ld_const_base`,
            // so if we are optimizing we leave an empty vector as a vector op.
            if self.in_constant_declaration || !args.is_empty() {
                self.fold_vector_exp(id, Value::Vector, "vector", args)
            } else {
                None
            }
        } else if args.len() == 1 {
            // unary op
            self.fold_unary_exp(id, oper, &args[0])
        } else if args.len() == 2 {
            // binary op
            self.fold_binary_exp(id, oper, &args[0], &args[1])
        } else {
            None
        }
    }

    fn rewrite_exp(&mut self, exp: Exp) -> Exp {
        let saved_error_status = self.clear_error_status();
        let result = self.rewrite_exp_descent(exp);
        self.merge_old_error_status(saved_error_status);
        match result.as_ref() {
            ExpData::Sequence(_, es) => {
                // If this is valid for constant, then all expressions are constants,
                // and only last expr is useful.
                if let Some(last_exp) = es.last() {
                    last_exp.clone()
                } else {
                    result
                }
            },
            _ => result,
        }
    }
}
