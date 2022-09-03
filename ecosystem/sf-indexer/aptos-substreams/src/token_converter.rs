// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use anyhow::{Context, Result};
use aptos_protos::{
    block_output::v1::{MoveResourceOutput, TableItemOutput},
    tokens::v1::{CollectionData, Token, TokenData, TokenDataId, TokenId},
};
use std::collections::HashMap;

pub fn get_table_handle_to_address(
    resource: &MoveResourceOutput,
    txn_version: u64,
) -> anyhow::Result<Option<HashMap<String, String>>> {
    match resource.type_str.as_str() {
        "0x3::token::Collections" => {
            let address = &resource.address;
            let data: serde_json::Value = serde_json::from_str(&resource.data)?;
            let collection_handle = data["data"]["collection_data"]["handle"]
                .as_str()
                .map(|s| s.to_string())
                .context(format!(
                    "version {} failed! collection data handle must be present {:?}",
                    txn_version, data
                ))?;

            Ok(Some(HashMap::from([(
                standardize_handle(&collection_handle),
                address.clone(),
            )])))
        }
        "0x3::token::TokenStore" => {
            let address = &resource.address;
            let data: serde_json::Value = serde_json::from_str(&resource.data)?;
            let token_store_handle = data["data"]["tokens"]["handle"]
                .as_str()
                .map(|s| s.to_string())
                .context(format!(
                    "version {} failed! token store data handle must be present {:?}",
                    txn_version, data
                ))?;
            Ok(Some(HashMap::from([(
                standardize_handle(&token_store_handle),
                address.clone(),
            )])))
        }
        _ => Ok(None),
    }
}

fn standardize_handle(handle: &String) -> String {
    format!("0x{}", &handle[2..].trim_start_matches('0'))
}

pub fn get_token(
    table_item: &TableItemOutput,
    txn_version: u64,
    table_handle_to_address: &HashMap<String, String>,
) -> anyhow::Result<Option<Token>> {
    if table_item.key_type != "0x3::token::TokenId" {
        return Ok(None);
    }
    let owner_address = table_handle_to_address
        .get(&standardize_handle(&table_item.handle))
        .context(format!(
            "version {} failed! token resource was missing, table handle {} not in map {:?}",
            txn_version,
            standardize_handle(&table_item.handle),
            table_handle_to_address,
        ))?
        .clone();
    let key: serde_json::Value = serde_json::from_str(&table_item.decoded_key)?;
    let token_data_id = TokenDataId {
        creator_address: key["token_data_id"]["creator"]
            .as_str()
            .map(|s| s.to_string())
            .context(format!(
                "version {} failed! token_data_id.creator missing from token_id {:?}",
                txn_version, key
            ))?,
        collection_name: key["token_data_id"]["collection"]
            .as_str()
            .map(|s| s.to_string())
            .context(format!(
                "version {} failed! token_data_id.collection missing from token_id {:?}",
                txn_version, key
            ))?,
        name: key["token_data_id"]["name"]
            .as_str()
            .map(|s| s.to_string())
            .context(format!(
                "version {} failed! name missing from token_id {:?}",
                txn_version, key
            ))?,
    };
    let property_version = key["property_version"]
        .as_str()
        .map(|s| s.parse::<u64>())
        .context(format!(
            "version {} failed! token_data_id.property_version missing from token id {:?}",
            txn_version, key
        ))?
        .context(format!(
            "version {} failed! failed to parse property_version {:?}",
            txn_version, key["property_version"]
        ))?;
    let token_id = TokenId {
        token_data_id: Some(token_data_id),
        property_version,
    };
    if table_item.value_type == "0x3::token::Token" {
        let value: serde_json::Value = serde_json::from_str(&table_item.decoded_value)?;
        return Ok(Some(Token {
            token_id: Some(token_id),
            transaction_version: txn_version,
            token_properties: serde_json::to_string(&value["token_properties"])?,
            amount: value["amount"]
                .as_str()
                .map(|s| s.parse::<u64>())
                .context(format!(
                    "version {} failed! amount missing from token",
                    txn_version
                ))?
                .context(format!("failed to parse amount {:?}", value["amount"]))?,
            owner_address: owner_address.clone(),
        }));
    } else if table_item.is_deleted {
        return Ok(Some(Token {
            token_id: Some(token_id),
            transaction_version: txn_version,
            token_properties: String::default(),
            amount: 0,
            owner_address: owner_address.clone(),
        }));
    }
    Ok(None)
}

