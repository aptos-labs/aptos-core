// Copyright © Aptos Foundation

use std::process::Command;
use std::fs;
use anyhow::{Result, bail};





#[cfg(target_arch = "arm")]
pub fn witness_gen(body: &str) -> Result<()> {

    fs::write("/tmp/rapidsnark_input.json", body.as_bytes())?;

    let output = Command::new("node")
            .args(&[
                  "/usr/local/share/aptos-prover-service/generate_witness.js",
                  "/usr/local/share/aptos-prover-service/main.wasm",
                  "/tmp/rapidsnark_input.json",
                  "/tmp/rapidsnark_witness.wtns",
            ])
            .output()
            .expect("Failed to execute command");

    // Check if the command executed successfully
    if output.status.success() {
        // Convert the output bytes to a string
        let stdout = String::from_utf8_lossy(&output.stdout);

        // Print the output
        println!("Command output:\n{}", stdout);
        Ok(())
    } else {
        // Print the error message if the command failed
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("Command failed:\n{}", stderr);
    }

}

#[cfg(target_arch = "x86_64")]
pub fn witness_gen(body: &str) -> Result<()> {


    fs::write("/tmp/rapidsnark_input.json", body.as_bytes())?;

    let output = Command::new("/usr/local/share/aptos-prover-service/main_c")
            .args(&["/tmp/rapidsnark_input.json", "/tmp/rapidsnark_witness.wtns"]) // Example arguments
            .output()
            .expect("Failed to execute command");

    // Check if the command executed successfully
    if output.status.success() {
        // Convert the output bytes to a string
        let stdout = String::from_utf8_lossy(&output.stdout);

        // Print the output
        println!("Command output:\n{}", stdout);
        Ok(())
    } else {
        // Print the error message if the command failed
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("Command failed:\n{}", stderr);
    }
}
