// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use std::convert::TryFrom;

use poem::web::Accept;
use poem_openapi::payload::Json;

use super::{AptosError, AptosErrorCode, AptosErrorResponse};

#[derive(PartialEq)]
pub enum AcceptType {
    Json,
    Bcs,
}

impl TryFrom<&Accept> for AcceptType {
    type Error = AptosErrorResponse;

    fn try_from(accept: &Accept) -> Result<Self, Self::Error> {
        for mime in &accept.0 {
            match mime.as_ref() {
                "application/json" => return Ok(AcceptType::Json),
                "application/x-bcs" => return Ok(AcceptType::Bcs),
                "*/*" => {}
                wildcard => {
                    return Err(AptosErrorResponse::BadRequest(Json(
                        AptosError::new(format!("Invalid Accept type: {:?}", wildcard))
                            .error_code(AptosErrorCode::UnsupportedAcceptType),
                    )));
                }
            }
        }

        // Default to returning content as JSON.
        Ok(AcceptType::Json)
    }
}
