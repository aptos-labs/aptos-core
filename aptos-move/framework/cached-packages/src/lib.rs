// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use framework_releases::{Release, ReleaseFetcher};
use once_cell::sync::Lazy;

static MODULE_BLOBS: Lazy<Vec<Vec<u8>>> = Lazy::new(||
    ReleaseFetcher::new(Release::Aptos, "fresh").module_blobs().unwrap());

pub fn module_blobs() -> Vec<Vec<u8>> {
    MODULE_BLOBS.clone()
}
