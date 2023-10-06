// Copyright (c) The Diem Core Contributors
// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::{
    account_address::AccountAddress,
    identifier::Identifier,
    language_storage::{StructTag, TypeTag},
};
use serde::de::DeserializeOwned;

pub trait MoveStructType {
    const ADDRESS: AccountAddress = crate::language_storage::CORE_CODE_ADDRESS;
    const MODULE_NAME: &'static str;
    const STRUCT_NAME: &'static str;

    fn module_identifier() -> Identifier {
        Self::MODULE_NAME.into()
    }

    fn struct_identifier() -> Identifier {
        Self::STRUCT_NAME.into()
    }

    fn type_params() -> Vec<TypeTag> {
        vec![]
    }

    fn struct_tag() -> StructTag {
        StructTag {
            address: Self::ADDRESS,
            name: Self::struct_identifier(),
            module: Self::module_identifier(),
            type_params: Self::type_params(),
        }
    }
}

pub trait MoveResource: MoveStructType + DeserializeOwned {
    fn resource_path() -> Vec<u8> {
        Self::struct_tag().access_vector()
    }
}
