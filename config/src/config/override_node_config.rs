// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::config::{Error, NodeConfig, PersistableConfig};
use anyhow::bail;
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Diff a config yaml with a base config yaml. Returns None if there is no diff.
fn diff_override_config_yaml(
    override_config: serde_yaml::Value,
    base_config: serde_yaml::Value,
) -> anyhow::Result<Option<serde_yaml::Value>> {
    match (override_config.clone(), base_config.clone()) {
        (
            serde_yaml::Value::Mapping(override_mapping),
            serde_yaml::Value::Mapping(base_mapping),
        ) => {
            let mut overrides = serde_yaml::Mapping::new();
            for (override_key, override_value) in override_mapping {
                match base_mapping.get(&override_key) {
                    Some(base_value) => {
                        if let Some(diff_value) =
                            diff_override_config_yaml(override_value, base_value.clone())?
                        {
                            overrides.insert(override_key, diff_value);
                        }
                    },
                    None => {
                        overrides.insert(override_key, override_value);
                    },
                }
            }
            if overrides.is_empty() {
                Ok(None)
            } else {
                Ok(Some(serde_yaml::Value::Mapping(overrides)))
            }
        },
        (serde_yaml::Value::Mapping(_), _) => Ok(Some(override_config)),
        (serde_yaml::Value::Null, serde_yaml::Value::Null) => Ok(None),
        (serde_yaml::Value::Bool(override_value), serde_yaml::Value::Bool(base_value)) => {
            if override_value == base_value {
                Ok(None)
            } else {
                Ok(Some(override_config))
            }
        },
        (serde_yaml::Value::Number(override_value), serde_yaml::Value::Number(base_value)) => {
            if override_value == base_value {
                Ok(None)
            } else {
                Ok(Some(override_config))
            }
        },
        (serde_yaml::Value::String(override_value), serde_yaml::Value::String(base_value)) => {
            if override_value == base_value {
                Ok(None)
            } else {
                Ok(Some(override_config))
            }
        },
        (serde_yaml::Value::Sequence(override_value), serde_yaml::Value::Sequence(base_value)) => {
            if override_value == base_value {
                Ok(None)
            } else {
                Ok(Some(override_config))
            }
        },
        (_, _) => bail!(
            "base does not match override: {:?}, {:?}",
            override_config,
            base_config
        ),
    }
}

/// A utility struct for managing a node config that is overriding a base config, and outputting a
/// yaml representation of it. In most cases, the base config will be the default config, and the
/// output will be a minimal yaml diff that an aptos node can read.
///
/// In rare cases you may want to explicitly write a yaml value that is the same as the default
/// config (e.g., to avoid the config being optimized by ConfigOptimizer). To do this, change the
/// base config to a different value before calling get_yaml().
#[derive(Deserialize, Serialize, PartialEq, Debug, Clone)]
pub struct OverrideNodeConfig {
    override_config: NodeConfig,
    base_config: NodeConfig,
}

impl OverrideNodeConfig {
    pub fn new(override_config: NodeConfig, base_config: NodeConfig) -> Self {
        Self {
            override_config,
            base_config,
        }
    }

    pub fn new_default() -> Self {
        Self::new(NodeConfig::default(), NodeConfig::default())
    }

    pub fn new_with_default_base(override_config: NodeConfig) -> Self {
        Self {
            override_config,
            base_config: NodeConfig::default(),
        }
    }

    pub fn override_config(&self) -> &NodeConfig {
        &self.override_config
    }

    pub fn override_config_mut(&mut self) -> &mut NodeConfig {
        &mut self.override_config
    }

    pub fn base_config(&self) -> &NodeConfig {
        &self.base_config
    }

    pub fn base_config_mut(&mut self) -> &mut NodeConfig {
        &mut self.base_config
    }

    pub fn get_yaml(&self) -> anyhow::Result<serde_yaml::Value> {
        let config_yaml = serde_yaml::to_value(&self.override_config)?;
        let base_yaml = serde_yaml::to_value(&self.base_config)?;
        diff_override_config_yaml(config_yaml, base_yaml).map(|diff_yaml| {
            diff_yaml.unwrap_or(serde_yaml::Value::Mapping(serde_yaml::Mapping::new()))
        })
    }

    fn merge_yaml(a: &mut serde_yaml::Value, b: serde_yaml::Value) {
        match (a, b) {
            (serde_yaml::Value::Mapping(a_map), serde_yaml::Value::Mapping(b_map)) => {
                for (key, b_value) in b_map {
                    let a_value = a_map.entry(key).or_insert(serde_yaml::Value::Null);
                    Self::merge_yaml(a_value, b_value);
                }
            },
            (serde_yaml::Value::Sequence(a_seq), serde_yaml::Value::Sequence(b_seq)) => {
                a_seq.extend(b_seq);
            },
            (a, b) => {
                *a = b;
            },
        }
    }

    pub fn get_yaml_with_override(
        &self,
        env_override: Option<serde_yaml::Value>,
    ) -> anyhow::Result<serde_yaml::Value> {
        let mut test_yaml = self.get_yaml()?;
        let Some(env_override_yaml) = env_override else {
            return Ok(test_yaml);
        };
        Self::merge_yaml(&mut test_yaml, env_override_yaml);
        Ok(test_yaml)
    }
}

