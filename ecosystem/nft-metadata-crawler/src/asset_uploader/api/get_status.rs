// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    asset_uploader::api::GetStatusResponseSuccess,
    models::asset_uploader_request_statuses_query::AssetUploaderRequestStatusesQuery, schema,
};
use ahash::AHashMap;
use axum::http::StatusCode;
use diesel::{
    r2d2::{ConnectionManager, Pool, PooledConnection},
    BoolExpressionMethods, ExpressionMethods, PgConnection, QueryDsl, RunQueryDsl,
};
use tracing::debug;

pub fn get_status(
    pool: Pool<ConnectionManager<PgConnection>>,
    idempotency_key: &str,
    application_id: &str,
) -> anyhow::Result<AHashMap<String, GetStatusResponseSuccess>> {
    let mut conn = pool.get()?;
    let mut status_response = AHashMap::new();
    let rows = query_status(&mut conn, idempotency_key, application_id)?;
    for row in rows {
        if row.status_code == StatusCode::OK.as_u16() as i64 {
            status_response.insert(row.asset_uri, GetStatusResponseSuccess::Success {
                status_code: StatusCode::OK.as_u16(),
                cdn_image_uri: row.cdn_image_uri.unwrap_or_default(),
            });
        } else {
            status_response.insert(row.asset_uri, GetStatusResponseSuccess::Error {
                status_code: row.status_code as u16,
                error_message: row.error_messages,
            });
        };
    }

    Ok(status_response)
}

fn query_status(
    conn: &mut PooledConnection<ConnectionManager<PgConnection>>,
    id_key: &str,
    app_id: &str,
) -> anyhow::Result<Vec<AssetUploaderRequestStatusesQuery>> {
    use schema::nft_metadata_crawler::asset_uploader_request_statuses::dsl::*;

    let query = asset_uploader_request_statuses
        .filter(idempotency_key.eq(id_key).and(application_id.eq(app_id)));

    let debug_query = diesel::debug_query::<diesel::pg::Pg, _>(&query).to_string();
    debug!("Executing Query: {}", debug_query);
    let rows = query.load(conn)?;
    Ok(rows)
}
