// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! This module defines the gas parameters for walking a Move value graph, which
//! both the interpreter and native functions charge for.

use crate::{gas_schedule::VMGasParameters, ver::gas_feature_versions::RELEASE_V1_50};
use aptos_gas_algebra::{AbstractValueSize, InternalGasPerAbstractValueUnit};
use move_core_types::gas_algebra::InternalGas;

crate::gas_schedule::macros::define_gas_parameters!(
    ValueTraversalGasParameters,
    "misc.value_traversal",
    VMGasParameters => .misc.value_traversal,
    [
        [base: InternalGas, { RELEASE_V1_50.. => "base" }, 11010],
        [
            per_abs_val_unit: InternalGasPerAbstractValueUnit,
            { RELEASE_V1_50.. => "per_abs_val_unit" },
            420
        ],
    ]
);

// Hardcoded constants, because the charges were enabled by a timed feature
// flag before the gas schedule carried these parameters.
const BASE_BEFORE_V1_50: InternalGas = InternalGas::new(11010);
const PER_ABS_VAL_UNIT_BEFORE_V1_50: InternalGasPerAbstractValueUnit =
    InternalGasPerAbstractValueUnit::new(420);

impl ValueTraversalGasParameters {
    /// Returns the cost of walking a value graph of the given abstract size.
    ///
    /// Invariant: the interpreter and the native side both charge through here,
    /// so the two cannot drift apart.
    pub fn cost(&self, feature_version: u64, size: AbstractValueSize) -> InternalGas {
        if feature_version >= RELEASE_V1_50 {
            self.base + self.per_abs_val_unit * size
        } else {
            BASE_BEFORE_V1_50 + PER_ABS_VAL_UNIT_BEFORE_V1_50 * size
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{traits::InitialGasSchedule, ver::gas_feature_versions::RELEASE_V1_49};

    #[test]
    fn test_cost_is_unchanged_across_v1_50() {
        let params = ValueTraversalGasParameters::initial();

        for size in [0, 1, 1000] {
            let size = AbstractValueSize::new(size);
            assert_eq!(
                params.cost(RELEASE_V1_49, size),
                params.cost(RELEASE_V1_50, size)
            );
        }

        assert_eq!(
            params.cost(RELEASE_V1_50, AbstractValueSize::new(1000)),
            InternalGas::new(11010 + 420 * 1000)
        );
    }
}
