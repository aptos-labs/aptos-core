// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

mod menu;

use crate::menu::create_wallet_menu;

fn main() {
    let wallet_menu = create_wallet_menu();
    tauri::Builder::default()
        .menu(wallet_menu)
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
