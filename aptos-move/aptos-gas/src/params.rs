// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

macro_rules! define_gas_parameters_extract_key_at_version {
    ($key: literal, $cur_ver: expr) => {
        Some($key)
    };

    ({ $($ver: pat => $key: literal),+ }, $cur_ver: expr) => {
        match $cur_ver {
            $($ver => Some($key)),+,
            #[allow(unreachable_patterns)]
            _ => None,
        }
    }
}

macro_rules! define_gas_parameters {
    (
        $params_name: ident,
        $prefix: literal,
        [$(
            [$name: ident: $ty: ty, $key_bindings: tt, $initial: expr $(,)?]
        ),* $(,)?]
    ) => {
        #[derive(Debug, Clone)]
        pub struct $params_name {
            $(pub $name : $ty),*
        }

        impl $crate::gas_meter::FromOnChainGasSchedule for $params_name {
            #[allow(unused_variables)]
            fn from_on_chain_gas_schedule(gas_schedule: &std::collections::BTreeMap<String, u64>, feature_version: u64) -> Option<Self> {
                let mut params = $params_name::zeros();

                $(
                    if let Some(key) = $crate::params::define_gas_parameters_extract_key_at_version!($key_bindings, feature_version) {
                        params.$name = gas_schedule.get(&format!("{}.{}", $prefix, key)).cloned()?.into();
                    }
                )*

                Some(params)
            }
        }

        impl $crate::gas_meter::ToOnChainGasSchedule for $params_name {
            #[allow(unused_variables)]
            fn to_on_chain_gas_schedule(&self, feature_version: u64) -> Vec<(String, u64)> {
                let mut output = vec![];

                $(
                    if let Some(key) = $crate::params::define_gas_parameters_extract_key_at_version!($key_bindings, feature_version) {
                        output.push((format!("{}.{}", $prefix, key), self.$name.into()))
                    }
                )*

                output
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
        fn keys_should_be_unique_for_all_versions() {
            for ver in 0..=$crate::gas_meter::LATEST_GAS_FEATURE_VERSION {
                let mut map = std::collections::BTreeMap::<&str, ()>::new();

                $(
                    if let Some(key) = $crate::params::define_gas_parameters_extract_key_at_version!($key_bindings, ver) {
                        if map.insert(key, ()).is_some() {
                            panic!("duplicated key {} at version {}", key, ver);
                        }
                    }
                )*
            }
        }
    };
}

pub(crate) use define_gas_parameters;
pub(crate) use define_gas_parameters_extract_key_at_version;
