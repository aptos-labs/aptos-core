use crate::operator::MutationOperator;
use std::fmt;

/// A mutant is a piece of code that has been mutated by the mutation operator.
/// This represents mutant as a mutation operator.
#[derive(Debug, Clone)]
pub struct Mutant {
    operator: MutationOperator,
}

impl Mutant {
    /// Creates a new mutant.
    pub fn new(operator: MutationOperator) -> Self {
        Self { operator }
    }
}

impl fmt::Display for Mutant {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Mutant: {}", self.operator)
    }
}
