// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use std::borrow::Borrow;
use std::hash::{Hash, Hasher};

use super::runtime_types::{Type, StructInstantiationIndex};

pub(crate) trait KeyPair {
    /// Obtains the first element of the pair.
    fn struct_idx(&self) -> &StructInstantiationIndex;
    /// Obtains the second element of the pair.
    fn ty_args(&self) -> &[Type];
}

impl<'a> Borrow<dyn KeyPair + 'a> for (StructInstantiationIndex, Vec<Type>) {
    fn borrow(&self) -> &(dyn KeyPair + 'a) {
        self
    }
}

impl Hash for dyn KeyPair + '_ {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.struct_idx().hash(state);
        self.ty_args().hash(state);
    }
}

impl PartialEq for dyn KeyPair + '_ {
    fn eq(&self, other: &Self) -> bool {
        self.struct_idx() == other.struct_idx() && self.ty_args() == other.ty_args()
    }
}

impl Eq for dyn KeyPair + '_ {}

impl KeyPair for (StructInstantiationIndex, Vec<Type>) {
    fn struct_idx(&self) -> &StructInstantiationIndex {
        &self.0
    }
    fn ty_args(&self) -> &[Type] {
        &self.1
    }
}
impl KeyPair for (&StructInstantiationIndex, &[Type]) {
    fn struct_idx(&self) -> &StructInstantiationIndex {
        self.0
    }
    fn ty_args(&self) -> &[Type] {
        self.1
    }
}
