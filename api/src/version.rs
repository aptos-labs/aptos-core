// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::param::LedgerVersionParam;

use serde::Deserialize;

#[derive(Clone, Debug, Deserialize)]
pub(crate) struct Version {
    pub(crate) version: Option<LedgerVersionParam>,
}
