// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

mod utils;

use wasm_bindgen::prelude::*;

// When the `wee_alloc` feature is enabled, use `wee_alloc` as the global
// allocator.
#[cfg(feature = "wee_alloc")]
#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

const WASM_MEMORY_BUFFER_SIZE: usize = 100;
#[no_mangle]
static mut INPUT_PARAMS: [u8; WASM_MEMORY_BUFFER_SIZE] = [0; WASM_MEMORY_BUFFER_SIZE];
#[no_mangle]
static mut OUTPUT_PARAMS: [u8; WASM_MEMORY_BUFFER_SIZE] = [0; WASM_MEMORY_BUFFER_SIZE];

#[wasm_bindgen]
pub fn entry() {
    unsafe {
        OUTPUT_PARAMS[0] = INPUT_PARAMS[0] + INPUT_PARAMS[1];
    }
}
