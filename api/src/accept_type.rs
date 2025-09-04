// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use velor_api_types::mime_types::{BCS, JSON};
use poem::{web::Accept, FromRequest, Request, RequestBody, Result};

/// Accept types from input headers
///
/// Determines the output type of each API
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum AcceptType {
    /// Convert and resolve types to JSON
    Json,
    /// Take types with as little conversion as possible from the database
    Bcs,
}

/// This impl allows us to get the data straight from the arguments to the
/// endpoint handler.
impl<'a> FromRequest<'a> for AcceptType {
    async fn from_request(request: &'a Request, _body: &mut RequestBody) -> Result<Self> {
        let accept = Accept::from_request_without_body(request).await?;
        parse_accept(&accept)
    }
}

/// Check that the accept type is one of the allowed variants. If there is no
/// overriding explicit accept type, default to JSON.
fn parse_accept(accept: &Accept) -> Result<AcceptType> {
    for mime in &accept.0 {
        if matches!(mime.as_ref(), JSON) {
            return Ok(AcceptType::Json);
        }
        if matches!(mime.as_ref(), BCS) {
            return Ok(AcceptType::Bcs);
        }
    }

    // Default to returning content as JSON.
    Ok(AcceptType::Json)
}
