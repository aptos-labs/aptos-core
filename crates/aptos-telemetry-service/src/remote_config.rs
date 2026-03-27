// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::{
    auth::authorize_request,
    context::Context,
    types::{auth::Claims, common::NodeType},
};
use axum::{extract::Extension, http::HeaderMap, Json};
use serde_json::Value;

pub async fn get_telemetry_log_env(
    Extension(context): Extension<Context>,
    headers: HeaderMap,
) -> Result<Json<Value>, crate::errors::ServiceError> {
    let claims = authorize_request(&context, &headers, &[
        NodeType::Validator,
        NodeType::ValidatorFullNode,
        NodeType::PublicFullNode,
    ])
    .await?;
    Ok(Json(handle_telemetry_log_env(claims, context).await))
}

async fn handle_telemetry_log_env(claims: Claims, context: Context) -> Value {
    let env: Option<String> = context
        .log_env_map()
        .get(&claims.chain_id)
        .and_then(|inner| inner.get(&claims.peer_id))
        .cloned();
    serde_json::to_value(&env).unwrap_or(Value::Null)
}

#[cfg(test)]
mod tests {
    use crate::{jwt_auth::create_jwt_token, tests::test_context, types::common::NodeType};
    use aptos_config::config::PeerSet;
    use aptos_types::{chain_id::ChainId, PeerId};
    use std::collections::HashMap;
    use uuid::Uuid;

    #[tokio::test]
    async fn test_handle_telemetry_log_env() {
        let log_level: String = String::from("debug,hyper=off");
        let peer_id = PeerId::random();
        let chain_id = ChainId::default();
        let epoch = 10;
        let node_type = NodeType::Validator;

        let mut test_context = test_context::new_test_context().await;
        test_context
            .inner
            .log_env_map_mut()
            .insert(chain_id, HashMap::from([(peer_id, log_level.clone())]));

        test_context
            .inner
            .peers()
            .validators()
            .write()
            .insert(chain_id, (epoch, PeerSet::default()));

        let jwt_token = create_jwt_token(
            test_context.inner.jwt_service(),
            chain_id,
            peer_id,
            node_type,
            epoch,
            Uuid::default(),
        )
        .unwrap();

        let value = test_context
            .with_bearer_auth(jwt_token)
            .get("/api/v1/config/env/telemetry-log")
            .await;

        assert_eq!(value, serde_json::json!(log_level))
    }
}