pub fn get_token_data(
    table_item: &TableItemOutput,
    txn_version: u64,
) -> anyhow::Result<Option<TokenData>> {
    if table_item.value_type == "0x3::token::TokenData" {
        let key: serde_json::Value = serde_json::from_str(&table_item.decoded_key)?;
        let token_data_id =
            TokenDataId {
                creator_address: key["creator"].as_str().map(|s| s.to_string()).context(
                    format!(
                        "version {} failed! creator missing from key {:?}",
                        txn_version, key
                    ),
                )?,
                collection_name: key["collection"].as_str().map(|s| s.to_string()).context(
                    format!(
                        "version {} failed! collection missing from key {:?}",
                        txn_version, key
                    ),
                )?,
                name: key["name"]
                    .as_str()
                    .map(|s| s.to_string())
                    .context(format!(
                        "version {} failed! name missing from key {:?}",
                        txn_version, key
                    ))?,
            };

        let value: serde_json::Value = serde_json::from_str(&table_item.decoded_value)?;
        return Ok(Some(TokenData {
            token_data_id: Some(token_data_id),
            transaction_version: txn_version,
            maximum: value["maximum"]
                .as_str()
                .map(|s| s.parse::<u64>())
                .context(format!(
                    "version {} failed! maximum missing from token data {:?}",
                    txn_version, value
                ))?
                .context(format!(
                    "version {} failed! failed to parse maximum {:?}",
                    txn_version, value["maximum"]
                ))?,
            supply: value["supply"]
                .as_str()
                .map(|s| s.parse::<u64>())
                .context(format!(
                    "version {} failed! supply missing from token data {:?}",
                    txn_version, value
                ))?
                .context(format!(
                    "version {} failed! failed to parse supply {:?}",
                    txn_version, value["maximum"]
                ))?,
            largest_property_version: value["largest_property_version"]
                .as_str()
                .map(|s| s.parse::<u64>())
                .context(format!(
                    "version {} failed! largest_property_version missing from token data {:?}",
                    txn_version, value
                ))?
                .context(format!(
                    "version {} failed! failed to parse largest_property_version {:?}",
                    txn_version, value["maximum"]
                ))?,
            metadata_uri: value["uri"]
                .as_str()
                .map(|s| s.to_string())
                .context(format!(
                    "version {} failed! uri missing from token data {:?}",
                    txn_version, value
                ))?,
            payee_address: value["royalty"]["payee_address"]
                .as_str()
                .map(|s| s.to_string())
                .context(format!(
                    "version {} failed! royalty.payee_address missing {:?}",
                    txn_version, value
                ))?,
            royalty_points_numerator: value["royalty"]["royalty_points_numerator"]
                .as_str()
                .map(|s| s.parse::<u64>())
                .context(format!(
                    "version {} failed! royalty.royalty_points_numerator missing {:?}",
                    txn_version, value
                ))?
                .context(format!(
                    "version {} failed! failed to parse royalty_points_numerator {:?}",
                    txn_version, value["royalty"]["royalty_points_numerator"]
                ))?,
            royalty_points_denominator: value["royalty"]["royalty_points_denominator"]
                .as_str()
                .map(|s| s.parse::<u64>())
                .context(format!(
                    "version {} failed! royalty.royalty_points_denominator missing {:?}",
                    txn_version, value
                ))?
                .context(format!(
                    "version {} failed! failed to parse royalty_points_denominator {:?}",
                    txn_version, value["royalty"]["royalty_points_denominator"]
                ))?,
            maximum_mutable: value["mutability_config"]["maximum"]
                .as_bool()
                .context(format!(
                    "version {} failed! mutability_config.maximum missing {:?}",
                    txn_version, value
                ))?,
            uri_mutable: value["mutability_config"]["uri"]
                .as_bool()
                .context(format!(
                    "version {} failed! mutability_config.uri missing {:?}",
                    txn_version, value
                ))?,
            description_mutable: value["mutability_config"]["description"]
                .as_bool()
                .context(format!(
                    "version {} failed! mutability_config.description missing {:?}",
                    txn_version, value
                ))?,
            properties_mutable: value["mutability_config"]["properties"].as_bool().context(
                format!(
                    "version {} failed! mutability_config.properties missing {:?}",
                    txn_version, value
                ),
            )?,
            royalty_mutable: value["mutability_config"]["royalty"]
                .as_bool()
                .context(format!(
                    "version {} failed! mutability_config.royalty missing {:?}",
                    txn_version, value
                ))?,
            default_properties: value["default_properties"]
                .as_object()
                .map(|s| serde_json::to_string(s))
                .context(format!(
                    "version {} failed! default_properties missing from token data {:?}",
                    txn_version, value
                ))?
                .context(format!(
                    "version {} failed! failed to stringify default_properties {:?}",
                    txn_version, value["default_properties"]
                ))?,
        }));
    }
    Ok(None)
}

