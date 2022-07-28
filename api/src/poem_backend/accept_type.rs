// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use poem::web::Accept;

use super::{AptosErrorCode, BadRequestError};

#[derive(PartialEq)]
pub enum AcceptType {
    Json,
    Bcs,
}

// TODO: Make this middleware instead, it could do the check and then add data.

// I can't use TryFrom here right now:
// https://stackoverflow.com/questions/73072492/apply-trait-bounds-to-associated-type
pub fn parse_accept<E: BadRequestError>(accept: &Accept) -> Result<AcceptType, E> {
    for mime in &accept.0 {
        match mime.as_ref() {
            "application/json" => return Ok(AcceptType::Json),
            "application/x-bcs" => return Ok(AcceptType::Bcs),
            "*/*" => {}
            wildcard => {
                // TODO: Consider using 406 instead.
                return Err(E::bad_request_str(&format!(
                    "Unsupported Accept type: {:?}",
                    wildcard
                ))
                .error_code(AptosErrorCode::UnsupportedAcceptType));
            }
        }
    }

    // Default to returning content as JSON.
    Ok(AcceptType::Json)
}
