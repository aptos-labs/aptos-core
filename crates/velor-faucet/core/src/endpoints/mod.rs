// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

mod api;
mod basic;
mod captcha;
mod error_converter;
mod errors;
mod fund;

pub use self::captcha::{CaptchaApi, CAPTCHA_KEY, CAPTCHA_VALUE};
pub use api::build_openapi_service;
pub use basic::BasicApi;
pub use error_converter::convert_error;
pub use errors::{
    VelorTapError, VelorTapErrorCode, RejectionReason, RejectionReasonCode, USE_HELPFUL_ERRORS,
};
pub use fund::{mint, FundApi, FundApiComponents, FundRequest, FundResponse};
use poem_openapi::Tags;

/// API categories for the OpenAPI spec
#[derive(Tags)]
pub enum ApiTags {
    /// API for funding accounts.
    Fund,

    /// General information
    General,

    /// Captcha API
    Captcha,
}
