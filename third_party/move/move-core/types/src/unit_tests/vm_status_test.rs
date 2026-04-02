// Parts of the file are Copyright (c) The Diem Core Contributors
// Parts of the file are Copyright (c) The Move Contributors
// Parts of the file are Copyright (c) Aptos Foundation
// All Aptos Foundation code and content is licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::vm_status::{StatusCode, VMStatus};

#[test]
fn test_stats_code() {
    let vm_status = VMStatus::error(StatusCode::OUT_OF_GAS, None);
    let code = vm_status.status_code();
    assert_eq!(code, StatusCode::OUT_OF_GAS);
}
