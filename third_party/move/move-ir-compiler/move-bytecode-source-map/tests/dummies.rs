// Parts of the file are Copyright (c) The Diem Core Contributors
// Parts of the file are Copyright (c) The Move Contributors
// Parts of the file are Copyright (c) Aptos Foundation
// All Aptos Foundation code and content is licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use move_binary_format::{binary_views::BinaryIndexedView, file_format::empty_script};
use move_bytecode_source_map::mapping::SourceMapping;
use move_ir_types::location::Spanned;

#[test]
fn test_empty_script() {
    let script = empty_script();
    let view = BinaryIndexedView::Script(&script);
    let location = Spanned::unsafe_no_loc(()).loc;
    SourceMapping::new_from_view(view, location)
        .expect("unable to build source mapping for empty script");
}
