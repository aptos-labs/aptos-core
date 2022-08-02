// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use poem::{http::StatusCode, web::Accept, Error, FromRequest, Request, RequestBody, Result};

#[derive(PartialEq)]
pub enum AcceptType {
    Json,
    Bcs,
}

// This impl allows us to get the data straight from the arguments to the
// endpoint handler.
#[async_trait::async_trait]
impl<'a> FromRequest<'a> for AcceptType {
    async fn from_request(request: &'a Request, _body: &mut RequestBody) -> Result<Self> {
        let accept = Accept::from_request_without_body(request).await?;
        parse_accept(&accept)
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
