// Copyright © Aptos Foundation

use crate::server::utils::CONTENT_TYPE_TEXT;
use hyper::{Body, StatusCode};
use std::{
    env,
    fs::OpenOptions,
    io::Write,
    process,
    process::{exit, Command},
};
extern crate rstack_self;
use std::{fs::File, path::Path, thread, time::Duration};

pub fn handle_thread_dump_request() -> (StatusCode, Body, String) {
    let trace = rstack_self::trace(
        Command::new("cargo")
            .arg("run")
            .arg("-p")
            .arg("aptos-iuchild")
            .arg("--release"),
    )
    .unwrap();

    // Open a file for writing
    let mut file = File::create("./thread_dump.txt").unwrap();

    // Write the trace information to the file
    write!(file, "{:#?}", trace).unwrap();

    (StatusCode::OK, Body::from("555"), CONTENT_TYPE_TEXT.into())
}
