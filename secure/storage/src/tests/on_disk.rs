// Copyright © Velor Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::{tests::suite, OnDiskStorage, Storage};
use velor_temppath::TempPath;

#[test]
fn on_disk() {
    let path_buf = TempPath::new().path().to_path_buf();
    let mut storage = Storage::from(OnDiskStorage::new(path_buf));
    suite::execute_all_storage_tests(&mut storage);
}
