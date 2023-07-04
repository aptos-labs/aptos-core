// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

pub(crate) mod block;
pub(crate) mod dag;
pub(crate) mod quorum_certificate;
pub(crate) mod single_entry;

use anyhow::{ensure, Result};

pub(crate) fn ensure_slice_len_eq(data: &[u8], len: usize) -> Result<()> {
    ensure!(
        data.len() == len,
        "Unexpected data len {}, expected {}.",
        data.len(),
        len,
    );
    Ok(())
}

pub use block::BLOCK_CF_NAME;
pub use dag::{CERTIFIED_NODE_CF_NAME, NODE_CF_NAME};
pub use quorum_certificate::QC_CF_NAME;
pub use single_entry::SINGLE_ENTRY_CF_NAME;
