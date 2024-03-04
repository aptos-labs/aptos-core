
use std::fs::{self};
use std::io::{self, Read};
use indicatif::{ProgressBar, ProgressStyle};
use reqwest::Url;
use crate::config::{
    ZKEY_FILE_PATH, 
    ZKEY_FILE_URL, 
    VKEY_FILE_URL,
    VKEY_FILE_PATH,
    ZKEY_SHASUM, 
    RESOURCES_DIR
};
use std::path::Path;
use sha2::{Digest, Sha256};
use tokio::stream::*;
use tokio::io::AsyncWriteExt; // Import AsyncWriteExt for async writing
use tokio::io::AsyncWrite; // Import AsyncWriteExt for async writing

use tokio::fs::File;

use futures::Stream;
use futures_util::StreamExt;


fn compute_sha256(filename: &str) -> io::Result<String> {
    let mut file = fs::File::open(filename)?;
    let mut hasher = Sha256::new();
    let mut buffer = [0; 1024];

    loop {
        let bytes_read = file.read(&mut buffer)?;
        if bytes_read == 0 {
            break;
        }
        hasher.update(&buffer[..bytes_read]);
    }

    let hash_result = hasher.finalize();
    Ok(format!("{:x}", hash_result))
}

fn verify_checksum(filename: &str, expected_checksum: &str) -> io::Result<bool> {
    let actual_checksum = compute_sha256(filename)?;
    Ok(actual_checksum == expected_checksum)
}


// Function to download a file with a progress bar
async fn download_file_with_progress(url: &str, file_path: &str) {
    // Create a progress bar
    let pb = ProgressBar::new(1024); // Set the total size of the file in bytes (1GB = 1024 * 1024 * 1024 bytes)
    pb.set_style(ProgressStyle::default_bar()
        .template("[{elapsed_precise}] [{bar:40.cyan/blue}] {bytes}/{total_bytes} ({eta})")
        .progress_chars("#>-"));

    // Perform the request
    let client = reqwest::Client::new();
    let mut response = client.get(url).send().await.unwrap();
    let total_size = response.content_length().unwrap_or(0);

    // Prepare the file for writing
    let mut file = File::create(file_path).await.unwrap();

     // Stream and write data with progress tracking
    let mut downloaded_size = 0;
    let mut stream = response.bytes_stream();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.unwrap();
        let bytes_read = chunk.len() as u64;
        downloaded_size += bytes_read;
        file.write_all(&chunk).await.unwrap();

        // Update progress bar
        pb.set_position(downloaded_size);
    }

    // Finish progress bar
    pb.finish_with_message("Download complete.");

}

pub async fn cached_prover_key() -> &'static str {

    if ! fs::metadata(RESOURCES_DIR).is_ok() {
         match fs::create_dir(RESOURCES_DIR) {
            Ok(_) => println!("Directory '{}' created successfully.", RESOURCES_DIR),
            Err(err) => panic!("Error creating directory: {}", err),
        }       
    }


    if ! fs::metadata(ZKEY_FILE_PATH).is_ok() {
        println!("Prover key not found. Downloading now.");
        download_file_with_progress(ZKEY_FILE_URL, ZKEY_FILE_PATH).await;
    }

//    println!("Verifying prover key checksum...");
//    match verify_checksum(ZKEY_FILE_PATH, ZKEY_SHASUM) {
//        Ok(true) => println!("Checksum is valid."),
//        Ok(false) => panic!("Checksum is invalid."),
//        Err(e) => panic!("Error: {}", e),
//    }

    ZKEY_FILE_PATH
}


pub async fn cached_verification_key() -> &'static str {

    if ! fs::metadata(RESOURCES_DIR).is_ok() {
         match fs::create_dir(RESOURCES_DIR) {
            Ok(_) => println!("Directory '{}' created successfully.", RESOURCES_DIR),
            Err(err) => panic!("Error creating directory: {}", err),
        }       
    }


    if ! fs::metadata(VKEY_FILE_PATH).is_ok() {
        println!("Verification key not found. Downloading now.");
        download_file_with_progress(VKEY_FILE_URL, VKEY_FILE_PATH).await;
    }


    VKEY_FILE_PATH
}
