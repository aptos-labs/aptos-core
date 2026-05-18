// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::tests::common;

#[test]
fn build_publish_payload_success() {
    let pkg = common::make_package("publish_me", &[(
        "publish_me",
        "module 0xCAFE::publish_me {
    public fun hello(): u64 { 42 }
}",
    )]);
    let dir = pkg.path().to_str().unwrap();
    let output_file = pkg.path().join("payload.json");
    let output_file_str = output_file.to_str().unwrap();

    let output = common::run_cli(&[
        "build-publish-payload",
        "--package-dir",
        dir,
        "--skip-fetch-latest-git-deps",
        "--json-output-file",
        output_file_str,
        "--assume-yes",
    ]);
    assert!(
        output.result.is_ok(),
        "build-publish-payload failed: {:?}",
        output.result
    );

    // Verify the JSON payload file was written and is valid JSON
    assert!(output_file.exists(), "payload.json should exist");
    let content = std::fs::read_to_string(&output_file).unwrap();
    let parsed: serde_json::Value =
        serde_json::from_str(&content).expect("payload.json should be valid JSON");
    assert!(
        parsed.get("function_id").is_some(),
        "payload should contain function_id"
    );
}
