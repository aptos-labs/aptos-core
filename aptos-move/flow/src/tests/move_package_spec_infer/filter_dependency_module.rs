// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::tests::common;
use aptos_package_builder::PackageBuilder;

#[tokio::test]
async fn move_package_spec_infer_filter_dependency_module() {
    let mut dep_builder = PackageBuilder::new("DepPkg");
    dep_builder.add_alias("dep_addr", "0xD");
    dep_builder.add_source(
        "dep_mod",
        "module dep_addr::dep_mod {
    public fun dep_fun() {}
}",
    );
    let dep_pkg = dep_builder
        .write_to_temp()
        .expect("failed to create dependency package");

    let mut root_builder = PackageBuilder::new("RootPkg");
    root_builder.add_alias("root_addr", "0xCAFE");
    root_builder.add_local_dep(
        "DepPkg",
        dep_pkg
            .path()
            .to_str()
            .expect("dependency path must be utf-8"),
    );
    root_builder.add_source(
        "root_mod",
        "module root_addr::root_mod {
    public fun root_fun() {}
}",
    );
    let root_pkg = root_builder
        .write_to_temp()
        .expect("failed to create root package");

    let dir = root_pkg.path().to_str().unwrap();
    let client = common::make_client().await;
    let result = common::call_tool_raw(
        &client,
        "move_package_spec_infer",
        serde_json::json!({ "package_path": dir, "filter": "dep_mod" }),
    )
    .await;
    let formatted = match result {
        Ok(tool_result) => common::format_tool_result(&tool_result),
        Err(err) => common::format_service_error(&err),
    };
    common::check_baseline(file!(), &formatted);
}
