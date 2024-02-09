use crate::operator::{MutantInfo, MutationOperator};
use crate::report::{Mutation, Range};
use move_command_line_common::files::FileHash;
use move_compiler::typing::ast;
use move_compiler::typing::ast::UnannotatedExp_;
use std::fmt;
use std::fmt::Debug;

pub const OPERATOR_NAME: &str = "if_else_replacement";

/// IfElse mutation operator.
/// Replaces expressions under the if/else statements with literals.
#[derive(Debug, Clone)]
pub struct IfElse {
    operation: ast::Exp,
}

impl IfElse {
    /// Creates a new instance of the if/else mutation operator.
    #[must_use]
    pub fn new(operation: ast::Exp) -> Self {
        Self { operation }
    }
}

impl MutationOperator for IfElse {
    fn apply(&self, source: &str) -> Vec<MutantInfo> {
        let if_op = &self.operation.exp.value;

        let (start, end, cur_op) = match if_op {
            UnannotatedExp_::IfElse(if_exp, _, _) => {
                let start = if_exp.exp.loc.start() as usize;
                let end = if_exp.exp.loc.end() as usize;
                let cur_op = &source[start..end];

                (start, end, cur_op)
            },
            _ => panic!("IfElse operator called on non-if expression."), // That should never happen!
        };

        // Change if/else expression to true/false.
        let ops: Vec<String> = vec![
            "true".to_owned(),
            "false".to_owned(),
            format!("!({})", cur_op),
        ];

        ops.into_iter()
            .map(|op| {
                let mut mutated_source = source.to_string();
                mutated_source.replace_range(start..end, op.as_str());
                MutantInfo::new(
                    mutated_source,
                    Mutation::new(
                        Range::new(start, end),
                        OPERATOR_NAME.to_string(),
                        cur_op.to_string(),
                        op.to_string(),
                    ),
                )
            })
            .collect()
    }

    fn get_file_hash(&self) -> FileHash {
        self.operation.exp.loc.file_hash()
    }

    fn name(&self) -> String {
        OPERATOR_NAME.to_string()
    }
}

impl fmt::Display for IfElse {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "IfElseOperator({:?}, location: file hash: {}, index start: {}, index stop: {})",
            self.operation.exp.value,
            self.operation.exp.loc.file_hash(),
            self.operation.exp.loc.start(),
            self.operation.exp.loc.end()
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use move_command_line_common::files::FileHash;
    use move_compiler::naming::ast::{Type, Type_};
    use move_compiler::typing::ast::{Exp, UnannotatedExp, UnannotatedExp_};
    use move_ir_types::location::Loc;

    fn crate_exp(loc: Loc) -> ast::Exp {
        Exp {
            exp: UnannotatedExp {
                value: UnannotatedExp_::IfElse(
                    Box::new(Exp {
                        exp: UnannotatedExp {
                            value: UnannotatedExp_::Value(
                                move_compiler::expansion::ast::Value::new(
                                    loc,
                                    move_compiler::expansion::ast::Value_::Bool(true),
                                ),
                            ),
                            loc,
                        },
                        ty: Type {
                            value: Type_::Anything,
                            loc,
                        },
                    }),
                    Box::new(Exp {
                        exp: UnannotatedExp {
                            value: UnannotatedExp_::Break,
                            loc,
                        },
                        ty: Type {
                            value: Type_::Anything,
                            loc,
                        },
                    }),
                    Box::new(Exp {
                        exp: UnannotatedExp {
                            value: UnannotatedExp_::Break,
                            loc,
                        },
                        ty: Type {
                            value: Type_::Anything,
                            loc,
                        },
                    }),
                ),
                loc,
            },
            ty: Type {
                value: Type_::Anything,
                loc,
            },
        }
    }

    #[test]
    fn test_apply_ifelse() {
        let loc = Loc::new(FileHash::new(""), 4, 5);
        let exp = crate_exp(loc);
        let operator = IfElse::new(exp);
        let source = "if (a) { }";
        let expected = vec!["if (true) { }", "if (false) { }", "if (!(a)) { }"];
        let result = operator.apply(source);
        assert_eq!(result.len(), expected.len());
        for (i, r) in result.iter().enumerate() {
            assert_eq!(r.mutated_source, expected[i]);
        }
    }

    #[test]
    fn test_get_file_hash() {
        let loc = Loc::new(FileHash::new("1234567890"), 0, 5);
        let exp = crate_exp(loc);
        let operator = IfElse::new(exp);
        assert_eq!(operator.get_file_hash(), FileHash::new("1234567890"));
    }
}
