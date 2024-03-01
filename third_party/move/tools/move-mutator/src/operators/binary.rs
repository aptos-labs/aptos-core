use crate::operator::{MutantInfo, MutationOperator};
use crate::operators::ExpLoc;
use crate::report::{Mutation, Range};
use codespan::FileId;
use move_model::ast::Operation;
use move_model::model::Loc;
use std::fmt;

pub const OPERATOR_NAME: &str = "binary_operator_replacement";

/// The binary mutation operator.
#[derive(Debug, Clone)]
pub struct Binary {
    operation: Operation,
    loc: Loc,
    exps: Vec<ExpLoc>,
}

impl Binary {
    pub fn new(operation: Operation, loc: Loc, exps: Vec<ExpLoc>) -> Self {
        Self {
            operation,
            loc,
            exps,
        }
    }
}

impl MutationOperator for Binary {
    fn apply(&self, source: &str) -> Vec<MutantInfo> {
        if self.exps.len() != 2 {
            warn!(
                "BinaryOperator: Expected exactly two expressions, got {}",
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
        let cur_op = &source[start..end];

        // Group of exchangeable binary operators - we only want to replace the operator with a different one
        // within the same group.
        let ops: Vec<&str> = match self.operation {
            Operation::Add | Operation::Sub | Operation::Mul | Operation::Div | Operation::Mod => {
                vec!["+", "-", "*", "/", "%"]
            },
            Operation::BitOr | Operation::BitAnd | Operation::Xor => {
                vec!["|", "&", "^"]
            },
            Operation::Shl | Operation::Shr => {
                vec!["<<", ">>"]
            },
            Operation::Or | Operation::And => {
                vec!["||", "&&"]
            },
            Operation::Eq
            | Operation::Neq
            | Operation::Lt
            | Operation::Gt
            | Operation::Le
            | Operation::Ge => {
                vec!["==", "!=", "<", ">", "<=", ">="]
            },
            _ => {
                vec![]
            },
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

impl fmt::Display for Binary {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "BinaryOperator({:?}, location: file id: {:?}, index start: {}, index stop: {})",
            self.operation,
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
    use move_model::ast::{ExpData, Value};
    use move_model::model::NodeId;

    #[test]
    fn test_apply_binary_operator() {
        let mut files = Files::new();
        let fid = files.add("test", "test");
        let loc = Loc::new(fid, codespan::Span::new(0, 3));
        let loc2 = Loc::new(fid, codespan::Span::new(0, 1));
        let loc3 = Loc::new(fid, codespan::Span::new(2, 3));
        let e1 = ExpData::Value(NodeId::new(1), Value::Bool(true));
        let e2 = ExpData::Value(NodeId::new(2), Value::Bool(false));
        let exp1 = ExpLoc::new(e1.into_exp(), loc2);
        let exp2 = ExpLoc::new(e2.into_exp(), loc3);

        let operator = Binary::new(Operation::Add, loc, vec![exp1, exp2]);
        let source = "5+2";
        let expected = vec!["5-2", "5*2", "5/2", "5%2"];
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
        let operator = Binary::new(Operation::Add, loc, vec![]);
        assert_eq!(operator.get_file_id(), fid);
    }
}
