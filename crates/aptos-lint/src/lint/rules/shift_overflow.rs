// The visitor checks for potential overflow scenarios where the number of bits being shifted exceeds the bit width of
// the variable being shifted, which could lead to unintended behavior or loss of data. If such a
// potential overflow is detected, a warning is generated to alert the developer.
use std::str::FromStr;

use move_compiler::typing::ast::Exp;
use move_compiler::parser::ast as AST1;
use move_compiler::expansion::ast as AST2;
use move_compiler::naming::ast as AST3;
use move_compiler::typing::ast as AST4;
use move_ir_types::sp;

use crate::lint::context::VisitorContext;
use crate::lint::visitor::{ LintVisitor, LintUtilities };

pub struct ShiftOverflowVisitor {}

impl ShiftOverflowVisitor {
    pub fn new() -> Self {
        Self {}
    }

    pub fn visitor() -> Box<dyn LintVisitor> {
        Box::new(Self::new())
    }

    fn check_shift_overflow(&mut self, exp: &Exp, context: &mut VisitorContext) {
        match &exp.exp.value {
            AST4::UnannotatedExp_::BinopExp(e1, op, _, e2) => {
                match &op.value {
                    AST1::BinOp_::Shl | AST1::BinOp_::Shr => {
                        // e1 << e2 | e1 >> e2
                        if let AST3::Type_::Apply(_, sp!(_, AST3::TypeName_::Builtin(sp!(_, typ))), _) = &e1.ty.value {
                            // bit of e1
                            let v1_bit: Option<u128> = match &typ {
                                AST3::BuiltinTypeName_::U8 => Some(8),
                                AST3::BuiltinTypeName_::U16 => Some(16),
                                AST3::BuiltinTypeName_::U32 => Some(32),
                                AST3::BuiltinTypeName_::U64 => Some(64),
                                AST3::BuiltinTypeName_::U128 => Some(128),
                                AST3::BuiltinTypeName_::U256 => Some(256),
                                _ => None,
                            };
                            if let Some(v1_bit) = v1_bit {
                                // AST4::UnannotatedExp_::Value // constant node
                                if let AST4::UnannotatedExp_::Value(v2) = &e2.exp.value {
                                    let is_overflow = match &v2.value {
                                        AST2::Value_::InferredNum(v) | AST2::Value_::U256(v) => {
                                            if
                                                let Ok(v1_bit_256) = move_core_types::u256::U256::from_str(
                                                    v1_bit.to_string().as_str()
                                                )
                                            {
                                                v >= &v1_bit_256
                                            } else {
                                                false
                                            }
                                        }
                                        AST2::Value_::U8(v) => (*v as u128) >= v1_bit,
                                        AST2::Value_::U16(v) => (*v as u128) >= v1_bit,
                                        AST2::Value_::U32(v) => (*v as u128) >= v1_bit,
                                        AST2::Value_::U64(v) => (*v as u128) >= v1_bit,
                                        AST2::Value_::U128(v) => (*v as u128) >= v1_bit,
                                        _ => false,
                                    };
                                    if is_overflow {
                                        let message = "Potential overflow detected during a shift operation.";
                                        self.add_warning(context, &exp.exp.loc, message);
                                    }
                                }
                            }
                        }
                    }
                    _ => (),
                }
            }
            _ => (),
        }
    }
}

impl LintVisitor for ShiftOverflowVisitor {
    fn visit_exp(&mut self, exp: &Exp, context: &mut VisitorContext) {
        self.check_shift_overflow(exp, context);
    }
}

impl LintUtilities for ShiftOverflowVisitor {}
