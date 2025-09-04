// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::errors::{LimitExceededError, SafeNativeError, SafeNativeResult};
use velor_gas_algebra::{
    AbstractValueSize, DynamicExpression, GasExpression, GasQuantity, InternalGasUnit,
};
use velor_gas_schedule::{
    gas_feature_versions::RELEASE_V1_32, AbstractValueSizeGasParameters, MiscGasParameters,
    NativeGasParameters,
};
use velor_types::on_chain_config::{Features, TimedFeatureFlag, TimedFeatures};
use move_binary_format::errors::{Location, PartialVMResult, VMResult};
use move_core_types::{
    gas_algebra::InternalGas, identifier::Identifier, language_storage::ModuleId,
};
use move_vm_runtime::{
    native_extensions::NativeContextExtensions,
    native_functions::{LoaderContext, NativeContext},
    Function,
};
use move_vm_types::values::Value;
use std::{
    ops::{Deref, DerefMut},
    sync::Arc,
};

/// A proxy between the VM and the native functions, allowing the latter to query VM configurations
/// or access certain VM functionalities.
///
/// It is a wrapper around Move VM's [`NativeContext`], providing improved and Velor-specific APIs.
/// Major features include incremental gas charging and less ambiguous error handling. For this
/// reason, native functions should always use [`SafeNativeContext`] instead of [`NativeContext`].
#[allow(unused)]
pub struct SafeNativeContext<'a, 'b, 'c, 'd> {
    pub(crate) inner: &'d mut NativeContext<'a, 'b, 'c>,

    pub(crate) timed_features: &'d TimedFeatures,
    pub(crate) features: &'d Features,
    pub(crate) gas_feature_version: u64,

    pub(crate) native_gas_params: &'d NativeGasParameters,
    pub(crate) misc_gas_params: &'d MiscGasParameters,

    // The fields below were used when there was no access to gas meter in native context. This is
    // no longer the case, so these can be removed when the feature is stable.
    pub(crate) legacy_gas_used: InternalGas,
    pub(crate) legacy_enable_incremental_gas_charging: bool,
    pub(crate) legacy_heap_memory_usage: u64,

    pub(crate) gas_hook: Option<&'d (dyn Fn(DynamicExpression) + Send + Sync)>,
}

impl<'a, 'b, 'c> Deref for SafeNativeContext<'a, 'b, 'c, '_> {
    type Target = NativeContext<'a, 'b, 'c>;

    fn deref(&self) -> &Self::Target {
        self.inner
    }
}

impl DerefMut for SafeNativeContext<'_, '_, '_, '_> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.inner
    }
}

impl<'b, 'c> SafeNativeContext<'_, 'b, 'c, '_> {
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