impl PersistableConfig for OverrideNodeConfig {
    fn load_config<P: AsRef<Path>>(path: P) -> Result<Self, Error> {
        let config = NodeConfig::load_config(path)?;
        Ok(Self::new_with_default_base(config))
    }

    fn save_config<P: AsRef<Path>>(&self, output_file: P) -> Result<(), Error> {
        let yaml_value = self.get_yaml()?;
        let yaml_string = serde_yaml::to_string(&yaml_value).map_err(|e| {
            Error::Yaml(
                "Unable to serialize override config to yaml. Error: {}".to_string(),
                e,
            )
        })?;
        let yaml_bytes = yaml_string.as_bytes().to_vec();
        Self::write_file(yaml_bytes, output_file)
    }
}

#[cfg(test)]
mod test {
    use crate::config::{
        NetworkConfig, NodeConfig, OverrideNodeConfig, PersistableConfig, WaypointConfig,
    };
    use std::{env::temp_dir, path::PathBuf};

    #[test]
    fn test_override_node_config_no_diff() {
        let override_config = OverrideNodeConfig::new(NodeConfig::default(), NodeConfig::default());
        let diff_yaml = override_config.get_yaml().unwrap();
        assert_eq!(
            diff_yaml,
            serde_yaml::Value::Mapping(serde_yaml::Mapping::new())
        );
    }

    #[test]
    fn test_override_node_config_with_bool() {
        let mut override_config =
            OverrideNodeConfig::new(NodeConfig::default(), NodeConfig::default());
        let config = override_config.override_config_mut();
        config.api.enabled = false;

        let diff_yaml = override_config.get_yaml().unwrap();
        let expected_yaml: serde_yaml::Value = serde_yaml::from_str(
            r#"
            api:
                enabled: false
            "#,
        )
        .unwrap();
        assert_eq!(diff_yaml, expected_yaml);
    }

    #[test]
    fn test_override_node_config_with_enum() {
        let mut override_config =
            OverrideNodeConfig::new(NodeConfig::default(), NodeConfig::default());
        let config = override_config.override_config_mut();
        config.base.waypoint = WaypointConfig::FromFile(PathBuf::from("test"));

        let diff_yaml = override_config.get_yaml().unwrap();
        let expected_yaml: serde_yaml::Value = serde_yaml::from_str(
            r#"
            base:
                waypoint:
                    from_file: test
            "#,
        )
        .unwrap();
        assert_eq!(diff_yaml, expected_yaml);
    }

    fn assert_equal_yaml(actual: serde_yaml::Value, expected: serde_yaml::Value) {
        // TODO: why don't the actual Values match, only matches with as_str?
        assert_eq!(actual.as_str(), expected.as_str());
    }

    #[test]
    fn test_override_node_config_with_empty_base_vector() {
        let mut override_config =
            OverrideNodeConfig::new(NodeConfig::default(), NodeConfig::default());
        let config = override_config.override_config_mut();
        config.full_node_networks.push(Default::default());
        config.full_node_networks.push(Default::default());

        let diff_yaml = override_config.get_yaml().unwrap();
        let default_node_config = serde_yaml::to_value(&NetworkConfig::default()).unwrap();
        let mut expected_yaml: serde_yaml::Value = serde_yaml::Value::Null;
        expected_yaml["full_node_networks"] =
            serde_yaml::Value::Sequence(vec![default_node_config.clone(), default_node_config]);
        assert_equal_yaml(diff_yaml, expected_yaml);
    }

    #[test]
    fn test_override_node_config_with_non_empty_base_vector() {
        let mut override_config =
            OverrideNodeConfig::new(NodeConfig::default(), NodeConfig::default());
        let config = override_config.override_config_mut();
        config.full_node_networks.push(Default::default());
        config.full_node_networks.push(Default::default());
        let base = override_config.base_config_mut();
        base.full_node_networks.push(Default::default());

        // Note, the diff will include the entire vector, not just the non-equal elements
        let diff_yaml = override_config.get_yaml().unwrap();
        let default_node_config = serde_yaml::to_value(&NetworkConfig::default()).unwrap();
        let mut expected_yaml: serde_yaml::Value = serde_yaml::Value::Null;
        expected_yaml["full_node_networks"] =
            serde_yaml::Value::Sequence(vec![default_node_config.clone(), default_node_config]);
        assert_equal_yaml(diff_yaml, expected_yaml);
    }

    #[test]
    fn test_override_node_config_with_base_change() {
        let mut override_config =
            OverrideNodeConfig::new(NodeConfig::default(), NodeConfig::default());
        let base = override_config.base_config_mut();
        base.api.enabled = false;

        let diff_yaml = override_config.get_yaml().unwrap();
        let expected_yaml: serde_yaml::Value = serde_yaml::from_str(
            r#"
            api:
                enabled: true
            "#,
        )
        .unwrap();
        assert_eq!(diff_yaml, expected_yaml);
    }

    #[test]
    fn test_override_config_load_save() {
        let mut override_config =
            OverrideNodeConfig::new(NodeConfig::default(), NodeConfig::default());
        let config = override_config.override_config_mut();
        config.api.enabled = false;

        let temp_file = temp_dir().join("override_config.yaml");
        override_config.save_config(temp_file.as_path()).unwrap();
        let loaded_config = OverrideNodeConfig::load_config(temp_file.as_path()).unwrap();
        assert_eq!(override_config, loaded_config);
    }
}
