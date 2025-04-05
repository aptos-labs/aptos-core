// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::errors::{SafeNativeError, SafeNativeResult};
use aptos_gas_algebra::{
    AbstractValueSize, DynamicExpression, GasExpression, GasQuantity, InternalGasUnit,
};
use aptos_gas_schedule::{
    gas_params::txn::{DEPENDENCY_PER_BYTE, DEPENDENCY_PER_MODULE},
    AbstractValueSizeGasParameters, NativeGasParameters, VMGasParameters,
};
use aptos_types::on_chain_config::{Features, TimedFeatureFlag, TimedFeatures};
use move_binary_format::errors::{Location, PartialVMError, VMResult};
use move_core_types::{
    gas_algebra::{InternalGas, NumBytes},
    identifier::Identifier,
    language_storage::ModuleId,
    value::MoveTypeLayout,
    vm_status::StatusCode,
};
use move_vm_runtime::{native_functions::NativeContext, Function};
use move_vm_types::{loaded_data::runtime_types::Type, values::Value};
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
pub struct SafeNativeContext<'a, 'b, 'c, 'd> {
    pub(crate) inner: &'c mut NativeContext<'a, 'b, 'd>,

    pub(crate) timed_features: &'c TimedFeatures,
    pub(crate) features: &'c Features,
    pub(crate) gas_feature_version: u64,

    pub(crate) vm_gas_params: &'c VMGasParameters,
    pub(crate) native_gas_params: &'c NativeGasParameters,

    pub(crate) gas_budget: InternalGas,
    pub(crate) gas_used: InternalGas,

    pub(crate) enable_incremental_gas_charging: bool,

    pub(crate) gas_hook: Option<&'c (dyn Fn(DynamicExpression) + Send + Sync)>,
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
        self.vm_gas_params
            .misc
            .abs_val
            .abstract_value_size(val, self.gas_feature_version)
    }

    /// Computes the abstract size of the input value.
    pub fn abs_val_size_dereferenced(&self, val: &Value) -> AbstractValueSize {
        self.vm_gas_params
            .misc
            .abs_val
            .abstract_value_size_dereferenced(val, self.gas_feature_version)
    }

    /// Returns the gas parameters that are used to define abstract value sizes.
    pub fn abs_val_gas_params(&self) -> &AbstractValueSizeGasParameters {
        &self.vm_gas_params.misc.abs_val
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

    pub fn load_function(
        &mut self,
        module_id: &ModuleId,
        function_name: &Identifier,
    ) -> VMResult<Arc<Function>> {
        // MODULE LOADING METERING:
        //   Metering is done when native returns LoadModule result, so this access will never load
        //   anything and will access cached and metered modules. Currently, native implementations
        //   check if the loading was metered or not before calling here, but this kept as an extra
        //   redundancy check in case there is a mistake and gas is not charged somehow.
        self.traversal_context()
            .check_is_special_or_visited(module_id.address(), module_id.name())
            .map_err(|err| err.finish(Location::Undefined))?;

        let (_, function) = self.module_storage().unmetered_get_function_definition(
            module_id.address(),
            module_id.name(),
            function_name,
        )?;
        Ok(function)
    }

    pub fn type_to_type_layout(&mut self, ty: &Type) -> SafeNativeResult<MoveTypeLayout> {
        if self.features.is_lazy_loading_enabled() {
            let cost_fn = |size: u64| -> InternalGas {
                (DEPENDENCY_PER_MODULE + DEPENDENCY_PER_BYTE * NumBytes::new(size))
                    .evaluate(self.gas_feature_version, self.vm_gas_params)
            };
            self.inner
                .metered_lazy_type_to_type_layout(ty, self.gas_used, cost_fn)
                .map_err(|err| self.convert_to_safe_error(err))
        } else {
            Ok(self.inner.unmetered_type_to_type_layout(ty)?)
        }
    }

    pub fn type_to_type_layout_with_identifier_mappings(
        &mut self,
        ty: &Type,
    ) -> SafeNativeResult<(MoveTypeLayout, bool)> {
        if self.features.is_lazy_loading_enabled() {
            let cost_fn = |size: u64| -> InternalGas {
                (DEPENDENCY_PER_MODULE + DEPENDENCY_PER_BYTE * NumBytes::new(size))
                    .evaluate(self.gas_feature_version, self.vm_gas_params)
            };
            self.inner
                .metered_lazy_type_to_type_layout_with_identifier_mappings(
                    ty,
                    self.gas_used,
                    cost_fn,
                )
                .map_err(|err| self.convert_to_safe_error(err))
        } else {
            Ok(self
                .inner
                .unmetered_type_to_type_layout_with_identifier_mappings(ty)?)
        }
    }

    pub fn type_to_fully_annotated_layout(
        &mut self,
        ty: &Type,
    ) -> SafeNativeResult<MoveTypeLayout> {
        if self.features.is_lazy_loading_enabled() {
            let cost_fn = |size: u64| -> InternalGas {
                (DEPENDENCY_PER_MODULE + DEPENDENCY_PER_BYTE * NumBytes::new(size))
                    .evaluate(self.gas_feature_version, self.vm_gas_params)
            };
            self.inner
                .metered_lazy_type_to_fully_annotated_layout(ty, self.gas_used, cost_fn)
                .map_err(|err| self.convert_to_safe_error(err))
        } else {
            Ok(self.inner.unmetered_type_to_fully_annotated_layout(ty)?)
        }
    }

    fn convert_to_safe_error(&self, err: PartialVMError) -> SafeNativeError {
        // Use out of gas, so that we can handle it on native finish and charge gas properly via
        // the meter.
        if err.major_status() == StatusCode::OUT_OF_GAS
            || err.major_status() == StatusCode::DEPENDENCY_LIMIT_REACHED
        {
            return SafeNativeError::OutOfGas;
        }

        // Otherwise, use the same behaviour as for other natives returning partial error.
        SafeNativeError::from(err)
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
}
