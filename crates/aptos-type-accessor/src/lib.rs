// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! # Aptos Type Accessor
//!
//! This crate provides the [`TypeAccessor`], which allows you to query the types of fields
//! in Move resources at runtime. This is best explained with an example.
//!
//! Say you have this Move module:
//!
//! ```move
//! module addr::food {
//!     use aptos_std::simple_map::SimpleMap;
//!     use aptos_std::table::Table;
//!     use std::string::String;
//!
//!     struct Color has store {
//!         red: u8,
//!         blue: u8,
//!         green: u8,
//!     }
//!
//!     struct Fruit has store {
//!         name: String,
//!         color: Color,
//!     }
//!
//!     struct Buyer has store {
//!         name: String,
//!         address: address,
//!     }
//!
//!     struct FruitManager has key {
//!         // The key is just an incrementing counter. This tracks all the fruit we have.
//!         fruit_inventory: Table<u64, Fruit>,
//!
//!         // A map from fruit name to price.
//!         prices: SimpleMap<String, u64>,
//!
//!         // A list of addresses authorized to buy the fruit.
//!         authorized_buyers: vector<Buyer>,
//!
//!         // The last time a piece of fruit was sold.
//!         last_sale_time: u64,
//!     }
//! }
//! ```
//!
//! If you fetch the `FruitManager` resource, it might look like this (where some
//! fields are skipped for conciseness):
//!
//! ```json
//! {
//!     "last_sale_time": "1681320398",
//!     "authorized_buyers": ["0x321"],
//!     "prices": [],
//! }
//! ```
//!
//! This leaves you guessing what `prices` is, since all you see is an empty vec. Using the `TypeAccessor` you can figure this out:
//!
//! ```no_run
//! use aptos_api_types::MoveStructTag;
//! use aptos_rest_client::Client as RestClient;
//! use aptos_types::account_address::AccountAddress;
//! use aptos_type_accessor::builder::RemoteTypeAccessorBuilder;
//! use aptos_type_accessor::module_retriever::{ModuleRetriever, ApiModuleRetriever};
//! use move_core_types::{identifier::Identifier, language_storage::ModuleId};
//! use std::sync::Arc;
//! # use std::str::FromStr;
//!
//! #[tokio::main]
//! async fn main() -> anyhow::Result<()> {
//!     # let url = url::Url::from_str("https://fullnode.mainnet.aptoslabs.com").unwrap();
//!     let aptos_client = Arc::new(RestClient::new(url));
//!     let module_retriever = ModuleRetriever::from(ApiModuleRetriever::new(aptos_client));
//!
//!     let account_address = AccountAddress::from_hex_literal("0x123")?;
//!     let module_name = Identifier::new("food")?;
//!     let module_id = ModuleId::new(account_address, module_name);
//!     let type_accessor = RemoteTypeAccessorBuilder::new(module_retriever)
//!         .lookup_module(module_id.clone())
//!         .build()
//!         .await?;
//!
//!     let struct_name = Identifier::new("FruitManager")?;
//!     type_accessor.get_type(&module_id, &struct_name, "prices");
//!     #
//!     # Ok(())
//! }
//! ```
//!
//! Output:
//!
//! ```text
//! MoveStructTag {
//!     address: Address(
//!         0000000000000000000000000000000000000000000000000000000000000001,
//!     ),
//!     module: IdentifierWrapper(
//!         Identifier(
//!             "simple_map",
//!         ),
//!     ),
//!     name: IdentifierWrapper(
//!         Identifier(
//!             "SimpleMap",
//!         ),
//!     ),
//!     generic_type_params: [
//!         Struct(
//!             MoveStructTag {
//!                 address: Address(
//!                     0000000000000000000000000000000000000000000000000000000000000001,
//!                 ),
//!                 module: IdentifierWrapper(
//!                     Identifier(
//!                         "string",
//!                     ),
//!                 ),
//!                 name: IdentifierWrapper(
//!                     Identifier(
//!                         "String",
//!                     ),
//!                 ),
//!                 generic_type_params: [],
//!             },
//!         ),
//!         U64,
//!     ],
//! }
//! ```
//!
//! Now you know that it is a `SimpleMap`, great!
//!
//! You can use the `TypeAccessor` to resolve other similar issues with this output:
//! - With the output alone you cannot determine what type `last_sale_time` is since
//! it is represented as a string. Using the [`TypeAccessor`] you can determine that
//! it is a `u64`.
//! - With the output alone you cannot determine what type the items in
//! `authorized_buyers` are. Just because it is a string that looks like an address
//! does not guarantee that it is one. Using the [`TypeAccessor`] you can determine
//! that it is indeed an `address`.
//!
//! The [`TypeAccessor`] also supports nested queries:
//!
//! ```no_run
//! #
//! # use aptos_types::account_address::AccountAddress;
//! # use move_core_types::{identifier::Identifier, language_storage::ModuleId};
//! # use aptos_type_accessor::builder::LocalTypeAccessorBuilder;
//! #
//! # let type_accessor = LocalTypeAccessorBuilder::new().build()?;
//! #
//! # let account_address = AccountAddress::from_hex_literal("0x123")?;
//! # let module_name = Identifier::new("food")?;
//! # let module_id = ModuleId::new(account_address, module_name);
//! # let struct_name = Identifier::new("FruitManager")?;
//! #
//! type_accessor.get_type(&module_id, &struct_name, "fruit_inventory.1.color.red")?;
//! #
//! # Ok::<(), anyhow::Error>(())
//! ```
//!
//! Output:
//! ```text
//! MoveType::U64
//! ```
//!
//! ## Path Syntax
//!
//! In its raw format, a path is defined as a vec of [`PathComponent`].
//!
//! ```
//! # use aptos_type_accessor::path::PathComponent;
//! # use move_core_types::identifier::Identifier;
//! #
//! let path = vec![
//!    PathComponent::Field(Identifier::new("fruit_inventory").unwrap()),
//!    PathComponent::GenericTypeParamIndex(1),
//!    PathComponent::Field(Identifier::new("color").unwrap()),
//!    PathComponent::Field(Identifier::new("red").unwrap()),
//! ];
//! ```
//! This means get the `fruit_inventory` field, then the 1st generic type param
//! (0-indexed), then the `color` field, then the `red` field.
//!
//! This is a bit verbose, so we also provide a syntax for specifying paths as a string.
//! The syntax is inspired by [jq](https://stedolan.github.io/jq/), though it is not
//! identical.
//!
//! These are some examples of path strings and what they get parsed to:
//!
//! An example that uses `.1` to access the 1st (0-indexed) generic type param:
//! ```
//! "fruit_inventory.1.color.red";
//! # ()
//! ```
//! ```
//! # use aptos_type_accessor::path::PathComponent;
//! # use move_core_types::identifier::Identifier;
//! #
//! vec![
//!    PathComponent::Field(Identifier::new("fruit_inventory").unwrap()),
//!    PathComponent::GenericTypeParamIndex(1),
//!    PathComponent::Field(Identifier::new("color").unwrap()),
//!    PathComponent::Field(Identifier::new("red").unwrap()),
//! ];
//! ```
//!
//! An example that uses `.[]` to access the type of the thing inside a vector:
//! ```
//! "authorized_buyers.[].address";
//! # ()
//! ```
//! ```
//! # use aptos_type_accessor::path::PathComponent;
//! # use move_core_types::identifier::Identifier;
//! #
//! vec![
//!   PathComponent::Field(Identifier::new("authorized_buyers").unwrap()),
//!   PathComponent::EnterArray,
//!   PathComponent::Field(Identifier::new("address").unwrap()),
//! ];
//! ```
//!
//! You can read more about this syntax in the documentation for [`parse_path`].
//!
//! ## Builder Types
//!
//! This crate offers two types of builders for building a [`TypeAccessor`]:
//! - [`LocalTypeAccessorBuilder`]: Use this when you have all the modules you need
//! locally already.
//! - [`RemoteTypeAccessorBuilder`]: Use this when you want to fetch modules from the
//! API as part of building the TypeAccessor.
//!
//! The documentation for each of these builders will help you decide which one is
//! appropriate for your use case.

pub mod accessor;
pub mod builder;
pub mod module_retriever;
pub mod path;
#[cfg(test)]
mod test_helpers;

// The TypeAccessor lookup methods should have an error that returns any modules that
// were missing or incomplete if the lookup fails. Then we can have a manager on top
// that holds on to the builder, and the builder can be told to refetch those modules
// and then rebuild.
