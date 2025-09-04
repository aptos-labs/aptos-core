// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::config::{Error, NodeConfig, SafetyRulesConfig};
use serde::{de::DeserializeOwned, Serialize};
use std::{
    fs::{read_to_string, File},
    io::Write,
    path::Path,
};

pub trait PersistableConfig: Serialize + DeserializeOwned {
    /// Load the config from disk at the given path
    fn load_config<P: AsRef<Path>>(path: P) -> Result<Self, Error> {
        // Read the file into a string
        let file_contents = Self::read_config_file(&path)?;

        // Parse the file string
        Self::parse_serialized_config(&file_contents)
    }

    /// Save the config to disk at the given output path
    fn save_config<P: AsRef<Path>>(&self, output_file: P) -> Result<(), Error> {
        // Serialize the config to a string
        let serialized_config = serde_yaml::to_vec(&self)
            .map_err(|e| Error::Yaml(output_file.as_ref().to_str().unwrap().to_string(), e))?;

        Self::write_file(serialized_config, output_file)
    }

    /// Read the config at the given path and return the contents as a string
    fn read_config_file<P: AsRef<Path>>(path: P) -> Result<String, Error> {
        let config_path_string = path.as_ref().to_str().unwrap().to_string();
        read_to_string(config_path_string.clone()).map_err(|error| {
            Error::Unexpected(format!(
                "Failed to read the config file into a string: {:?}. Error: {:?}",
                config_path_string, error
            ))
        })
    }

    /// Create the file and write the serialized config to the file
    fn write_file<P: AsRef<Path>>(serialized_config: Vec<u8>, output_file: P) -> Result<(), Error> {
        let mut file = File::create(output_file.as_ref())
            .map_err(|e| Error::IO(output_file.as_ref().to_str().unwrap().to_string(), e))?;
        file.write_all(&serialized_config)
            .map_err(|e| Error::IO(output_file.as_ref().to_str().unwrap().to_string(), e))?;

        Ok(())
    }

    /// Parse the config from the serialized string
    fn parse_serialized_config(serialized_config: &str) -> Result<Self, Error> {
        serde_yaml::from_str(serialized_config).map_err(|e| Error::Yaml("config".to_string(), e))
    }
}

// We only implement PersistableConfig for the configs that should be read/written to disk
impl PersistableConfig for NodeConfig {}
impl PersistableConfig for SafetyRulesConfig {}
