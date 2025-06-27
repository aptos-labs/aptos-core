// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    accept_type::AcceptType,
    bcs_payload::Bcs,
    context::{api_spawn_blocking, FunctionStats},
    failpoint::fail_point_poem,
    response::{
        BadRequestError, BasicErrorWith404, BasicResponse, BasicResponseStatus, BasicResultWith404,
        ForbiddenError, InternalError,
    },
    ApiTags, Context,
};
use anyhow::Context as anyhowContext;
use aptos_api_types::{
    AptosErrorCode, AsConverter, MoveValue, ViewFunction, ViewRequest, MAX_RECURSIVE_TYPES_ALLOWED,
    U64,
};
use aptos_bcs_utils::serialize_uleb128;
use aptos_types::transaction::ViewFunctionError;
use aptos_vm::AptosVM;
use itertools::Itertools;
use move_core_types::language_storage::TypeTag;
use poem_openapi::{param::Query, payload::Json, ApiRequest, OpenApi};
use std::sync::Arc;

/// API for executing Move view function.
#[derive(Clone)]
pub struct ViewFunctionApi {
    pub context: Arc<Context>,
}

#[derive(ApiRequest, Debug)]
pub enum ViewFunctionRequest {
    #[oai(content_type = "application/json")]
    Json(Json<ViewRequest>),

    #[oai(content_type = "application/x.aptos.view_function+bcs")]
    Bcs(Bcs),
}

#[OpenApi]
impl ViewFunctionApi {
    /// Execute view function of a module
    ///
    /// Execute the Move function with the given parameters and return its execution result.
    ///
    /// The Aptos nodes prune account state history, via a configurable time window.
    /// If the requested ledger version has been pruned, the server responds with a 410.
    #[oai(
        path = "/view",
        method = "post",
        operation_id = "view",
        tag = "ApiTags::View"
    )]
    async fn view_function(
        &self,
        accept_type: AcceptType,
        /// View function request with type and position arguments
        request: ViewFunctionRequest,
        /// Ledger version to get state of account
        ///
        /// If not provided, it will be the latest version
        ledger_version: Query<Option<U64>>,
    ) -> BasicResultWith404<Vec<MoveValue>> {
        fail_point_poem("endpoint_view_function")?;
        self.context
            .check_api_output_enabled("View function", &accept_type)?;

        let context = self.context.clone();
        api_spawn_blocking(move || view_request(context, accept_type, request, ledger_version))
            .await
    }
}

fn view_request(
    context: Arc<Context>,
    accept_type: AcceptType,
    request: ViewFunctionRequest,
    ledger_version: Query<Option<U64>>,
) -> BasicResultWith404<Vec<MoveValue>> {
    // Retrieve the current state of the chain
    let (ledger_info, requested_version) = context
        .get_latest_ledger_info_and_verify_lookup_version(ledger_version.map(|inner| inner.0))?;

    let state_view = context
        .state_view_at_version(requested_version)
        .map_err(|err| {
            BasicErrorWith404::bad_request_with_code(
                err,
                AptosErrorCode::InternalError,
                &ledger_info,
            )
        })?;

    let view_function: ViewFunction = match request {
        ViewFunctionRequest::Json(data) => state_view
            .as_converter(context.db.clone(), context.indexer_reader.clone())
            .convert_view_function(data.0)
            .map_err(|err| {
                BasicErrorWith404::bad_request_with_code(
                    err,
                    AptosErrorCode::InvalidInput,
                    &ledger_info,
                )
            })?,
        ViewFunctionRequest::Bcs(data) => {
            bcs::from_bytes_with_limit(data.0.as_slice(), MAX_RECURSIVE_TYPES_ALLOWED as usize)
                .context("Failed to deserialize input into ViewRequest")
                .map_err(|err| {
                    BasicErrorWith404::bad_request_with_code(
                        err,
                        AptosErrorCode::InvalidInput,
                        &ledger_info,
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
        return Err(BasicErrorWith404::forbidden_with_code_no_info(
            format!(
                "Function {}::{} is not allowed",
                view_function.module, view_function.function
            ),
            AptosErrorCode::InvalidInput,
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
        let (err_string, vm_error_code) = match status {
            ViewFunctionError::ExecutionStatus(status, vm_error_code) => {
                let vm_status = state_view
                    .as_converter(context.db.clone(), context.indexer_reader.clone())
                    .explain_vm_status(&status, None);
                (vm_status, vm_error_code)
            },
            ViewFunctionError::ErrorMessage(message, vm_error_code) => (message, vm_error_code),
        };
        BasicErrorWith404::bad_request_with_optional_vm_status_and_ledger_info(
            anyhow::anyhow!(err_string),
            AptosErrorCode::InvalidInput,
            vm_error_code,
            Some(&ledger_info),
        )
    })?;
    let result = match accept_type {
        AcceptType::Bcs => {
            // The return values are already BCS encoded, but we still need to encode the outside
            // vector without re-encoding the inside values
            let num_vals = values.len();

            // Push the length of the return values
            let mut length = vec![];
            serialize_uleb128(&mut length, num_vals as u64).map_err(|err| {
                BasicErrorWith404::internal_with_code(
                    err,
                    AptosErrorCode::InternalError,
                    &ledger_info,
                )
            })?;

            // Combine all of the return values
            let values = values.into_iter().concat();
            let ret = [length, values].concat();

            BasicResponse::try_from_encoded((ret, &ledger_info, BasicResponseStatus::Ok))
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
                    BasicErrorWith404::bad_request_with_code(
                        err,
                        AptosErrorCode::InternalError,
                        &ledger_info,
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
                    BasicErrorWith404::bad_request_with_code(
                        err,
                        AptosErrorCode::InternalError,
                        &ledger_info,
                    )
                })?;

            BasicResponse::try_from_json((move_vals, &ledger_info, BasicResponseStatus::Ok))
        },
    };
    context.view_function_stats().increment(
        FunctionStats::function_to_key(&view_function.module, &view_function.function),
        output.gas_used,
    );
    result.map(|r| r.with_gas_used(Some(output.gas_used)))
}
