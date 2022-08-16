// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

mod accounts_test;
mod blocks_test;
mod converter_test;
mod events_test;
mod index_test;
mod invalid_post_request_test;
mod state_test;
mod string_resource_test;
mod transaction_vector_test;
mod transactions_test;

use aptos_api_test_context::{new_test_context as super_new_test_context, TestContext};

fn new_test_context(test_name: String) -> TestContext {
    super_new_test_context(test_name, false)
}
