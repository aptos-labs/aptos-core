// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

pub fn choose(flag: bool, when_true: u32, when_false: u32) -> u32 {
    if flag { when_true } else { when_false }
}
