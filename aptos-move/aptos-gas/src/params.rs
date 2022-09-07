// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

macro_rules! expand_get_for_gas_parameters {
    ($params: ident . $name: ident, $map: ident, $prefix: literal, optional $key: literal) => {
        if let Some(val) = $map.get(&format!("{}.{}", $prefix, $key)) {
            $params.$name = (*val).into();
        }
    };
    ($params: ident . $name: ident, $map: ident, $prefix: literal, $key: literal) => {
        $params.$name = $map.get(&format!("{}.{}", $prefix, $key)).cloned()?.into();
    };
}

macro_rules! define_gas_parameters {
    (
        $params_name: ident,
        $prefix: literal,
        [$(
            [$name: ident: $ty: ty, $(optional $($dummy: ident)?)? $key: literal $(,)?, $initial: expr $(,)?]
        ),* $(,)?]
    ) => {
        #[derive(Debug, Clone)]
        pub struct $params_name {
            $(pub $name : $ty),*
        }

        impl $crate::gas_meter::FromOnChainGasSchedule for $params_name {
            fn from_on_chain_gas_schedule(gas_schedule: &std::collections::BTreeMap<String, u64>) -> Option<Self> {
                let mut params = $params_name::zeros();

                $(
                    $crate::params::expand_get_for_gas_parameters!(params . $name, gas_schedule, $prefix, $(optional $($dummy)?)? $key);
                )*

                Some(params)
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
pub(crate) use expand_get_for_gas_parameters;

#[cfg(test)]
mod tests {
    use crate::gas_meter::FromOnChainGasSchedule;

    use super::*;
    use move_core_types::gas_algebra::InternalGas;

    define_gas_parameters!(
        GasParameters,
        "test",
        [[foo: InternalGas, "foo", 0], [bar: InternalGas, optional "bar", 0]]
    );

    #[test]
    fn optional_should_be_honored() {
        assert!(
            matches!(
                GasParameters::from_on_chain_gas_schedule(
                    &[("test.foo".to_string(), 0)].into_iter().collect(),
                ),
                Some(_)
            )
        );

        assert!(matches!(
            GasParameters::from_on_chain_gas_schedule(&[].into_iter().collect()),
            None
        ));
    }
}
