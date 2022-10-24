// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use framework::{BuiltPackage, ReleaseOptions, ReleaseTarget};
use tempfile::tempdir;

/// Test which ensures that generated artifacts (sdks, documentation, etc) are up-to-date.
/// Those artifacts are updated as a side-effect when `cached-packages` is build. However,
/// a user may have not needed to trigger building this crate. This test forces them to do so.
#[test]
fn artifacts_are_updated() {
    let temp_dir = tempdir().unwrap();
    let ReleaseOptions {
        build_options,
        packages,
        ..
    } = ReleaseTarget::Head.create_release_options(false, None);
    for package_path in packages {
        // Copy the package content over to tempdir.
        let copied_path = temp_dir.path().join(package_path.file_name().unwrap());
        fs_extra::dir::copy(
            &package_path,
            &copied_path,
            &fs_extra::dir::CopyOptions {
                copy_inside: true,
                ..Default::default()
            },
        )
        .unwrap();

        // Build the copy of the package and compare the generated documentation. Because the
        // documentation reflects the source, it is the strictest indicator whether
        // `cached_packages` had been build and therefore things are up-to-date.
        BuiltPackage::build(copied_path.clone(), build_options.clone()).unwrap();
        assert!(
            !dir_diff::is_different(package_path.join("doc"), copied_path.join("doc")).unwrap(),
            "Artifacts generated from Move code are not up-to-date (sdk, docs, ...). Those \
artifacts are automatically updated if you run a build step depending on them. To force a \
build step, run `cargo build -p cached-packages`.
"
        )
    }
}
