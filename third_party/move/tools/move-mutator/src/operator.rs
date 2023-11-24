use move_compiler::parser::ast::{BinOp, UnaryOp};
use std::fmt;

/// The mutation operator to apply.
#[derive(Debug, Copy, Clone)]
pub enum MutationOperator {
    BinaryOperator(BinOp),
    UnaryOperator(UnaryOp),
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
