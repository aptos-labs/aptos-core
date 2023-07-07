// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

extern crate rstack_self;

use std::env;
use std::process::{Command, exit};
use std::io::Write;
use std::fs::File;
use std::process;
use std::thread;
use std::time::Duration;

fn main() {

    let _ = rstack_self::child();


}
