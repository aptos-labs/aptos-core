// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use velor_keyless_pepper_example_client_rust::run_client_example;
use clap::Parser;

#[derive(Parser, Debug)]
#[command(author, version, about)]
struct Args {
    /// The URL of the Pepper Service
    #[arg(long, default_value = "http://localhost:8000")]
    pepper_service_url: String,

    /// The Google project ID used by Firestore
    #[arg(long)]
    firestore_google_project_id: String,

    /// The database ID used by Firestore
    #[arg(long, default_value = "(default)")]
    firestore_database_id: String,
}

#[tokio::main]
async fn main() {
    // Fetch the command line arguments
    let args = Args::parse();

    // Run the client example
    run_client_example(
        args.pepper_service_url,
        args.firestore_google_project_id,
        args.firestore_database_id,
    )
    .await;
}
