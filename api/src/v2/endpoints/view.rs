// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-core/blob/main/LICENSE

use crate::v2::{
    context::{spawn_blocking, V2Context},
    error::{ErrorCode, V2Error},
    types::{LedgerVersionParam, V2Response},
};
use aptos_api_types::AsConverter;
use axum::{
    extract::{Query, State},
    Json,
};
use move_core_types::language_storage::TypeTag;

/// POST /v2/view
///
/// Accepts a JSON ViewRequest and returns the view function result as JSON values.
#[utoipa::path(
    post,
    path = "/v2/view",
    tag = "View",
    params(LedgerVersionParam),
    request_body(content = Object, description = "View function request (function, type_arguments, arguments)"),
    responses(
        (status = 200, description = "View function result values", body = Object),
        (status = 400, description = "Invalid input or view function failed", body = V2Error),
    )
)]
pub async fn view_handler(
    State(ctx): State<V2Context>,
    Query(params): Query<LedgerVersionParam>,
    Json(request): Json<aptos_api_types::ViewRequest>,
) -> Result<Json<V2Response<Vec<serde_json::Value>>>, V2Error> {
    let ctx = ctx.clone();
    spawn_blocking(move || {
        let (ledger_info, _version, state_view) = ctx.state_view_at(params.ledger_version)?;

        let converter =
            state_view.as_converter(ctx.inner().db.clone(), ctx.inner().indexer_reader.clone());

        let view_function = converter.convert_view_function(request).map_err(|e| {
            V2Error::bad_request(ErrorCode::InvalidInput, e.to_string())
        })?;

        let output = aptos_vm::AptosVM::execute_view_function(
            &state_view,
            view_function.module.clone(),
            view_function.function.clone(),
            view_function.ty_args.clone(),
            view_function.args.clone(),
            ctx.v2_config.max_gas_view_function,
        );

        let values = output.values.map_err(|status| {
            V2Error::bad_request(
                ErrorCode::ViewFunctionFailed,
                format!("View function execution failed: {:?}", status),
            )
        })?;

        // Convert return values to JSON
        let return_types = converter
            .function_return_types(&view_function)
            .and_then(|tys| {
                tys.iter()
                    .map(TypeTag::try_from)
                    .collect::<anyhow::Result<Vec<_>>>()
            })
            .map_err(V2Error::internal)?;

        let move_vals: Vec<serde_json::Value> = values
            .into_iter()
            .zip(return_types.into_iter())
            .map(|(v, ty)| {
                let move_val = converter.try_into_move_value(&ty, &v)?;
                Ok(move_val.json()?)
            })
            .collect::<anyhow::Result<Vec<_>>>()
            .map_err(V2Error::internal)?;

        Ok(Json(V2Response::new(move_vals, &ledger_info)))
    })
    .await
}
