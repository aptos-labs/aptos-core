
// Copyright © Aptos Foundation

// This is required because a diesel macro makes clippy sad
#![allow(clippy::extra_unused_lifetimes)]

/**
 * This file defines deserialized vault_events module types as defined in mirage protocol module.
 */

use crate::{
    mirage_utils::MIRAGE_ADDRESS,
    util::standardize_address,
};

use aptos_api_types::{deserialize_from_string, MoveStructTag};

use anyhow::{Context, Result};
use bigdecimal::BigDecimal;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ExchangeRateEvent {
    #[serde(deserialize_with = "deserialize_from_string")]
    pub rate: BigDecimal,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AccrueFeesEvent {
    #[serde(deserialize_with = "deserialize_from_string")]
    pub accrued_amount: BigDecimal,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct RegisterUserEvent {
    pub user_addr: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AddCollateralEvent {
    pub user_addr: String,
    #[serde(deserialize_with = "deserialize_from_string")]
    pub collateral_amount: BigDecimal,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct RemoveCollateralEvent {
    pub user_addr: String,
    #[serde(deserialize_with = "deserialize_from_string")]
    pub collateral_amount: BigDecimal,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct BorrowEvent {
    pub user_addr: String,
    #[serde(deserialize_with = "deserialize_from_string")]
    pub borrow_amount: BigDecimal,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct RepayEvent {
    pub user_addr: String,
    #[serde(deserialize_with = "deserialize_from_string")]
    pub repay_amount: BigDecimal,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct LiquidationEvent {
    pub liquidator_addr: String,
    pub user_addr: String,
    #[serde(deserialize_with = "deserialize_from_string")]
    pub collateral_amount: BigDecimal,
    #[serde(deserialize_with = "deserialize_from_string")]
    pub borrow_amount: BigDecimal,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct WithdrawFeesEvent {
    pub withdraw_addr: String,
    pub dev_address: String,
    #[serde(deserialize_with = "deserialize_from_string")]
    pub fees_earned: BigDecimal,
    #[serde(deserialize_with = "deserialize_from_string")]
    pub borrow_amount: BigDecimal,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct InterestRateChangeEvent {
    #[serde(deserialize_with = "deserialize_from_string")]
    pub old_interest_per_second: BigDecimal,
    #[serde(deserialize_with = "deserialize_from_string")]
    pub new_interest_per_second: BigDecimal,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum VaultEvent {
    ExchangeRateEvent(ExchangeRateEvent),
    AccrueFeesEvent(AccrueFeesEvent),
    RegisterUserEvent(RegisterUserEvent),
    AddCollateralEvent(AddCollateralEvent),
    RemoveCollateralEvent(RemoveCollateralEvent),
    BorrowEvent(BorrowEvent),
    RepayEvent(RepayEvent),
    LiquidationEvent(LiquidationEvent),
    WithdrawFeesEvent(WithdrawFeesEvent),
    InterestRateChangeEvent(InterestRateChangeEvent),
}

impl VaultEvent {
    pub fn is_event_supported(move_type: &MoveStructTag) -> bool {
        standardize_address(&move_type.address.to_string()) == MIRAGE_ADDRESS
            && move_type.module.to_string() == "vault_event"
            && move_type.generic_type_params.len() == 2
    }

    pub fn from_event(
        event_name: &str,
        data: &serde_json::Value,
        txn_version: i64,
    ) -> Result<Self> {
        match event_name {
            "ExchangeRateEvent" => serde_json::from_value(data.clone())
                .map(|inner| Some(VaultEvent::ExchangeRateEvent(inner))),
            "AccrueFeesEvent" => serde_json::from_value(data.clone())
                .map(|inner| Some(VaultEvent::AccrueFeesEvent(inner))),
            "RegisterUserEvent" => serde_json::from_value(data.clone())
                .map(|inner| Some(VaultEvent::RegisterUserEvent(inner))),
            "AddCollateralEvent" => serde_json::from_value(data.clone())
                .map(|inner| Some(VaultEvent::AddCollateralEvent(inner))),
            "RemoveCollateralEvent" => serde_json::from_value(data.clone())
                .map(|inner| Some(VaultEvent::RemoveCollateralEvent(inner))),
            "BorrowEvent" => serde_json::from_value(data.clone())
                .map(|inner| Some(VaultEvent::BorrowEvent(inner))),
            "RepayEvent" => serde_json::from_value(data.clone())
                .map(|inner| Some(VaultEvent::RepayEvent(inner))),
            "LiquidationEvent" => serde_json::from_value(data.clone())
                .map(|inner| Some(VaultEvent::LiquidationEvent(inner))),
            "WithdrawFeesEvent" => serde_json::from_value(data.clone())
                .map(|inner| Some(VaultEvent::WithdrawFeesEvent(inner))),
            "InterestRateChangeEvent" => serde_json::from_value(data.clone())
                .map(|inner| Some(VaultEvent::InterestRateChangeEvent(inner))),
            _ => Ok(None),
        }
        .context(format!(
            "version {} failed! failed to parse event {}, data {:?}",
            txn_version, event_name, data
        ))?
        .context(format!(
            "Event unsupported! Call is_event_supported first. version {} event {}",
            txn_version, event_name
        ))
    }
}
