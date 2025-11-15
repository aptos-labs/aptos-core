use ark_ff::FftField;
use serde::{Deserialize, Serialize};

use crate::arkworks::shamir::ShamirThresholdConfig;


#[allow(non_snake_case)]
#[derive(Clone, Deserialize, Serialize, Debug, PartialEq, Eq)]
pub struct WeightedConfig<F: FftField> {
    tc: ShamirThresholdConfig<F>,
}
