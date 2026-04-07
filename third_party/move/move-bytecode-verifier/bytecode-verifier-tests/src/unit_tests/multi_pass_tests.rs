// Parts of the file are Copyright (c) The Diem Core Contributors
// Parts of the file are Copyright (c) The Move Contributors
// Parts of the file are Copyright (c) Aptos Foundation
// All Aptos Foundation code and content is licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use move_binary_format::CompiledModule;
use move_bytecode_verifier::{
    constants, instantiation_loops::InstantiationLoopChecker, DuplicationChecker,
    InstructionConsistency, RecursiveStructDefChecker,
};
use proptest::prelude::*;

proptest! {
    #[test]
    fn check_verifier_passes(module in CompiledModule::valid_strategy(20)) {
        DuplicationChecker::verify_module(&module).expect("DuplicationChecker failure");
        InstructionConsistency::verify_module(&module).expect("InstructionConsistency failure");
        constants::verify_module(&module).expect("constants failure");
        RecursiveStructDefChecker::verify_module(&module).expect("RecursiveStructDefChecker failure");
        InstantiationLoopChecker::verify_module(&module).expect("InstantiationLoopChecker failure");
    }
}
