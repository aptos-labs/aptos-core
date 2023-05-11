// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::smoke_test_environment::new_local_swarm_with_aptos;
use aptos_forge::Swarm;

#[tokio::test]
async fn test_inspection_service_connection() {
    let mut swarm = new_local_swarm_with_aptos(1).await;
    let info = swarm.aptos_public_info();
    // Ping the inspection service index page and verify we get a successful response
    let resp = reqwest::get(info.inspection_service_url().to_owned())
        .await
        .unwrap();
    assert_eq!(reqwest::StatusCode::OK, resp.status());
}
