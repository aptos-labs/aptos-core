// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::{context::Context, index};
use diem_genesis_tool::validator_builder::ValidatorBuilder;
use diem_temppath::TempPath;
use diem_types::chain_id::ChainId;
use diem_vm::DiemVM;
use diemdb::DiemDB;
use executor::db_bootstrapper;
use storage_interface::DbReaderWriter;

use serde_json::Value;

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
    let routes = index::routes(context);

    let resp = warp::test::request()
        .method(method)
        .path(path)
        .reply(&routes)
        .await;
    let body = serde_json::from_slice(resp.body()).expect("response body is JSON");
    assert_eq!(status_code, resp.status(), "\nresponse: {}", pretty(&body));
    body
}

pub fn pretty(val: &Value) -> String {
    serde_json::to_string_pretty(val).unwrap()
}
