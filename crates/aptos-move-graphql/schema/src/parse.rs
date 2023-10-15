// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{common::BuilderOptions, discover::MoveStructWithModuleId};
use anyhow::{bail, Context, Result};
use aptos_api_types::MoveType;
use aptos_move_graphql_scalars::{Address, Any, TypeName, U128, U16, U256, U32, U64, U8};
use async_graphql::dynamic::{Field, FieldFuture, Object, TypeRef};
use move_core_types::language_storage::{StructTag, CORE_CODE_ADDRESS};
use std::collections::HashSet;

// Type names that are reserved by GraphQL.
pub const RESERVED_TYPE_NAMES: &[&str] = &[
    TypeRef::INT,
    TypeRef::FLOAT,
    TypeRef::STRING,
    TypeRef::BOOLEAN,
    TypeRef::ID,
];

/// This function takes in a Vec of structs + information about which module they came
/// from and returns a vec of GraphQL objects that can be used to build a schema. It
/// is essential that `structs` and `repeated_struct_names` are in sync (came from the
/// same set of modules) otherwise schema building might fail due to name collision.
pub fn parse_structs(
    structs: Vec<MoveStructWithModuleId>,
    repeated_struct_names: &HashSet<String>,
    options: &BuilderOptions,
) -> Result<Vec<Object>> {
    let mut objects = Vec::new();

    // For each struct in the module build an Object to include in the schema.
    for struc in structs {
        let mut types_to_resolve = Vec::new();

        let struct_tag = struc.struct_tag();

        let mut object = Object::new(get_object_name(&struct_tag, repeated_struct_names, options));

        for field in struc.struc.fields {
            types_to_resolve.push(field.typ.clone());
            let field_type = move_type_to_field_type(&field.typ, repeated_struct_names, options)
                .with_context(|| {
                    format!(
                        "Failed to parse field {} of struct {}",
                        field.name, struct_tag.name,
                    )
                })?;
            // TODO: When we have an enhanced ABI with comments set Field.description.
            let field = Field::new(
                field.name.to_string(),
                field_type,
                // The resolved value doesn't matter. These Fields will be used to
                // build an Object that we feed into a Schema only for the puspose of
                // getting a schema file. We won't ever execute queries against this
                // directly.
                move |_| FieldFuture::new(async move { Ok(Some(())) }),
            );
            object = object.field(field);
        }

        objects.push(object);
    }

    Ok(objects)
}

