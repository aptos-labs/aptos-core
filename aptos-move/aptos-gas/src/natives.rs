// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

//! This module defines some macros to help implement the mapping between the on-chain gas schedule
//! and its rust representation.

macro_rules! expand_get {
    (test_only $(.$field: ident)+, $key: literal, $initial_val: expr, $param_ty: ty, $package_name: literal, $params: ident, $gas_schedule: ident) => {
        // TODO(Gas): this is a hack to work-around issue
        // https://github.com/rust-lang/rust/issues/15701
        {
            #[cfg(feature = "testing")]
            fn assign(params: &mut $param_ty, gas_schedule: &std::collections::BTreeMap<String, u64>) -> Option<()> {
                params $(.$field)+ = gas_schedule.get(&format!("{}.{}", $package_name, $key)).cloned()?.into();
                Some(())
            }

            #[cfg(not(feature = "testing"))]
            fn assign(_params: &mut $param_ty, _gas_schedule: &std::collections::BTreeMap<String, u64>) -> Option<()> {
                Some(())
            }

            assign(&mut $params, &$gas_schedule)?;
        }
    };
    ($(.$field: ident)+, $key: literal, $initial_val: expr, $param_ty: ty, $package_name: literal, $params: ident, $gas_schedule: ident) => {
        $params $(.$field)+ = $gas_schedule.get(&format!("{}.{}", $package_name, $key)).cloned()?.into();
    }
}

macro_rules! expand_set {
    (test_only $(.$field: ident)+, $key: literal, $initial_val: expr, $param_ty: ty, $package_name: literal, $params: ident) => {
        {
            #[cfg(feature = "testing")]
            fn assign(params: &mut $param_ty)  {
                params $(.$field)+ = $initial_val.into();
            }

            #[cfg(not(feature = "testing"))]
            fn assign(_params: &mut $param_ty) {
            }

            assign(&mut $params);
        }
    };
    ($(.$field: ident)+, $key: literal, $initial_val: expr, $param_ty: ty, $package_name: literal, $params: ident) => {
        $params $(.$field)+ = $initial_val.into()
    };
}

macro_rules! expand_kv {
    (test_only $(.$field: ident)+, $key: literal, $initial_val: expr, $self: ident) => {
        #[cfg(feature = "testing")]
        ($key, u64::from($self $(.$field)+))
    };
    ($(.$field: ident)+, $key: literal, $initial_val: expr, $self: ident) => {
        ($key, u64::from($self $(.$field)+))
    }
}

#[cfg(test)]
macro_rules! extract_key {
    (test_only $(.$field: ident)+, $key: literal, $initial_val: expr) => {
        #[cfg(feature = "testing")]
        $key
    };
    ($(.$field: ident)+, $key: literal, $initial_val: expr) => {
        $key
    };
}

#[cfg(test)]
macro_rules! extract_path {
    (test_only $(.$field: ident)+, $key: literal, $initial_val: expr) => {
        #[cfg(feature = "testing")]
        stringify!($($field).*)
    };
    ($(.$field: ident)+, $key: literal, $initial_val: expr) => {
        stringify!($($field).*)
    };
}

macro_rules! define_gas_parameters_for_natives {
    ($param_ty: ty, $package_name: literal, [$([$($t: tt)*]),* $(,)?] $(, allow_unmapped = $allow_unmapped: expr)?) => {
        impl crate::gas_meter::FromOnChainGasSchedule for $param_ty {
            fn from_on_chain_gas_schedule(gas_schedule: &std::collections::BTreeMap<String, u64>) -> Option<Self> {
                let mut params = <$param_ty>::zeros();

                $(
                    crate::natives::expand_get!($($t)*, $param_ty, $package_name, params, gas_schedule);
                )*

                Some(params)
            }
        }

        impl crate::gas_meter::ToOnChainGasSchedule for $param_ty {
            fn to_on_chain_gas_schedule(&self) -> Vec<(String, u64)> {
                [$(crate::natives::expand_kv!($($t)*, self)),*]
                    .into_iter().map(|(key, val)| (format!("{}.{}", $package_name, key), val)).collect()
            }
        }

        impl crate::gas_meter::InitialGasSchedule for $param_ty {
            fn initial() -> Self {
                let mut params = <$param_ty>::zeros();

                $(
                    crate::natives::expand_set!($($t)*, $param_ty, $package_name, params);
                )*

                params
            }
        }

        #[test]
        fn keys_should_be_unique() {
            let mut map = std::collections::BTreeMap::<&str, ()>::new();

            for key in [$(crate::natives::extract_key!($($t)*)),*] {
                if map.insert(key.clone(), ()).is_some() {
                    panic!("duplicated key {}", key);
                }
            }
        }

        #[test]
        fn paths_must_be_unique() {
            let mut map = std::collections::BTreeMap::<&str, ()>::new();

            for path in [$(crate::natives::extract_path!($($t)*)),*] {
                if map.insert(path.clone(), ()).is_some() {
                    panic!("duplicated path {}", path);
                }
            }
        }

        #[test]
        fn all_parameters_mapped() {
            let total = format!("{:?}", &<$param_ty>::zeros()).matches(": 0").count();
            let mapped = [$(crate::natives::extract_key!($($t)*)),*].len() $(+ $allow_unmapped)?;
            if mapped != total {
                panic!("only {} out of the {} entries are mapped", mapped, total)
            }
        }
    };
}

pub(crate) use define_gas_parameters_for_natives;
pub(crate) use expand_get;
pub(crate) use expand_kv;
pub(crate) use expand_set;

#[cfg(test)]
pub(crate) use extract_key;
#[cfg(test)]
pub(crate) use extract_path;
