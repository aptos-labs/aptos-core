use crate::report::Mutation;
use codespan::FileId;
use std::{
    fmt,
    fmt::{Debug, Display},
};

/// Mutation result that contains the mutated source code and the modification that was applied.
#[derive(Debug, Clone, PartialEq)]
pub struct MutantInfo {
    /// The mutated source code.
    pub mutated_source: String,
    /// The modification that was applied.
    pub mutation: Mutation,
}

impl MutantInfo {
    /// Creates a new mutation result.
    pub fn new(mutated_source: String, mutation: Mutation) -> Self {
        Self {
            mutated_source,
            mutation,
        }
    }
}

/// Trait for mutation operators.
/// Mutation operators are used to apply mutations to the source code. To keep adding new mutation operators simple,
/// we use a trait that all mutation operators implement.
#[allow(clippy::module_name_repetitions)]
pub trait MutationOperator: Display + Debug + MutationOperatorClone {
    /// Applies the mutation operator to the given source code.
    /// Returns differently mutated source code listings in a vector.
    ///
    /// # Arguments
    ///
    /// * `source` - The source code to apply the mutation operator to.
    ///
    /// # Returns
    ///
    /// * `Vec<MutantInfo>` - A vector of `MutantInfo` instances representing the mutated source code.
    fn apply(&self, source: &str) -> Vec<MutantInfo>;

    /// Returns the id of the file that this mutation operator is in.
    fn get_file_id(&self) -> FileId;

    /// Returns the name of the mutation operator.
    fn name(&self) -> String;
}

/// Trait for cloning mutation operators.
/// This is used to clone the mutation operators - workaround for cloning Boxes.
pub trait MutationOperatorClone {
    fn clone_box(&self) -> Box<dyn MutationOperator>;
}

impl<T> MutationOperatorClone for T
where
    T: 'static + MutationOperator + Clone,
{
    fn clone_box(&self) -> Box<dyn MutationOperator> {
        Box::new(self.clone())
    }
}

impl Clone for Box<dyn MutationOperator> {
    fn clone(&self) -> Box<dyn MutationOperator> {
        self.clone_box()
    }
}

/// The mutation operator to apply.
#[derive(Debug, Clone)]
pub struct MutationOp {
    operator: Box<dyn MutationOperator>,
}

impl MutationOp {
    /// Creates a new mutation operator.
    pub fn new(operator: Box<dyn MutationOperator>) -> Self {
        Self { operator }
    }
}

impl MutationOperator for MutationOp {
    fn apply(&self, source: &str) -> Vec<MutantInfo> {
        debug!("Applying mutation operator: {self}");

        self.operator.apply(source)
    }

    fn get_file_id(&self) -> FileId {
        self.operator.get_file_id()
    }

    fn name(&self) -> String {
        self.operator.name()
    }
}

impl Display for MutationOp {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.operator)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::operators::binary::Binary;
    use codespan::Files;
    use move_model::{ast::Operation, model::Loc};

    #[test]
    fn test_get_file_id() {
        let mut files = Files::new();
        let fid = files.add("test", "test");
        let loc = Loc::new(fid, codespan::Span::new(0, 0));
        let operator = MutationOp::new(Box::new(Binary::new(Operation::Add, loc, vec![])));
        assert_eq!(operator.get_file_id(), fid);
    }
}
