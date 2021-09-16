// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::{context::Context, filters};

use diem_genesis_tool::validator_builder::ValidatorBuilder;
use diem_temppath::TempPath;
use diem_types::chain_id::ChainId;
use diem_vm::DiemVM;
use diemdb::DiemDB;
use executor::db_bootstrapper;
use storage_interface::DbReaderWriter;

use serde_json::{json, Value};

#[tokio::test]
async fn test_get_ledger_info() {
    let context = new_test_context();
    let filter = filters::routes(context.clone());

    let resp = warp::test::request()
        .method("GET")
        .path("/")
        .reply(&filter)
        .await;
    assert_eq!(resp.status(), 200);

    let ledger_info = context.get_latest_ledger_info().unwrap();
    let ledger_version = ledger_info.ledger_info().version();
    let ledger_timestamp = ledger_info.ledger_info().timestamp_usecs();

    assert_eq!(
        serde_json::from_slice::<Value>(resp.body()).unwrap(),
        json!({
            "chain_id": 4,
            "ledger_version": ledger_version.to_string(),
            "ledger_timestamp": ledger_timestamp.to_string(),
        })
    );
}

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
