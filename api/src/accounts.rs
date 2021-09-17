// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::context::Context;

use diem_api_types::{Address, Error, MoveResource};
use diem_types::account_state::AccountState;
use resource_viewer::MoveValueAnnotator;

use anyhow::Result;
use std::{convert::TryFrom, str::FromStr};
use warp::{Filter, Rejection, Reply};

pub fn routes(context: Context) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    get_account_resources(context)
}

// GET /accounts/<address>/resources
pub fn get_account_resources(
    context: Context,
) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    warp::path!("accounts" / String / "resources")
        .and(warp::get())
        .and(context.filter())
        .and_then(handle_get_account_resources)
}

async fn handle_get_account_resources(
    address_str: String,
    context: Context,
) -> Result<impl Reply, Rejection> {
    let address = Address::from_str(&address_str).map_err(Error::bad_request)?;

    let ledger_info = context.get_latest_ledger_info().map_err(Error::internal)?;

    let state = context
        .get_account_state(address.into_inner(), ledger_info.ledger_info().version())
        .map_err(Error::internal)?
        .ok_or_else(|| account_not_found(&address_str))?;

    let db = context.db();
    let annotator = MoveValueAnnotator::new(&db);
    let account_state = AccountState::try_from(&state).map_err(Error::internal)?;
    let mut resources = vec![];
    for (typ, bytes) in account_state.get_resources() {
        let resource = annotator
            .view_resource(&typ, bytes)
            .map_err(Error::internal)?;
        resources.push(MoveResource::from(resource));
    }
    Ok(warp::reply::json(&resources))
}

fn account_not_found(address: &str) -> Error {
    Error::not_found(format!("could not find account by address: {}", address))
}

#[cfg(any(test))]
mod tests {
    use crate::test_utils::{new_test_context, send_request};
    use serde_json::{json, Value};

    #[tokio::test]
    async fn test_get_account_resources_returns_empty_array_for_account_has_no_resources() {
        let context = new_test_context();
        let address = "0x1";

        let resp = send_request(context, "GET", &account_resources(address), 200).await;
        assert_eq!(json!([]), resp);
    }

    #[tokio::test]
    async fn test_get_account_resources_by_address_0x0() {
        let context = new_test_context();
        let address = "0x0";

        let resp = send_request(context, "GET", &account_resources(address), 404).await;
        assert_eq!(
            json!({
                "code": 404,
                "message": "could not find account by address: 0x0"
            }),
            resp
        );
    }

    #[tokio::test]
    async fn test_get_account_resources_by_invalid_address_missing_0x_prefix() {
        let context = new_test_context();
        let invalid_addresses = vec!["1", "0xzz", "01"];
        for invalid_address in &invalid_addresses {
            let path = account_resources(invalid_address);
            let resp = send_request(context.clone(), "GET", &path, 400).await;
            assert_eq!(
                json!({
                    "code": 400,
                    "message": format!("invalid account address: {}", invalid_address),
                }),
                resp
            );
        }
    }

    #[tokio::test]
    async fn test_get_account_resources_by_valid_account_address() {
        let context = new_test_context();
        let addresses = vec![
            "0xdd",
            "000000000000000000000000000000dd",
            "0x000000000000000000000000000000dd",
        ];
        for address in &addresses {
            send_request(context.clone(), "GET", &account_resources(address), 200).await;
        }
    }

    #[tokio::test]
    async fn test_account_resources_response() {
        let context = new_test_context();
        let address = "0xdd";

        let resp = send_request(context, "GET", &account_resources(address), 200).await;

        assert_include_resource(
            &resp,
            "0x1::DiemAccount::Balance<0x1::XDX::XDX>",
            json!({
                "type": "0x1::DiemAccount::Balance<0x1::XDX::XDX>",
                "type_tag": {
                    "type": "struct",
                    "address": "0x1",
                    "module": "DiemAccount",
                    "name": "Balance",
                    "type_params": [
                        {
                            "type": "struct",
                            "address": "0x1",
                            "module": "XDX",
                            "name": "XDX",
                            "type_params": []
                        }
                    ]
                },
                "value": {
                    "coin": {
                        "value": "0"
                    }
                }
            }),
        );

        assert_include_resource(
            &resp,
            "0x1::Event::EventHandleGenerator",
            json!({
                "type": "0x1::Event::EventHandleGenerator",
                "type_tag": {
                    "type": "struct",
                    "address": "0x1",
                    "module": "Event",
                    "name": "EventHandleGenerator",
                    "type_params": []
                },
                "value": {
                    "counter": "5",
                    "addr": "0xdd"
                }
            }),
        );
    }

    fn assert_include_resource(resp: &Value, type_id: &str, expected: Value) {
        let resources = resp.as_array().expect("array");
        let mut balances = resources.iter().filter(|res| res["type"] == type_id);
        let resource = balances.next().expect(type_id);
        assert_eq!(&expected, resource);
        assert!(balances.next().is_none());
    }

    fn account_resources(address: &str) -> String {
        format!("/accounts/{}/resources", address)
    }
}
