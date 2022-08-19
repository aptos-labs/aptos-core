// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

macro_rules! define_gas_parameters {
    (
        $params_name: ident,
        $prefix: literal,
        [$(
            [$name: ident: $ty: ty, $key: literal $(,)?, $initial: expr $(,)?]
        ),* $(,)?]
    ) => {
        #[derive(Debug, Clone)]
        pub struct $params_name {
            $(pub $name : $ty),*
        }

        impl $crate::gas_meter::FromOnChainGasSchedule for $params_name {
            fn from_on_chain_gas_schedule(gas_schedule: &std::collections::BTreeMap<String, u64>) -> Option<Self> {
                Some($params_name { $($name: gas_schedule.get(&format!("{}.{}", $prefix, $key)).cloned()?.into()),* })
            }
        }

        impl $crate::gas_meter::ToOnChainGasSchedule for $params_name {
            fn to_on_chain_gas_schedule(&self) -> Vec<(String, u64)> {
                vec![$((format!("{}.{}", $prefix, $key), self.$name.into())),*]
            }
        }

        impl $params_name {
            pub fn zeros() -> Self {
                Self {
                    $($name: 0.into()),*
                }
            }
        }

        impl $crate::gas_meter::InitialGasSchedule for $params_name {
            fn initial() -> Self {
                Self {
                    $($name: $initial.into()),*
                }
            }
        }

        #[test]
        fn keys_should_be_unique() {
            let mut map = std::collections::BTreeMap::<&str, ()>::new();

            for key in [$($key),*] {
                assert!(map.insert(key, ()).is_none());
            }
        }
    };
}

pub(crate) use define_gas_parameters;