        if self.has_direct_gas_meter_access_in_native_context() {
            self.gas_meter()
                .charge_native_execution(amount)
                .map_err(LimitExceededError::from_err)?;
            Ok(())
        } else {
            self.legacy_gas_used += amount;
            if self.legacy_gas_used > self.legacy_gas_budget()
                && self.legacy_enable_incremental_gas_charging
            {
                Err(SafeNativeError::LimitExceeded(
                    LimitExceededError::LegacyOutOfGas,
                ))
            } else {
                Ok(())
            }
        }
    }

    /// Returns true if native functions have access to gas meter and cah charge gas. Otherwise,
    /// only VM's interpreter needs to charge gas.
    pub fn has_direct_gas_meter_access_in_native_context(&self) -> bool {
        self.gas_feature_version >= RELEASE_V1_32
    }

    /// Charges gas for transitive dependencies of the specified module. Used for native dynamic
    /// dispatch.
    pub fn charge_gas_for_dependencies(&mut self, module_id: ModuleId) -> SafeNativeResult<()> {
        self.inner
            .loader_context()
            .charge_gas_for_dependencies(module_id)
            .map_err(LimitExceededError::from_err)
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
    pub fn abs_val_size(&self, val: &Value) -> PartialVMResult<AbstractValueSize> {
        self.misc_gas_params
            .abs_val
            .abstract_value_size(val, self.gas_feature_version)
    }

    /// Returns extensions with loader context and gas parameters. Allows to use mutable loader
    /// context (wrapper around mutable references), while immutably borrowing the extension and
    /// gas parameters.
    pub fn extensions_with_loader_context_and_gas_params(
        &mut self,
    ) -> (
        &NativeContextExtensions<'b>,
        LoaderContext<'_, 'c>,
        &AbstractValueSizeGasParameters,
        u64,
    ) {
        let (extensions, native_layout_converter) = self.inner.extensions_with_loader_context();
        (
            extensions,
            native_layout_converter,
            &self.misc_gas_params.abs_val,
            self.gas_feature_version,
        )
    }

    /// Computes the abstract size of the input value.
    pub fn abs_val_size_dereferenced(&self, val: &Value) -> PartialVMResult<AbstractValueSize> {
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

    pub fn max_value_nest_depth(&self) -> Option<u64> {
        self.module_storage()
            .runtime_environment()
            .vm_config()
            .enable_depth_checks
            .then(|| {
                self.module_storage()
                    .runtime_environment()
                    .vm_config()
                    .max_value_nest_depth
            })
            .flatten()
    }

    /// Returns a reference to the struct representing on-chain features.
    pub fn get_feature_flags(&self) -> &Features {
        self.features
    }

    /// Checks if the timed feature corresponding to the given flag is enabled.
    pub fn timed_feature_enabled(&self, flag: TimedFeatureFlag) -> bool {
        self.timed_features.is_enabled(flag)
    }

    /// If gas metering in native context is available:
    ///   - Records heap memory usage. If exceeds the maximum allowed limit, an error is returned.
    ///
    /// If not available:
    ///   - Signals to the VM (and by extension, the gas meter) that the native function has
    ///     incurred additional heap memory usage that should be tracked.
    ///   - Charged by the VM after execution.
    pub fn use_heap_memory(&mut self, amount: u64) -> SafeNativeResult<()> {
        if self.timed_feature_enabled(TimedFeatureFlag::FixMemoryUsageTracking) {
            if self.has_direct_gas_meter_access_in_native_context() {
                self.gas_meter()
                    .use_heap_memory_in_native_context(amount)
                    .map_err(LimitExceededError::from_err)?;
            } else {
                self.legacy_heap_memory_usage =
                    self.legacy_heap_memory_usage.saturating_add(amount);
            }
        }
        Ok(())
    }

    /// Loads a function definition corresponding to the given name. The module where the function
    /// is defined must have been visited and metered (an error is returned otherwise).
    pub fn load_function(
        &mut self,
        module_id: &ModuleId,
        function_name: &Identifier,
    ) -> VMResult<Arc<Function>> {
        // INVARIANT:
        //   There is no need to meter module loading due to function access. This is because this
        //   function is only called for native dynamic dispatch, which pre-charges gas before the
        //   dispatch logic:
        //      1. Native function to load & charge modules is called.
        //      2. Native is called to dispatch, which calls this function from native context.
        //   Currently, native implementations in step (2) check if the module loading was metered,
        //   but we still keep an invariant check here in case there is a mistake and the gas is
        //   not charged.
        let module = if self.features.is_lazy_loading_enabled() {
            self.inner
                .traversal_context()
                .check_is_special_or_visited(module_id.address(), module_id.name())
                .map_err(|err| err.finish(Location::Undefined))?;
            self.inner
                .module_storage()
                .unmetered_get_existing_lazily_verified_module(module_id)?
        } else {
            self.inner
                .module_storage()
                .unmetered_get_existing_eagerly_verified_module(
                    module_id.address(),
                    module_id.name(),
                )?
        };
        module.get_function(function_name)
    }
}
