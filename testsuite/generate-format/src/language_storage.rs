// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

use move_core_types::{account_address::AccountAddress, identifier::Identifier};
use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};

#[derive(Serialize, Deserialize, Debug, PartialEq, Hash, Eq, Clone, PartialOrd, Ord)]
pub enum TypeTag {
    Bool,
    U8,
    U64,
    U128,
    Address,
    Signer,
    Vector(Box<TypeTag>),
    Struct(StructTag),
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Hash, Eq, Clone, PartialOrd, Ord)]
pub struct StructTag {
    pub address: AccountAddress,
    pub module: Identifier,
    pub name: Identifier,
    pub type_params: Vec<TypeTag>,
}

impl Display for StructTag {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        write!(
            f,
            "0x{}::{}::{}",
            self.address.short_str_lossless(),
            self.module,
            self.name
        )?;
        if let Some(first_ty) = self.type_params.first() {
            write!(f, "<")?;
            write!(f, "{}", first_ty)?;
            for ty in self.type_params.iter().skip(1) {
                write!(f, ", {}", ty)?;
            }
            write!(f, ">")?;
        }
        Ok(())
    }
}

impl Display for TypeTag {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        match self {
            TypeTag::Struct(s) => write!(f, "{}", s),
            TypeTag::Vector(ty) => write!(f, "vector<{}>", ty),
            TypeTag::U8 => write!(f, "U8"),
            TypeTag::U64 => write!(f, "U64"),
            TypeTag::U128 => write!(f, "U128"),
            TypeTag::Address => write!(f, "Address"),
            TypeTag::Signer => write!(f, "Signer"),
            TypeTag::Bool => write!(f, "Bool"),
        }
    }
}

impl From<StructTag> for TypeTag {
    fn from(t: StructTag) -> TypeTag {
        TypeTag::Struct(t)
    }
}
