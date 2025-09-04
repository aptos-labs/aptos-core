// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

//! This module defines a Poem payload type for BCS. JSON is already natively
//! supported. This type just helps with representing BCS bytes in the spec.

// Previously the Bcs payload type took a T, not Vec<u8>. For more information
// about that effort, see https://github.com/velor-chain/velor-core/issues/2277.

use velor_api_types::mime_types::BCS;
use poem::{http::header, FromRequest, IntoResponse, Request, RequestBody, Response, Result};
use poem_openapi::{
    impl_apirequest_for_payload,
    payload::{ParsePayload, Payload},
    registry::{MetaMediaType, MetaResponse, MetaResponses, MetaSchemaRef, Registry},
    types::Type,
    ApiResponse,
};
use std::ops::{Deref, DerefMut};

/// A wrapper struct for a payload containing BCS encoded bytes
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Bcs(pub Vec<u8>);

impl Deref for Bcs {
    type Target = Vec<u8>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Bcs {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl Payload for Bcs {
    const CONTENT_TYPE: &'static str = BCS;

    fn schema_ref() -> MetaSchemaRef {
        Vec::<u8>::schema_ref()
    }

    #[allow(unused_variables)]
    fn register(registry: &mut Registry) {
        Vec::<u8>::register(registry);
    }
}

impl ParsePayload for Bcs {
    const IS_REQUIRED: bool = true;

    async fn from_request(request: &Request, body: &mut RequestBody) -> Result<Self> {
        let data = Vec::<u8>::from_request(request, body).await?;
        Ok(Self(data))
    }
}

impl IntoResponse for Bcs {
    fn into_response(self) -> Response {
        Response::builder()
            .header(header::CONTENT_TYPE, Self::CONTENT_TYPE)
            .body(self.0)
    }
}

impl ApiResponse for Bcs {
    fn meta() -> MetaResponses {
        MetaResponses {
            responses: vec![MetaResponse {
                description: "BCS: Binary Canonical Serialization",
                status: Some(200),
                content: vec![MetaMediaType {
                    content_type: Self::CONTENT_TYPE,
                    schema: Self::schema_ref(),
                }],
                headers: vec![],
            }],
        }
    }

    fn register(registry: &mut Registry) {
        Vec::<u8>::register(registry);
    }
}

impl_apirequest_for_payload!(Bcs);
