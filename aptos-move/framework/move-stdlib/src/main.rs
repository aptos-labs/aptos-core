// Copyright (c) The Diem Core Contributors
// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use aptos_move_stdlib::utils::time_it;

fn main() {
    // Generate documentation
    {
        time_it("Generating stdlib documentation", || {
            std::fs::remove_dir_all(aptos_move_stdlib::move_stdlib_docs_full_path()).unwrap_or(());
            //std::fs::create_dir_all(&aptos_move_stdlib::move_stdlib_docs_full_path()).unwrap();
            aptos_move_stdlib::build_stdlib_doc(&aptos_move_stdlib::move_stdlib_docs_full_path());
        });

        time_it("Generating nursery documentation", || {
            std::fs::remove_dir_all(aptos_move_stdlib::move_nursery_docs_full_path()).unwrap_or(());
            aptos_move_stdlib::build_nursery_doc(&aptos_move_stdlib::move_nursery_docs_full_path());
        });

        time_it("Generating error explanations", || {
            std::fs::remove_file(aptos_move_stdlib::move_stdlib_errmap_full_path()).unwrap_or(());
            aptos_move_stdlib::build_error_code_map(&aptos_move_stdlib::move_stdlib_errmap_full_path());
        });
    }
}
