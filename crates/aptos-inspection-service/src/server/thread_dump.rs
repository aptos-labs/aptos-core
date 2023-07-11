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
use std::fs::File;


pub fn handle_thread_dump_request() -> (StatusCode, Body, String) {


    println!("







    sdfsdfsdf









    ");

    let trace = rstack_self::trace(Command::new("cargo").arg("run").arg("-p").arg("aptos-iuchild").arg("--release")).unwrap();

    println!("







    hahahahah









    ");

    // Open a file for writing
    let mut file = File::create("./thread_dump.txt").unwrap();

    // Write the trace information to the file
    write!(file, "{:#?}", trace).unwrap();

    println!("







    uyuyuyuyuyuy









    ");


    (
        StatusCode::OK,
        Body::from("555"),
        CONTENT_TYPE_TEXT.into(),
    )
}
