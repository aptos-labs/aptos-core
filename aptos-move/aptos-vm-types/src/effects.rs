// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// use std::fmt::{Debug, Formatter, Result};

use anyhow::bail;
use move_binary_format::errors::PartialVMResult;

/// Represents a write op at the VM level.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Op<T> {
    // TODO: Add variants with metadata.
    Creation(T),
    Modification(T),
    Deletion,
}

impl<T> Op<T> {
    pub fn squash(&mut self, op: Self) -> anyhow::Result<bool> {
        match (&self, op) {
            (Self::Deletion, Self::Creation(data)) => {
                *self = Self::Creation(data);
            },
            (Self::Deletion, Self::Deletion) => bail!("Cannot delete already deleted data"),
            (Self::Deletion, Self::Modification(_)) => bail!("Cannot modify already deleted data"),
            (Self::Creation(_) | Self::Modification(_), Self::Creation(_)) => {
                bail!("Cannot create already created data")
            },
            (Self::Creation(_), Self::Deletion) => return Ok(false),
            (Self::Creation(_) | Self::Modification(_), Self::Modification(data)) => {
                *self = Self::Modification(data);
            },
            (Self::Modification(_), Self::Deletion) => {
                *self = Self::Deletion;
            },
        }
        Ok(true)
    }
}
