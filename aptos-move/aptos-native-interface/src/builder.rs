// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    context::SafeNativeContext,
    errors::{SafeNativeError, SafeNativeResult},
};
use aptos_gas_algebra::Expression;
use aptos_gas_schedule::{MiscGasParameters, NativeGasParameters};
use aptos_types::on_chain_config::{Features, TimedFeatures};
use move_vm_runtime::native_functions::{NativeContext, NativeFunction};
use move_vm_types::{
    loaded_data::runtime_types::Type, natives::function::NativeResult, values::Value,
};
use smallvec::SmallVec;
use std::{collections::VecDeque, sync::Arc};

#[derive(Debug)]
struct SharedData {
    gas_feature_version: u64,
    native_gas_params: NativeGasParameters,
    misc_gas_params: MiscGasParameters,
    timed_features: TimedFeatures,
    features: Features,
}

//#[derive(Debug)]
pub struct SafeNativeBuilder {
    data: Arc<SharedData>,
    enable_incremental_gas_charging: bool,
    gas_hook: Option<Arc<dyn Fn(Expression) + Send + Sync>>,
}

impl SafeNativeBuilder {
    pub fn new(
        gas_feature_version: u64,
        native_gas_params: NativeGasParameters,
        misc_gas_params: MiscGasParameters,
        timed_features: TimedFeatures,
        features: Features,
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
            gas_hook: None,
        }
    }

    pub fn set_gas_hook<F>(&mut self, action: F)
    where
        //// todo: look into FnOnce, Fn, FnMut
        //// look into Send Sync
        //// look into closure more indepth
        F: Fn(Expression) + Send + Sync + 'static,
    {
        self.gas_hook = Some(Arc::new(action));
    }

    pub fn set_incremental_gas_charging(&mut self, enable: bool) {
        self.enable_incremental_gas_charging = enable;
    }

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

            let gas_budget = context.gas_balance();

            let mut context = SafeNativeContext {
                inner: context,

                timed_features: &data.timed_features,
                features: &data.features,
                gas_feature_version: data.gas_feature_version,
                native_gas_params: &data.native_gas_params,
                misc_gas_params: &data.misc_gas_params,

                gas_budget,
                gas_used: 0.into(),

                enable_incremental_gas_charging,

                gas_hook: hook.as_ref().map(|h| &**h),
            };

            let res: Result<SmallVec<[Value; 1]>, SafeNativeError> =
                native(&mut context, ty_args, args);

            match res {
                Ok(ret_vals) => Ok(NativeResult::ok(context.gas_used, ret_vals)),
                Err(err) => match err {
                    Abort { abort_code } => Ok(NativeResult::err(context.gas_used, abort_code)),
                    OutOfGas => Ok(NativeResult::out_of_gas(context.gas_used)),
                    // TODO(Gas): Check if err is indeed an invariant violation.
                    InvariantViolation(err) => Err(err),
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
