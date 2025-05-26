// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::errors::{SafeNativeError, SafeNativeResult};
use aptos_gas_algebra::{
    AbstractValueSize, DynamicExpression, GasExpression, GasQuantity, InternalGasUnit,
};
use aptos_gas_schedule::{AbstractValueSizeGasParameters, MiscGasParameters, NativeGasParameters};
use aptos_types::on_chain_config::{Features, TimedFeatureFlag, TimedFeatures};
use move_binary_format::errors::VMResult;
use move_core_types::{
    gas_algebra::InternalGas, identifier::Identifier, language_storage::ModuleId,
};
use move_vm_runtime::{native_functions::NativeContext, Function};
use move_vm_types::values::Value;
use std::{
    ops::{Deref, DerefMut},
    sync::Arc,
};

/// A proxy between the VM and the native functions, allowing the latter to query VM configurations
/// or access certain VM functionalities.
///
/// It is a wrapper around Move VM's [`NativeContext`], providing improved and Aptos-specific APIs.
/// Major features include incremental gas charging and less ambiguous error handling. For this
/// reason, native functions should always use [`SafeNativeContext`] instead of [`NativeContext`].
#[allow(unused)]
pub struct SafeNativeContext<'a, 'b, 'c> {
    pub(crate) inner: &'c mut NativeContext<'a, 'b>,

    pub(crate) timed_features: &'c TimedFeatures,
    pub(crate) features: &'c Features,
    pub(crate) gas_feature_version: u64,

    pub(crate) native_gas_params: &'c NativeGasParameters,
    pub(crate) misc_gas_params: &'c MiscGasParameters,

    pub(crate) gas_budget: InternalGas,
    pub(crate) gas_used: InternalGas,

    pub(crate) enable_incremental_gas_charging: bool,

    pub(crate) gas_hook: Option<&'c (dyn Fn(DynamicExpression) + Send + Sync)>,
}

impl<'a, 'b> Deref for SafeNativeContext<'a, 'b, '_> {
    type Target = NativeContext<'a, 'b>;

    fn deref(&self) -> &Self::Target {
        self.inner
    }
}

impl DerefMut for SafeNativeContext<'_, '_, '_> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.inner
    }
}

impl SafeNativeContext<'_, '_, '_> {
    /// Always remember: first charge gas, then execute!
    ///
    /// In other words, this function **MUST** always be called **BEFORE** executing **any**
    /// gas-metered operation or library call within a native function.
    #[must_use = "must always propagate the error returned by this function to the native function that called it using the ? operator"]
    pub fn charge(
        &mut self,
        abstract_amount: impl GasExpression<NativeGasParameters, Unit = InternalGasUnit>,
    ) -> SafeNativeResult<()> {
        let amount = abstract_amount.evaluate(self.gas_feature_version, self.native_gas_params);

        if let Some(hook) = self.gas_hook {
            let node = abstract_amount.to_dynamic();
            hook(node);
        }

        self.gas_used += amount;

        if self.gas_used > self.gas_budget && self.enable_incremental_gas_charging {
            Err(SafeNativeError::OutOfGas)
        } else {
            Ok(())
        }
    }

    /// Evaluates the given gas expression within the current context immediately.
    ///
    /// This can be useful if you have branch conditions depending on gas parameters.
    pub fn eval_gas<E>(&self, abstract_amount: E) -> GasQuantity<E::Unit>
    where
        E: GasExpression<NativeGasParameters>,
    {
        abstract_amount.evaluate(self.gas_feature_version, self.native_gas_params)
    }

    /// Computes the abstract size of the input value.
    pub fn abs_val_size(&self, val: &Value) -> AbstractValueSize {
        self.misc_gas_params
            .abs_val
            .abstract_value_size(val, self.gas_feature_version)
    }

    /// Computes the abstract size of the input value.
    pub fn abs_val_size_dereferenced(&self, val: &Value) -> AbstractValueSize {
        self.misc_gas_params
            .abs_val
            .abstract_value_size_dereferenced(val, self.gas_feature_version)
    }

    /// Returns the gas parameters that are used to define abstract value sizes.
    pub fn abs_val_gas_params(&self) -> &AbstractValueSizeGasParameters {
        &self.misc_gas_params.abs_val
    }

    /// Returns the current gas feature version.
    pub fn gas_feature_version(&self) -> u64 {
        self.gas_feature_version
    }

    /// Returns a reference to the struct representing on-chain features.
    pub fn get_feature_flags(&self) -> &Features {
        self.features
    }

    /// Checks if the timed feature corresponding to the given flag is enabled.
    pub fn timed_feature_enabled(&self, flag: TimedFeatureFlag) -> bool {
        self.timed_features.is_enabled(flag)
    }

    /// Signals to the VM (and by extension, the gas meter) that the native function has
    /// incurred additional heap memory usage that should be tracked.
    pub fn use_heap_memory(&mut self, amount: u64) {
        if self.timed_feature_enabled(TimedFeatureFlag::FixMemoryUsageTracking) {
            self.inner.use_heap_memory(amount);
        }
    }

    /// Configures the behavior of [`Self::charge()`].
    /// - If enabled, it will return an out of gas error as soon as the amount of gas used
    ///   exceeds the remaining balance.
    /// - If disabled, it will not return early errors, but the gas usage is still recorded,
    ///   and the total amount will be reported back to the VM after the native function returns.
    ///   This should only be used for backward compatibility reasons.
    pub fn set_incremental_gas_charging(&mut self, enable: bool) {
        self.enable_incremental_gas_charging = enable;
    }

    pub fn load_function(
        &mut self,
        module_id: &ModuleId,
        function_name: &Identifier,
    ) -> VMResult<Arc<Function>> {
        let (_, function) = self.inner.module_storage().fetch_function_definition(
            module_id.address(),
            module_id.name(),
            function_name,
        )?;
        Ok(function)
    }
}