pub fn get_collection_data(
    table_item: &TableItemOutput,
    txn_version: u64,
    table_handle_to_address: &HashMap<String, String>,
) -> Result<Option<CollectionData>> {
    if table_item.value_type == "0x3::token::CollectionData" {
        let value: serde_json::Value = serde_json::from_str(&table_item.decoded_value)?;
        let creator_address = table_handle_to_address
            .get(&standardize_handle(&table_item.handle))
            .context(format!(
                "version {} failed! collection creator resource was missing, table handle {} not in map {:?}",
                txn_version, standardize_handle(&table_item.handle), table_handle_to_address,
            ))?
            .clone();
        return Ok(Some(CollectionData {
            collection_name: value["name"]
                .as_str()
                .map(|s| s.to_string())
                .context(format!(
                    "version {} failed! name missing from collection {:?}",
                    txn_version, value
                ))?,
            creator_address,
            description: value["description"]
                .as_str()
                .map(|s| s.to_string())
                .context(format!(
                    "version {} failed! description missing from collection {:?}",
                    txn_version, value
                ))?,
            transaction_version: txn_version,
            metadata_uri: value["uri"]
                .as_str()
                .map(|s| s.to_string())
                .context(format!(
                    "version {} failed! uri missing from collection {:?}",
                    txn_version, value
                ))?,
            supply: value["supply"]
                .as_str()
                .map(|s| s.parse::<u64>())
                .context(format!(
                    "version {} failed! supply missing from collection {:?}",
                    txn_version, value
                ))?
                .context(format!(
                    "version {} failed! failed to parse supply {:?}",
                    txn_version, value["supply"]
                ))?,
            maximum: value["maximum"]
                .as_str()
                .map(|s| s.parse::<u64>())
                .context(format!(
                    "version {} failed! maximum missing from collection {:?}",
                    txn_version, value
                ))?
                .context(format!(
                    "version {} failed! failed to parse maximum {:?}",
                    txn_version, value["maximum"]
                ))?,
            maximum_mutable: value["mutability_config"]["maximum"]
                .as_bool()
                .context(format!(
                    "version {} failed! mutability_config.maximum missing {:?}",
                    txn_version, value
                ))?,
            uri_mutable: value["mutability_config"]["uri"]
                .as_bool()
                .context(format!(
                    "version {} failed! mutability_config.uri missing {:?}",
                    txn_version, value
                ))?,
            description_mutable: value["mutability_config"]["description"]
                .as_bool()
                .context(format!(
                    "version {} failed! mutability_config.description missing {:?}",
                    txn_version, value
                ))?,
        }));
    }
    Ok(None)
}
