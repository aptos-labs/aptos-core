// Copyright © Eiger
// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    operator::{MutantInfo, MutationOperator},
    operators::ExpLoc,
    report::{Mutation, Range},
};
use codespan::FileId;
use move_model::{ast::Operation, model::Loc};
use std::fmt;

pub const OPERATOR_NAME: &str = "binary_operator_swap";

/// The binary swap mutation operator.
#[derive(Debug, Clone)]
pub struct BinarySwap {
    operation: Operation,
    loc: Loc,
    exps: Vec<ExpLoc>,
}

impl BinarySwap {
    pub fn new(operation: Operation, loc: Loc, exps: Vec<ExpLoc>) -> Self {
        Self {
            operation,
            loc,
            exps,
        }
    }
}

impl MutationOperator for BinarySwap {
    fn apply(&self, source: &str) -> Vec<MutantInfo> {
        // There is no point in swapping the operator for these cases as it would result in the same expression.
        if self.operation == Operation::Add
            || self.operation == Operation::Mul
            || self.operation == Operation::Eq
            || self.operation == Operation::Neq
        {
            return vec![];
        }

        // Check if we've exactly two expressions.
        if self.exps.len() != 2 {
            warn!(
                "BinarySwapOperator: Expected exactly two expressions, got {}",
                self.exps.len()
            );
            return vec![];
        }

        // We need to extract operator position, but we must use the positions of expressions to avoid
        // extracting the operator of a different binary expression.
        let left = &self.exps[0].loc;
        let right = &self.exps[1].loc;
        let start = left.span().end().to_usize();
        // Adjust start to omit whitespaces before the operator
        let start = source[start..]
            .find(|c: char| !c.is_whitespace())
            .map_or(start, |i| start + i);
        let end = right.span().start().to_usize();
        // Adjust end to omit whitespaces after the operator
        let end = source[..end]
            .rfind(|c: char| !c.is_whitespace())
            .map_or(end, |i| i + 1);
        let binop_str = &source[start..end];

        let start = left.span().start().to_usize();
        let end = right.span().end().to_usize();
        let cur_op = &source[start..end];

        let left_str = &source[left.span().start().to_usize()..left.span().end().to_usize()];
        let right_str = &source[right.span().start().to_usize()..right.span().end().to_usize()];

        let mut mutated_source = source.to_string();
        let mut op = right_str.to_owned();
        op.push_str(binop_str);
        op.push_str(left_str);

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
    fn get_file_id(&self) -> FileId {
        self.loc.file_id()
    }

    fn name(&self) -> String {
        OPERATOR_NAME.to_string()
    }
}

impl fmt::Display for BinarySwap {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "BinarySwapOperator(location: file id: {:?}, index start: {}, index stop: {})",
            self.loc.file_id(),
            self.exps[0].loc.span().start().to_usize(),
            self.exps[1].loc.span().end().to_usize()
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use codespan::Files;

    #[test]
    fn test_get_file_id() {
        let mut files = Files::new();
        let fid = files.add("test", "test");
        let loc = Loc::new(fid, codespan::Span::new(0, 0));
        let operator = BinarySwap::new(Operation::Add, loc, vec![]);
        assert_eq!(operator.get_file_id(), fid);
    }
}
