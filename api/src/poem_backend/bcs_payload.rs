// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

/// This module defines a Poem payload type for BCS. JSON is already natively
/// supported. This payload type is as permissive as possible, as long as it
/// can be (de)serialized with serde, meaning it can be used with BCS, it can
/// be used with this payload type. For the most part this is just following
/// the custom payload example in the Poem repo.
use std::ops::{Deref, DerefMut};

use bcs::{from_bytes, to_bytes};
use poem::{
    http::{header, StatusCode},
    FromRequest, IntoResponse, Request, RequestBody, Response, Result,
};
use poem_openapi::{
    error::ParseRequestPayloadError,
    impl_apirequest_for_payload,
    payload::{ParsePayload, Payload},
    registry::{MetaMediaType, MetaResponse, MetaResponses, MetaSchemaRef, Registry},
    types::Type,
    ApiResponse,
};
use serde::{Deserialize, Serialize};

pub const CONTENT_TYPE: &str = "application/x-bcs";

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Bcs<T>(pub T)
where
    T: Type;

impl<T: Type> Deref for Bcs<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T: Type> DerefMut for Bcs<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<T: Type> Payload for Bcs<T> {
    const CONTENT_TYPE: &'static str = CONTENT_TYPE;

    fn schema_ref() -> MetaSchemaRef {
        T::schema_ref()
    }

    #[allow(unused_variables)]
    fn register(registry: &mut Registry) {
        T::register(registry);
    }
}

#[poem::async_trait]
impl<T: Type + for<'b> Deserialize<'b>> ParsePayload for Bcs<T> {
    const IS_REQUIRED: bool = true;

    async fn from_request(request: &Request, body: &mut RequestBody) -> Result<Self> {
        let data: Vec<u8> = FromRequest::from_request(request, body).await?;
        let value: T = from_bytes(&data).map_err(|err| ParseRequestPayloadError {
            reason: err.to_string(),
        })?;
        Ok(Self(value))
    }
}

impl<T: Serialize + Send + Type> IntoResponse for Bcs<T> {
    fn into_response(self) -> Response {
        let data = match to_bytes(&self.0) {
            Ok(data) => data,
            Err(err) => {
                return Response::builder()
                    .status(StatusCode::INTERNAL_SERVER_ERROR)
                    .body(err.to_string())
            }
        };
        Response::builder()
            .header(header::CONTENT_TYPE, Self::CONTENT_TYPE)
            .body(data)
    }
}

impl<T: Serialize + Type> ApiResponse for Bcs<T> {
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
        T::register(registry);
    }
}

impl_apirequest_for_payload!(Bcs<T>, T: Type + for<'b> Deserialize<'b>);
