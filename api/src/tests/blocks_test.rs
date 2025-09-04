// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use super::new_test_context;
use velor_api_test_context::current_function_name;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_get_genesis_block_by_height() {
    let mut context = new_test_context(current_function_name!());

    let resp = context.get(&blocks_by_height(0)).await;
    context.check_golden_output(resp);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_get_unknown_block_by_height() {
    let mut context = new_test_context(current_function_name!());

    let resp = context
        .expect_status_code(404)
        .get(&blocks_by_height(1000))
        .await;
    context.check_golden_output(resp);
}

fn blocks_by_height(height: u64) -> String {
    format!("/blocks/by_height/{}", height)
}
