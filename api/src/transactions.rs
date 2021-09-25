// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::context::Context;

use diem_api_types::{Error, Event, LedgerInfo, Response, Transaction};
use diem_types::contract_event::ContractEvent;
use resource_viewer::MoveValueAnnotator;

use anyhow::{format_err, Result};
use serde::Deserialize;
use serde_json::json;
use std::str::FromStr;
use warp::{Filter, Rejection, Reply};

const DEFAULT_PAGE_SIZE: u32 = 25;
const MAX_PAGE_SIZE: u32 = 1000;

pub fn routes(context: Context) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    get_transactions(context)
}

// GET /transactions?start={u64}&limit={u16}
pub fn get_transactions(
    context: Context,
) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    warp::path!("transactions")
        .and(warp::get())
        .and(warp::query::<PageQueryParam>())
        .and(context.filter())
        .and_then(handle_get_transactions)
}

async fn handle_get_transactions(
    page: PageQueryParam,
    context: Context,
) -> Result<impl Reply, Rejection> {
    Ok(Transactions::new(page, context)?.page()?)
}

#[derive(Clone, Debug, Deserialize)]
struct PageQueryParam {
    start: Option<String>,
    limit: Option<String>,
}

impl PageQueryParam {
    pub fn start(&self, latest_ledger_version: u64) -> Result<u64, Error> {
        let v = parse_param("start", &self.start, latest_ledger_version)?;
        if v > latest_ledger_version {
            return Err(transaction_not_found(v, latest_ledger_version));
        }
        Ok(v)
    }

    pub fn limit(&self) -> Result<u16, Error> {
        let v = parse_param("limit", &self.limit, DEFAULT_PAGE_SIZE)?;
        if v > MAX_PAGE_SIZE {
            return Err(Error::bad_request(format_err!(
                "invalid parameter: limit={}, exceed limit {}",
                v,
                MAX_PAGE_SIZE
            )));
        }
        Ok(v as u16)
    }
}

struct Transactions {
    start_version: u64,
    limit: u16,
    ledger_info: LedgerInfo,
    context: Context,
}

impl Transactions {
    fn new(page: PageQueryParam, context: Context) -> Result<Self, Error> {
        let ledger_info = context.get_latest_ledger_info()?;
        let ledger_version = ledger_info.version();
        Ok(Self {
            start_version: page.start(ledger_version)?,
            limit: page.limit()?,
            ledger_info,
            context,
        })
    }

    pub fn page(self) -> Result<impl Reply, Error> {
        let ledger_version = self.ledger_info.version();
        let data = self
            .context
            .get_transactions(self.start_version, self.limit, ledger_version)?;

        let txn_start_version = data.first_transaction_version.unwrap_or(0);
        let submitted = data.transactions;
        let infos = data.proof.transaction_infos;
        let events = data.events.unwrap_or_default();

        if submitted.len() != infos.len() || submitted.len() != events.len() {
            return Err(format_err!(
                "invalid data size from database: {}, {}, {}",
                submitted.len(),
                infos.len(),
                events.len(),
            )
            .into());
        }

        let txns: Vec<Transaction> = submitted
            .iter()
            .enumerate()
            .map(|(i, txn)| (txn_start_version + i as u64, txn, &infos[i], &events[i]))
            .map(|(version, txn, info, events)| {
                Ok((version, txn, info, self.events(version, events)?).into())
            })
            .collect::<Result<_>>()?;
        Response::new(self.ledger_info, &txns)
    }

    fn events(&self, txn_version: u64, events: &[ContractEvent]) -> Result<Vec<Event>> {
        let db = self.context.db();
        let annotator = MoveValueAnnotator::new(&db);
        let mut ret = vec![];
        for event in events {
            let data = annotator.view_value(event.type_tag(), event.event_data())?;
            ret.push((txn_version, event, data).into());
        }
        Ok(ret)
    }
}

fn parse_param<T: FromStr>(
    param_name: &str,
    data: &Option<String>,
    default: T,
) -> Result<T, Error> {
    match data {
        Some(n) => n.parse::<T>().map_err(|_| {
            Error::bad_request(format_err!("invalid parameter: {}={}", param_name, n))
        }),
        None => Ok(default),
    }
}

fn transaction_not_found(version: u64, ledger_version: u64) -> Error {
    Error::not_found(
        format!("could not find transaction by version: {}", version),
        json!({"ledger_version": ledger_version.to_string()}),
    )
}

#[cfg(any(test))]
mod tests {
    use crate::test_utils::{assert_json, new_test_context, send_request};

    use diem_types::transaction::TransactionInfoTrait;

    use serde_json::json;

    #[tokio::test]
    async fn test_get_transactions() {
        let context = new_test_context();
        let resp = send_request(context.clone(), "GET", "/transactions", 200).await;
        assert_eq!(resp[0]["type"], "genesis_transaction");
        assert_eq!(resp[0]["version"], "0");

        let txns = context
            .get_transactions(0, 1, context.get_latest_ledger_info().unwrap().version())
            .unwrap();
        let info = txns.proof.transaction_infos[0].clone();
        assert_eq!(resp[0]["hash"], info.transaction_hash().to_hex());
        assert_eq!(resp[0]["state_root_hash"], info.state_root_hash().to_hex());
        assert_eq!(resp[0]["event_root_hash"], info.event_root_hash().to_hex());
        assert!(resp[0]["data"].as_str().unwrap().starts_with("0x"));

        let first_event = resp[0]["events"][0].clone();
        assert_json(
            first_event,
            json!({
                "key": "00000000000000000000000000000000000000000a550c18",
                "sequence_number": 0,
                "transaction_version": 0,
                "type": {
                    "type": "struct",
                    "address": "0x1",
                    "module": "DiemAccount",
                    "name": "CreateAccountEvent",
                    "generic_type_params": []
                },
                "data": {
                    "created": "0xa550c18",
                    "role_id": "0"
                }
            }),
        );
    }

    #[tokio::test]
    async fn test_get_transactions_with_start_version_is_too_large() {
        let context = new_test_context();
        let ledger_version = context.get_latest_ledger_info().unwrap().version();
        let resp = send_request(context, "GET", "/transactions?start=1000000&limit=10", 404).await;
        assert_json(
            resp,
            json!({
              "code": 404,
              "message": "could not find transaction by version: 1000000",
              "data": {
                "ledger_version": ledger_version.to_string()
              }
            }),
        );
    }

    #[tokio::test]
    async fn test_get_transactions_with_invalid_start_version_param() {
        let context = new_test_context();
        let resp = send_request(context, "GET", "/transactions?start=hello", 400).await;
        assert_json(
            resp,
            json!({
              "code": 400,
              "message": "invalid parameter: start=hello"
            }),
        );
    }

    #[tokio::test]
    async fn test_get_transactions_with_invalid_limit_param() {
        let context = new_test_context();
        let resp = send_request(context, "GET", "/transactions?limit=hello", 400).await;
        assert_json(
            resp,
            json!({
              "code": 400,
              "message": "invalid parameter: limit=hello"
            }),
        );
    }

    #[tokio::test]
    async fn test_get_transactions_param_limit_exceeds_limit() {
        let context = new_test_context();
        let resp = send_request(context, "GET", "/transactions?limit=2000", 400).await;
        assert_json(
            resp,
            json!({
              "code": 400,
              "message": "invalid parameter: limit=2000, exceed limit 1000"
            }),
        );
    }
}
