use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
#[allow(non_camel_case_types)]
pub enum ConsensusExtraFeature {
    ValidatorTransaction = 0,
}

/// An extensible feature flag vector indexed by `ConsensusExtraFeature`.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
pub struct ConsensusExtraFeatures {
    features: Vec<bool>,
}

impl ConsensusExtraFeatures {
    pub fn is_enabled(&self, feature: ConsensusExtraFeature) -> bool {
        self.features
            .get(feature as usize)
            .copied()
            .unwrap_or(false)
    }

    pub fn default_for_genesis() -> Self {
        Self {
            features: vec![true],
        }
    }

    pub fn default_if_missing() -> Self {
        Self {
            features: vec![false],
        }
    }

    pub fn update_extra_features(
        &mut self,
        features_to_enable: Vec<ConsensusExtraFeature>,
        features_to_disable: Vec<ConsensusExtraFeature>,
    ) {
        for feature in features_to_enable {
            *self.get_feature_status_mut(feature) = true;
        }

        for feature in features_to_disable {
            *self.get_feature_status_mut(feature) = false;
        }
    }

    fn get_feature_status_mut(&mut self, feature: ConsensusExtraFeature) -> &mut bool {
        let idx = feature as usize;
        if idx >= self.features.len() {
            self.features.resize(idx + 1, false);
        }
        self.features.get_mut(idx).unwrap()
    }
}
