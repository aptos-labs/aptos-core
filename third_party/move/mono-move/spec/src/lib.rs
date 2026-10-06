// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Spec sheets bound to their implementation: the data types that the
//! [`spec`] and [`checks`] attributes emit, and the attributes themselves.
//! See the macro crate for the attribute grammar.

pub use mono_move_spec_macro::{checks, spec};
use std::fmt;

/// One row of a spec table.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CheckSpec {
    /// Stable identifier, e.g. `F1`.
    pub id: &'static str,
    /// The table's `## ` heading.
    pub group: &'static str,
    /// The property in English.
    pub property: &'static str,
    /// The predicate.
    pub condition: &'static str,
    /// Why the property matters, or empty.
    pub rationale: &'static str,
}

/// What one method implements, as declared with `#[check(...)]` tags in its
/// body and an optional `#[complexity(...)]`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MethodChecks {
    /// The method's name.
    pub method: &'static str,
    /// Ids of the checks it evaluates, sorted and deduplicated.
    pub checks: &'static [&'static str],
    /// Its declared time complexity, if any.
    pub complexity: Option<Complexity>,
}

/// A method's declared time complexity.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Complexity {
    pub class: ComplexityClass,
    /// What `N` measures, or empty.
    pub measured_in: &'static str,
    /// Why the method has this class, or empty.
    pub because: &'static str,
}

/// The complexity classes a method may declare. There is deliberately
/// nothing beyond `O(N * log(N))`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ComplexityClass {
    Constant,
    Log,
    Linear,
    NLogN,
}

impl fmt::Display for ComplexityClass {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            ComplexityClass::Constant => "O(1)",
            ComplexityClass::Log => "O(log(N))",
            ComplexityClass::Linear => "O(N)",
            ComplexityClass::NLogN => "O(N * log(N))",
        })
    }
}
