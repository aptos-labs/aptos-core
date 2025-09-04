// Copyright © Velor Foundation
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
        $env: ty => $(.$field: ident)*,
        [$(
            [$name: ident: $ty: ident, $key_bindings: tt, $initial: expr $(, $tn: ident)? $(,)?]
        ),* $(,)?]
    ) => {
        #[derive(Debug, Clone)]
        pub struct $params_name {
            $(pub $name : $ty),*
        }

        impl $crate::traits::FromOnChainGasSchedule for $params_name {
            #[allow(unused)]
            fn from_on_chain_gas_schedule(gas_schedule: &std::collections::BTreeMap<String, u64>, feature_version: u64) -> Result<Self, String> {
                let mut params = $params_name::zeros();

                $(
                    if let Some(key) = $crate::gas_schedule::macros::define_gas_parameters_extract_key_at_version!($key_bindings, feature_version) {
                        let name = format!("{}.{}", $prefix, key);
                        params.$name = gas_schedule.get(&name).cloned().ok_or_else(|| format!("Gas parameter {} does not exist. Feature version: {}.", name, feature_version))?.into();
                    }
                )*

                Ok(params)
            }
        }

        impl $crate::traits::ToOnChainGasSchedule for $params_name {
            #[allow(unused)]
            fn to_on_chain_gas_schedule(&self, feature_version: u64) -> Vec<(String, u64)> {
                let mut output = vec![];

                $(
                    if let Some(key) = $crate::gas_schedule::macros::define_gas_parameters_extract_key_at_version!($key_bindings, feature_version) {
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

            #[cfg(feature = "testing")]
            pub fn random() -> Self {
                Self {
                    $($name: rand::random::<u64>().into()),*
                }
            }
        }

        impl $crate::traits::InitialGasSchedule for $params_name {
            fn initial() -> Self {
                Self {
                    $($name: $initial.into()),*
                }
            }
        }

        #[allow(unused)]
        pub mod gas_params {
            use super::*;
            use velor_gas_algebra::{GasExpression, GasExpressionVisitor, GasMul, GasAdd};
            use $crate::{
                gas_schedule::VelorGasParameters,
            };
            use std::ops::{Add, Mul};
            use move_core_types::gas_algebra::{GasQuantity, GasQuantityGetUnit};

            macro_rules! get {
                ($gas_params: expr, $leaf: ident) => {
                    $gas_params $(.$field)* .$leaf
                }
            }

            $(
                paste::paste! {
                    #[derive(Debug)]
                    #[doc = "Type representing the `" $name "` gas parameter."]
                    #[allow(non_camel_case_types)]
                    pub struct [<$name:upper>];

                    impl GasExpression<$env> for [<$name:upper>] {
                        type Unit =  <super::$ty as GasQuantityGetUnit>::Unit;

                        #[inline]
                        fn evaluate(
                            &self,
                            _feature_version: u64,
                            gas_params: &$env,
                        ) -> GasQuantity<Self::Unit> {
                            get!(gas_params, $name)
                        }

                        #[inline]
                        fn visit(&self, visitor: &mut impl GasExpressionVisitor) {
                            visitor.gas_param::<Self>();
                        }
                    }

                    impl<T> Add<T> for [<$name:upper>]
                    {
                        type Output = GasAdd<Self, T>;

                        #[inline]
                        fn add(self, rhs: T) -> Self::Output {
                            GasAdd { left: self, right: rhs }
                        }
                    }

                    impl<T> Mul<T> for [<$name:upper>]
                    {
                        type Output = GasMul<Self, T>;

                        #[inline]
                        fn mul(self, rhs: T) -> Self::Output {
                            GasMul { left: self, right: rhs }
                        }
                    }

                    impl Add<[<$name:upper>]> for GasQuantity<<super::$ty as GasQuantityGetUnit>::Unit> {
                        type Output = GasAdd<Self, [<$name:upper>]>;

                        #[inline]
                        fn add(self, rhs: [<$name:upper>]) -> Self::Output {
                            GasAdd { left: self, right: rhs }
                        }
                    }

                    impl Mul<[<$name:upper>]> for GasQuantity<<super::$ty as GasQuantityGetUnit>::Unit> {
                        type Output = GasMul<Self, [<$name:upper>]>;

                        #[inline]
                        fn mul(self, rhs: [<$name:upper>]) -> Self::Output {
                            GasMul { left: self, right: rhs }
                        }
                    }
                }
            )*
        }

        #[test]
        fn keys_should_be_unique_for_all_versions() {
            for ver in 0..=$crate::LATEST_GAS_FEATURE_VERSION {
                let mut map = std::collections::BTreeMap::<&str, ()>::new();

                $(
                    if let Some(key) = $crate::gas_schedule::macros::define_gas_parameters_extract_key_at_version!($key_bindings, ver) {
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
