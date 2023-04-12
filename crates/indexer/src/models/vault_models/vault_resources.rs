// Copyright © Aptos Foundation

// This is required because a diesel macro makes clippy sad
#![allow(clippy::extra_unused_lifetimes)]

/**
 * This file defines resources deserialized vault module.
 */

use super::rebase::Rebase;

use crate::{
    mirage_utils::{trunc_type, hash_types, MIRAGE_ADDRESS},
    util::{standardize_address},
    models::coin_models::coin_utils::{Coin},
    models::move_resources::MoveResource,
};

use crate::schema::{user_infos, vaults};

use aptos_api_types::{deserialize_from_string, MoveStructTag, WriteResource};

use anyhow::{Context, Result};
use bigdecimal::BigDecimal;
use field_count::FieldCount;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, FieldCount)]
pub struct FeeAccrureInfo {
    #[serde(deserialize_with = "deserialize_from_string")]
    pub last_time: BigDecimal,
    pub fees_earned: Coin,
}

#[derive(Debug, Deserialize, FieldCount, Identifiable, Insertable, Serialize)]
#[diesel(primary_key(collateral_type, borrow_type))]
#[diesel(table_name = vaults)]
pub struct Vault {
    pub transaction_version: i64,
    pub collateral_type: String,
    pub borrow_type: String,
    pub type_hash: String,
    pub total_collateral: BigDecimal,
    pub borrow_elastic: BigDecimal,
    pub borrow_base: BigDecimal,
    pub last_fees_accrue_time: BigDecimal,
    pub fees_accrued: BigDecimal,
    pub interest_per_second: BigDecimal,
    pub collateralization_rate: BigDecimal,
    pub liquidation_multiplier: BigDecimal,
	pub borrow_fee: BigDecimal,
    pub distribution_part: BigDecimal,
    pub fee_to: String,
	pub cached_exchange_rate: BigDecimal,
	pub last_interest_update: BigDecimal,
	pub is_emergency: bool,
	pub dev_cut: BigDecimal,
    pub transaction_timestamp: chrono::NaiveDateTime,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct VaultResource {
    #[serde(deserialize_with = "deserialize_from_string")]
    pub total_collateral: BigDecimal,
    pub borrow: Rebase,
    pub fees: FeeAccrureInfo,
    #[serde(deserialize_with = "deserialize_from_string")]
    pub interest_per_second: BigDecimal,
    #[serde(deserialize_with = "deserialize_from_string")]
    pub collateralization_rate: BigDecimal,
    #[serde(deserialize_with = "deserialize_from_string")]
    pub liquidation_multiplier: BigDecimal,
    #[serde(deserialize_with = "deserialize_from_string")]
	pub borrow_fee: BigDecimal,
    #[serde(deserialize_with = "deserialize_from_string")]
    pub distribution_part: BigDecimal,
    pub fee_to: String,
    #[serde(deserialize_with = "deserialize_from_string")]
	pub cached_exchange_rate: BigDecimal,
    #[serde(deserialize_with = "deserialize_from_string")]
	pub last_interest_update: BigDecimal,
	pub emergency: bool,
    #[serde(deserialize_with = "deserialize_from_string")]
	pub dev_cut: BigDecimal,
}

impl Vault {
    /// We can find user info from resources.
    pub fn from_resource(
        vault_resource: &VaultResource,
        collateral_type: &str,
        borrow_type: &str,
        txn_version: i64,
        txn_timestamp: chrono::NaiveDateTime,
    ) -> Self {
        Self {
            transaction_version: txn_version,
            type_hash: hash_types(collateral_type, borrow_type),
            collateral_type: trunc_type(collateral_type),
            borrow_type: trunc_type(borrow_type),
            total_collateral: vault_resource.total_collateral.clone(),
            borrow_elastic: vault_resource.borrow.elastic.clone(),
            borrow_base: vault_resource.borrow.base.clone(),
            last_fees_accrue_time: vault_resource.fees.last_time.clone(),
            fees_accrued: vault_resource.fees.fees_earned.value.clone(),
            interest_per_second: vault_resource.interest_per_second.clone(),
            collateralization_rate: vault_resource.collateralization_rate.clone(),
            liquidation_multiplier: vault_resource.liquidation_multiplier.clone(),
            borrow_fee: vault_resource.borrow_fee.clone(),
            distribution_part: vault_resource.distribution_part.clone(),
            fee_to: vault_resource.fee_to.clone(),
            cached_exchange_rate: vault_resource.cached_exchange_rate.clone(),
            last_interest_update: vault_resource.last_interest_update.clone(),
            is_emergency: vault_resource.emergency,
            dev_cut: vault_resource.dev_cut.clone(),
            transaction_timestamp: txn_timestamp,
        }
    }
}

#[derive(Debug, Deserialize, FieldCount, Identifiable, Insertable, Serialize)]
#[diesel(primary_key(user_address, collateral_type, borrow_type))]
#[diesel(table_name = user_infos)]
pub struct UserInfo {
    pub transaction_version: i64,
    pub collateral_type: String,
    pub borrow_type: String,
    pub type_hash: String,
    pub user_address: String,
    pub user_collateral: BigDecimal,
    pub user_borrow_part: BigDecimal,
    pub transaction_timestamp: chrono::NaiveDateTime,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct UserInfoResource {
    #[serde(deserialize_with = "deserialize_from_string")]
    pub user_collateral: BigDecimal,
    #[serde(deserialize_with = "deserialize_from_string")]
    pub user_borrow_part: BigDecimal,
}

impl UserInfo {
    /// We can find user info from resources.
    pub fn from_resource(
        user_info_resource: &UserInfoResource,
        user_address: &str,
        collateral_type: &str,
        borrow_type: &str,
        txn_version: i64,
        txn_timestamp: chrono::NaiveDateTime,
    ) -> Self {
        Self {
            transaction_version: txn_version,
            user_address: standardize_address(user_address),
            type_hash: hash_types(collateral_type, borrow_type),
            collateral_type: trunc_type(collateral_type),
            borrow_type: trunc_type(borrow_type),
            user_collateral: user_info_resource.user_collateral.clone(),
            user_borrow_part: user_info_resource.user_borrow_part.clone(),
            transaction_timestamp: txn_timestamp,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum VaultModuleResource {
    UserInfoResource(UserInfoResource),
    VaultResource(VaultResource),
}

impl VaultModuleResource {
    pub fn is_resource_supported(move_type: &MoveStructTag) -> bool {
        standardize_address(&move_type.address.to_string()) == MIRAGE_ADDRESS
            && move_type.module.to_string() == "vault"
            && (move_type.name.to_string() == "UserInfo"
              || move_type.name.to_string() == "Vault")
            && move_type.generic_type_params.len() == 2
    }

    pub fn from_write_resource(
        write_resource: &WriteResource,
        txn_version: i64,
    ) -> Result<VaultModuleResource> {
        let resource_name = write_resource.data.typ.name.to_string();

        let resource = MoveResource::from_write_resource(
            write_resource,
            0, // Placeholder, this isn't used anyway
            txn_version,
            0, // Placeholder, this isn't used anyway
        );
        let data = resource.data.as_ref().unwrap();

        match &resource_name as &str {
            "UserInfo" => serde_json::from_value(data.clone())
                .map(|inner| Some(VaultModuleResource::UserInfoResource(inner))),
            "Vault" => serde_json::from_value(data.clone())
                .map(|inner| Some(VaultModuleResource::VaultResource(inner))),
            _ => Ok(None)
        }
        .context(format!(
            "version {} failed! failed to parse vault resource {}, data {:?}",
            txn_version, resource_name, data
        ))?
        .context(format!(
            "Resource unsupported! Call is_resource_supported first. version {} type {}",
            txn_version, resource_name
        ))
    }
}
