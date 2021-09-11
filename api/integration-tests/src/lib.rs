// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

use forge::{PublicUsageContext, PublicUsageTest, Result, Test};

pub struct GetIndex;

impl Test for GetIndex {
    fn name(&self) -> &'static str {
        "api::get-index"
    }
}

impl PublicUsageTest for GetIndex {
    fn run<'t>(&self, ctx: &mut PublicUsageContext<'t>) -> Result<()> {
        let resp = reqwest::blocking::get(ctx.rest_api_url().to_owned())?;
        assert_eq!(reqwest::StatusCode::OK, resp.status());

        Ok(())
    }
}
