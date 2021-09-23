// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::{context::Context, index};
use diem_api_types::{X_DIEM_CHAIN_ID, X_DIEM_LEDGER_TIMESTAMP, X_DIEM_LEDGER_VERSION};
use diem_genesis_tool::validator_builder::ValidatorBuilder;
use diem_temppath::TempPath;
use diem_types::chain_id::ChainId;
use diem_vm::DiemVM;
use diemdb::DiemDB;
use executor::db_bootstrapper;
use storage_interface::DbReaderWriter;

use serde_json::Value;
use warp::http::header::CONTENT_TYPE;

pub fn new_test_context() -> Context {
    let tmp_dir = TempPath::new();
    tmp_dir.create_as_dir().unwrap();

    let rng = rand::thread_rng();
    let builder = ValidatorBuilder::new(
        &tmp_dir,
        diem_framework_releases::current_module_blobs().to_vec(),
    );
    let (_root_keys, genesis, genesis_waypoint, _validators) = builder.build(rng).unwrap();

    let (db, db_rw) = DbReaderWriter::wrap(DiemDB::new_for_test(&tmp_dir));
    db_bootstrapper::maybe_bootstrap::<DiemVM>(&db_rw, &genesis, genesis_waypoint).unwrap();

    Context::new(ChainId::test(), db)
}

pub async fn send_request(context: Context, method: &str, path: &str, status_code: u16) -> Value {
    let routes = index::routes(context.clone());

    let resp = warp::test::request()
        .method(method)
        .path(path)
        .reply(&routes)
        .await;

    let headers = resp.headers();
    assert_eq!(headers[CONTENT_TYPE], "application/json");

    let body = serde_json::from_slice(resp.body()).expect("response body is JSON");
    assert_eq!(status_code, resp.status(), "\nresponse: {}", pretty(&body));

    if status_code < 300 {
        let ledger_info = context.get_latest_ledger_info().unwrap();
        assert_eq!(headers[X_DIEM_CHAIN_ID], "4");
        assert_eq!(
            headers[X_DIEM_LEDGER_VERSION],
            ledger_info.version().to_string()
        );
        assert_eq!(
            headers[X_DIEM_LEDGER_TIMESTAMP],
            ledger_info.timestamp().to_string()
        );
    }

    body
}

pub fn find_value(val: &Value, filter: for<'r> fn(&'r &Value) -> bool) -> Value {
    let resources = val.as_array().expect("array");
    let mut balances = resources.iter().filter(filter);
    match balances.next() {
        Some(resource) => {
            let more = balances.next();
            if let Some(val) = more {
                panic!("found multiple items by the filter: {}", pretty(val));
            }
            resource.clone()
        }
        None => {
            panic!("\ncould not find item in {}", pretty(val))
        }
    }
}

pub fn assert_json(ret: Value, expected: Value) {
    assert_eq!(
        &ret,
        &expected,
        "\nexpected: {}, \nbut got: {}",
        pretty(&expected),
        pretty(&ret)
    )
}

pub fn pretty(val: &Value) -> String {
    serde_json::to_string_pretty(val).unwrap()
}
