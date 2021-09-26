// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::{context::Context, index};
use diem_api_types::{LedgerInfo, X_DIEM_CHAIN_ID, X_DIEM_LEDGER_TIMESTAMP, X_DIEM_LEDGER_VERSION};
use diem_genesis_tool::validator_builder::ValidatorBuilder;
use diem_temppath::TempPath;
use diem_types::chain_id::ChainId;
use diem_vm::DiemVM;
use diemdb::DiemDB;
use executor::db_bootstrapper;
use storage_interface::DbReaderWriter;

use serde_json::Value;
use warp::http::header::CONTENT_TYPE;

pub fn new_test_context() -> TestContext {
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

    TestContext::new(Context::new(ChainId::test(), db))
}

pub struct TestContext {
    pub context: Context,
    pub expect_status_code: u16,
}

impl TestContext {
    pub fn new(context: Context) -> Self {
        Self {
            context,
            expect_status_code: 200,
        }
    }

    pub fn get_latest_ledger_info(&self) -> LedgerInfo {
        self.context.get_latest_ledger_info().unwrap()
    }

    pub fn expect_status_code(&self, status_code: u16) -> TestContext {
        Self {
            context: self.context.clone(),
            expect_status_code: status_code,
        }
    }

    pub async fn get(&self, path: &str) -> Value {
        self.execute(warp::test::request().method("GET").path(path))
            .await
    }

    pub async fn execute(&self, req: warp::test::RequestBuilder) -> Value {
        let routes = index::routes(self.context.clone());
        let resp = req.reply(&routes).await;

        let headers = resp.headers();
        assert_eq!(headers[CONTENT_TYPE], "application/json");

        let body = serde_json::from_slice(resp.body()).expect("response body is JSON");
        assert_eq!(
            self.expect_status_code,
            resp.status(),
            "\nresponse: {}",
            pretty(&body)
        );

        if self.expect_status_code < 300 {
            let ledger_info = self.context.get_latest_ledger_info().unwrap();
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
