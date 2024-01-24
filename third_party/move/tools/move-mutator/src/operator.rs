use crate::operators::binary::BinaryOperator;
use crate::operators::unary::UnaryOperator;
use crate::report::Mutation;
use move_command_line_common::files::FileHash;
use std::fmt;
use std::fmt::Debug;

/// Mutation result that contains the mutated source code and the modification that was applied.
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
pub trait MutationOperatorTrait {
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
}

/// The mutation operator to apply.
#[derive(Debug, Copy, Clone)]
pub enum MutationOperator {
    BinaryOperator(BinaryOperator),
    UnaryOperator(UnaryOperator),
}

impl MutationOperatorTrait for MutationOperator {
    fn apply(&self, source: &str) -> Vec<MutantInfo> {
        debug!("Applying mutation operator: {self}");

        match self {
            MutationOperator::BinaryOperator(bin_op) => bin_op.apply(source),
            MutationOperator::UnaryOperator(unary_op) => unary_op.apply(source),
        }
    }

    fn get_file_hash(&self) -> FileHash {
        match self {
            MutationOperator::BinaryOperator(bin_op) => bin_op.get_file_hash(),
            MutationOperator::UnaryOperator(unary_op) => unary_op.get_file_hash(),
        }
    }
}

impl fmt::Display for MutationOperator {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            MutationOperator::BinaryOperator(bin_op) => write!(f, "{}", bin_op),
            MutationOperator::UnaryOperator(unary_op) => write!(f, "{}", unary_op),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use move_command_line_common::files::FileHash;
    use move_compiler::parser::ast::{BinOp, BinOp_, UnaryOp, UnaryOp_};
    use move_ir_types::location::Loc;

    #[test]
    fn test_apply_binary_operator() {
        let loc = Loc::new(FileHash::new(""), 0, 1);
        let bin_op = BinOp {
            value: BinOp_::Mul,
            loc,
        };
        let operator = MutationOperator::BinaryOperator(BinaryOperator::new(bin_op));
        let source = "*";
        let expected = vec!["+", "-", "/", "%"];
        let result = operator.apply(source);
        assert_eq!(result.len(), expected.len());
        for (i, r) in result.iter().enumerate() {
            assert_eq!(r.mutated_source, expected[i]);
        }
    }

    #[test]
    fn test_apply_unary_operator() {
        let loc = Loc::new(FileHash::new(""), 0, 1);
        let unary_op = UnaryOp {
            value: UnaryOp_::Not,
            loc,
        };
        let operator = MutationOperator::UnaryOperator(UnaryOperator::new(unary_op));
        let source = "!";
        let expected = vec![" "];
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
        let operator = MutationOperator::BinaryOperator(BinaryOperator::new(bin_op));
        assert_eq!(operator.get_file_hash(), FileHash::new(""));
    }
}
