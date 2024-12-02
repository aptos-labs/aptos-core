// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::loaded_data::runtime_types::{AbilityInfo, Type};
use itertools::Itertools;
use std::fmt;

#[derive(Debug, Clone, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct FunctionType {
    arg_tys: Vec<Type>,
    return_tys: Vec<Type>,
    ability: AbilityInfo,
    // TODO(LAMBDA): visibility?
}

impl FunctionType {
    pub(crate) fn new(arg_tys: Vec<Type>, return_tys: Vec<Type>, ability: AbilityInfo) -> Self {
        Self {
            arg_tys,
            return_tys,
            ability,
        }
    }

    pub fn arg_tys(&self) -> &[Type] {
        &self.arg_tys
    }

    pub fn return_tys(&self) -> &[Type] {
        &self.return_tys
    }

    pub fn ability(&self) -> &AbilityInfo {
        &self.ability
    }
}

impl fmt::Display for FunctionType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let arg_tys = self.arg_tys.iter().map(|t| t.to_string()).join(",");
        let return_tys = self.return_tys.iter().map(|t| t.to_string()).join(",");
        write!(f, "fun ({}) -> ({})", arg_tys, return_tys)
    }
}
