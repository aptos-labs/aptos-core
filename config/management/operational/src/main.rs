// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

#![forbid(unsafe_code)]
use aptos_operational_tool::command::{Command, ResultWrapper};
use std::process::exit;
use structopt::StructOpt;

#[tokio::main]
async fn main() {
    let result = Command::from_args().execute().await;

    match result {
        Ok(val) => println!("{}", val),
        Err(err) => {
            let result: ResultWrapper<()> = ResultWrapper::Error(err.to_string());
            println!("{}", serde_json::to_string_pretty(&result).unwrap());
            exit(1);
        }
    }
}
