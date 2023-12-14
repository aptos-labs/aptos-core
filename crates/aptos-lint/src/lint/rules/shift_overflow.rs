use crate::lint::visitor::{ExpDataVisitor, LintUtilities};
/// Detect potential overflow scenarios where the number of bits being shifted exceeds the bit width of
/// the variable being shifted, which could lead to unintended behavior or loss of data. If such a
/// potential overflow is detected, a warning is generated to alert the developer.
use move_model::ast::{ExpData, Operation, Value};
use move_model::{
    model::{FunctionEnv, GlobalEnv},
    ty::{PrimitiveType, Type},
};
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

    pub fn visitor() -> Box<dyn ExpDataVisitor> {
        Box::new(Self::new())
    }

    fn check_shift_overflow(&mut self, exp: &ExpData, env: &GlobalEnv) {
        if let ExpData::Block(_, _, Some(call), _) = exp {
            if let ExpData::Call(call_node_id, op, exp_vec) = call.as_ref() {
                match op {
                    Operation::Shl | Operation::Shr => {
                        // e1 << e2 | e1 >> e2
                        let ty = env.get_node_type(*call_node_id);
                        let v1_bit = match ty {
                            Type::Primitive(PrimitiveType::U8) => Some(8),
                            Type::Primitive(PrimitiveType::U16) => Some(16),
                            Type::Primitive(PrimitiveType::U32) => Some(32),
                            Type::Primitive(PrimitiveType::U64) => Some(64),
                            Type::Primitive(PrimitiveType::U128) => Some(128),
                            Type::Primitive(PrimitiveType::U256) => Some(256),
                            _ => None,
                        };

                        if let Some(v1_bit) = v1_bit {
                            // e1 << e2
                            if let ExpData::Value(_, value) = &exp_vec[1].as_ref() {
                                let is_overflow = match value {
                                    Value::Number(v) => v.to_u128().unwrap() > v1_bit,
                                    _ => false,
                                };
                                if is_overflow {
                                    let message =
                                        "Potential overflow detected. The number of bits being shifted exceeds the bit width of the variable being shifted";
                                    self.add_diagnostic_and_emit(
                                        &env.get_node_loc(*call_node_id),
                                        message,
                                        codespan_reporting::diagnostic::Severity::Warning,
                                        env,
                                    );
                                }
                            }
                        }
                    },
                    _ => (),
                }
            }
        }
    }
}

impl ExpDataVisitor for ShiftOverflowVisitor {
    fn visit(&mut self, func_env: &FunctionEnv, env: &GlobalEnv) {
        let mut visitor = |exp: &ExpData| {
            self.check_shift_overflow(exp, env);
        };
        if let Some(func) = func_env.get_def().as_ref() {
            func.visit(&mut visitor);
        };
    }
}

impl LintUtilities for ShiftOverflowVisitor {}
