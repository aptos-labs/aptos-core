use crate::operator::{MutantInfo, MutationOperatorTrait};
use crate::report::{Mutation, Range};
use move_command_line_common::files::FileHash;
use move_compiler::parser::ast::{BinOp, BinOp_};
use std::fmt;

/// The binary mutation operator.
#[derive(Debug, Copy, Clone)]
pub struct BinaryOperator {
    operation: BinOp,
}

impl BinaryOperator {
    pub fn new(operation: BinOp) -> Self {
        Self { operation }
    }
}

impl MutationOperatorTrait for BinaryOperator {
    fn apply(&self, source: &str) -> Vec<MutantInfo> {
        let start = self.operation.loc.start() as usize;
        let end = self.operation.loc.end() as usize;
        let cur_op = &source[start..end];

        // Group of exchangeable binary operators - we only want to replace the operator with a different one
        // within the same group.
        let ops: Vec<&str> = match self.operation.value {
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
    }

    fn get_file_hash(&self) -> FileHash {
        self.operation.loc.file_hash()
    }
}

impl fmt::Display for BinaryOperator {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "BinaryOperator({}, location: file hash: {}, index start: {}, index stop: {})",
            self.operation.value,
            self.operation.loc.file_hash(),
            self.operation.loc.start(),
            self.operation.loc.end()
        )
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
            value: BinOp_::Add,
            loc,
        };
        let operator = BinaryOperator::new(bin_op);
        let source = "+";
        let expected = vec!["-", "*", "/", "%"];
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
        let operator = BinaryOperator::new(bin_op);
        assert_eq!(operator.get_file_hash(), FileHash::new(""));
    }
}
