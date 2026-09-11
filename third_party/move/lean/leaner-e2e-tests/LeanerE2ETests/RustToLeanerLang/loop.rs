// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

pub fn clear(mut flag: bool) -> bool {
    while flag {
        flag = false;
    }
    flag
}
