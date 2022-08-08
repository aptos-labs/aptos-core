// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0
use crate::gas_meter::{FromOnChainGasSchedule, InitialGasSchedule, ToOnChainGasSchedule};
use std::collections::BTreeMap;

macro_rules! define_gas_parameters_for_transaction {
    ($([$name: ident, $key: literal, $initial: expr $(,)?]),* $(,)?) => {
        /// Transaction-level gas parameters.
        ///
        /// Note: due to performance considerations, this is represented as a fixed struct instead of
        /// some other data structures that require complex lookups.
        #[derive(Debug, Clone)]
        pub struct TransactionGasParameters {
            $(pub $name : u64),*
        }

        impl FromOnChainGasSchedule for TransactionGasParameters {
            fn from_on_chain_gas_schedule(gas_schedule: &BTreeMap<String, u64>) -> Option<Self> {
                Some(Self { $($name: gas_schedule.get(&format!("txn.{}", $key)).cloned()?),* })
            }
        }

        impl ToOnChainGasSchedule for TransactionGasParameters {
            fn to_on_chain_gas_schedule(&self) -> Vec<(String, u64)> {
                vec![$((format!("txn.{}", $key), self.$name)),*]
            }
        }

        impl TransactionGasParameters {
            pub fn zeros() -> Self {
                Self {
                    $($name: 0),*
                }
            }
        }

        impl InitialGasSchedule for TransactionGasParameters {
            fn initial() -> Self {
                Self {
                    $($name: $initial),*
                }
            }
        }

        #[test]
        fn keys_should_be_unique() {
            let mut map: BTreeMap<&str, ()> = BTreeMap::new();

            for key in [$($key),*] {
                assert!(map.insert(key, ()).is_none());
            }
        }
    }
}

define_gas_parameters_for_transaction!(
    // The flat minimum amount of gas required for any transaction.
    // Charged at the start of execution.
    [min_transaction_gas_units, "min_transaction_gas_units", 600],
    // Any transaction over this size will be charged an additional amount per byte.
    [large_transaction_cutoff, "large_transaction_cutoff", 600],
    // The units of gas that to be charged per byte over the `large_transaction_cutoff` in addition to
    // `min_transaction_gas_units` for transactions whose size exceeds `large_transaction_cutoff`.
    [intrinsic_gas_per_byte, "intrinsic_gas_per_byte", 8],
    // ~5 microseconds should equal one unit of computational gas. We bound the maximum
    // computational time of any given transaction at roughly 20 seconds. We want this number and
    // `MAX_PRICE_PER_GAS_UNIT` to always satisfy the inequality that
    // MAXIMUM_NUMBER_OF_GAS_UNITS * MAX_PRICE_PER_GAS_UNIT < min(u64::MAX, GasUnits<GasCarrier>::MAX)
    [
        maximum_number_of_gas_units,
        "maximum_number_of_gas_units",
        4_000_000
    ],
    // The minimum gas price that a transaction can be submitted with.
    // TODO(Gas): should probably change this to something > 0
    [min_price_per_gas_unit, "min_price_per_gas_unit", 0],
    // The maximum gas unit price that a transaction can be submitted with.
    [max_price_per_gas_unit, "max_price_per_gas_unit", 10_000],
    [
        max_transaction_size_in_bytes,
        "max_transaction_size_in_bytes",
        8192
    ],
    [gas_unit_scaling_factor, "gas_unit_scaling_factor", 1000],
);

impl TransactionGasParameters {
    // TODO(Gas): Right now we are relying on this to avoid div by zero errors when using the all-zero
    //            gas parameters. See if there's a better way we can handle this.
    fn scaling_factor(&self) -> u64 {
        match self.gas_unit_scaling_factor {
            0 => 1,
            x => x,
        }
    }

    /// Calculate the intrinsic gas for the transaction based upon its size in bytes/words.
    pub fn calculate_intrinsic_gas(&self, transaction_size: u64) -> u64 {
        let min_transaction_fee = self.min_transaction_gas_units;

        if transaction_size > self.large_transaction_cutoff {
            let excess = transaction_size - self.large_transaction_cutoff;
            min_transaction_fee + (self.intrinsic_gas_per_byte * excess)
        } else {
            min_transaction_fee
        }
    }

    pub fn to_external_units(&self, internal_units: u64) -> u64 {
        internal_units / self.scaling_factor()
    }

    pub fn to_internal_units(&self, external_units: u64) -> u64 {
        external_units * self.scaling_factor()
    }
}
