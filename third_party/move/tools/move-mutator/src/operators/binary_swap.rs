use crate::operator::{MutantInfo, MutationOperator};
use crate::report::{Mutation, Range};
use move_command_line_common::files::FileHash;
use move_compiler::parser::ast::{BinOp, BinOp_};
use move_compiler::typing::ast::Exp;
use std::fmt;

pub const OPERATOR_NAME: &str = "binary_operator_swap";

/// The binary swap mutation operator.
#[derive(Debug, Clone)]
pub struct BinarySwap {
    operator: BinOp,
    left: Exp,
    right: Exp,
}

impl BinarySwap {
    pub fn new(operator: BinOp, left: Exp, right: Exp) -> Self {
        Self {
            operator,
            left,
            right,
        }
    }
}

impl MutationOperator for BinarySwap {
    fn apply(&self, source: &str) -> Vec<MutantInfo> {
        // There is no point in swapping the operator for these cases as it would result in the same expression.
        if self.operator.value == BinOp_::Add
            || self.operator.value == BinOp_::Mul
            || self.operator.value == BinOp_::Eq
            || self.operator.value == BinOp_::Neq
        {
            return vec![];
        }

        let start = self.left.exp.loc.start() as usize;
        let end = self.right.exp.loc.end() as usize;
        let cur_op = &source[start..end];

        let left_str =
            &source[self.left.exp.loc.start() as usize..self.left.exp.loc.end() as usize];
        let right_str =
            &source[self.right.exp.loc.start() as usize..self.right.exp.loc.end() as usize];
        let binop_str =
            &source[self.operator.loc.start() as usize..self.operator.loc.end() as usize];

        let mut mutated_source = source.to_string();
        let mut op = right_str.to_owned();
        op.push_str(&binop_str);
        op.push_str(&left_str);

        mutated_source.replace_range(start..end, op.as_str());

        vec![MutantInfo::new(
            mutated_source,
            Mutation::new(
                Range::new(start, end),
                OPERATOR_NAME.to_string(),
                cur_op.to_string(),
                op.to_string(),
            ),
        )]
    }

    // We can use either `left` or `right` as they have to be in one file.
    fn get_file_hash(&self) -> FileHash {
        self.left.exp.loc.file_hash()
    }

    fn name(&self) -> String {
        OPERATOR_NAME.to_string()
    }
}

impl fmt::Display for BinarySwap {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "BinarySwapOperator(location: file hash: {}, index start: {}, index stop: {})",
            self.get_file_hash(),
            self.left.exp.loc.start(),
            self.right.exp.loc.end()
        )
    }
}
