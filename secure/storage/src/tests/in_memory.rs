// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Innovation-Enabling Source Code License

use crate::{tests::suite, InMemoryStorage, Storage};

#[test]
fn in_memory() {
    let mut storage = Storage::from(InMemoryStorage::new());
    suite::execute_all_storage_tests(&mut storage);
}
