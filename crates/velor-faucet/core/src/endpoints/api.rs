// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use super::{basic::BasicApi, fund::FundApi, CaptchaApi};
use poem_openapi::{ContactObject, LicenseObject, OpenApiService};

const VERSION: &str = include_str!("../../../doc/.version");

pub fn build_openapi_service(
    basic_api: BasicApi,
    captcha_api: CaptchaApi,
    fund_api: FundApi,
) -> OpenApiService<(BasicApi, CaptchaApi, FundApi), ()> {
    let version = VERSION.to_string();
    let license =
        LicenseObject::new("Apache 2.0").url("https://www.apache.org/licenses/LICENSE-2.0.html");
    let contact = ContactObject::new()
        .name("Velor Labs")
        .url("https://github.com/velor-chain");

    let apis = (basic_api, captcha_api, fund_api);

    OpenApiService::new(apis, "Velor Tap", version.trim())
        .server("/v1")
        .description("todo")
        .license(license)
        .contact(contact)
}
