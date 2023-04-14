// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::path::{parse_path, PathComponent};
use anyhow::{bail, Context, Result};
use aptos_api_types::MoveType;
use move_core_types::{identifier::Identifier, language_storage::ModuleId};
use std::collections::BTreeMap;

/// [`TypeAccessor`] is a utility for looking up the types of fields in Move data at runtime.
///
#[derive(Clone, Debug)]
pub struct TypeAccessor {
    /// This is a map of ModuleId (address, name) to a map of struct name to a map of field name to field type.
    field_info: BTreeMap<
        // Module address + name.
        ModuleId,
        BTreeMap<
            // Struct name.
            Identifier,
            BTreeMap<
                // Field name.
                Identifier,
                // Field type.
                MoveType,
            >,
        >,
    >,
}

impl TypeAccessor {
    /// Create a new [`TypeAccessor`]
    ///
    /// **Note**: You should not use this function directly but rather make use of a
    /// builder. See the [crate level documentation](crate) for more information about builders.
    pub fn new(
        field_info: BTreeMap<ModuleId, BTreeMap<Identifier, BTreeMap<Identifier, MoveType>>>,
    ) -> Self {
        Self { field_info }
    }

    /// Look up the type of a field at a given path in a struct. You can see examples
    /// of how to use this function in the [crate level documentation](crate).
    pub fn get_type(
        &self,
        module_id: &ModuleId,
        struct_name: &Identifier,
        path: &str,
    ) -> Result<&MoveType> {
        self.get_type_structured(
            module_id,
            struct_name,
            parse_path(path)
                .with_context(|| format!("Failed to parse path {}", path))?
                .as_slice(),
        )
        .with_context(|| {
            format!(
                "Failed to get type at path {} of struct {} in module {}",
                path, struct_name, module_id
            )
        })
    }

    pub fn get_type_structured(
        &self,
        module_id: &ModuleId,
        struct_name: &Identifier,
        path: &[PathComponent],
    ) -> Result<&MoveType> {
        if path.is_empty() {
            bail!("Path cannot be empty");
        }

        // Get the type of the first field of the requested struct.
        let fields = self.get_fields(module_id, struct_name)?;
        let mut typ = match &path[0] {
            PathComponent::Field(field_name) => fields.get(field_name).with_context(|| {
                format!(
                    "Could not find top level field {} in struct {} in module {}",
                    field_name, struct_name, module_id
                )
            })?,
            _ => {
                bail!("First component of path must be a field");
            },
        };

        for (i, component) in path[1..].iter().enumerate() {
            match component {
                PathComponent::Field(field_name) => {
                    if let MoveType::Struct(struct_tag) = typ {
                        let fields =
                            self.get_fields(&struct_tag.module_id(), &struct_tag.name.0)?;
                        typ = fields.get(field_name).with_context(|| {
                            format!(
                                "Could not find field {} in struct {} in module {}",
                                field_name, struct_name, module_id
                            )
                        })?;
                    } else {
                        bail!(
                            "Tried to access field {} of non-struct type {} at path {:?}",
                            field_name,
                            typ,
                            &path[..i]
                        );
                    }
                },
                PathComponent::GenericTypeParamIndex(index) => {
                    if let MoveType::Struct(struct_tag) = typ {
                        typ = struct_tag
                            .generic_type_params
                            .get(*index as usize)
                            .with_context(|| {
                                format!(
                                    "Tried to access generic type param at index {} but there are only {} generic type params",
                                    index, struct_tag.generic_type_params.len()
                                )
                            })?;
                    } else {
                        bail!(
                            "Tried to access generic type param of non-struct type {} at path {:?}",
                            typ,
                            &path[..i]
                        );
                    }
                },
                PathComponent::EnterArray => {
                    if let MoveType::Vector { items: inner_typ } = typ {
                        typ = inner_typ;
                    } else {
                        bail!(
                            "Tried to access array element of non-array type {} at path {:?}",
                            typ,
                            &path[..i]
                        );
                    }
                },
            }
        }

        Ok(typ)
    }

