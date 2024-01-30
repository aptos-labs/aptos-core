use crate::operator::{MutantInfo, MutationOperator};
use crate::operators::MOVE_EMPTY_STMT;
use crate::report::{Mutation, Range};
use move_command_line_common::files::FileHash;
use move_compiler::typing::ast;
use move_compiler::typing::ast::BuiltinFunction_;
use std::fmt;
use std::fmt::Debug;

/// Statement delete operator.
/// Deletes statements which can be potentially deleted, still allowing the code to compile
/// properly.
#[derive(Debug, Clone)]
pub struct DeleteStmt {
    operation: ast::Exp,
}

impl DeleteStmt {
    /// Creates a new instance of the delete mutation operator.
    #[must_use]
    pub fn new(operation: ast::Exp) -> Self {
        Self { operation }
    }
}

impl MutationOperator for DeleteStmt {
    fn apply(&self, source: &str) -> Vec<MutantInfo> {
        let (start, end) = match &self.operation.exp.value {
            ast::UnannotatedExp_::Builtin(f, _) => {
                match &f.value {
                    // move_to can be changed to the empty statement as it does not return anything
                    BuiltinFunction_::MoveTo(_) => (
                        self.operation.exp.loc.start() as usize,
                        self.operation.exp.loc.end() as usize,
                    ),
                    _ => {
                        return vec![];
                    },
                }
            },
            _ => {
                return vec![];
            },
        };

        let cur_op = &source[start..end];

        let ops: Vec<&str> = vec![MOVE_EMPTY_STMT];

        ops.into_iter()
            .map(|op| {
                let mut mutated_source = source.to_string();
                mutated_source.replace_range(start..end, op);
                MutantInfo::new(
                    mutated_source,
                    Mutation::new(
                        Range::new(start, end),
                        "delete_stmt_replacement".to_string(),
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
}

impl fmt::Display for DeleteStmt {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "DeleteStmtOperator({:?}, location: file hash: {}, index start: {}, index stop: {})",
            self.operation.exp.value,
            self.operation.exp.loc.file_hash(),
            self.operation.exp.loc.start(),
            self.operation.exp.loc.end()
        )
    }
}
