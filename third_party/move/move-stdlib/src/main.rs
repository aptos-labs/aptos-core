// Parts of the file are Copyright (c) The Diem Core Contributors
// Parts of the file are Copyright (c) The Move Contributors
// Parts of the file are Copyright (c) Aptos Foundation
// All Aptos Foundation code and content is licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use move_stdlib::utils::time_it;

fn main() {
    // Generate documentation
    {
        time_it("Generating stdlib documentation", || {
            std::fs::remove_dir_all(move_stdlib::move_stdlib_docs_full_path()).unwrap_or(());
            //std::fs::create_dir_all(&move_stdlib::move_stdlib_docs_full_path()).unwrap();
            move_stdlib::build_stdlib_doc(&move_stdlib::move_stdlib_docs_full_path());
        });

        time_it("Generating nursery documentation", || {
            std::fs::remove_dir_all(move_stdlib::move_nursery_docs_full_path()).unwrap_or(());
            move_stdlib::build_nursery_doc(&move_stdlib::move_nursery_docs_full_path());
        });
    }
}
