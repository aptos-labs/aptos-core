use serde::{Deserialize, Serialize};

use crate::on_chain_config::validator_info::ValidatorInfo;

// Just to represent one memory layout for other repo to pass

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
// #[cfg_attr(any(test, feature = "fuzzing"))]
pub struct ValidatorSet {
    pub active_validators: Vec<ValidatorInfo>,
    pub pending_inactive: Vec<ValidatorInfo>,
    pub pending_active: Vec<ValidatorInfo>,
    pub total_voting_power: u128,
    pub total_joining_power: u128,
}