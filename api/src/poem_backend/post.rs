// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use poem_openapi::{
    payload::Json,
    types::{ParseFromJSON, ToJSON, Type},
    ApiRequest,
};
use serde::Deserialize;

use super::bcs_payload::Bcs;

#[derive(ApiRequest)]
pub enum AptosPost<T: ToJSON + ParseFromJSON + Send + Sync + Type + for<'b> Deserialize<'b>> {
    #[oai(content_type = "application/x-bcs")]
    Bcs(Bcs<T>),

    #[oai(content_type = "application/json")]
    Json(Json<T>),
}
