// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    accept_type::AcceptType,
    failpoint::fail_point_poem,
    response::{
        BadRequestError, BasicErrorWith404, BasicResponse, BasicResponseStatus, BasicResultWith404,
    },
    ApiTags, Context,
};
use aptos_api_types::{AptosErrorCode, AsConverter, MoveValue, ViewRequest, U64};
use aptos_vm::{data_cache::AsMoveResolver, AptosVM};
use move_core_types::language_storage::TypeTag;
use poem_openapi::{param::Query, payload::Json, OpenApi};
use std::sync::Arc;

/// API for executing Move view function.
pub struct ViewFunctionApi {
    pub context: Arc<Context>,
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
        request: Json<ViewRequest>,
        /// Ledger version to get state of account
        ///
        /// If not provided, it will be the latest version
        ledger_version: Query<Option<U64>>,
    ) -> BasicResultWith404<Vec<MoveValue>> {
        fail_point_poem("endpoint_view_function")?;
        self.context
            .check_api_output_enabled("View function", &accept_type)?;

        let (ledger_info, requested_version) = self
            .context
            .get_latest_ledger_info_and_verify_lookup_version(
                ledger_version.map(|inner| inner.0),
            )?;

        let state_view = self.context.latest_state_view_poem(&ledger_info)?;
        let resolver = state_view.as_move_resolver();

        let entry_func = resolver
            .as_converter(self.context.db.clone())
            .convert_view_function(request.0)
            .map_err(|err| {
                BasicErrorWith404::bad_request_with_code(
                    err,
                    AptosErrorCode::InvalidInput,
                    &ledger_info,
                )
            })?;
        let state_view = self
            .context
            .state_view_at_version(requested_version)
            .map_err(|err| {
                BasicErrorWith404::bad_request_with_code(
                    err,
                    AptosErrorCode::InternalError,
                    &ledger_info,
                )
            })?;

        let return_vals = AptosVM::execute_view_function(
            &state_view,
            entry_func.module().clone(),
            entry_func.function().to_owned(),
            entry_func.ty_args().to_owned(),
            entry_func.args().to_owned(),
            self.context.node_config.api.max_gas_view_function,
        )
        .map_err(|err| {
            BasicErrorWith404::bad_request_with_code_no_info(err, AptosErrorCode::InvalidInput)
        })?;
        match accept_type {
            AcceptType::Bcs => {
                BasicResponse::try_from_bcs((return_vals, &ledger_info, BasicResponseStatus::Ok))
            },
            AcceptType::Json => {
                let return_types = resolver
                    .as_converter(self.context.db.clone())
                    .function_return_types(&entry_func)
                    .and_then(|tys| {
                        tys.into_iter()
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

                let move_vals = return_vals
                    .into_iter()
                    .zip(return_types.into_iter())
                    .map(|(v, ty)| {
                        resolver
                            .as_converter(self.context.db.clone())
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
        }
    }
}
