// Parts of the file are Copyright (c) The Diem Core Contributors
// Parts of the file are Copyright (c) The Move Contributors
// Parts of the file are Copyright (c) Aptos Foundation
// All Aptos Foundation code and content is licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use move_package::BuildConfig;
use std::{io::Write, path::Path};
use tempfile::tempdir;

#[test]
fn package_hash_skips_non_move_files() {
    let path = Path::new("tests/test_sources/resolution/dep_good_digest");

    // resolution graph diagnostics are only needed for CLI commands so ignore them in both cases by
    // passing a vector as the writer

    let pkg1 = BuildConfig {
        install_dir: Some(tempdir().unwrap().path().to_path_buf()),
        ..Default::default()
    }
    .resolution_graph_for_package(path, &mut Vec::new())
    .unwrap();

    let dummy_path = path.join("deps_only/other_dep/sources/dummy_text");
    std::fs::File::create(&dummy_path)
        .unwrap()
        .write_all("hello".as_bytes())
        .unwrap();

    let pkg2 = BuildConfig {
        install_dir: Some(tempdir().unwrap().path().to_path_buf()),
        ..Default::default()
    }
    .resolution_graph_for_package(path, &mut Vec::new())
    .unwrap();

    std::fs::remove_file(&dummy_path).unwrap();
    for (pkg, res_pkg) in pkg1.package_table {
        let other_res_pkg = pkg2.get_package(&pkg);
        assert_eq!(
            res_pkg.source_digest, other_res_pkg.source_digest,
            "source digests differ for {}",
            pkg
        )
    }
}
