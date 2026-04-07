// Parts of the file are Copyright (c) The Diem Core Contributors
// Parts of the file are Copyright (c) The Move Contributors
// Parts of the file are Copyright (c) Aptos Foundation
// All Aptos Foundation code and content is licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use move_binary_format::file_format::CompiledModule;
use proptest::prelude::*;

proptest! {
    #[test]
    fn identifier_serializer_roundtrip(module in CompiledModule::valid_strategy(20)) {
        let module_id = module.self_id();
        let deserialized_module_id = {
            let serialized_key = bcs::to_bytes(&module_id).unwrap();
            bcs::from_bytes(&serialized_key).expect("Deserialize should work")
        };
        prop_assert_eq!(module_id, deserialized_module_id);
    }
}
