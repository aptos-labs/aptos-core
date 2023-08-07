// Copyright © Aptos Foundation

use move_core_types::ident_str;
use move_core_types::identifier::IdentStr;
use move_core_types::move_resource::MoveStructType;
use crate::validator_info::ValidatorInfo;
use serde::{Serialize, Deserialize};
use anyhow::Result;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StartDKGEvent {
    pub locked_new_validator_set: Vec<ValidatorInfo>,
}

impl StartDKGEvent {
    pub fn try_from_bytes(bytes: &[u8]) -> Result<Self> {
        bcs::from_bytes(bytes).map_err(Into::into)
    }
}
impl MoveStructType for StartDKGEvent {
    const MODULE_NAME: &'static IdentStr = ident_str!("dkg");
    const STRUCT_NAME: &'static IdentStr = ident_str!("StartDKGEvent");
}
