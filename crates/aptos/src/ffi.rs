// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

#![allow(unsafe_code)]

use crate::Tool;
use clap::Parser;
use std::{
    ffi::{c_char, CStr, CString},
    thread,
};
use tokio::runtime::Runtime;

/// # Safety
///
/// Run the aptos CLI synchronously
/// Note: This function should only be called from other SDK (i.g Typescript)
///
/// Return: the pointer to CLIResult c string
#[no_mangle]
pub unsafe extern "C" fn run_aptos_sync(s: *const c_char) -> *const c_char {
    let c_str = unsafe {
        assert!(!s.is_null());
        CStr::from_ptr(s)
    };

    // split string by spaces
    let input_string = c_str.to_str().unwrap().split_whitespace();

    // Create a new Tokio runtime and block on the execution of `cli.execute()`
    let result_string = Runtime::new().unwrap().block_on(async move {
        let cli = Tool::parse_from(input_string);
        cli.execute().await
    });

    let res_cstr = CString::new(result_string.unwrap()).unwrap();

    // Return a pointer to the C string
    res_cstr.into_raw()
}

/// # Safety
///
/// Run the aptos CLI async; Use this function if you are expecting the aptos CLI command
/// to run in the background, or different thread
/// Note: This function should only be called from other SDK (i.g Typescript)
///
/// Return: the pointer to c string: 'true'
#[no_mangle]
pub unsafe extern "C" fn run_aptos_async(s: *mut c_char) -> *mut c_char {
    println!("Running aptos...");
    let c_str = unsafe {
        assert!(!s.is_null());
        CStr::from_ptr(s)
    };

    // Spawn a new thread to run the CLI
    thread::spawn(move || {
        let rt = Runtime::new().unwrap();
        let input_string = c_str.to_str().unwrap().split_whitespace();
        let cli = Tool::parse_from(input_string);

        // Run the CLI once
        rt.block_on(async { cli.execute().await })
            .expect("Failed to run CLI");
    });

    // Return pointer
    CString::new("true").unwrap().into_raw()
}

/// # Safety
///
/// After running the aptos CLI using FFI. Make sure to invoke this method to free up or
/// deallocate the memory
#[no_mangle]
pub unsafe extern "C" fn free_cstring(s: *mut c_char) {
    unsafe {
        if s.is_null() {
            return;
        }
        let _ = CString::from_raw(s);
    };
}
