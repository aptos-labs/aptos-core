// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::{
    accept_type::AcceptType,
    context::FunctionStats,
    response_axum::{AptosErrorResponse, AptosResponse},
    Context,
};
use anyhow::Context as anyhowContext;
use aptos_api_types::{
    AptosErrorCode, AsConverter, MoveValue, ViewFunction, ViewRequest, MAX_RECURSIVE_TYPES_ALLOWED,
    U64,
};
use aptos_bcs_utils::serialize_uleb128;
use aptos_types::{state_store::StateView, transaction::ViewFunctionError, vm_status::StatusCode};
use aptos_vm::AptosVM;
use itertools::Itertools;
use move_core_types::language_storage::TypeTag;
use std::sync::Arc;

pub fn convert_view_function_error(
    error: &ViewFunctionError,
    state_view: &impl StateView,
    context: &Context,
) -> (String, Option<StatusCode>) {
    match error {
        ViewFunctionError::MoveAbort(status, vm_error_code) => {
            let vm_status = state_view
                .as_converter(context.db.clone(), context.indexer_reader.clone())
                .explain_vm_status(status, None);
            (vm_status, *vm_error_code)
        },
        ViewFunctionError::ErrorMessage(message, vm_error_code) => {
            (message.clone(), *vm_error_code)
        },
    }
}

#[derive(Debug)]
pub enum ViewFunctionRequest {
    Json(ViewRequest),
    Bcs(Vec<u8>),
}

/// Framework-agnostic business logic for the view function endpoint.
/// Called by the Axum handler directly, bypassing the Poem bridge.
pub fn view_request_inner(
    context: Arc<Context>,
    accept_type: AcceptType,
    request: ViewFunctionRequest,
    ledger_version: Option<U64>,
) -> Result<AptosResponse<Vec<MoveValue>>, AptosErrorResponse> {
    // Retrieve the current state of the chain
    let (ledger_info, requested_version) = context
        .get_latest_ledger_info_and_verify_lookup_version::<AptosErrorResponse>(
            ledger_version.map(|v| v.0),
        )?;

    let state_view = context
        .state_view_at_version(requested_version)
        .map_err(|err| {
            AptosErrorResponse::bad_request(err, AptosErrorCode::InternalError, Some(&ledger_info))
        })?;

    let view_function: ViewFunction = match request {
        ViewFunctionRequest::Json(data) => state_view
            .as_converter(context.db.clone(), context.indexer_reader.clone())
            .convert_view_function(data)
            .map_err(|err| {
                AptosErrorResponse::bad_request(
                    err,
                    AptosErrorCode::InvalidInput,
                    Some(&ledger_info),
                )
            })?,
        ViewFunctionRequest::Bcs(data) => {
            bcs::from_bytes_with_limit(data.as_slice(), MAX_RECURSIVE_TYPES_ALLOWED as usize)
                .context("Failed to deserialize input into ViewRequest")
                .map_err(|err| {
                    AptosErrorResponse::bad_request(
                        err,
                        AptosErrorCode::InvalidInput,
                        Some(&ledger_info),
                    )
                })?
        },
    };

    // Reject the request if it's not allowed by the filter.
    if !context.node_config.api.view_filter.allows(
        view_function.module.address(),
        view_function.module.name().as_str(),
        view_function.function.as_str(),
    ) {
        return Err(AptosErrorResponse::forbidden(
            format!(
                "Function {}::{} is not allowed",
                view_function.module, view_function.function
            ),
            AptosErrorCode::InvalidInput,
            None,
        ));
    }

    let output = AptosVM::execute_view_function(
        &state_view,
        view_function.module.clone(),
        view_function.function.clone(),
        view_function.ty_args.clone(),
        view_function.args.clone(),
        context.node_config.api.max_gas_view_function,
    );

    let values = output.values.map_err(|status| {
        let (err_string, vm_error_code) =
            convert_view_function_error(&status, &state_view, &context);
        if let Some(vm_error_code) = vm_error_code {
            AptosErrorResponse::bad_request_with_vm_status(
                anyhow::anyhow!(err_string),
                AptosErrorCode::InvalidInput,
                vm_error_code,
                Some(&ledger_info),
            )
        } else {
            AptosErrorResponse::bad_request(
                anyhow::anyhow!(err_string),
                AptosErrorCode::InvalidInput,
                Some(&ledger_info),
            )
        }
    })?;

    let result = match accept_type {
        AcceptType::Bcs => {
            // The return values are already BCS encoded, but we still need to encode the outside
            // vector without re-encoding the inside values
            let num_vals = values.len();

            // Push the length of the return values
            let mut length = vec![];
            serialize_uleb128(&mut length, num_vals as u64).map_err(|err| {
                AptosErrorResponse::internal(err, AptosErrorCode::InternalError, Some(&ledger_info))
            })?;

            // Combine all of the return values
            let values = values.into_iter().concat();
            let ret = [length, values].concat();

            AptosResponse::try_from_encoded(ret, &ledger_info)
        },
        AcceptType::Json => {
            let return_types = state_view
                .as_converter(context.db.clone(), context.indexer_reader.clone())
                .function_return_types(&view_function)
                .and_then(|tys| {
                    tys.iter()
                        .map(TypeTag::try_from)
                        .collect::<anyhow::Result<Vec<_>>>()
                })
                .map_err(|err| {
                    AptosErrorResponse::bad_request(
                        err,
                        AptosErrorCode::InternalError,
                        Some(&ledger_info),
                    )
                })?;

            let move_vals = values
                .into_iter()
                .zip(return_types.into_iter())
                .map(|(v, ty)| {
                    state_view
                        .as_converter(context.db.clone(), context.indexer_reader.clone())
                        .try_into_move_value(&ty, &v)
                })
                .collect::<anyhow::Result<Vec<_>>>()
                .map_err(|err| {
                    AptosErrorResponse::bad_request(
                        err,
                        AptosErrorCode::InternalError,
                        Some(&ledger_info),
                    )
                })?;

            AptosResponse::try_from_json(move_vals, &ledger_info)
        },
    };

    context.view_function_stats().increment(
        FunctionStats::function_to_key(&view_function.module, &view_function.function),
        output.gas_used,
    );
    result.map(|r| r.with_gas_used(Some(output.gas_used)))
}
