use crate::operators::binary::Binary;
use crate::operators::break_continue::BreakContinue;
use crate::operators::delete_stmt::DeleteStmt;
use crate::operators::ifelse::IfElse;
use crate::operators::literal::Literal;
use crate::operators::unary::Unary;
use crate::report::Mutation;
use move_command_line_common::files::FileHash;
use std::fmt;
use std::fmt::Debug;

/// Mutation result that contains the mutated source code and the modification that was applied.
#[derive(Debug, Clone, PartialEq)]
pub struct MutantInfo {
    /// The mutated source code.
    pub mutated_source: String,
    /// The modification that was applied.
    pub mutation: Mutation,
}

impl MutantInfo {
    /// Creates a new mutation result.
    pub fn new(mutated_source: String, mutation: Mutation) -> Self {
        Self {
            mutated_source,
            mutation,
        }
    }
}

/// Trait for mutation operators.
/// Mutation operators are used to apply mutations to the source code. To keep adding new mutation operators simple,
/// we use a trait that all mutation operators implement.
#[allow(clippy::module_name_repetitions)]
pub trait MutationOperator {
    /// Applies the mutation operator to the given source code.
    /// Returns differently mutated source code listings in a vector.
    ///
    /// # Arguments
    ///
    /// * `source` - The source code to apply the mutation operator to.
    ///
    /// # Returns
    ///
    /// * `Vec<MutantInfo>` - A vector of `MutantInfo` instances representing the mutated source code.
    fn apply(&self, source: &str) -> Vec<MutantInfo>;

    /// Returns the file hash of the file that this mutation operator is in.
    fn get_file_hash(&self) -> FileHash;

    /// Returns the name of the mutation operator.
    fn name(&self) -> String;
}

/// The mutation operator to apply.
#[derive(Debug, Clone)]
pub enum MutationOp {
    BinaryOp(Binary),
    UnaryOp(Unary),
    BreakContinue(BreakContinue),
    Literal(Literal),
    IfElse(IfElse),
    DeleteStmt(DeleteStmt),
}

impl MutationOperator for MutationOp {
    fn apply(&self, source: &str) -> Vec<MutantInfo> {
        debug!("Applying mutation operator: {self}");

        match self {
            MutationOp::BinaryOp(bin_op) => bin_op.apply(source),
            MutationOp::UnaryOp(unary_op) => unary_op.apply(source),
            MutationOp::BreakContinue(break_continue) => break_continue.apply(source),
            MutationOp::Literal(literal) => literal.apply(source),
            MutationOp::IfElse(if_else) => if_else.apply(source),
            MutationOp::DeleteStmt(delete_stmt) => delete_stmt.apply(source),
        }
    }

    fn get_file_hash(&self) -> FileHash {
        match self {
            MutationOp::BinaryOp(bin_op) => bin_op.get_file_hash(),
            MutationOp::UnaryOp(unary_op) => unary_op.get_file_hash(),
            MutationOp::BreakContinue(break_continue) => break_continue.get_file_hash(),
            MutationOp::Literal(literal) => literal.get_file_hash(),
            MutationOp::IfElse(if_else) => if_else.get_file_hash(),
            MutationOp::DeleteStmt(delete_stmt) => delete_stmt.get_file_hash(),
        }
    }

    fn name(&self) -> String {
        match self {
            MutationOp::BinaryOp(bin_op) => bin_op.name(),
            MutationOp::UnaryOp(unary_op) => unary_op.name(),
            MutationOp::BreakContinue(break_continue) => break_continue.name(),
            MutationOp::Literal(literal) => literal.name(),
            MutationOp::IfElse(if_else) => if_else.name(),
            MutationOp::DeleteStmt(delete_stmt) => delete_stmt.name(),
        }
    }
}

impl fmt::Display for MutationOp {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            MutationOp::BinaryOp(bin_op) => write!(f, "{bin_op}"),
            MutationOp::UnaryOp(unary_op) => write!(f, "{unary_op}"),
            MutationOp::BreakContinue(break_continue) => write!(f, "{break_continue}"),
            MutationOp::Literal(literal) => write!(f, "{literal}"),
            MutationOp::IfElse(if_else) => write!(f, "{if_else}"),
            MutationOp::DeleteStmt(delete_stmt) => write!(f, "{delete_stmt}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use move_command_line_common::files::FileHash;
    use move_compiler::parser::ast::{BinOp, BinOp_};
    use move_ir_types::location::Loc;

    #[test]
    fn test_apply_binary_operator() {
        let loc = Loc::new(FileHash::new(""), 0, 1);
        let bin_op = BinOp {
            value: BinOp_::Mul,
            loc,
        };
        let operator = MutationOp::BinaryOp(Binary::new(bin_op));
        let source = "*";
        let expected = vec!["+", "-", "/", "%"];
        let result = operator.apply(source);
        assert_eq!(result.len(), expected.len());
        for (i, r) in result.iter().enumerate() {
            assert_eq!(r.mutated_source, expected[i]);
        }
    }

    #[test]
    fn test_get_file_hash() {
        let loc = Loc::new(FileHash::new(""), 0, 0);
        let bin_op = BinOp {
            value: BinOp_::Add,
            loc,
        };
        let operator = MutationOp::BinaryOp(Binary::new(bin_op));
        assert_eq!(operator.get_file_hash(), FileHash::new(""));
    }
}
