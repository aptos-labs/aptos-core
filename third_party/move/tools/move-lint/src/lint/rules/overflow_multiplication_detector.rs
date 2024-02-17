use std::collections::BTreeMap;

use crate::lint::utils::{add_diagnostic_and_emit, LintConfig};
use crate::lint::visitor::ExpressionAnalysisVisitor;
use move_model::ast::{Exp, ExpData, Operation, Pattern, Value};
use move_model::model::{FunctionEnv, GlobalEnv};
use move_model::symbol::Symbol;
use move_model::ty::{PrimitiveType, Type};
use num::ToPrimitive;
use num_bigint::BigInt;

pub struct OverflowMultiplicationDetectorVisitor {
    declared_vars: BTreeMap<Symbol, BigInt>,
}

impl Default for OverflowMultiplicationDetectorVisitor {
    fn default() -> Self {
        Self::new()
    }
}

impl OverflowMultiplicationDetectorVisitor {
    pub fn new() -> Self {
        Self {
            declared_vars: BTreeMap::new(),
        }
    }

    pub fn visitor() -> Box<dyn ExpressionAnalysisVisitor> {
        Box::new(Self::new())
    }

    pub fn insert_declared_var(&mut self, symbol: Symbol, exp: &BigInt) {
        self.declared_vars.insert(symbol, exp.clone());
    }

    /// Checks for multiplications that can overflow.
    fn check_overflow_multiplication(&self, exp: &ExpData, env: &GlobalEnv) {
        if let ExpData::Call(_, Operation::Mul, args) = exp {
            if args.len() == 2 {
                let lhs_exp = &args[0];
                let rhs_exp = &args[1];

                if let (Some(lhs_value), Some(rhs_value)) = (
                    self.get_numeric_value(lhs_exp),
                    self.get_numeric_value(rhs_exp),
                ) {
                    let lhs_type = env.get_node_type(lhs_exp.node_id());
                    let rhs_type = env.get_node_type(rhs_exp.node_id());
                    if self.could_overflow(lhs_value, rhs_value, &lhs_type, &rhs_type) {
                        let message = "Potential multiplication overflow detected.";
                        add_diagnostic_and_emit(
                            &env.get_node_loc(exp.node_id()),
                            message,
                            codespan_reporting::diagnostic::Severity::Warning,
                            env,
                        );
                    }
                }
            }
        }
    }

    /// Retrieves the numeric value from an expression, if it is a constant.
    fn get_numeric_value(&self, exp: &Exp) -> Option<u128> {
        if let ExpData::LocalVar(_, symbol) = exp.as_ref() {
            self.declared_vars
                .get(symbol)
                .and_then(|value| value.to_u128())
        } else {
            None
        }
    }

    /// Checks if the multiplication of two numbers could overflow based on their types.
    fn could_overflow(&self, lhs: u128, rhs: u128, lhs_type: &Type, rhs_type: &Type) -> bool {
        let max_value = match (lhs_type, rhs_type) {
            (Type::Primitive(PrimitiveType::U8), Type::Primitive(PrimitiveType::U8)) => {
                u8::MAX as u128
            },
            (Type::Primitive(PrimitiveType::U64), Type::Primitive(PrimitiveType::U64)) => {
                u64::MAX as u128
            },
            _ => return false,
        };
        lhs.checked_mul(rhs)
            .map_or(true, |result| result > max_value)
    }
}

impl ExpressionAnalysisVisitor for OverflowMultiplicationDetectorVisitor {
    fn post_visit_expression(
        &mut self,
        exp: &ExpData,
        _func_env: &FunctionEnv,
        env: &GlobalEnv,
        _: &LintConfig,
    ) {
        if let ExpData::Block(_, Pattern::Var(_, symbol), Some(binding_exp), _) = exp {
            if let ExpData::Value(_, Value::Number(num)) = binding_exp.as_ref() {
                self.insert_declared_var(*symbol, num);
            }
        }
        self.check_overflow_multiplication(exp, env);
    }
}
