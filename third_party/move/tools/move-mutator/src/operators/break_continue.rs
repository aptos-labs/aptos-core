// Copyright © Eiger
// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    operator::{MutantInfo, MutationOperator},
    operators::{MOVE_BREAK, MOVE_CONTINUE, MOVE_EMPTY_STMT},
    report::{Mutation, Range},
};
use codespan::FileId;
use move_model::model::Loc;
use std::{fmt, fmt::Debug};

pub const OPERATOR_NAME: &str = "break_continue_replacement";

/// Break and continue mutation operator.
/// Replaces break and continue statements with each other or deletes them.
#[derive(Debug, Clone)]
pub struct BreakContinue {
    loc: Loc,
}

impl BreakContinue {
    /// Creates a new instance of the break/continue mutation operator.
    #[must_use]
    pub fn new(loc: Loc) -> Self {
        Self { loc }
    }
}

impl MutationOperator for BreakContinue {
    fn apply(&self, source: &str) -> Vec<MutantInfo> {
        let start = self.loc.span().start().to_usize();
        let end = self.loc.span().end().to_usize();
        let cur_op = &source[start..end];

        // Group of exchangeable break/continue statements.
        let ops: Vec<&str> = match cur_op {
            "break" => {
                vec![MOVE_CONTINUE, MOVE_EMPTY_STMT]
            },
            "continue" => {
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

    fn get_file_id(&self) -> FileId {
        self.loc.file_id()
    }

    fn name(&self) -> String {
        OPERATOR_NAME.to_string()
    }
}

impl fmt::Display for BreakContinue {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "BreakContinueOperator(location: file id: {:?}, index start: {}, index stop: {})",
            self.loc.file_id(),
            self.loc.span().start(),
            self.loc.span().end()
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use codespan::Files;

    #[test]
    fn test_apply_continue() {
        let mut files = Files::new();
        let fid = files.add("test", "test");
        let loc = Loc::new(fid, codespan::Span::new(0, 8));

        let operator = BreakContinue::new(loc);
        let source = MOVE_CONTINUE;
        let expected = [MOVE_BREAK, MOVE_EMPTY_STMT];
        let result = operator.apply(source);
        assert_eq!(result.len(), expected.len());
        for (i, r) in result.iter().enumerate() {
            assert_eq!(r.mutated_source, expected[i]);
        }
    }

    #[test]
    fn test_get_file_id() {
        let mut files = Files::new();
        let fid = files.add("test", "test");
        let loc = Loc::new(fid, codespan::Span::new(0, 0));
        let operator = BreakContinue::new(loc);
        assert_eq!(operator.get_file_id(), fid);
    }
}
