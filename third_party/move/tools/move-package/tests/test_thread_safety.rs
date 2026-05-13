// Parts of the file are Copyright (c) The Diem Core Contributors
// Parts of the file are Copyright (c) The Move Contributors
// Parts of the file are Copyright (c) Aptos Foundation
// All Aptos Foundation code and content is licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use move_package::BuildConfig;
use std::path::Path;

#[test]
fn cross_thread_synchronization() {
    let handle = std::thread::spawn(|| {
        BuildConfig::default()
            .compile_package(
                Path::new("./tests/thread_safety_package_test_sources/Package1"),
                &mut std::io::stdout(),
            )
            .unwrap()
    });

    BuildConfig::default()
        .compile_package(
            Path::new("./tests/thread_safety_package_test_sources/Package2"),
            &mut std::io::stdout(),
        )
        .unwrap();
    handle.join().unwrap();
}
