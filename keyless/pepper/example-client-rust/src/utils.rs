// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    DEFAULT_CLIENT_TIMEOUT_SECS, DEFAULT_JWT, DEFAULT_PEPPER_SERVICE_URL, ENV_PEPPER_SERVICE_URL,
};
use aptos_types::keyless::Configuration;
use reqwest::Client;
use serde::Serialize;
use std::{fs, io::stdin, time::Duration};

/// Creates and returns a new reqwest client
pub fn create_request_client() -> Client {
    Client::builder()
        .timeout(Duration::from_secs(DEFAULT_CLIENT_TIMEOUT_SECS))
        .build()
        .unwrap()
}

/// Fetches the JWT from the user, or uses a test token if no input is provided
pub fn get_jwt() -> String {
    // Prompt the user for the token or file path
    print(
        "Enter the JWT, or a text file path that contains the JWT. If nothing is entered, \
    a test token will be used.",
        true,
    );
    let user_input = read_input_from_stdin().trim().to_string();

    // Fetch the JWT or use the test token
    if !user_input.is_empty() {
        match fs::read_to_string(user_input.clone()) {
            Ok(jwt) => {
                print("Read the JWT from the file!", false);
                jwt.trim().to_string()
            },
            Err(_) => {
                print("Using the input as the JWT itself!", false);
                user_input
            },
        }
    } else {
        print("Using the test JWT token!", false);
        DEFAULT_JWT.to_string()
    }
}

/// Returns a keyless configuration (in this case, for devnet)
pub fn get_keyless_configuration() -> Configuration {
    Configuration::new_for_devnet()
}

/// Fetches the pepper service URL from the environment variable or prompts the user for input
pub fn get_pepper_service_url() -> String {
    // Fetch the URL from the environment variable (if it exists)
    if let Ok(pepper_service_url) = std::env::var(ENV_PEPPER_SERVICE_URL) {
        print(
            &format!(
                "Pepper service url found from environment variable ({}): {}",
                ENV_PEPPER_SERVICE_URL, pepper_service_url
            ),
            false,
        );
        return pepper_service_url;
    }

    // Otherwise, prompt the user for input
    print(
        &format!(
            "Pepper service url not found from environment variable ({}). Enter the URL of the \
        pepper service deployment. If nothing is entered, the default will be used ({}).",
            ENV_PEPPER_SERVICE_URL, DEFAULT_PEPPER_SERVICE_URL
        ),
        false,
    );
    let user_input = read_input_from_stdin();
    let pepper_service_url = if user_input.is_empty() {
        DEFAULT_PEPPER_SERVICE_URL.to_string()
    } else {
        user_input
    };

    print(
        &format!("Using pepper service url: {}", pepper_service_url),
        false,
    );
    pepper_service_url
}

/// Prints the given string. If `newline_header` is true, adds an empty line before the string.
pub fn print(string: &str, newline_header: bool) {
    if newline_header {
        println!();
    }
    println!("{}", string);
}

/// Reads a line from stdin and returns it as string
pub fn read_input_from_stdin() -> String {
    let mut line = String::new();
    stdin().read_line(&mut line).unwrap();
    line.trim().to_string()
}

/// Serializes a value to a pretty JSON string
pub fn to_string_pretty<T: ?Sized + Serialize>(value: &T) -> String {
    serde_json::to_string_pretty(value).unwrap()
}
