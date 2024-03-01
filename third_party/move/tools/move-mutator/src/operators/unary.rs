use crate::operator::{MutantInfo, MutationOperator};
use crate::operators::ExpLoc;
use crate::report::{Mutation, Range};
use codespan::FileId;
use move_model::ast::Operation;
use move_model::model::Loc;
use std::fmt;

pub const OPERATOR_NAME: &str = "unary_operator_replacement";

/// Represents a unary operator mutation.
#[derive(Debug, Clone)]
pub struct Unary {
    operation: Operation,
    loc: Loc,
    exps: Vec<ExpLoc>,
}

impl Unary {
    pub fn new(operation: Operation, loc: Loc, exps: Vec<ExpLoc>) -> Self {
        Self {
            operation,
            loc,
            exps,
        }
    }
}

impl MutationOperator for Unary {
    fn apply(&self, source: &str) -> Vec<MutantInfo> {
        if self.exps.len() != 1 {
            warn!(
                "UnaryOperator: Expected exactly one expression, got {}",
                self.exps.len()
            );
            return vec![];
        }

        let start = self.loc.span().start().to_usize();
        // Adjust start to omit whitespaces before the operator
        let start = source[start..]
            .find(|c: char| !c.is_whitespace())
            .map_or(start, |i| start + i);
        let end = self.loc.span().end().to_usize();
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

    fn get_file_id(&self) -> FileId {
        self.loc.file_id()
    }

    fn name(&self) -> String {
        OPERATOR_NAME.to_string()
    }
}

impl fmt::Display for Unary {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "UnaryOperator({:?}, location: file id: {:?}, index start: {}, index stop: {})",
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
    fn test_apply_unary_operator() {
        let mut files = Files::new();
        let fid = files.add("test", "test");
        let loc = Loc::new(fid, codespan::Span::new(0, 1));
        let e1 = ExpData::Value(NodeId::new(1), Value::Bool(true));
        let exp = ExpLoc::new(e1.into_exp(), loc.clone());

        let operator = Unary::new(Operation::Not, loc, vec![exp]);
        let source = "!";
        let expected = vec![" "];
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
        let operator = Unary::new(Operation::Not, loc, vec![]);
        assert_eq!(operator.get_file_id(), fid);
    }
}