    fn get_fields(
        &self,
        module_id: &ModuleId,
        struct_name: &Identifier,
    ) -> Result<&BTreeMap<Identifier, MoveType>> {
        let structs = self
            .field_info
            .get(module_id)
            .with_context(|| format!("Could not find module {}", module_id))?;
        let fields = structs.get(struct_name).with_context(|| {
            format!(
                "Could not find struct {} in module {}",
                struct_name, module_id
            )
        })?;
        Ok(fields)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        builder::{LocalTypeAccessorBuilder, TypeAccessorBuilderTrait},
        test_helpers::compile_package,
    };
    use aptos_types::account_address::AccountAddress;
    use std::path::PathBuf;

    #[tokio::test]
    async fn test_food_get_type() -> Result<()> {
        // Compile the food package and all the packages we know it recursively depends on.
        let mut modules = Vec::new();
        for name in &["aptos-framework", "aptos-stdlib", "move-stdlib"] {
            let path = PathBuf::try_from(format!("../../aptos-move/framework/{}", name)).unwrap();
            modules.extend(compile_package(path)?);
        }
        modules.extend(compile_package(PathBuf::try_from("move/food").unwrap())?);

        // Build the TypeAccessor.
        let type_accessor = LocalTypeAccessorBuilder::new()
            .add_modules(modules)
            .build()
            .context("Failed to build TypeAccessor with all the modules")?;

        let module_name = "food";
        let module_id = ModuleId::new(AccountAddress::TWO, Identifier::new(module_name).unwrap());
        let struct_name = Identifier::new("FruitManager").unwrap();

        // Confirm that we can access a leaf type.
        assert_eq!(
            type_accessor
                .get_type(&module_id, &struct_name, "fruit_inventory.handle")
                .context("Failed to get type")?,
            &MoveType::Address,
        );

        // Confirm that we can access a leaf type with a generic type param in the
        // path.
        assert_eq!(
            type_accessor
                .get_type(&module_id, &struct_name, "fruit_inventory.1.color.red")
                .context("Failed to get type")?,
            &MoveType::U8,
        );

        // Confirm that we can access a type with a generic type param at the end of
        // the path.
        let fruit_inventory_value_type = type_accessor
            .get_type(&module_id, &struct_name, "fruit_inventory.1")
            .context("Failed to get type")?;
        match fruit_inventory_value_type {
            MoveType::Struct(struct_tag) => {
                assert_eq!(struct_tag.address.inner(), &AccountAddress::TWO);
                assert_eq!(struct_tag.module.0.as_str(), module_name);
                assert_eq!(struct_tag.name.0.as_str(), "Fruit");
            },
            _ => bail!("Expected type to be a struct"),
        }

        // Confirm that we can access the type of a top level field in the struct.
        assert_eq!(
            type_accessor
                .get_type(&module_id, &struct_name, "last_sale_time")
                .context("Failed to get type")?,
            &MoveType::U64,
        );

        // Confirm that we can access the type of a vector.
        assert!(matches!(
            type_accessor
                .get_type(&module_id, &struct_name, "authorized_buyers")
                .context("Failed to get type")?,
            &MoveType::Vector { items: _ }
        ));

        // Confirm that we can access the type of the thing inside a vector.
        assert!(matches!(
            type_accessor
                .get_type(&module_id, &struct_name, "authorized_buyers.[]")
                .context("Failed to get type")?,
            &MoveType::Struct(_)
        ));

        // Confirm that we can access a field of a type of the thing inside a vector.
        assert!(matches!(
            type_accessor
                .get_type(&module_id, &struct_name, "authorized_buyers.[].address")
                .context("Failed to get type")?,
            &MoveType::Address
        ));

        // Confirm that using an empty path is an error.
        assert!(type_accessor
            .get_type(&module_id, &struct_name, "")
            .is_err());

        // Confirm that it is an error to try access a field that doesn't exist.
        assert!(type_accessor
            .get_type(&module_id, &struct_name, "fruit_inventory.1.color.orange")
            .is_err());

        // Confirm that it is an error to keep trying to access fields when the
        // type is not a struct.
        assert!(type_accessor
            .get_type(
                &module_id,
                &struct_name,
                "fruit_inventory.1.color.red.something"
            )
            .is_err());

        // Confirm that it is an error to try to use a generic path param as the first
        // part of the path.
        assert!(type_accessor
            .get_type(&module_id, &struct_name, "1.color.red.something")
            .is_err());

        Ok(())
    }
}
