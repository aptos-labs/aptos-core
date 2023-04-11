// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::config::{Error, NodeConfig, SafetyRulesConfig};
use serde::{de::DeserializeOwned, Serialize};
use std::{
    fs::File,
    io::{Read, Write},
    path::Path,
};

pub trait PersistableConfig: Serialize + DeserializeOwned {
    fn load_config<P: AsRef<Path>>(path: P) -> Result<Self, Error> {
        // Open the file and read it into a string
        let config_path_string = path.as_ref().to_str().unwrap().to_string();
        let mut file = File::open(&path).map_err(|error| {
            Error::Unexpected(format!(
                "Failed to open config file: {:?}. Error: {:?}",
                config_path_string, error
            ))
        })?;
        let mut contents = String::new();
        file.read_to_string(&mut contents).map_err(|error| {
            Error::Unexpected(format!(
                "Failed to read the config file into a string: {:?}. Error: {:?}",
                config_path_string, error
            ))
        })?;

        // Parse the file string
        Self::parse(&contents)
    }

    fn save_config<P: AsRef<Path>>(&self, output_file: P) -> Result<(), Error> {
        let contents = serde_yaml::to_vec(&self)
            .map_err(|e| Error::Yaml(output_file.as_ref().to_str().unwrap().to_string(), e))?;
        let mut file = File::create(output_file.as_ref())
            .map_err(|e| Error::IO(output_file.as_ref().to_str().unwrap().to_string(), e))?;
        file.write_all(&contents)
            .map_err(|e| Error::IO(output_file.as_ref().to_str().unwrap().to_string(), e))?;
        Ok(())
    }

    fn parse(serialized: &str) -> Result<Self, Error> {
        serde_yaml::from_str(serialized).map_err(|e| Error::Yaml("config".to_string(), e))
    }
}

// We only implement PersistableConfig for the configs that should be read/written to disk
impl PersistableConfig for NodeConfig {}
impl PersistableConfig for SafetyRulesConfig {}
