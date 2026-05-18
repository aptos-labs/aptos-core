// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! Fuzz value generation for the `#[test]` attribute.
//!
//! The compiler does not pick fuzz values itself. It collects the parameter
//! constraints (`a in <domain>`, `a != <exclude>`, or absence-of-spec) into a
//! [`ParamSpec`] and asks a [`FuzzValueSource`] to materialize concrete values
//! for each parameter at plan-build time. The default source ([`NoFuzzSource`])
//! errors loudly — install a real source via [`construct_test_plan`] to enable
//! fuzz expansion.

use move_core_types::value::MoveValue;
use move_model::{ast::AttributeValue, ty::Type};

/// A single inclusive/half-open range. Both endpoints are model-AST literals
/// because the compiler does not commit to a concrete representation until the
/// source materializes a sample for the parameter type.
#[derive(Debug, Clone)]
pub struct RangeSpec {
    pub lo: AttributeValue,
    pub hi: AttributeValue,
    pub inclusive_hi: bool,
}

/// A union of discrete literals and ranges. An empty domain is treated as
/// "unrestricted" when used as a fuzz `domain`, and as "no exclusions" when
/// used as an `exclude`.
#[derive(Debug, Clone, Default)]
pub struct Domain {
    pub literals: Vec<AttributeValue>,
    pub ranges: Vec<RangeSpec>,
}

impl Domain {
    pub fn is_empty(&self) -> bool {
        self.literals.is_empty() && self.ranges.is_empty()
    }
}

/// What the compiler resolved for one function parameter after reading the
/// `#[test(...)]` attribute and any constraints attached to it.
#[derive(Debug, Clone)]
pub enum ParamSpec {
    /// `a = <literal>` — a single explicit value.
    Concrete(MoveValue),
    /// `a = [<literal>, <literal>, ...]` — a matrix that expands into N cases.
    Matrix(Vec<MoveValue>),
    /// `a` not mentioned, or `a in ...` / `a != ...`. The fuzz source samples
    /// `n` values from `domain` (unrestricted when empty), subject to
    /// `exclude`.
    Fuzz {
        domain: Domain,
        exclude: Domain,
    },
}

/// Plugged in by the unit-test entrypoint. The compiler never instantiates
/// fuzz values itself; it only collects constraints.
pub trait FuzzValueSource: Send + Sync {
    /// Materialize `n` values of type `ty`, drawn from `domain` (unrestricted
    /// when empty) and avoiding any value in `exclude`. `seed` is provided for
    /// reproducibility.
    fn sample(
        &self,
        ty: &Type,
        domain: &Domain,
        exclude: &Domain,
        n: usize,
        seed: u64,
    ) -> Result<Vec<MoveValue>, String>;
}

/// Default source that produces no samples and reports a clear error. Plug a
/// real implementation in to light up the fuzz path.
pub struct NoFuzzSource;

impl FuzzValueSource for NoFuzzSource {
    fn sample(
        &self,
        _ty: &Type,
        _domain: &Domain,
        _exclude: &Domain,
        _n: usize,
        _seed: u64,
    ) -> Result<Vec<MoveValue>, String> {
        Err(
            "no fuzz value source is registered; install a `FuzzValueSource` to enable \
             implicit-fuzz #[test] expansion"
                .to_string(),
        )
    }
}
