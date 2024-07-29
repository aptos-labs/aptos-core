// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::metrics::{BIG_QUERY_REQUEST_FAILURES_TOTAL, BIG_QUERY_REQUEST_TOTAL};
use aptos_infallible::RwLock;
use aptos_types::PeerId;
use gcp_bigquery_client::{model::query_request::QueryRequest, Client as BigQueryClient};
use std::{collections::HashMap, str::FromStr, sync::Arc, time::Duration};

const ANALYTICS_PROJECT_ID: &str = "analytics-test-345723";

#[derive(Clone, Debug)]
pub struct PeerLocation {
    pub peer_id: PeerId,
    pub country: Option<String>,
    pub region: Option<String>,
    pub geo_updated_at: Option<String>,
}

pub struct PeerLocationUpdater {
    client: BigQueryClient,
    peer_locations: Arc<RwLock<HashMap<PeerId, PeerLocation>>>,
}

impl PeerLocationUpdater {
    pub fn new(
        client: BigQueryClient,
        peer_locations: Arc<RwLock<HashMap<PeerId, PeerLocation>>>,
    ) -> Self {
        Self {
            client,
            peer_locations,
        }
    }

    pub fn run(self) -> anyhow::Result<()> {
        tokio::spawn(async move {
            loop {
                let locations = query_peer_locations(&self.client).await.unwrap();
                {
                    let mut peer_locations = self.peer_locations.write();
                    *peer_locations = locations;
                }
                tokio::time::sleep(Duration::from_secs(3600)).await; // 1 hour
            }
        });
        Ok(())
    }
}

pub async fn query_peer_locations(
    client: &BigQueryClient,
) -> anyhow::Result<HashMap<PeerId, PeerLocation>> {
    let req = QueryRequest::new("
        SELECT
            sq.peer_id,
            sq.country,
            sq.region,
            '1985-04-12T23:20:50.52Z' as geo_updated_at
        FROM (
            SELECT
            tm.peer_id,
            tm.epoch,
            ROW_NUMBER() OVER (PARTITION BY tm.peer_id ORDER BY tm.epoch DESC) AS row_number,
            tm.country,
            tm.region
            FROM
            `node-telemetry.aptos_node_telemetry.custom_events_mainnet_telemetry_rollup_metrics` tm) sq
        WHERE
            sq.row_number = 1
        LIMIT
            1000
    ");
    let req = QueryRequest {
        timeout_ms: Some(10000),
        ..req
    };

    BIG_QUERY_REQUEST_TOTAL.inc();

    let mut res = client
        .job()
        .query(ANALYTICS_PROJECT_ID, req)
        .await
        .map_err(|e| {
            BIG_QUERY_REQUEST_FAILURES_TOTAL.inc();
            aptos_logger::error!("Failed to query peer locations: {}", e);
            e
        })?;

    let mut map = HashMap::new();
    while res.next_row() {
        if let Some(peer_id_raw) = res.get_string_by_name("peer_id")? {
            match PeerId::from_str(&peer_id_raw) {
                Ok(peer_id) => {
                    let location = PeerLocation {
                        peer_id,
                        geo_updated_at: res.get_string_by_name("geo_updated_at")?,
                        country: res.get_string_by_name("country")?,
                        region: res.get_string_by_name("region")?,
                    };
                    map.entry(peer_id).or_insert(location);
                },
                Err(e) => {
                    aptos_logger::error!("Failed to parse peer_id: {}", e);
                },
            }
        }
    }
    Ok(map)
}
#[cfg(feature = "bigquery_integration_tests")]
mod tests {
    use super::*;
    use gcp_bigquery_client::Client as BigQueryClient;

    #[tokio::test]
    async fn test_query() {
        let client = BigQueryClient::from_application_default_credentials()
            .await
            .unwrap();
        let result = query_peer_locations(&client).await.unwrap();
        println!("{:?}", result);
        assert!(!result.is_empty());
    }
}
