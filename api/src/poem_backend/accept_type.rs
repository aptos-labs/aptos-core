// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use anyhow::Context;
use poem::{http::StatusCode, web::Accept, Error, FromRequest, Request, RequestBody, Result};

#[derive(PartialEq)]
pub enum AcceptType {
    Json,
    Bcs,
}

// This impl allows us to get the data straight from the arguments to the
// endpoint handler.
#[async_trait::async_trait]
impl<'a> FromRequest<'a> for &'a AcceptType {
    async fn from_request(req: &'a Request, _body: &mut RequestBody) -> Result<Self> {
        Ok(req
            .extensions()
            .get::<AcceptType>()
            .context("AcceptType not found in request extensions, make sure you're using middleware_accept_type")?)
    }
}

// Check that the accept type is one of the allowed variants. If there is no
// accept type and nothing explicitly not allowed, default to JSON.
fn parse_accept(accept: &Accept) -> Result<AcceptType> {
    for mime in &accept.0 {
        match mime.as_ref() {
            "application/json" => return Ok(AcceptType::Json),
            "application/x-bcs" => return Ok(AcceptType::Bcs),
            "*/*" => {}
            wildcard => {
                return Err(Error::from_string(
                    &format!("Unsupported Accept type: {:?}", wildcard),
                    StatusCode::NOT_ACCEPTABLE,
                ));
            }
        }
    }

    // Default to returning content as JSON.
    Ok(AcceptType::Json)
}

// Attach the AcceptType to the request.
pub async fn middleware_accept_type(mut request: Request) -> Result<Request> {
    let accept = Accept::from_request_without_body(&request).await?;
    let accept_type = parse_accept(&accept)?;

    request.extensions_mut().insert(accept_type);

    Ok(request)
}
