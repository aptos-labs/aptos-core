// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use aptos_api_types::mime_types::JSON;
use aptos_api_types::{AptosError, AptosErrorCode};
use poem::http::header::{HeaderValue, CONTENT_TYPE};
use poem::{IntoResponse, Response};
use poem_openapi::payload::Json;

// The way I'm determining which errors are framework errors is very janky, as
// is the way I'm building the response. See:
// - https://github.com/poem-web/poem/issues/343

/// In the OpenAPI spec for this API, we say that every response we return will
/// be a JSON representation of AptosError. For our own code, this is exactly
/// what we do. The problem is the Poem framework does not conform to this
/// format, it can return errors in a different format. The purpose of this
/// function is to catch those errors and convert them to the correct format.
pub async fn convert_error(error: poem::Error) -> impl poem::IntoResponse {
    // This is a bit of a hack but errors we return have no source, whereas
    // those returned by the framework do. As such, if we cannot downcast the
    // error we know it's one of ours and we just return it directly.
    let error_string = error.to_string();
    let is_framework_error = error.has_source();
    if is_framework_error {
        // Build the response.
        let mut response = error.into_response();
        // Replace the body with the response.
        response.set_body(build_error_response(error_string).take_body());
        response
            .headers_mut()
            .insert(CONTENT_TYPE, HeaderValue::from_static(JSON));
        response
    } else {
        error.into_response()
    }
}

fn build_error_response(error_string: String) -> Response {
    Json(AptosError::new_with_error_code(
        error_string,
        AptosErrorCode::WebFrameworkError,
    ))
    .into_response()
}
