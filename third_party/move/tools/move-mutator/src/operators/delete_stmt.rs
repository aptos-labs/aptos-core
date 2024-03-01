use crate::operator::{MutantInfo, MutationOperator};
use crate::operators::MOVE_EMPTY_STMT;
use crate::report::{Mutation, Range};
use codespan::FileId;
use move_model::ast::Exp;
use move_model::model::Loc;
use std::fmt;
use std::fmt::Debug;

pub const OPERATOR_NAME: &str = "delete_statement";

/// Statement delete operator.
/// Deletes statements which can be potentially deleted, still allowing the code to compile
/// properly.
#[derive(Debug, Clone)]
pub struct DeleteStmt {
    operation: Exp,
    loc: Loc,
}

impl DeleteStmt {
    /// Creates a new instance of the delete mutation operator.
    #[must_use]
    pub fn new(operation: Exp, loc: Loc) -> Self {
        Self { operation, loc }
    }
}

impl MutationOperator for DeleteStmt {
    fn apply(&self, source: &str) -> Vec<MutantInfo> {
        let (start, end) = (
            self.loc.span().start().to_usize(),
            self.loc.span().end().to_usize(),
        );
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
                        OPERATOR_NAME.to_string(),
                        cur_op.to_string(),
                        op.to_string(),
                    ),
                )
            })
            .collect()
    }

    fn get_file_id(&self) -> FileId {
        self.loc.file_id()
    }

    fn name(&self) -> String {
        OPERATOR_NAME.to_string()
    }
}

impl fmt::Display for DeleteStmt {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "DeleteStmtOperator({:?}, location: file hash: {:?}, index start: {}, index stop: {})",
            self.operation,
            self.loc.file_id(),
            self.loc.span().start(),
            self.loc.span().end()
        )
    }
}
