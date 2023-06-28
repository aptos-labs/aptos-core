// Copyright (c) The Diem Core Contributors
// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

//use move_ir_to_bytecode::parser::parse_module_llvm;
/*
#[cfg(test)]
mod tests {
    use std::{path::PathBuf};
    #[test]
    fn test_empty_module() {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let source_path = root.join("tests/testdata/empty-module.mvir");
        let source = std::fs::read_to_string(source_path).unwrap();
        use inkwell::context::Context;
        let llvm_context = Context::create();
        let parsed_module = move_ir_to_bytecode::parser::parse_module_llvm(&source, &llvm_context).unwrap();
        assert_eq!(parsed_module.get_functions().count(), 0);
        assert_eq!(parsed_module.get_first_global(), None);
        assert_eq!(parsed_module.get_name().to_str().unwrap(), "00000000000000000000000000000001.TestBinaryOps");
        assert_eq!(parsed_module.get_source_file_name().to_str().unwrap(), "00000000000000000000000000000001.TestBinaryOps");
    }
    #[test]
    fn test_friend() {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let source_path = root.join("tests/testdata/friend.mvir");
        let source = std::fs::read_to_string(source_path).unwrap();
        use inkwell::context::Context;
        let llvm_context = Context::create();
        let parsed_module = move_ir_to_bytecode::parser::parse_module_llvm(&source, &llvm_context).unwrap();
        assert_eq!(parsed_module.get_functions().count(), 0);
        assert_eq!(parsed_module.get_first_global(), None);
        assert_eq!(parsed_module.get_name().to_str().unwrap(), "00000000000000000000000000000042.A");
        assert_eq!(parsed_module.get_source_file_name().to_str().unwrap(), "00000000000000000000000000000042.A");
    }
    #[test]
    fn test_struct() {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let source_path = root.join("tests/testdata/struct.mvir");
        let source = std::fs::read_to_string(source_path).unwrap();
        use inkwell::context::Context;
        let llvm_context = Context::create();
        let parsed_module = move_ir_to_bytecode::parser::parse_module_llvm(&source, &llvm_context).unwrap();
        assert_eq!(parsed_module.get_functions().count(), 0);
        assert_eq!(parsed_module.get_first_global(), None); // FIXME: Should be 1
        assert_eq!(parsed_module.get_name().to_str().unwrap(), "00000000000000000000000000000001.TestBinaryOps");
        assert_eq!(parsed_module.get_source_file_name().to_str().unwrap(), "00000000000000000000000000000001.TestBinaryOps");
    }
}*/