/// This function takes a Move type and returns the corresponding GraphQL type. GraphQL
/// nullability is quite interesting when it comes to vectors, for example you can have
/// a non nullable vec with nullable values. Make sure to read up on GraphQL
/// nullability before modifying this function.
///
/// This function has a variety of special behavior for certain Move types. Except for
/// the special string handling, which is always enabled, everything else is
/// configurable. For example, this function can "unpack" Options and represent them as
/// nullable values of the inner type, rather than a struct with a vec inside it.
///
/// Any type that can be unpacked into something more easily consumable by a client
/// should be handled here. Handling it here is only part of the picture though, the
/// server must return the resource in a way that aligns with the configured client
/// behavior.
///
/// The `repeated_struct_names` argument is used to determine whether to use the fully
/// qualified name (address, module, struct name) or not (just struct name). See the
/// `get_object_name` function to learn more about how we make this determination.
pub fn move_type_to_field_type(
    field_type: &MoveType,
    repeated_struct_names: &HashSet<String>,
    options: &BuilderOptions,
) -> Result<TypeRef> {
    match field_type {
        MoveType::Bool => Ok(TypeRef::NonNull(Box::new(TypeRef::Named(
            TypeRef::BOOLEAN.into(),
        )))),
        // You'll see that we use custom scalar types in the schema for these types.
        // This doesn't directly affect the way we encode the responses. Indeed, we
        // encode u8, u16, and u32 as ints and u64, u128, and u256 as strings in
        // the messages over the wire. It is then up to the client to choose how
        // to interpret these values. For Rust, aptos-move-graphql-scalars can be
        // used to correctly handle these values.
        MoveType::U8 => Ok(TypeRef::NonNull(Box::new(TypeRef::Named(
            std::borrow::Cow::Borrowed(U8::type_name()),
        )))),
        MoveType::U16 => Ok(TypeRef::NonNull(Box::new(TypeRef::Named(
            std::borrow::Cow::Borrowed(U16::type_name()),
        )))),
        MoveType::U32 => Ok(TypeRef::NonNull(Box::new(TypeRef::Named(
            std::borrow::Cow::Borrowed(U32::type_name()),
        )))),
        MoveType::U64 => Ok(TypeRef::NonNull(Box::new(TypeRef::Named(
            std::borrow::Cow::Borrowed(U64::type_name()),
        )))),
        MoveType::U128 => Ok(TypeRef::NonNull(Box::new(TypeRef::Named(
            std::borrow::Cow::Borrowed(U128::type_name()),
        )))),
        MoveType::U256 => Ok(TypeRef::NonNull(Box::new(TypeRef::Named(
            std::borrow::Cow::Borrowed(U256::type_name()),
        )))),
        MoveType::Address => Ok(TypeRef::NonNull(Box::new(TypeRef::Named(
            std::borrow::Cow::Borrowed(Address::type_name()),
        )))),
        // TODO: Do we want special behavior for vectors of u8? What about other byte
        // vectors? Okay yeah I know for vector<u8> at the least I want to represent
        // this a different way. Sort of hard to differentiate between byte data and
        // someone just wanting to store a small vec of u8 though, I wish we had some
        // kind of bytes wrapper type. I can bring this up, though it's ofc too late
        // in most cases.
        MoveType::Vector { items: move_type } => {
            Ok(TypeRef::NonNull(Box::new(TypeRef::List(Box::new(
                move_type_to_field_type(move_type, repeated_struct_names, options)?,
            )))))
        },
        MoveType::Struct(struct_tag) => {
            // We have special handling for the following:
            //   - Strings
            //   - Options
            //
            // TODO: We should have special handling for the following as well:
            //   - FixedPoint32,
            //   - FixedPoint64
            //   - Aggregator
            let struct_tag = StructTag::try_from(struct_tag.clone())
                .context("Unexpectedly failed to build StructTag")?;
            if struct_tag.is_std_string(&CORE_CODE_ADDRESS) {
                // We "unwrap" the string::String and just represent it as a string in
                // the schema. The value builder will do the same, pulling the bytes
                // out from the String and returning them as a normal UTF-8 string.
                Ok(TypeRef::NonNull(Box::new(TypeRef::Named(
                    TypeRef::STRING.into(),
                ))))
            } else if options.use_special_handling_for_option
                && struct_tag.is_std_option(&CORE_CODE_ADDRESS)
            {
                // Extract the inner type of the Option and get the type of that.
                // Because for all other Move types we return them as non-nullable we
                // pull out the inner type and return just that, to indicate it could
                // possibly be null.
                let type_tag = struct_tag.type_params.into_iter().next().context(
                    "Option unexpectedly had no generic type params, this should be impossible",
                )?;
                let move_type = MoveType::from(type_tag);
                let field_type =
                    move_type_to_field_type(&move_type, repeated_struct_names, options)?;
                // There is no great way to represent Option<Option<T>> in a GraphQL
                // schema. Theoretically we could add some artifical struct like `inner`
                // to store the inner option, but in reality this pattern never gets
                // used. Indeed, at the time of writing no Move code in aptos-move uses
                // Option<Option<T>>. So we choose not to handle it.
                if let TypeRef::NonNull(field_type) = field_type {
                    Ok(*field_type)
                } else {
                    Err(anyhow::anyhow!(
                        "Expected non-null type for Option inner type but got: {:?}. \
                            Likely this means you have an Option<Option<T>>. The schema \
                            generator does not support this.",
                        field_type
                    ))
                }
            } else {
                // TODO: This needs to take generics into account.
                Ok(TypeRef::NonNull(Box::new(TypeRef::Named(
                    get_object_name(&struct_tag, repeated_struct_names, options).into(),
                ))))
            }
        },
        MoveType::GenericTypeParam { index: _ } => {
            // TODO: Currently we're pretty much just declaring bankruptcy on generics.
            // For example, if something uses a SimpleMap, the key and value will just
            // be represented as Any even though they have actual types.
            Ok(TypeRef::NonNull(Box::new(TypeRef::Named(
                std::borrow::Cow::Borrowed(Any::type_name()),
            ))))
        },
        // These types cannot appear in structs that we read from storage:
        //   - Signer is not store
        //   - References aren't store.
        //   - Unparseable is only used on the input side
        MoveType::Signer | MoveType::Reference { mutable: _, to: _ } | MoveType::Unparsable(_) => {
            bail!(
                "Type {:?} should not appear in a struct from storage",
                field_type
            )
        },
    }
}

/// Based on whether the struct name is repeated and the builder options, determine
/// whether to use the fully qualified name (address, module, struct name) or not
/// (just struct name).
fn get_object_name(
    struct_tag: &StructTag,
    repeated_struct_names: &HashSet<String>,
    options: &BuilderOptions,
) -> String {
    let struct_name = struct_tag.name.to_string();
    let use_fully_qualified_name = options.always_use_fully_qualifed_names
        || repeated_struct_names.contains(&struct_name)
        || RESERVED_TYPE_NAMES.contains(&struct_name.as_str());
    if use_fully_qualified_name {
        get_fully_qualified_object_name(struct_tag)
    } else {
        struct_name
    }
}

// TODO: It'd be good to use the named address instead of the raw address. Not a high
// priority though since we only used the fully qualified name when there is a name
// collision, which is fairly rare.
fn get_fully_qualified_object_name(struct_tag: &StructTag) -> String {
    format!(
        "_{}__{}__{}",
        struct_tag.address.to_standard_string(),
        struct_tag.module,
        struct_tag.name,
    )
}

