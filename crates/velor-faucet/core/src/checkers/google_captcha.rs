// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use super::{CheckerData, CheckerTrait};
use crate::endpoints::{VelorTapError, VelorTapErrorCode, RejectionReason, RejectionReasonCode};
use anyhow::Result;
use velor_logger::debug;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::{env, str::FromStr};

const GOOGLE_CAPTCHA_ENDPOINT: &str = "https://www.google.com/recaptcha/api/siteverify";
pub const COMPLETED_CAPTCHA_TOKEN: &str = "COMPLETED_CAPTCHA_TOKEN";

#[derive(Clone, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct KeyString(pub String);

impl std::fmt::Debug for KeyString {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "<key hidden>")
    }
}

impl FromStr for KeyString {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self(s.to_string()))
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct GoogleCaptchaCheckerConfig {
    pub google_captcha_api_key: KeyString,
}

pub struct CaptchaChecker {
    config: GoogleCaptchaCheckerConfig,
}

impl CaptchaChecker {
    pub fn new(config: GoogleCaptchaCheckerConfig) -> Result<Self> {
        Ok(Self { config })
    }
}

/// Request payload for captcha verify API. See https://developers.google.com/recaptcha/docs/verify#api_request
#[derive(Serialize)]
struct VerifyRequest {
    // The shared key between tap and reCAPTCHA.
    secret: String,
    // The user response token provided by the reCAPTCHA client-side
    response: String,
    // The user's IP address.
    remoteip: String,
}

#[async_trait]
impl CheckerTrait for CaptchaChecker {
    async fn check(
        &self,
        data: CheckerData,
        _dry_run: bool,
    ) -> Result<Vec<RejectionReason>, VelorTapError> {
        let captcha_token = match data.headers.get(COMPLETED_CAPTCHA_TOKEN) {
            Some(header_value) => header_value.to_str().map_err(|e| {
                VelorTapError::new_with_error_code(e, VelorTapErrorCode::InvalidRequest)
            })?,
            None => {
                return Ok(vec![RejectionReason::new(
                    format!("Captcha header {} not found", COMPLETED_CAPTCHA_TOKEN),
                    RejectionReasonCode::CaptchaInvalid,
                )])
            },
        };

        let verify_result = reqwest::Client::new()
            .post(GOOGLE_CAPTCHA_ENDPOINT)
            // Google captcha API only accepts form encoded payload, lol
            .form::<VerifyRequest>(&VerifyRequest {
                secret: self.config.google_captcha_api_key.0.clone(),
                response: captcha_token.to_string(),
                remoteip: data.source_ip.to_string(),
            })
            .send()
            .await
            .map_err(|e| VelorTapError::new_with_error_code(e, VelorTapErrorCode::CheckerError))?;

        let status_code = verify_result.status();
        let resp = verify_result
            .text()
            .await
            .map_err(|e| VelorTapError::new_with_error_code(e, VelorTapErrorCode::CheckerError))?;
        if !status_code.is_success() {
            debug!(
                message = "Google captcha API returned error status code",
                status = status_code.as_str(),
                resp = resp
            );
        } else {
            // Rather than `verify_result.json`, we parse the result with serde_json to have more flexibilities
            let resp: serde_json::Value = serde_json::from_str(resp.as_str()).map_err(|e| {
                VelorTapError::new_with_error_code(e, VelorTapErrorCode::CheckerError)
            })?;

            if resp["success"].as_bool().unwrap_or(false) {
                return Ok(vec![]);
            } else {
                debug!(
                    message = "Invalid captcha token",
                    source_ip = data.source_ip,
                    resp = resp
                );
            }
        };

        Ok(vec![RejectionReason::new(
            "Failed to pass captcha check".to_string(),
            RejectionReasonCode::CaptchaInvalid,
        )])
    }

    fn cost(&self) -> u8 {
        10
    }
}
