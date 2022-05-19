// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{models::events::Event, schema::tokens};
use aptos_rest_client::types;
use std::{collections::HashMap, fmt, fmt::Formatter, str::FromStr};

use serde::{Deserialize, Deserializer, Serialize};

#[derive(Associations, Debug, Identifiable, Insertable, Queryable, Serialize, Clone)]
#[diesel(table_name = "tokens")]
#[primary_key(token_id)]
pub struct Token {
    pub token_id: String,
    pub creator: String,
    pub collection: String,
    pub name: String,
    pub description: String,
    pub max_amount: Option<i64>,
    pub supply: i64,
    pub uri: String,
    pub minted_at: chrono::NaiveDateTime,
    pub inserted_at: chrono::NaiveDateTime,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TokenId {
    pub creator: String,
    pub collection: String,
    pub name: String,
}

impl TokenId {
    pub fn from_serde_value(value: serde_json::Value) -> Self {
        serde_json::from_value(value).unwrap()
    }
}

impl fmt::Display for TokenId {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        write!(f, "{}::{}::{}", self.creator, self.collection, self.name)
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TokenData {
    pub collection: String,
    pub description: String,
    pub name: String,
    #[serde(deserialize_with = "deserialize_option_from_string")]
    pub maximum: MoveOption<i64>,
    #[serde(deserialize_with = "deserialize_option_from_string")]
    pub supply: MoveOption<i64>,
    pub uri: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MoveOption<T> {
    pub value: Option<T>,
}

impl<T: std::str::FromStr> FromStr for MoveOption<T> {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.is_empty() {
            Ok(MoveOption { value: None })
        } else {
            s.parse::<T>().map_or_else(
                |_e| Err(anyhow::format_err!("Invalid MoveOption {:?}", s)),
                |v| Ok(MoveOption { value: Some(v) }),
            )
        }
    }
}

pub fn deserialize_option_from_string<'de, D, T>(deserializer: D) -> Result<T, D::Error>
where
    T: FromStr,
    D: Deserializer<'de>,
    <T as FromStr>::Err: std::fmt::Display,
    <T as FromStr>::Err: std::fmt::Debug,
{
    use serde::de::Error;

    let res: Result<HashMap<String, Vec<String>>, _> = Deserialize::deserialize(deserializer);
    // let res = serde_json::from_str::<HashMap<String, Vec<String>>>(s);

    res.map_or_else(
        |e| Err(D::Error::custom(e)),
        |v| {
            if v["vec"].is_empty() {
                Ok("".parse::<T>().unwrap())
            } else {
                v["vec"][0].parse::<T>().map_err(D::Error::custom)
            }
        },
    )
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct WithdrawEventType {
    #[serde(deserialize_with = "types::deserialize_from_string")]
    pub amount: i64,
    pub id: TokenId,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DepositEventType {
    #[serde(deserialize_with = "types::deserialize_from_string")]
    pub amount: i64,
    pub id: TokenId,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CreationEventType {
    pub id: TokenId,
    pub token_data: TokenData,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MintEventType {
    #[serde(deserialize_with = "types::deserialize_from_string")]
    pub amount: i64,
    pub id: TokenId,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum TokenEvent {
    WithdrawEvent(WithdrawEventType),
    DepositEvent(DepositEventType),
    CreationEvent(CreationEventType),
    MintEvent(MintEventType),
    BurnEvent,
}

impl TokenEvent {
    pub fn from_event(event: &Event) -> Option<TokenEvent> {
        let data = event.data.clone();
        match event.type_.as_str() {
            "0x1::Token::WithdrawEvent" => {
                let event = serde_json::from_value::<WithdrawEventType>(data).unwrap();
                Some(TokenEvent::WithdrawEvent(event))
            }
            "0x1::Token::DepositEvent" => {
                let event = serde_json::from_value::<DepositEventType>(data).unwrap();
                Some(TokenEvent::DepositEvent(event))
            }
            "0x1::Token::CreateTokenEvent" => {
                let event = serde_json::from_value::<CreationEventType>(data).unwrap();
                Some(TokenEvent::CreationEvent(event))
            }
            _ => None,
        }
    }
}