#[cfg(test)]
mod test {
    use super::*;
    use anyhow::Result;
    use aptos_api_types::{Address, MoveStructTag};
    use move_core_types::identifier::Identifier;
    use std::str::FromStr;

    fn build_function_options() -> BuilderOptions {
        BuilderOptions {
            use_special_handling_for_option: true,
            always_use_fully_qualifed_names: false,
        }
    }

    /// This function builds the following Move type:
    ///
    /// Option<T>
    ///
    /// In GraphQL schema syntax this is represented as `T`
    fn build_option(inner: MoveType) -> MoveType {
        MoveType::Struct(MoveStructTag {
            address: Address::from_str("0x1").unwrap(),
            module: Identifier::new("option").unwrap().into(),
            name: Identifier::new("Option").unwrap().into(),
            generic_type_params: vec![inner],
        })
    }

    /// This function builds the following Move type:
    ///
    /// vector<u32>
    ///
    /// This is a mandatory vector filled with mandatory u32s.
    ///
    /// In GraphQL schema syntax this is represented as `[Int!]!`
    fn build_vec_of_u32s() -> MoveType {
        MoveType::Vector {
            items: Box::new(MoveType::U32),
        }
    }

    /// This function builds the following Move type:
    ///
    /// vector<vector<u32>>
    ///
    /// This is a mandatory vector filled with mandatory vectors filled with u32s.
    ///
    /// In GraphQL schema syntax this is represented as `[[Int!]!]!`
    fn build_vec_of_vecs_of_u32s() -> MoveType {
        MoveType::Vector {
            items: Box::new(build_vec_of_u32s()),
        }
    }

    /// This function builds the following Move type:
    ///
    /// vector<Option<vector<u32>>>
    ///
    /// So, it's a mandatory vector containing optional vectors filled with
    /// mandatory u32s.
    ///
    /// In GraphQL schema syntax this is represented as `[[Int!]]!`
    fn build_vec_of_optional_vecs_of_u32s() -> MoveType {
        MoveType::Vector {
            items: Box::new(build_option(build_vec_of_u32s())),
        }
    }

    /// This function builds the following Move type:
    ///
    /// Option<vector<vector<Option<vector<u32>>>>>,
    ///
    /// So, it's an optional vector containing non-optional vectors filled with
    /// optional vectors of mandatory u32s.
    ///
    /// In GraphQL schema syntax this is represented as `[[[Int!]]!]`
    fn build_complex_type() -> MoveType {
        build_option(MoveType::Vector {
            items: Box::new(build_vec_of_optional_vecs_of_u32s()),
        })
    }

    #[test]
    fn test_option() -> Result<()> {
        let options = build_function_options();
        let testing_move_type = build_option(MoveType::U32);
        let field_type = move_type_to_field_type(&testing_move_type, &HashSet::new(), &options)?;
        assert_eq!(&field_type.to_string(), "U32");
        Ok(())
    }

    /// See the comment in move_type_to_field_type for an explanation for why we
    /// expect this to fail.
    #[test]
    fn test_option_of_option() -> Result<()> {
        let options = build_function_options();
        let testing_move_type = build_option(build_option(MoveType::U16));
        assert!(move_type_to_field_type(&testing_move_type, &HashSet::new(), &options).is_err());
        Ok(())
    }

    #[test]
    fn test_vec_of_u32s() -> Result<()> {
        let options = build_function_options();
        let testing_move_type = build_vec_of_u32s();
        let field_type = move_type_to_field_type(&testing_move_type, &HashSet::new(), &options)?;
        assert_eq!(&field_type.to_string(), "[U32!]!");
        Ok(())
    }

    #[test]
    fn test_vec_of_vecs_of_u32s() -> Result<()> {
        let options = build_function_options();
        let testing_move_type = build_vec_of_vecs_of_u32s();
        let field_type = move_type_to_field_type(&testing_move_type, &HashSet::new(), &options)?;
        assert_eq!(&field_type.to_string(), "[[U32!]!]!");
        Ok(())
    }

    #[test]
    fn test_vec_of_optional_vecs_of_u32s() -> Result<()> {
        let options = build_function_options();
        let testing_move_type = build_vec_of_optional_vecs_of_u32s();
        let field_type = move_type_to_field_type(&testing_move_type, &HashSet::new(), &options)?;
        assert_eq!(&field_type.to_string(), "[[U32!]]!");
        Ok(())
    }

    #[test]
    fn test_complex_move_type() -> Result<()> {
        let options = build_function_options();
        let testing_move_type = build_complex_type();
        let field_type = move_type_to_field_type(&testing_move_type, &HashSet::new(), &options)?;
        assert_eq!(&field_type.to_string(), "[[[U32!]]!]");
        Ok(())
    }
}
