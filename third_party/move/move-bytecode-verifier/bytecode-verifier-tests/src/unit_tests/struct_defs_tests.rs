// Parts of the file are Copyright (c) The Diem Core Contributors
// Parts of the file are Copyright (c) The Move Contributors
// Parts of the file are Copyright (c) Aptos Foundation
// All Aptos Foundation code and content is licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use move_binary_format::file_format::CompiledModule;
use move_bytecode_verifier::RecursiveStructDefChecker;
use proptest::prelude::*;

proptest! {
    #[test]
    fn valid_recursive_struct_defs(module in CompiledModule::valid_strategy(20)) {
        prop_assert!(RecursiveStructDefChecker::verify_module(&module).is_ok());
    }
}
