// Copyright © Aptos Foundation

use hyper::{Body, StatusCode};
use crate::server::utils::CONTENT_TYPE_TEXT;
use std::env;
use std::process::{Command, exit};
use std::io::Write;
use std::fs::OpenOptions;
use std::process;
extern crate rstack_self;
use std::thread;
use std::time::Duration;

pub fn handle_thread_dump_request(status: &str) -> (StatusCode, Body, String) {
    let mut output = Vec::new();
    let trace = rstack_self::trace(Command::new("cargo").arg("run").arg("-p").arg("aptos-iuchild").arg("--release")).unwrap();

    for thread in trace.threads() {
        writeln!(output, "{} - {}", thread.id(), thread.name()).unwrap();
        for frame in thread.frames() {
            writeln!(output, "{:#016x}", frame.ip()).unwrap();

            for symbol in frame.symbols() {
                write!(output, "    - {}", symbol.name().unwrap_or("????")).unwrap();
                if let Some(file) = symbol.file() {
                    write!(output, " {}:{}", file.display(), symbol.line().unwrap_or(0)).unwrap();
                }
                writeln!(output).unwrap();
            }
        }
        writeln!(output).unwrap();
    }


    // Write the thread information to a file
let file_path = "thread_dump.txt";
let mut file = OpenOptions::new()
    .create(true)
    .append(true)
    .open(file_path)
    .expect("Failed to open the file");

file.write_all(&output).expect("Failed to write to the file");

    (
        StatusCode::OK,
        Body::from("555"),
        CONTENT_TYPE_TEXT.into(),
    )
}
