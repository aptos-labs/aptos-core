// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

/// Script used by the CLI e2e test `test_run_script_public_struct_arg`.
///
/// Accepts a `Point` (a public copy struct from `struct_enum_tests`) as a script argument.
/// Because `Point` is a user-defined module type, the test first compiles this script via
/// `aptos move compile-script --package-dir` (so that the struct_enum_tests dep is resolved),
/// then passes the resulting bytecode to `aptos move run-script --compiled-script-path`.
/// The struct value in the JSON args file is encoded to BCS by the CLI's StructArgParser.
script {
    use struct_enum_tests::struct_enum_tests::Point;

    fun main(_sender: &signer, p: Point) {
        assert!(p.x == 10 && p.y == 20, 100);
    }
}
