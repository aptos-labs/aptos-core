// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

#[macro_export]
macro_rules! time_fn {
    ($func:expr, $($arg:expr), *) => {{
        // start timer
        let start = tokio::time::Instant::now();

        // call the flow
        let result = $func($($arg),+).await;

        // end timer
        let time = (tokio::time::Instant::now() - start).as_micros() as f64;

        // return
        (result, time)
    }};
}
