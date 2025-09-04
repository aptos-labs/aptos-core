// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use super::errors::VelorTapErrorResponse;
use crate::endpoints::{VelorTapError, VelorTapErrorCode};
use poem::{IntoResponse, Response};

/// In the OpenAPI spec for this API, we say that every response we return will
/// be a JSON representation of VelorTapError. For our own errors, this is exactly
/// what we do. The problem is the Poem framework does not conform to this
/// format, it can return errors in a different format. The purpose of this
/// function is to catch those errors and convert them to the correct format.
pub async fn convert_error(error: poem::Error) -> impl poem::IntoResponse {
    // This is a bit of a hack but errors we return have no source, whereas
    // those returned by the framework do. As such, if we cannot downcast the
    // error we know it's one of ours and we just return it directly.
    let is_framework_error = error.has_source();
    if is_framework_error {
        // Build an VelorTapErrorResponse and then reset its status code
        // to the originally intended status code in the error.
        let mut response = build_error_response(error.to_string()).into_response();
        response.set_status(error.status());
        response
    } else {
        error.into_response()
    }
}

fn build_error_response(error_string: String) -> Response {
    VelorTapErrorResponse::from(VelorTapError::new_with_error_code(
        error_string,
        VelorTapErrorCode::WebFrameworkError,
    ))
    .into_response()
}
