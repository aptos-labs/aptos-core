// Parts of the file are Copyright (c) The Diem Core Contributors
// Parts of the file are Copyright (c) The Move Contributors
// Parts of the file are Copyright (c) Aptos Foundation
// All Aptos Foundation code and content is licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use std::{io::Write, time::Instant};

pub fn time_it<F, R>(msg: &str, f: F) -> R
where
    F: FnOnce() -> R,
{
    let now = Instant::now();
    print!("{} ... ", msg);
    let _ = std::io::stdout().flush();
    let res = f();
    println!("(took {:.3}s)", now.elapsed().as_secs_f64());
    res
}
