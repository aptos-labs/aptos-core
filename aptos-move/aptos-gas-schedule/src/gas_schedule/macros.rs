// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

macro_rules! impl_arithmetic_operations {
    ($name:ty : $ty:ty) => {
        impl<T> Add<T> for $name {
            type Output = GasAdd<Self, T>;

            #[inline]
            fn add(self, rhs: T) -> Self::Output {
                GasAdd {
                    left: self,
                    right: rhs,
                }
            }
        }

        impl<T> Mul<T> for $name {
            type Output = GasMul<Self, T>;

            #[inline]
            fn mul(self, rhs: T) -> Self::Output {
                GasMul {
                    left: self,
                    right: rhs,
                }
            }
        }

        impl Add<$name> for GasQuantity<<$ty as GasQuantityGetUnit>::Unit> {
            type Output = GasAdd<Self, $name>;

            #[inline]
            fn add(self, rhs: $name) -> Self::Output {
                GasAdd {
                    left: self,
                    right: rhs,
                }
            }
        }

        impl Mul<$name> for GasQuantity<<$ty as GasQuantityGetUnit>::Unit> {
            type Output = GasMul<Self, $name>;

            #[inline]
            fn mul(self, rhs: $name) -> Self::Output {
                GasMul {
                    left: self,
                    right: rhs,
                }
            }
        }
    };
}

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
            use aptos_gas_algebra::{GasExpression, GasExpressionVisitor, GasMul, GasAdd};
            use $crate::{
                gas_schedule::AptosGasParameters,
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
                    #[doc = "Type representing the `" $name "` gas parameter. This is generated using the macros in the `aptos-gas-schedule` crate."]
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

                    $crate::gas_schedule::macros::impl_arithmetic_operations!([<$name:upper>]: super::$ty);
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

macro_rules! define_dummy_gas_parameters {
    ($($name:ident : $ty:ident),* $(,)?) => {
        #[allow(unused)]
        pub mod dummy_gas_params {
            use super::*;
            use aptos_gas_algebra::{GasExpression, GasExpressionVisitor, GasMul, GasAdd};
            use std::ops::{Add, Mul};
            use move_core_types::gas_algebra::{GasQuantity, GasQuantityGetUnit};
            $(
                paste::paste! {
                    #[derive(Debug)]
                    #[doc = "Type representing the `" $name "` gas parameter. This is generated using the macros in the `aptos-gas-schedule` crate."]
                    #[doc = "\n\n"]
                    #[doc = "Note: this is a dummy parameter that always evaluates to 0. It should only be used to denote abstract usage for test-only natives."]
                    #[allow(non_camel_case_types)]
                    pub struct [<$name:upper>];

                    impl<E> GasExpression<E> for [<$name:upper>] {
                        type Unit =  <super::$ty as GasQuantityGetUnit>::Unit;

                        #[inline]
                        fn evaluate(
                            &self,
                            _feature_version: u64,
                            _env: &E,
                        ) -> GasQuantity<Self::Unit> {
                            0.into()
                        }

                        #[inline]
                        fn visit(&self, visitor: &mut impl GasExpressionVisitor) {
                            visitor.gas_param::<Self>();
                        }
                    }

                    $crate::gas_schedule::macros::impl_arithmetic_operations!([<$name:upper>]: super::$ty);
                }
            )*
        }
    };
}

pub(crate) use define_dummy_gas_parameters;
pub(crate) use define_gas_parameters;
pub(crate) use define_gas_parameters_extract_key_at_version;
pub(crate) use impl_arithmetic_operations;
