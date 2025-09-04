// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::account_address::AccountAddress;
use anyhow::{bail, Context};
use move_core_types::{identifier::Identifier, language_storage::ModuleId};
use std::str::FromStr;

/// Identifier of a module member (function or struct).
#[derive(Debug, Clone)]
pub struct MemberId {
    pub module_id: ModuleId,
    pub member_id: Identifier,
}

fn parse_member_id(function_id: &str) -> anyhow::Result<MemberId> {
    let ids: Vec<&str> = function_id.split_terminator("::").collect();
    if ids.len() != 3 {
        bail!(
            "FunctionId is not well formed.  Must be of the form <address>::<module>::<function>"
                .to_string()
        );
    }
    let address = AccountAddress::from_str(ids.first().unwrap())?;
    let module = Identifier::from_str(ids.get(1).unwrap()).context("Module Name")?;
    let member_id = Identifier::from_str(ids.get(2).unwrap()).context("Member Name")?;
    let module_id = ModuleId::new(address, module);
    Ok(MemberId {
        module_id,
        member_id,
    })
}

impl FromStr for MemberId {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        parse_member_id(s)
    }
}

pub mod as_move_value;
pub mod move_event_v1;
pub mod move_event_v2;
