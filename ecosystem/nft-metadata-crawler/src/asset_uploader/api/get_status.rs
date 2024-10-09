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
    ExpressionMethods, PgConnection, QueryDsl, RunQueryDsl,
};
use tracing::debug;
use uuid::Uuid;

pub fn get_status(
    pool: Pool<ConnectionManager<PgConnection>>,
    request_id: &str,
) -> anyhow::Result<AHashMap<String, GetStatusResponseSuccess>> {
    let mut conn = pool.get()?;
    let request_id = Uuid::parse_str(request_id)?;

    let mut status_response = AHashMap::new();
    let rows = query_status(&mut conn, &request_id)?;
    for row in rows {
        if row.status_code == StatusCode::OK.as_u16() as i64 {
            status_response.insert(row.asset_uri, GetStatusResponseSuccess::Success {
                status_code: StatusCode::OK.as_u16(),
                cdn_image_uri: row.cdn_image_uri.unwrap_or_default(),
            });
        } else {
            status_response.insert(row.asset_uri, GetStatusResponseSuccess::Error {
                status_code: row.status_code as u16,
                error_message: row.error_message,
            });
        };
    }

    Ok(status_response)
}

fn query_status(
    conn: &mut PooledConnection<ConnectionManager<PgConnection>>,
    uuid: &Uuid,
) -> anyhow::Result<Vec<AssetUploaderRequestStatusesQuery>> {
    use schema::nft_metadata_crawler::asset_uploader_request_statuses::dsl::*;

    let query = asset_uploader_request_statuses.filter(request_id.eq(uuid));

    let debug_query = diesel::debug_query::<diesel::pg::Pg, _>(&query).to_string();
    debug!("Executing Query: {}", debug_query);
    let rows = query.load(conn)?;
    Ok(rows)
}
