// Parts of the file are Copyright (c) The Diem Core Contributors
// Parts of the file are Copyright (c) The Move Contributors
// Parts of the file are Copyright (c) Aptos Foundation
// All Aptos Foundation code and content is licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

pub mod binary_samples;
pub mod bounds_tests;
pub mod catch_unwind;
pub mod code_unit_tests;
pub mod constants_tests;
pub mod control_flow_tests;
pub mod dependencies_tests;
pub mod duplication_tests;
pub mod generic_ops_tests;
pub mod large_type_test;
pub mod limit_tests;
pub mod locals;
pub mod loop_summary_tests;
pub mod many_back_edges;
pub mod multi_pass_tests;
pub mod negative_stack_size_tests;
pub mod reference_safety_tests;
pub mod signature_tests;
pub mod struct_api_version_guard_test;
pub mod struct_defs_tests;
pub mod variant_name_test;
pub mod vec_pack_tests;

// TODO(#13806): add unit tests for enum verification
// TODO(#15664): add unit tests for closure verification
