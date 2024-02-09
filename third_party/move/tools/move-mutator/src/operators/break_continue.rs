use crate::operator::{MutantInfo, MutationOperator};
use crate::operators::{MOVE_BREAK, MOVE_CONTINUE, MOVE_EMPTY_STMT};
use crate::report::{Mutation, Range};
use move_command_line_common::files::FileHash;
use move_compiler::typing::ast;
use move_compiler::typing::ast::UnannotatedExp_;
use std::fmt;
use std::fmt::Debug;

pub const OPERATOR_NAME: &str = "break_continue_replacement";

/// Break and continue mutation operator.
/// Replaces break and continue statements with each other or deletes them.
#[derive(Debug, Clone)]
pub struct BreakContinue {
    operation: ast::Exp,
}

impl BreakContinue {
    /// Creates a new instance of the break/continue mutation operator.
    #[must_use]
    pub fn new(operation: ast::Exp) -> Self {
        Self { operation }
    }
}

impl MutationOperator for BreakContinue {
    fn apply(&self, source: &str) -> Vec<MutantInfo> {
        let start = self.operation.exp.loc.start() as usize;
        let end = self.operation.exp.loc.end() as usize;
        let cur_op = &source[start..end];

        // Group of exchangeable break/continue statements.
        let ops: Vec<&str> = match self.operation.exp.value {
            UnannotatedExp_::Break => {
                vec![MOVE_CONTINUE, MOVE_EMPTY_STMT]
            },
            UnannotatedExp_::Continue => {
                vec![MOVE_BREAK, MOVE_EMPTY_STMT]
            },
            _ => vec![],
        };

        ops.into_iter()
            .map(|op| {
                let mut mutated_source = source.to_string();
                mutated_source.replace_range(start..end, op);
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

impl fmt::Display for BreakContinue {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "BreakContinueOperator({:?}, location: file hash: {}, index start: {}, index stop: {})",
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

    #[test]
    fn test_apply_break() {
        let loc = Loc::new(FileHash::new(""), 0, 5);
        let exp = Exp {
            exp: UnannotatedExp {
                value: UnannotatedExp_::Break,
                loc,
            },
            ty: Type {
                value: Type_::Anything,
                loc,
            },
        };
        let operator = BreakContinue::new(exp);
        let source = MOVE_BREAK;
        let expected = vec![MOVE_CONTINUE, MOVE_EMPTY_STMT];
        let result = operator.apply(source);
        assert_eq!(result.len(), expected.len());
        for (i, r) in result.iter().enumerate() {
            assert_eq!(r.mutated_source, expected[i]);
        }
    }

    #[test]
    fn test_apply_continue() {
        let loc = Loc::new(FileHash::new(""), 0, 8);
        let exp = Exp {
            exp: UnannotatedExp {
                value: UnannotatedExp_::Continue,
                loc,
            },
            ty: Type {
                value: Type_::Anything,
                loc,
            },
        };
        let operator = BreakContinue::new(exp);
        let source = MOVE_CONTINUE;
        let expected = vec![MOVE_BREAK, MOVE_EMPTY_STMT];
        let result = operator.apply(source);
        assert_eq!(result.len(), expected.len());
        for (i, r) in result.iter().enumerate() {
            assert_eq!(r.mutated_source, expected[i]);
        }
    }

    #[test]
    fn test_get_file_hash() {
        let loc = Loc::new(FileHash::new(""), 0, 0);
        let exp = Exp {
            exp: UnannotatedExp {
                value: UnannotatedExp_::Break,
                loc,
            },
            ty: Type {
                value: Type_::Anything,
                loc,
            },
        };
        let operator = BreakContinue::new(exp);
        assert_eq!(operator.get_file_hash(), FileHash::new(""));
    }
}
