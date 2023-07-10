// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::errors::{SafeNativeError, SafeNativeResult};
use aptos_gas_algebra::{AbstractValueSize, GasExpression, GasQuantity, InternalGasUnit};
use aptos_gas_schedule::{MiscGasParameters, NativeGasParameters};
use aptos_types::on_chain_config::{Features, TimedFeatureFlag, TimedFeatures};
use move_core_types::gas_algebra::InternalGas;
use move_vm_runtime::native_functions::NativeContext;
use move_vm_types::values::Value;
use std::ops::{Deref, DerefMut};

#[allow(unused)]
pub struct SafeNativeContext<'a, 'b, 'c, 'd> {
    pub(crate) inner: &'c mut NativeContext<'a, 'b, 'd>,

    pub(crate) timed_features: &'c TimedFeatures,
    pub(crate) features: &'c Features,
    pub(crate) gas_feature_version: u64,

    pub(crate) native_gas_params: &'c NativeGasParameters,
    pub(crate) misc_gas_params: &'c MiscGasParameters,

    pub(crate) gas_budget: InternalGas,
    pub(crate) gas_used: InternalGas,

    pub(crate) enable_incremental_gas_charging: bool,
}

impl<'a, 'b, 'c, 'd> Deref for SafeNativeContext<'a, 'b, 'c, 'd> {
    type Target = NativeContext<'a, 'b, 'd>;

    fn deref(&self) -> &Self::Target {
        self.inner
    }
}

impl<'a, 'b, 'c, 'd> DerefMut for SafeNativeContext<'a, 'b, 'c, 'd> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.inner
    }
}

impl<'a, 'b, 'c, 'd> SafeNativeContext<'a, 'b, 'c, 'd> {
    /// Always remember: first charge gas, then execute!
    ///
    /// In other words, this function **MUST** always be called **BEFORE** executing **any**
    /// gas-metered operation or library call within a native function.
    #[must_use = "must always propagate the error returned by this function to the native function that called it using the ? operator"]
    pub fn charge(
        &mut self,
        abstract_amount: impl GasExpression<NativeGasParameters, Unit = InternalGasUnit>,
    ) -> SafeNativeResult<()> {
        // ==gas expr==> CalibratorAlgebra ==> Arc<Mutex<buffer>>
        // ==gas expr==> SafeNativeContext

        let amount = abstract_amount.evaluate(self.gas_feature_version, self.native_gas_params);

        self.gas_used += amount;

        if self.gas_used > self.gas_budget && self.enable_incremental_gas_charging {
            Err(SafeNativeError::OutOfGas)
        } else {
            Ok(())
        }
    }

    /// Evaluates the given gas expression to a concrete number.
    pub fn eval_gas<E>(&self, abstract_amount: E) -> GasQuantity<E::Unit>
    where
        E: GasExpression<NativeGasParameters>,
    {
        abstract_amount.evaluate(self.gas_feature_version, self.native_gas_params)
    }

    pub fn abs_val_size(&self, val: &Value) -> AbstractValueSize {
        self.misc_gas_params
            .abs_val
            .abstract_value_size(val, self.gas_feature_version)
    }

    pub fn gas_feature_version(&self) -> u64 {
        self.gas_feature_version
    }

    pub fn get_feature_flags(&self) -> &Features {
        self.features.deref()
    }

    pub fn timed_feature_enabled(&self, flag: TimedFeatureFlag) -> bool {
        self.timed_features.is_enabled(flag)
    }

    pub fn set_incremental_gas_charging(&mut self, enable: bool) {
        self.enable_incremental_gas_charging = enable;
    }
}
