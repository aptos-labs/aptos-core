// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

//! This module defines some macros to help implement the mapping between the on-chain gas schedule
//! and its rust representation.

macro_rules! expand_get_impl_for_native_gas_params {
    ($params: ident $(.$field: ident)+, $map: ident, $prefix: literal, optional $key: literal) => {
        if let Some(val) = $map.get(&format!("{}.{}", $prefix, $key)) {
            $params $(.$field)+ = (*val).into();
        }
    };
    ($params: ident $(.$field: ident)+, $map: ident, $prefix: literal, $key: literal) => {
        $params $(.$field)+ = $map.get(&format!("{}.{}", $prefix, $key)).cloned()?.into();
    };
}

macro_rules! define_gas_parameters_for_natives {
    (
        $param_ty: ty,
        $package_name: literal,
        [$([$(.$field: ident)+, $(optional $($dummy: ident)?)? $key: literal, $initial_val: expr]),+ $(,)?]
        $(, allow_unmapped = $allow_unmapped: expr)?
    ) => {
        impl crate::gas_meter::FromOnChainGasSchedule for $param_ty {
            fn from_on_chain_gas_schedule(gas_schedule: &std::collections::BTreeMap<String, u64>, feature_version: u64) -> Option<Self> {
                let mut params = <$param_ty>::zeros();

                $(
                    $crate::natives::expand_get_impl_for_native_gas_params!(params $(.$field)+, gas_schedule, $package_name, $(optional $($dummy)?)? $key);
                )*

                Some(params)
            }
        }

        impl crate::gas_meter::ToOnChainGasSchedule for $param_ty {
            fn to_on_chain_gas_schedule(&self, feature_version: u64) -> Vec<(String, u64)> {
                [$(($key, u64::from(self $(.$field)+))),*]
                    .into_iter().map(|(key, val)| (format!("{}.{}", $package_name, key), val)).collect()
            }
        }

        impl crate::gas_meter::InitialGasSchedule for $param_ty {
            fn initial() -> Self {
                let mut params = <$param_ty>::zeros();

                $(
                    params $(.$field)+ = $initial_val.into();
                )*

                params
            }
        }

        #[test]
        fn keys_should_be_unique() {
            let mut map = std::collections::BTreeMap::<&str, ()>::new();

            for key in [$($key),*] {
                if map.insert(key.clone(), ()).is_some() {
                    panic!("duplicated key {}", key);
                }
            }
        }

        #[test]
        fn paths_must_be_unique() {
            let mut map = std::collections::BTreeMap::<&str, ()>::new();

            for path in [$(stringify!($($field).*)),*] {
                if map.insert(path.clone(), ()).is_some() {
                    panic!("duplicated path {}", path);
                }
            }
        }

        #[test]
        fn all_parameters_mapped() {
            let total = format!("{:?}", &<$param_ty>::zeros()).matches(": 0").count();
            let mapped = [$($key),*].len() $(+ $allow_unmapped)?;
            if mapped != total {
                panic!("only {} out of the {} entries are mapped", mapped, total)
            }
        }
    };
}

pub(crate) use define_gas_parameters_for_natives;
pub(crate) use expand_get_impl_for_native_gas_params;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gas_meter::{FromOnChainGasSchedule, LATEST_GAS_FEATURE_VERSION};
    use move_core_types::gas_algebra::InternalGas;

    #[derive(Debug, Clone)]
    struct GasParameters {
        pub foo: InternalGas,
        pub bar: InternalGas,
    }

    impl GasParameters {
        pub fn zeros() -> Self {
            Self {
                foo: 0.into(),
                bar: 0.into(),
            }
        }
    }

    define_gas_parameters_for_natives!(
        GasParameters,
        "test",
        [[.foo, "foo", 0], [.bar, optional "bar", 0]]
    );

    #[test]
    fn optional_should_be_honored() {
        assert!(matches!(
            GasParameters::from_on_chain_gas_schedule(
                &[("test.foo".to_string(), 0)].into_iter().collect(),
                LATEST_GAS_FEATURE_VERSION
            ),
            Some(_)
        ));

        assert!(matches!(
            GasParameters::from_on_chain_gas_schedule(
                &[].into_iter().collect(),
                LATEST_GAS_FEATURE_VERSION
            ),
            None
        ));
    }
}
