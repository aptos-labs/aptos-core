// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! This API is for the in-house TapCaptchaChecker, it doesn't do anything for
//! the GoogleCaptchaChecker.

use super::{errors::AptosTapErrorResponse, ApiTags, AptosTapError, AptosTapErrorCode};
use crate::checkers::CaptchaManager;
use futures::lock::Mutex;
use poem::Result;
use poem_openapi::{
    payload::{Binary, Response},
    OpenApi,
};
use std::sync::Arc;

pub struct CaptchaApi {
    pub enabled: bool,
    pub captcha_manager: Arc<Mutex<CaptchaManager>>,
}

pub const CAPTCHA_KEY: &str = "CAPTCHA_KEY";
pub const CAPTCHA_VALUE: &str = "CAPTCHA_VALUE";

#[OpenApi]
impl CaptchaApi {
    /// Initiate captcha flow
    ///
    /// With this endpoint you can initiate a captcha flow. The response will
    /// contain an image (the captcha to solve) in the body and a code in the
    /// header that you must include in the call to `/fund`. This endpoint is
    /// only relevant if the CaptchaChecker is enabled.
    #[oai(
        path = "/request_captcha",
        method = "get",
        operation_id = "request_captcha",
        response_header(name = "CAPTCHA_KEY", ty = "u32", description = "Captcha key"),
        tag = "ApiTags::Captcha"
    )]
    async fn request_captcha(&self) -> Result<Response<Binary<Vec<u8>>>, AptosTapErrorResponse> {
        if !self.enabled {
            return Err(AptosTapError::new(
                "The CaptchaChecker is not enabled".to_string(),
                AptosTapErrorCode::EndpointNotEnabled,
            )
            .into());
        }
        let mut captcha_manager = self.captcha_manager.lock().await;
        let (key, image) = match captcha_manager.create_challenge() {
            Ok((key, image)) => (key, image),
            Err(e) => {
                return Err(
                    AptosTapError::new_with_error_code(e, AptosTapErrorCode::CheckerError).into(),
                );
            },
        };
        Ok(Response::new(Binary(image)).header(CAPTCHA_KEY, key))
    }
}
