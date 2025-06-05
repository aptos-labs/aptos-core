// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    context::SafeNativeContext,
    errors::{LimitExceededError, SafeNativeError, SafeNativeResult},
};
use aptos_gas_algebra::DynamicExpression;
use aptos_gas_schedule::{MiscGasParameters, NativeGasParameters};
use aptos_types::on_chain_config::{Features, TimedFeatures};
use move_vm_runtime::native_functions::{NativeContext, NativeFunction};
use move_vm_types::{
    loaded_data::runtime_types::Type, natives::function::NativeResult, values::Value,
};
use smallvec::SmallVec;
use std::{collections::VecDeque, sync::Arc};

/// Data shared by all native functions, mostly on-chain configurations.
#[derive(Debug)]
struct SharedData {
    gas_feature_version: u64,
    native_gas_params: NativeGasParameters,
    misc_gas_params: MiscGasParameters,
    timed_features: TimedFeatures,
    features: Features,
}

/// Factory object that allows one to build native functions with ease.
///
/// This enables native functions to access shared data, and interface with `SafeNativeContext`.
pub struct SafeNativeBuilder {
    data: Arc<SharedData>,
    enable_incremental_gas_charging: bool,
    gas_hook: Option<Arc<dyn Fn(DynamicExpression) + Send + Sync>>,
}

impl SafeNativeBuilder {
    /// Creates a new safe native builder.
    ///
    /// The configurations provided will be accessible by all native functions created later.
    pub fn new(
        gas_feature_version: u64,
        native_gas_params: NativeGasParameters,
        misc_gas_params: MiscGasParameters,
        timed_features: TimedFeatures,
        features: Features,
        gas_hook: Option<Arc<dyn Fn(DynamicExpression) + Send + Sync>>,
    ) -> Self {
        Self {
            data: Arc::new(SharedData {
                gas_feature_version,
                native_gas_params,
                misc_gas_params,
                timed_features,
                features,
            }),
            enable_incremental_gas_charging: true,
            gas_hook,
        }
    }

    /// Convenience function that allows one to set the incremental gas charging behavior only for
    /// natives created within the given closure.
    ///
    /// This can be useful if you want to configure the default for natives from a particular group
    /// without affecting the others.
    pub fn with_incremental_gas_charging<F, R>(&mut self, enable: bool, action: F) -> R
    where
        F: FnOnce(&mut Self) -> R,
    {
        let old = self.enable_incremental_gas_charging;
        self.enable_incremental_gas_charging = enable;
        let res = action(self);
        self.enable_incremental_gas_charging = old;
        res
    }

    /// Transforms a raw native function into a closure that can be used inside the Move VM.
    ///
    /// The closure will have access to the common Aptos configurations (features, gas params etc.),
    /// allowing the client to use [`SafeNativeContext`] instead of Move VM's [`NativeContext`].
    pub fn make_native<F>(&self, native: F) -> NativeFunction
    where
        F: Fn(
                &mut SafeNativeContext,
                Vec<Type>,
                VecDeque<Value>,
            ) -> SafeNativeResult<SmallVec<[Value; 1]>>
            + Send
            + Sync
            + 'static,
    {
        let data = Arc::clone(&self.data);
        let hook = self.gas_hook.clone();

        let enable_incremental_gas_charging = self.enable_incremental_gas_charging;

        let closure = move |context: &mut NativeContext, ty_args, args| {
            use SafeNativeError::*;

            let mut context = SafeNativeContext {
                inner: context,

                timed_features: &data.timed_features,
                features: &data.features,
                gas_feature_version: data.gas_feature_version,
                native_gas_params: &data.native_gas_params,
                misc_gas_params: &data.misc_gas_params,

                legacy_gas_used: 0.into(),
                legacy_enable_incremental_gas_charging: enable_incremental_gas_charging,

                gas_hook: hook.as_deref(),
            };

            let res: Result<SmallVec<[Value; 1]>, SafeNativeError> =
                native(&mut context, ty_args, args);

            // If enabled, metering and memory tracking must have been done in the native!
            if context.has_direct_gas_meter_access_in_native_context() {
                assert_eq!(context.legacy_gas_used, 0.into());
                assert_eq!(context.legacy_heap_memory_usage(), 0);
            }

            match res {
                Ok(ret_vals) => Ok(NativeResult::ok(context.legacy_gas_used, ret_vals)),
                Err(err) => match err {
                    Abort { abort_code } => {
                        Ok(NativeResult::err(context.legacy_gas_used, abort_code))
                    },
                    LimitExceeded(err) => match err {
                        LimitExceededError::LegacyOutOfGas => {
                            assert!(!context.has_direct_gas_meter_access_in_native_context());
                            Ok(NativeResult::out_of_gas(context.legacy_gas_used))
                        },
                        LimitExceededError::LimitExceeded(err) => {
                            // Return a VM error directly, so the native function returns early.
                            // There is no need to charge gas in the end because it was charged
                            // during the execution.
                            assert!(context.has_direct_gas_meter_access_in_native_context());
                            Err(err.unpack())
                        },
                    },
                    // TODO(Gas): Check if err is indeed an invariant violation.
                    InvariantViolation(err) => Err(err),
                    FunctionDispatch {
                        module_name,
                        func_name,
                        ty_args,
                        args,
                    } => Ok(NativeResult::CallFunction {
                        cost: context.legacy_gas_used,
                        module_name,
                        func_name,
                        ty_args,
                        args,
                    }),
                    LoadModule { module_name } => Ok(NativeResult::LoadModule { module_name }),
                },
            }
        };

        Arc::new(closure)
    }

    pub fn make_named_natives<'a, 'b, I, S, F>(
        &'a self,
        natives: I,
    ) -> impl Iterator<Item = (String, NativeFunction)> + 'a
    where
        'b: 'a,
        F: Fn(
                &mut SafeNativeContext,
                Vec<Type>,
                VecDeque<Value>,
            ) -> SafeNativeResult<SmallVec<[Value; 1]>>
            + Send
            + Sync
            + 'static,
        S: Into<String>,
        I: IntoIterator<Item = (S, F)> + 'b,
    {
        natives
            .into_iter()
            .map(|(func_name, func)| (func_name.into(), self.make_native(func)))
    }
}
