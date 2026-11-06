use codespan_reporting::term::termcolor::Buffer;
use legacy_move_compiler::{
    shared::{known_attributes::KnownAttribute, NumericalAddress, PackagePaths},
    Flags,
};
use move_model::{
    options::ModelBuilderOptions, run_model_builder_with_options_and_compilation_flags,
};
use std::{collections::BTreeMap, path::Path};

#[test]
fn apply_schema_does_not_match_test_only_functions() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("sources")
        .join("apply_schema_test_only_ok.move");
    let std_address = NumericalAddress::parse_str("0x1").unwrap();
    let targets = vec![PackagePaths {
        name: None,
        paths: vec![path.to_str().unwrap().to_string()],
        named_address_map: BTreeMap::from([("std".to_owned(), std_address)]),
    }];
    let unit_test_path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("move-stdlib")
        .join("sources")
        .join("unit_test.move");
    let test_dependencies = vec![PackagePaths {
        name: None,
        paths: vec![unit_test_path.to_str().unwrap().to_string()],
        named_address_map: BTreeMap::from([("std".to_owned(), std_address)]),
    }];
    let env = run_model_builder_with_options_and_compilation_flags(
        targets,
        test_dependencies,
        vec![],
        ModelBuilderOptions::default(),
        Flags::testing(),
        KnownAttribute::get_all_attribute_names(),
    )
    .unwrap();
    let is_test_only = |expected_name: &str| {
        env.get_modules()
            .find_map(|module| {
                module.get_functions().find_map(|function| {
                    (function.symbol_pool().string(function.get_name()).as_str() == expected_name)
                        .then(|| function.is_test_only())
                })
            })
            .unwrap()
    };
    assert!(is_test_only("test_binds_no_self"));
    assert!(is_test_only("helper_binds_no_self"));
    let mut writer = Buffer::no_color();
    env.report_diag(&mut writer, codespan_reporting::diagnostic::Severity::Error);
    let diagnostics = String::from_utf8_lossy(&writer.into_inner()).to_string();
    assert_eq!(
        env.diag_count(codespan_reporting::diagnostic::Severity::Error),
        0,
        "{diagnostics}"
    );
}
