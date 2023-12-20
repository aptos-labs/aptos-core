use crate::report::{Mutation, Range};
use move_command_line_common::files::FileHash;
use move_compiler::parser::ast::{BinOp, BinOp_, UnaryOp};
use std::fmt;

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

/// The mutation operator to apply.
#[derive(Debug, Copy, Clone)]
pub enum MutationOperator {
    BinaryOperator(BinOp),
    UnaryOperator(UnaryOp),
}

impl MutationOperator {
    /// Applies the mutation operator to the given source code.
    /// Returns differently mutated source code listings in a vector.
    pub fn apply(&self, source: &str) -> Vec<MutantInfo> {
        match self {
            MutationOperator::BinaryOperator(bin_op) => {
                let start = bin_op.loc.start() as usize;
                let end = bin_op.loc.end() as usize;
                let cur_op = &source[start..end];

                // Group of exchangeable binary operators - we only want to replace the operator with a different one
                // within the same group.
                let ops: Vec<&str> = match bin_op.value {
                    BinOp_::Add | BinOp_::Sub | BinOp_::Mul | BinOp_::Div | BinOp_::Mod => {
                        vec!["+", "-", "*", "/", "%"]
                    },
                    BinOp_::BitOr | BinOp_::BitAnd | BinOp_::Xor => {
                        vec!["|", "&", "^"]
                    },
                    BinOp_::Shl | BinOp_::Shr => {
                        vec!["<<", ">>"]
                    },
                    _ => vec![],
                };

                ops.into_iter()
                    .filter(|v| cur_op != *v)
                    .map(|op| {
                        let mut mutated_source = source.to_string();
                        mutated_source.replace_range(start..end, op);
                        MutantInfo::new(
                            mutated_source,
                            Mutation::new(
                                Range::new(start, end),
                                "binary_operator_replacement".to_string(),
                                cur_op.to_string(),
                                op.to_string(),
                            ),
                        )
                    })
                    .collect()
            },
            MutationOperator::UnaryOperator(unary_op) => {
                let start = unary_op.loc.start() as usize;
                let end = unary_op.loc.end() as usize;
                let cur_op = &source[start..end];

                // For unary operator mutations, we only need to replace the operator with a space (to ensure the same file length)
                vec![" "]
                    .into_iter()
                    .map(|op| {
                        let mut mutated_source = source.to_string();
                        mutated_source.replace_range(start..end, op);
                        MutantInfo::new(
                            mutated_source,
                            Mutation::new(
                                Range::new(start, end),
                                "unary_operator_replacement".to_string(),
                                cur_op.to_string(),
                                op.to_string(),
                            ),
                        )
                    })
                    .collect()
            },
        }
    }

    /// Returns the file hash of the file that this mutation operator is in.
    pub fn get_file_hash(&self) -> FileHash {
        match self {
            MutationOperator::BinaryOperator(bin_op) => bin_op.loc.file_hash(),
            MutationOperator::UnaryOperator(unary_op) => unary_op.loc.file_hash(),
        }
    }
}

impl fmt::Display for MutationOperator {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            MutationOperator::BinaryOperator(bin_op) => write!(
                f,
                "BinaryOperator({}, location: file hash: {}, index start: {}, index stop: {})",
                bin_op.value,
                bin_op.loc.file_hash(),
                bin_op.loc.start(),
                bin_op.loc.end()
            ),
            MutationOperator::UnaryOperator(unary_op) => write!(
                f,
                "UnaryOperator({}, location: file hash: {}, index start: {}, index stop: {})",
                unary_op.value,
                unary_op.loc.file_hash(),
                unary_op.loc.start(),
                unary_op.loc.end()
            ),
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
        let operator = MutationOperator::BinaryOperator(bin_op);
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
        let operator = MutationOperator::UnaryOperator(unary_op);
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
        let operator = MutationOperator::BinaryOperator(bin_op);
        assert_eq!(operator.get_file_hash(), FileHash::new(""));
    }
}
