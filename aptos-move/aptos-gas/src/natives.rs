// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! This module defines some macros to help implement the mapping between the on-chain gas schedule
//! and its rust representation.

macro_rules! define_gas_parameters_for_natives_extract_key_at_version {
    ($key: literal, $cur_ver: expr) => {
        Some($key)
    };

    ({ $($ver: pat => $key: literal),+ }, $cur_ver: expr) => {
        match $cur_ver {
            $($ver => Some($key)),+,
            _ => None,
        }
    }
}

macro_rules! define_gas_parameters_for_natives {
    (
        $param_ty: ty,
        $package_name: literal,
        [$([$(.$field: ident)+, $key_bindings: tt, $initial_val: expr]),+ $(,)?]
        $(, allow_unmapped = $allow_unmapped: expr)?
    ) => {
        impl crate::gas_meter::FromOnChainGasSchedule for $param_ty {
            #[allow(unused_variables)]
            fn from_on_chain_gas_schedule(gas_schedule: &std::collections::BTreeMap<String, u64>, feature_version: u64) -> Result<Self, String> {
                let mut params = <$param_ty>::zeros();

                $(
                    if let Some(key) =  $crate::natives::define_gas_parameters_for_natives_extract_key_at_version!($key_bindings, feature_version) {
                        let name = format!("{}.{}", $package_name, key);
                        params $(.$field)+ = gas_schedule.get(&name).cloned().ok_or_else(|| format!("Gas parameter {} does not exist. Feature version: {}.", name, feature_version))?.into();
                    }
                )*

                Ok(params)
            }
        }

        impl crate::gas_meter::ToOnChainGasSchedule for $param_ty {
            #[allow(unused_variables)]
            fn to_on_chain_gas_schedule(&self, feature_version: u64) -> Vec<(String, u64)> {
                let mut output = vec![];

                $(
                    if let Some(key) = $crate::natives::define_gas_parameters_for_natives_extract_key_at_version!($key_bindings, feature_version) {
                        output.push((format!("{}.{}", $package_name, key), u64::from(self $(.$field)+)));
                    }
                )*

                output
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
        fn keys_should_be_unique_for_all_versions() {
            for ver in 0..=$crate::gas_meter::LATEST_GAS_FEATURE_VERSION {
                let mut map = std::collections::BTreeMap::<&str, ()>::new();

                $(
                    if let Some(key) = $crate::natives::define_gas_parameters_for_natives_extract_key_at_version!($key_bindings, ver) {
                        if map.insert(key, ()).is_some() {
                            panic!("duplicated key {} at version {}", key, ver);
                        }
                    }
                )*
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
            let mapped = [$(stringify!($($field).*)),*].len() $(+ $allow_unmapped)?;
            if mapped != total {
                panic!("only {} out of the {} entries are mapped", mapped, total)
            }
        }
    };
}

pub(crate) use define_gas_parameters_for_natives;
pub(crate) use define_gas_parameters_for_natives_extract_key_at_version;
