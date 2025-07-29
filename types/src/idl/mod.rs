// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

//! JSON IDL (Interface Definition Language) tools for cross-language serialization
//! 
//! This module provides tools for serializing and deserializing Aptos types to/from JSON
//! format that can be used across different programming languages and libraries.
//! 
//! ## Usage Example
//! 
//! ```rust
//! use aptos_types::idl::{ValidatorInfoIdl, ValidatorInfoIdlError};
//! 
//! // Serialize ValidatorInfo to JSON
//! let validator_info = /* your ValidatorInfo instance */;
//! let json_string = validator_info.to_json_idl()?;
//! 
//! // Deserialize from JSON
//! let deserialized = ValidatorInfo::from_json_idl(&json_string)?;
//! ```

pub mod validator_info;
pub mod error;
pub mod api_types_converter;

pub use validator_info::*;
pub use error::*;
pub use api_types_converter::*; 