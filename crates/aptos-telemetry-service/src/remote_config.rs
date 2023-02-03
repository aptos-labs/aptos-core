// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{
    auth::with_auth,
    context::Context,
    types::{auth::Claims, common::NodeType},
};
use warp::{filters::BoxedFilter, reply, Filter, Rejection, Reply};

pub fn telemetry_log_env(context: Context) -> BoxedFilter<(impl Reply,)> {
    warp::path!("config" / "env" / "telemetry-log")
        .and(warp::get())
        .and(with_auth(context.clone(), vec![
            NodeType::Validator,
            NodeType::ValidatorFullNode,
            NodeType::PublicFullNode,
        ]))
        .and(context.filter())
        .and_then(handle_telemetry_log_env)
        .boxed()
}

async fn handle_telemetry_log_env(
    claims: Claims,
    context: Context,
) -> Result<impl Reply, Rejection> {
    let env: Option<String> = context
        .log_env_map()
        .get(&claims.chain_id)
        .and_then(|inner| inner.get(&claims.peer_id))
        .cloned();
    Ok(reply::json(&env))
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

        assert_eq!(value, log_level)
    }
}
