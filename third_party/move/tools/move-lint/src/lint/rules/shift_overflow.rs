//! Detect potential overflow scenarios where the number of bits being shifted exceeds the bit width of
//! the variable being shifted, which could lead to unintended behavior or loss of data. If such a
//! potential overflow is detected, a warning is generated to alert the developer.
use crate::lint::utils::{add_diagnostic_and_emit, LintConfig};
use crate::lint::visitor::ExpressionAnalysisVisitor;
use codespan::FileId;

use codespan_reporting::diagnostic::Diagnostic;
use move_model::ast::{Exp, ExpData, Operation, Value};
use move_model::model::{FunctionEnv, GlobalEnv, NodeId};
use move_model::ty::{PrimitiveType, Type};
use num::ToPrimitive;

pub struct ShiftOverflowVisitor {}

impl Default for ShiftOverflowVisitor {
    fn default() -> Self {
        Self::new()
    }
}

impl ShiftOverflowVisitor {
    pub fn new() -> Self {
        Self {}
    }

    pub fn visitor() -> Box<dyn ExpressionAnalysisVisitor> {
        Box::new(Self::new())
    }

    fn check_shift_overflow(
        &mut self,
        exp: &ExpData,
        env: &GlobalEnv,
        diags: &mut Vec<Diagnostic<FileId>>,
    ) {
        if let Some((call_node_id, op, exp_vec)) = self.extract_call_data(exp) {
            if let Some(number_size) = self.get_bit_width(op, env, call_node_id) {
                if let Some(value) = self.extract_value(&exp_vec[1]) {
                    if self.is_overflow(&value, number_size) {
                        let message = "Potential overflow detected. The number of bits being shifted exceeds the bit width of the variable being shifted";
                        add_diagnostic_and_emit(
                            &env.get_node_loc(call_node_id),
                            message,
                            codespan_reporting::diagnostic::Severity::Warning,
                            env,
                            diags,
                        );
                    }
                }
            }
        }
    }

    fn extract_call_data<'a>(
        &self,
        exp: &'a ExpData,
    ) -> Option<(NodeId, &'a Operation, &'a Vec<Exp>)> {
        if let ExpData::Block(_, _, Some(call), _) = exp {
            if let ExpData::Call(call_node_id, op, exp_vec) = call.as_ref() {
                return Some((*call_node_id, op, exp_vec));
            }
        }
        None
    }

    fn get_bit_width(&self, op: &Operation, env: &GlobalEnv, node_id: NodeId) -> Option<u128> {
        match op {
            Operation::Shl | Operation::Shr => {
                let ty = env.get_node_type(node_id);
                match ty {
                    Type::Primitive(PrimitiveType::U8) => Some(8),
                    Type::Primitive(PrimitiveType::U16) => Some(16),
                    Type::Primitive(PrimitiveType::U32) => Some(32),
                    Type::Primitive(PrimitiveType::U64) => Some(64),
                    Type::Primitive(PrimitiveType::U128) => Some(128),
                    Type::Primitive(PrimitiveType::U256) => Some(256),
                    _ => None,
                }
            },
            _ => None,
        }
    }

    fn extract_value(&self, exp_data: &ExpData) -> Option<Value> {
        if let ExpData::Value(_, value) = exp_data {
            Some(value.clone())
        } else {
            None
        }
    }

    fn is_overflow(&self, value: &Value, bit_width: u128) -> bool {
        match value {
            Value::Number(v) => v.to_u128().map_or(false, |num| num > bit_width),
            _ => false,
        }
    }
}

impl ExpressionAnalysisVisitor for ShiftOverflowVisitor {
    fn post_visit_expression(
        &mut self,
        exp: &ExpData,
        _func_env: &FunctionEnv,
        env: &GlobalEnv,
        _: &LintConfig,
        diags: &mut Vec<Diagnostic<FileId>>,
    ) {
        self.check_shift_overflow(exp, env, diags);
    }
}
