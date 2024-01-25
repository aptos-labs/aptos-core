use crate::operator::{MutantInfo, MutationOperator};
use crate::report::{Mutation, Range};
use move_command_line_common::files::FileHash;
use move_compiler::parser::ast::UnaryOp;
use std::fmt;

#[derive(Debug, Copy, Clone)]
pub struct Unary {
    op: UnaryOp,
}

impl Unary {
    pub fn new(op: UnaryOp) -> Self {
        Self { op }
    }
}

impl MutationOperator for Unary {
    fn apply(&self, source: &str) -> Vec<MutantInfo> {
        let start = self.op.loc.start() as usize;
        let end = self.op.loc.end() as usize;
        let cur_op = &source[start..end];

        // For unary operator mutations, we only need to replace the operator with a space (to ensure the same file length).
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
    }

    fn get_file_hash(&self) -> FileHash {
        self.op.loc.file_hash()
    }
}

impl fmt::Display for Unary {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "UnaryOperator({}, location: file hash: {}, index start: {}, index stop: {})",
            self.op.value,
            self.op.loc.file_hash(),
            self.op.loc.start(),
            self.op.loc.end()
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use move_command_line_common::files::FileHash;
    use move_compiler::parser::ast::{UnaryOp, UnaryOp_};
    use move_ir_types::location::Loc;

    #[test]
    fn test_apply_unary_operator() {
        let loc = Loc::new(FileHash::new(""), 0, 1);
        let unary_op = UnaryOp {
            value: UnaryOp_::Not,
            loc,
        };
        let operator = Unary::new(unary_op);
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
        let unary_op = UnaryOp {
            value: UnaryOp_::Not,
            loc,
        };
        let operator = Unary::new(unary_op);
        assert_eq!(operator.get_file_hash(), FileHash::new(""));
    }
}
