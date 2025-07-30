// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

fn main() -> shadow_rs::SdResult<()> {
    shadow_rs::ShadowBuilder::builder()
        .deny_const(Default::default())
        .build()?;
    Ok(())
}
