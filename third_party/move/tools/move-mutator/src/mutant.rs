use crate::operator::{MutantInfo, MutationOp, MutationOperator};
use move_command_line_common::files::FileHash;
use move_compiler::parser::ast::ModuleName;
use std::fmt;

/// A mutant is a piece of code that has been mutated by the mutation operator.
/// This represents mutant as a mutation operator.
#[derive(Debug, Clone)]
pub struct Mutant {
    operator: MutationOp,
    module_name: Option<ModuleName>,
}

impl Mutant {
    /// Creates a new mutant.
    pub fn new(operator: MutationOp, module_name: Option<ModuleName>) -> Self {
        Self {
            operator,
            module_name,
        }
    }

    /// Returns the file hash of the file that this mutant is in.
    pub fn get_file_hash(&self) -> FileHash {
        self.operator.get_file_hash()
    }

    /// Applies the mutation operator to the given source code, by calling the mutation operator's apply method.
    /// Returns differently mutated source code listings in a vector.
    pub fn apply(&self, source: &str) -> Vec<MutantInfo> {
        trace!("Applying mutation operator: {}", self.operator);
        self.operator.apply(source)
    }

    /// Returns the module name that this mutant is in.
    pub fn get_module_name(&self) -> Option<ModuleName> {
        self.module_name.clone()
    }

    /// Sets the module name that this mutant is in.
    pub fn set_module_name(&mut self, module_name: ModuleName) {
        self.module_name = Some(module_name);
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
    use crate::operators::binary::Binary;
    use move_command_line_common::files::FileHash;
    use move_compiler::parser::ast::{BinOp, BinOp_};
    use move_ir_types::location::Loc;

    #[test]
    fn test_new() {
        let loc = Loc::new(FileHash::new(""), 0, 0);
        let operator = MutationOp::BinaryOp(Binary::new(BinOp {
            value: BinOp_::Add,
            loc,
        }));
        let mutant = Mutant::new(operator, None);
        assert_eq!(format!("{}", mutant), "Mutant: BinaryOperator(+, location: file hash: e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855, index start: 0, index stop: 0)");
    }

    #[test]
    fn test_get_file_hash() {
        let loc = Loc::new(FileHash::new(""), 0, 0);
        let operator = MutationOp::BinaryOp(Binary::new(BinOp {
            value: BinOp_::Add,
            loc,
        }));
        let mutant = Mutant::new(operator, None);
        assert_eq!(mutant.get_file_hash(), FileHash::new(""));
    }

    #[test]
    fn test_apply() {
        let loc = Loc::new(FileHash::new(""), 0, 1);
        let operator = MutationOp::BinaryOp(Binary::new(BinOp {
            value: BinOp_::Add,
            loc,
        }));
        let mutant = Mutant::new(operator, None);
        let source = "+";
        let expected = vec!["-", "*", "/", "%"];
        let result = mutant.apply(source);
        assert_eq!(result.len(), expected.len());
        for (i, r) in result.iter().enumerate() {
            assert_eq!(r.mutated_source, expected[i]);
        }
    }
}
