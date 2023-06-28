// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use async_graphql::{EmptyMutation, EmptySubscription, Schema};

pub struct QueryRoot;

#[allow(dead_code)]
pub type ApiV2Schema = Schema<QueryRoot, EmptyMutation, EmptySubscription>;
