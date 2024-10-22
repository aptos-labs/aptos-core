// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    asset_uploader::api::BatchUploadRequest,
    models::asset_uploader_request_statuses::AssetUploaderRequestStatuses, schema,
};
use ahash::AHashMap;
use anyhow::Context;
use diesel::{
    r2d2::{ConnectionManager, Pool, PooledConnection},
    BoolExpressionMethods, ExpressionMethods, PgConnection, QueryDsl, RunQueryDsl,
};
use tracing::debug;
use url::Url;
use uuid::Uuid;

/// Uploads a batch of assets to the asset uploader worker
pub fn upload_batch(
    pool: Pool<ConnectionManager<PgConnection>>,
    request: &BatchUploadRequest,
) -> anyhow::Result<String> {
    let mut conn = pool.get()?;
    let existing_rows = get_existing_rows(&mut conn, &request.urls)?;

    let request_id = Uuid::new_v4();
    let application_id = Uuid::parse_str(&request.application_id)?;
    let mut request_statuses = vec![];
    for url in &request.urls {
        if let Some(cdn_image_uri) = existing_rows.get(url.as_str()) {
            request_statuses.push(AssetUploaderRequestStatuses::new_completed(
                request_id,
                url.as_str(),
                application_id,
                cdn_image_uri.as_deref().unwrap(), // Safe to unwrap because we checked for existence when querying
            ));
        } else {
            request_statuses.push(AssetUploaderRequestStatuses::new(
                request_id,
                url.as_str(),
                application_id,
            ));
        }
    }

    insert_request_statuses(&mut conn, &request_statuses)?;
    Ok(request_id.to_string())
}

fn get_existing_rows(
    conn: &mut PooledConnection<ConnectionManager<PgConnection>>,
    urls: &[Url],
) -> anyhow::Result<AHashMap<String, Option<String>>> {
    use schema::nft_metadata_crawler::parsed_asset_uris::dsl::*;

    let query = parsed_asset_uris
        .filter(
            asset_uri
                .eq_any(urls.iter().map(Url::as_str))
                .and(cdn_image_uri.is_not_null()),
        )
        .select((asset_uri, cdn_image_uri));

    let debug_query = diesel::debug_query::<diesel::pg::Pg, _>(&query).to_string();
    debug!("Executing Query: {}", debug_query);
    let rows = query.load(conn)?;
    Ok(AHashMap::from_iter(rows))
}

fn insert_request_statuses(
    conn: &mut PooledConnection<ConnectionManager<PgConnection>>,
    request_statuses: &[AssetUploaderRequestStatuses],
) -> anyhow::Result<usize> {
    use schema::nft_metadata_crawler::asset_uploader_request_statuses::dsl::*;

    let query =
        diesel::insert_into(schema::nft_metadata_crawler::asset_uploader_request_statuses::table)
            .values(request_statuses)
            .on_conflict((request_id, asset_uri))
            .do_nothing();

    let debug_query = diesel::debug_query::<diesel::pg::Pg, _>(&query).to_string();
    debug!("Executing Query: {}", debug_query);
    query.execute(conn).context(debug_query)
}
