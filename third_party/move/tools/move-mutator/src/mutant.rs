use crate::operator::{MutantInfo, MutationOp, MutationOperator};
use codespan::FileId;
use std::fmt;

/// A mutant is a piece of code that has been mutated by the mutation operator.
/// This represents mutant as a mutation operator.
#[derive(Debug, Clone)]
pub struct Mutant {
    operator: MutationOp,
    module_name: Option<String>,
    function_name: Option<String>,
}

impl Mutant {
    /// Creates a new mutant.
    /// `module_name` argument is optional as during the mutant creation the code may not know the module name uet.
    /// It can be set later using `set_module_name` method.
    pub fn new(operator: MutationOp) -> Self {
        Self {
            operator,
            module_name: None,
            function_name: None,
        }
    }

    /// Returns the file hash of the file that this mutant is in.
    pub fn get_file_id(&self) -> FileId {
        self.operator.get_file_id()
    }

    /// Applies the mutation operator to the given source code, by calling the mutation operator's apply method.
    /// Returns differently mutated source code listings in a vector.
    pub fn apply(&self, source: &str) -> Vec<MutantInfo> {
        trace!("Applying mutation operator: {}", self.operator);
        self.operator.apply(source)
    }

    /// Returns the module name that this mutant is in.
    pub fn get_module_name(&self) -> Option<String> {
        self.module_name.clone()
    }

    /// Sets the module name that this mutant is in.
    pub fn set_module_name(&mut self, module_name: String) {
        self.module_name = Some(module_name);
    }

    /// Returns the function name that this mutant is in.
    pub fn get_function_name(&self) -> Option<String> {
        self.function_name.clone()
    }

    /// Sets the function name that this mutant is in.
    pub fn set_function_name(&mut self, function_name: String) {
        self.function_name = Some(function_name);
    }
}

impl fmt::Display for Mutant {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Mutant: {}", self.operator)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::operators::{binary::Binary, ExpLoc};
    use codespan::Files;
    use move_model::{
        ast::{ExpData, Operation, Value},
        model::{Loc, NodeId},
    };

    #[test]
    fn test_new() {
        let mut files = Files::new();
        let fid = files.add("test", "test");
        let loc = Loc::new(fid, codespan::Span::new(0, 0));
        let operator = MutationOp::new(Box::new(Binary::new(Operation::Add, loc, vec![])));

        let mutant = Mutant::new(operator);
        assert_eq!(format!("{}", mutant), "Mutant: BinaryOperator(Add, location: file id: FileId(1), index start: 0, index stop: 0)");
    }

    #[test]
    fn test_get_file_hash() {
        let mut files = Files::new();
        let fid = files.add("test", "test");
        let loc = Loc::new(fid, codespan::Span::new(0, 0));
        let operator = MutationOp::new(Box::new(Binary::new(Operation::Add, loc, vec![])));

        let mutant = Mutant::new(operator);
        assert_eq!(mutant.get_file_id(), fid);
    }

    #[test]
    fn test_apply() {
        let mut files = Files::new();
        let fid = files.add("test", "test");
        let loc = Loc::new(fid, codespan::Span::new(0, 3));
        let loc2 = Loc::new(fid, codespan::Span::new(0, 1));
        let loc3 = Loc::new(fid, codespan::Span::new(2, 3));
        let e1 = ExpData::Value(NodeId::new(1), Value::Bool(true));
        let e2 = ExpData::Value(NodeId::new(2), Value::Bool(false));
        let exp1 = ExpLoc::new(e1.into_exp(), loc2);
        let exp2 = ExpLoc::new(e2.into_exp(), loc3);

        let operator =
            MutationOp::new(Box::new(Binary::new(Operation::Add, loc, vec![exp1, exp2])));

        let mutant = Mutant::new(operator);
        let source = "2+1";
        let expected = ["2-1", "2*1", "2/1", "2%1"];
        let result = mutant.apply(source);
        assert_eq!(result.len(), expected.len());
        for (i, r) in result.iter().enumerate() {
            assert_eq!(r.mutated_source, expected[i]);
        }
    }
}
