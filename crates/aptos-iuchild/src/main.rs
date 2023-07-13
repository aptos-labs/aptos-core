// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

extern crate rstack_self;

use std::{
    env,
    fs::File,
    io::Write,
    process,
    process::{exit, Command},
    thread,
    time::Duration,
};

fn main() {
    let _ = rstack_self::child();
}
