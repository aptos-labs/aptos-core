// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    asset_uploader::api::GetStatusResponseSuccess,
    models::asset_uploader_request_statuses_query::AssetUploaderRequestStatusesQuery, schema,
};
use ahash::AHashMap;
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
        let status_code = row.status_code.map(|x| x as u16);

        let response = if let Some(status_code) = status_code {
            if status_code == 200 {
                GetStatusResponseSuccess::Success {
                    status_code: Some(status_code),
                    cdn_image_uri: row.cdn_image_uri.unwrap_or_default(),
                }
            } else {
                GetStatusResponseSuccess::Error {
                    status_code: Some(status_code),
                    error_message: row.error_message,
                }
            }
        } else {
            GetStatusResponseSuccess::Error {
                status_code,
                error_message: row.error_message,
            }
        };

        status_response.insert(row.asset_uri, response);
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
