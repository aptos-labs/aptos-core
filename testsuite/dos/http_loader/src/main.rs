// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use core::time::Duration;
use reqwest;
use tokio::io;

#[tokio::main]
async fn main() -> tokio::io::Result<()> {
    for _ in 0..10 {
        tokio::spawn(async {
            let rep = reqwest::get("http://localhost:8080")
                .await
                .expect("This should work: is the server up?")
                .text()
                .await
                .expect("Well now...");
            println!("{rep}");
        });
    }
    tokio::time::sleep(Duration::from_secs(1)).await;
    Ok::<_, io::Error>(())
}
