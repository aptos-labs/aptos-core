use crate::operator::{MutantInfo, MutationOperator};
use crate::operators::ExpLoc;
use crate::report::{Mutation, Range};
use codespan::FileId;
use std::fmt;
use std::fmt::Debug;

pub const OPERATOR_NAME: &str = "if_else_replacement";

/// `IfElse` mutation operator.
/// Replaces conditional expressions in if/else statements with literals.
/// Currently only condition field is used.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct IfElse {
    cond: ExpLoc,
    ifexpr: ExpLoc,
    elseexpr: ExpLoc,
}

impl IfElse {
    /// Creates a new instance of the if/else mutation operator.
    #[must_use]
    pub fn new(cond: ExpLoc, ifexpr: ExpLoc, elseexpr: ExpLoc) -> Self {
        Self {
            cond,
            ifexpr,
            elseexpr,
        }
    }
}

impl MutationOperator for IfElse {
    fn apply(&self, source: &str) -> Vec<MutantInfo> {
        let start = self.cond.loc.span().start().to_usize();
        let end = self.cond.loc.span().end().to_usize();
        let cur_op = &source[start..end];

        // Change if/else expression to true/false.
        let ops: Vec<String> = vec![
            "true".to_owned(),
            "false".to_owned(),
            format!("!({})", cur_op),
        ];

        ops.into_iter()
            .map(|op| {
                let mut mutated_source = source.to_string();
                mutated_source.replace_range(start..end, op.as_str());
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
        self.cond.loc.file_id()
    }

    fn name(&self) -> String {
        OPERATOR_NAME.to_string()
    }
}

impl fmt::Display for IfElse {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "IfElseOperator(location: file id: {:?}, index start: {}, index stop: {})",
            self.cond.loc.file_id(),
            self.cond.loc.span().start().to_usize(),
            self.cond.loc.span().end().to_usize()
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use codespan::Files;
    use move_model::ast::{ExpData, Value};
    use move_model::model::Loc;

    #[test]
    fn test_apply_ifelse() {
        let mut files = Files::new();
        let fid = files.add("test", "test");
        let loc = Loc::new(fid, codespan::Span::new(4, 5));
        let expr = ExpData::Value(move_model::model::NodeId::new(1), Value::Bool(true));
        let exp = ExpLoc::new(expr.into_exp(), loc);
        let operator = IfElse::new(exp.clone(), exp.clone(), exp);
        let source = "if (a) { }";
        let expected = vec!["if (true) { }", "if (false) { }", "if (!(a)) { }"];
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
        let expr = ExpData::Value(move_model::model::NodeId::new(1), Value::Bool(true));
        let exp = ExpLoc::new(expr.into_exp(), loc);
        let operator = IfElse::new(exp.clone(), exp.clone(), exp);
        assert_eq!(operator.get_file_id(), fid);
    }
}
